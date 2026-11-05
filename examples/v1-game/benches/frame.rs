//! Integrated 10k simulation, extraction, upload, encode and GPU completion.
//! Deliberately serial and unpaced; the CLI separately exercises 60 Hz pacing.
#![cfg(not(target_arch = "wasm32"))]

use std::{num::NonZeroUsize, time::Duration};

use thrum_core::{StableId, sim::ExecutionMode};
use thrum_render::{present::SurfaceProfile, timing::GpuTimings};
use v1_game::{
    FrameHost, SurfaceOutput,
    simulation::{Command, ComponentEntity, Position, Velocity},
    simulation_with_count,
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench(args = [10_000, 100_000])]
fn game_tick_and_publication(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    for _ in 0..60 {
        sim.step(&[]).expect("warmup publication succeeds");
    }
    bencher.bench_local(move || {
        let result = sim.step(&[]).expect("one game tick commits");
        divan::black_box(result.tick);
        divan::black_box(result.simulated_entities);
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn game_tick_and_publication_rayon(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    sim.set_execution_mode(ExecutionMode::Rayon);
    for _ in 0..60 {
        sim.step(&[]).expect("warmup publication succeeds");
    }
    bencher.bench_local(move || {
        let result = sim.step(&[]).expect("one parallel candidate commits");
        divan::black_box(result.tick);
        divan::black_box(result.simulated_entities);
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn game_tick_with_rolling_capture(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    let mut history = sim
        .begin_history(NonZeroUsize::new(64).expect("window"))
        .expect("committed endpoint");
    for _ in 0..65 {
        history.step(&mut sim, &[]).expect("warmup tick");
    }
    bencher.bench_local(move || {
        history.step(&mut sim, &[]).expect("one recorded tick");
        divan::black_box(history.latest_tick());
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn game_tick_with_rolling_capture_and_churn(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("committed endpoint");
    let mut id = StableId((entity_count - 5) as u128);
    let mut next = StableId(entity_count as u128 + 1);
    let mut handle = sim.handle(id).expect("static churn target");
    let mut advance = || {
        let replacement = ComponentEntity {
            id: next,
            position: Position::default(),
            velocity: Velocity::default(),
            health: 100,
            ownership: None,
            visual: Some(1),
        };
        history
            .step(
                &mut sim,
                &[Command::Despawn(handle), Command::Spawn(replacement)],
            )
            .expect("recorded churn tick");
        id = next;
        next = StableId(next.0 + 1);
        handle = sim.handle(id).expect("replacement handle");
        history.latest_tick()
    };
    for _ in 0..100 {
        advance();
    }
    bencher.bench_local(move || divan::black_box(advance()));
}

#[divan::bench(args = [10_000, 100_000])]
fn game_rewind_and_replay_rolling_window(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    let mut history = sim
        .begin_history(NonZeroUsize::new(64).expect("window"))
        .expect("committed endpoint");
    for _ in 0..64 {
        history.step(&mut sim, &[]).expect("recorded tick");
    }
    bencher.bench_local(move || {
        let target = history.latest_tick() - 8;
        history
            .restore_to(&mut sim, target)
            .expect("eight retained ticks");
        for _ in 0..8 {
            history.step(&mut sim, &[]).expect("replay tick");
        }
        divan::black_box(sim.tick());
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn game_capture_committed(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    sim.step(&[]).expect("committed baseline");
    bencher.bench_local(move || {
        divan::black_box(sim.capture());
    });
}

#[divan::bench(args = [10_000, 100_000])]
fn game_restore_capture(bencher: divan::Bencher<'_, '_>, entity_count: usize) {
    let mut sim = simulation_with_count(entity_count).expect("V1 game fixture seeds");
    sim.step(&[]).expect("committed baseline");
    let saved = sim.capture();
    bencher.bench_local(move || {
        sim.restore(&saved).expect("same runtime capture");
        divan::black_box(sim.tick());
    });
}

fn measure(bencher: divan::Bencher<'_, '_>, gpu_timestamps: bool, present_surface: bool) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("real native adapter required for this explicit benchmark");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: if gpu_timestamps {
            GpuTimings::requested_features(&adapter)
        } else {
            wgpu::Features::empty()
        },
        ..Default::default()
    }))
    .expect("device request succeeds");
    let surface = present_surface.then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("V1 benchmark presentation target"),
            size: wgpu::Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    });
    let surface_view = surface
        .as_ref()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let profile = present_surface.then_some(SurfaceProfile {
        format: wgpu::TextureFormat::Bgra8Unorm,
        extent: [1280, 720],
    });
    let mut host = pollster::block_on(FrameHost::new(&device, &queue, profile))
        .expect("the prepared V1 workload must fit the adapter");
    let device_ref = &device;
    let mut now = Duration::ZERO;
    bencher.bench_local(move || {
        now += Duration::from_nanos(16_666_667);
        let output = surface_view.as_ref().map(|view| SurfaceOutput {
            view,
            extent: [1280, 720],
        });
        let frame = host
            .drive(now, output.as_ref())
            .expect("the measured fixed-clock host must keep progressing");
        assert!(frame.submitted, "one update gets a free view slot");
        assert_eq!(frame.surface_presented, present_surface);
        let complete = loop {
            if let Some(sample) = host.poll_completed().expect("GPU progress") {
                break sample;
            }
            device_ref
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .expect("GPU completion");
        };
        divan::black_box(complete);
    });
}

#[divan::bench]
fn assembled_frame_with_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(bencher, true, false);
}

#[divan::bench]
fn assembled_frame_without_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(bencher, false, false);
}

#[divan::bench]
fn assembled_frame_with_surface_pass(bencher: divan::Bencher<'_, '_>) {
    measure(bencher, true, true);
}
