//! Integrated 10k simulation, extraction, upload, encode and GPU completion.
//! Divan rows are serial and unpaced; the explicit threaded batch follows 120 Hz.
#![cfg(not(target_arch = "wasm32"))]

use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use thrum_core::{StableId, sim::ExecutionMode};
use thrum_render::{capabilities::DeviceCapabilities, present::SurfaceProfile};
use v1_game::{
    FrameConfig, FrameHost, FrameSample, SurfaceOutput, ThreadedFrameHost,
    simulation::{Command, ComponentEntity, Position, Velocity},
    simulation_with_count,
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameMode {
    Inline,
    Threaded,
}

fn main() {
    if std::env::args().any(|argument| argument == "--g0-inventory") {
        print_g0_inventory();
        return;
    }
    if std::env::args().any(|argument| argument == "--g3b-frame-batch") {
        measure(
            None,
            true,
            [1920, 1080],
            Some([1920, 1080]),
            true,
            Duration::from_nanos(16_666_667),
            false,
            200,
            FrameMode::Inline,
        );
        return;
    }
    if std::env::args().any(|argument| argument == "--g3b-threaded-batch") {
        measure(
            None,
            true,
            [1920, 1080],
            Some([1920, 1080]),
            false,
            Duration::from_nanos(8_333_333),
            true,
            200,
            FrameMode::Threaded,
        );
        return;
    }
    divan::main();
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit benchmark inventory reports fixed rows and missing consumers"
)]
fn print_g0_inventory() {
    const ROWS: &[(&str, &str)] = &[
        ("S1 tick+publication 10k/100k", "measured:g2r --g0-s1"),
        (
            "S1 added command/damage sensitivity",
            "measured:g2r --g0-s1-paired",
        ),
        ("S1 warmed 200-spawn burst", "measured:g2r --g0-burst"),
        (
            "S2 correction timing and allocations",
            "measured_old_path:g2r --g0-s2",
        ),
        (
            "S2 retained-record amendment and complete-state equality",
            "unsupported:amendment and complete-state oracle absent",
        ),
        (
            "G3b 10k/1080p frame CPU",
            "measured_offscreen:frame assembled_frame_1080p_late_surface",
        ),
        (
            "G3b 120 Hz callback requests",
            "measured_offscreen:frame g0_120hz_callbacks_at_1080p",
        ),
        (
            "G3b real 120 Hz present interval over ten minutes",
            "unsupported:display link and presentation timestamps absent",
        ),
        (
            "G3b iPhone 18 Pro simulator display-link function",
            "unsupported:simulator launch and configuration script absent",
        ),
        (
            "G3b input-to-photon latency",
            "unsupported:presentation timestamp and input correlation absent",
        ),
        (
            "G3b per-thread zero allocation",
            "unsupported:threaded runtime present but per-thread counters absent",
        ),
        (
            "G3b surface/background/device-loss recovery",
            "unsupported:unified lifecycle consumer absent",
        ),
        (
            "G3t 1080p GPU pass sum",
            "measured_offscreen:frame assembled_frame_1080p_late_surface",
        ),
        (
            "G3t screen attachment traffic with Metal counters",
            "unsupported:Metal counter capture absent",
        ),
        (
            "G3t present-inclusive GPU frame",
            "unsupported:real drawable presentation absent",
        ),
        (
            "G3t steady write_buffer count",
            "unsupported:upload instrumentation absent",
        ),
        (
            "G3t frame CPU 4-to-64 lights",
            "unsupported:game frame has one diagnostic sun",
        ),
        (
            "G3t mirrored-row GPU sensitivity",
            "unsupported:game frame has no mirrored-row control",
        ),
        (
            "G3c GPU time versus visibility",
            "unsupported:GPU culling pass absent",
        ),
        (
            "G3c encode CPU 10k-to-100k",
            "unsupported:game frame fixes 10k instances",
        ),
        (
            "G4 Apple recipe versus portable and image tolerance",
            "unsupported:Apple recipe absent",
        ),
        (
            "G4 iPhone GPU proxy at native resolution times five",
            "unsupported:Apple recipe and native-resolution capture absent",
        ),
        (
            "G4 cold-play pipeline compilation",
            "unsupported:pipeline archive consumer absent",
        ),
        (
            "G5 no-change scan and texture-edit latency",
            "unsupported:replacement content cache absent",
        ),
        (
            "G0 iOS runtime dependency tree",
            "measured_guard:repo_check iOS cargo tree",
        ),
        (
            "G5 shipping dependency closure",
            "unsupported:Mac browser Android and final-link guards absent",
        ),
        (
            "G6 1.5k-mover conditioned bandwidth",
            "unsupported:real transport and game network consumer absent",
        ),
        (
            "G6 reconciliation iPhone proxy",
            "unsupported:owned prediction and correction consumer absent",
        ),
        (
            "G7 four rendered peers and V1 exit",
            "unsupported:four-window game consumer absent",
        ),
        (
            "G7 remaining iPhone CPU/GPU proxy rows",
            "unsupported:four-peer game and named device profile absent",
        ),
    ];
    println!("fixed_row,baseline_state");
    for &(row, state) in ROWS {
        println!("{row},{state}");
    }
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
            .rewind(
                &mut sim,
                usize::try_from(history.latest_tick() - target).expect("retained count"),
            )
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

fn measure(
    bencher: Option<divan::Bencher<'_, '_>>,
    gpu_timestamps: bool,
    render_extent: [u32; 2],
    surface_extent: Option<[u32; 2]>,
    late_surface: bool,
    callback_period: Duration,
    allow_unsent_callback: bool,
    callbacks_per_sample: usize,
    mode: FrameMode,
) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("real Metal adapter required for this explicit benchmark");
    let adapter_info = adapter.get_info();
    assert_eq!(adapter_info.backend, wgpu::Backend::Metal);
    report_metal_adapter(&adapter_info);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: (if gpu_timestamps {
            DeviceCapabilities::requested_timing_features(&adapter)
        } else {
            wgpu::Features::empty()
        }) | DeviceCapabilities::requested_content_features(&adapter),
        ..Default::default()
    }))
    .expect("device request succeeds");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
    let surface = surface_extent.map(|extent| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("V1 benchmark presentation target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
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
    let profile = surface_extent.map(|extent| SurfaceProfile {
        format: wgpu::TextureFormat::Bgra8Unorm,
        extent,
    });
    let mut host = pollster::block_on(FrameHost::new_with_capabilities(
        &device,
        &queue,
        &capabilities,
        FrameConfig {
            render_extent,
            surface: profile,
        },
    ))
    .expect("the prepared V1 workload must fit the adapter");
    prepare_content(&mut host);
    // Optional fixed time of day, for paths that only run near sunset.
    if let Ok(hours) = std::env::var("THRUM_BENCH_HOURS") {
        let hours: f32 = hours
            .parse()
            .expect("THRUM_BENCH_HOURS is a number of hours");
        host.set_time_of_day(hours / 24.0)
            .expect("the bench time of day is valid");
    }
    let origin = Instant::now();
    let (mut host, mut threaded_host) = if mode == FrameMode::Threaded {
        let (threaded_host, controls) = host
            .into_threaded(origin, || {})
            .expect("resident world starts before frame sampling");
        (None, Some((threaded_host, controls)))
    } else {
        (Some(host), None)
    };
    let device_ref = &device;
    let mut now = Duration::ZERO;
    let mut samples: Vec<FrameSample> = Vec::with_capacity(16_384);
    let mut callback_cpu = Vec::with_capacity(callbacks_per_sample);
    let mut callbacks = 0_usize;
    let mut previous_display = 0_u32;
    let mut coalesced_displays = 0_u32;
    let mut run = || {
        for _ in 0..callbacks_per_sample {
            callbacks += 1;
            if mode == FrameMode::Threaded {
                // A display link forecasts the next display, rather than
                // replaying overdue callbacks after setup or a driver stall.
                let display =
                    u32::try_from(origin.elapsed().as_nanos() / callback_period.as_nanos())
                        .expect("bounded frame batch")
                        + 1;
                coalesced_displays += display.saturating_sub(previous_display + 1);
                previous_display = display;
                let target = origin + callback_period.saturating_mul(display);
                if let Some(delay) = target.checked_duration_since(Instant::now()) {
                    std::thread::sleep(delay);
                }
                now = origin.elapsed();
            } else {
                now += callback_period;
            }
            let output = surface_view
                .as_ref()
                .zip(surface_extent)
                .map(|(view, extent)| SurfaceOutput { view, extent });
            let callback_start = Instant::now();
            let frame = if let Some((threaded_host, _controls)) = &mut threaded_host {
                let advanced = threaded_host
                    .advance(now)
                    .expect("threaded world observation");
                if advanced.can_acquire_serial() {
                    advanced
                        .encode_scene()
                        .expect("threaded scene recording")
                        .submit(output.as_ref())
                        .expect("threaded scene and presentation submit")
                } else {
                    advanced.submit(None).expect("threaded frame progresses")
                }
            } else if late_surface {
                host.as_mut()
                    .expect("inline host")
                    .advance(now)
                    .expect("the measured world step must keep progressing")
                    .encode_scene()
                    .expect("the serial scene must encode before acquisition")
                    .submit(output.as_ref())
                    .expect("the encoded scene and surface must submit once")
            } else {
                host.as_mut()
                    .expect("inline host")
                    .drive(now, output.as_ref())
                    .expect("the measured fixed-clock host must keep progressing")
            };
            if !frame.submitted {
                assert!(allow_unsent_callback, "one update gets a free view slot");
                if let Some((threaded_host, _controls)) = &mut threaded_host {
                    drain_threaded(threaded_host, &mut samples);
                    callback_cpu.push(callback_start.elapsed());
                }
                continue;
            }
            assert_eq!(frame.surface_presented, surface_extent.is_some());
            if let Some((threaded_host, _controls)) = &mut threaded_host {
                drain_threaded(threaded_host, &mut samples);
                callback_cpu.push(callback_start.elapsed());
                continue;
            }
            let complete = loop {
                if let Some(sample) = host
                    .as_mut()
                    .expect("inline host")
                    .poll_completed()
                    .expect("GPU progress")
                {
                    break sample;
                }
                device_ref
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .expect("GPU completion");
            };
            samples.push(complete);
            divan::black_box(complete);
        }
    };
    if let Some(bencher) = bencher {
        bencher.bench_local(&mut run);
    } else {
        run();
    }
    if let Some((threaded_host, _controls)) = &mut threaded_host {
        while threaded_host.pending_frames() != 0 {
            device_ref
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .expect("threaded GPU completion");
            drain_threaded(threaded_host, &mut samples);
        }
        drain_threaded(threaded_host, &mut samples);
    }
    if mode == FrameMode::Threaded {
        report_coalesced_displays(coalesced_displays);
        report_threaded_callback(&callback_cpu);
    }
    assert!(
        !samples.is_empty(),
        "baseline produced no completed frames in {callbacks} callbacks"
    );
    report_frame_cpu(
        &samples,
        callbacks,
        gpu_timestamps,
        render_extent,
        surface_extent,
    );
    // Preserve partial profiles for diagnosis, but keep the fixed row failing
    // when it did not complete enough frames to support a baseline.
    assert!(
        samples.len() >= callbacks / 2,
        "baseline needs frames from at least half its callbacks: {} of {callbacks}",
        samples.len()
    );
}

fn prepare_content(host: &mut FrameHost) {
    let Some(stats) = v1_game::attach_cooked_scene(host).expect("selected game content") else {
        return;
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        {
            let stats = stats.lock().expect("content observations");
            assert!(
                stats.errors.is_empty(),
                "content errors: {:?}",
                stats.errors
            );
            if stats.drawn.is_some() {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "benchmark content did not become resident"
        );
        drop(host.capture_pixels().expect("cold content upload"));
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the benchmark labels missed display forecasts under load"
)]
fn report_coalesced_displays(count: u32) {
    println!("V1 threaded coalesced display forecasts={count}");
}

fn drain_threaded(host: &mut ThreadedFrameHost, samples: &mut Vec<FrameSample>) {
    while let Some(sample) = host.poll_completed().expect("threaded GPU progress") {
        samples.push(sample);
    }
}

#[expect(
    clippy::print_stderr,
    reason = "the explicit benchmark reports render-owner callback cost including GPU polling"
)]
fn report_threaded_callback(callbacks: &[Duration]) {
    assert!(!callbacks.is_empty(), "threaded callback samples required");
    let mut micros: Vec<f64> = callbacks
        .iter()
        .map(|duration| duration.as_secs_f64() * 1e6)
        .collect();
    micros.sort_by(f64::total_cmp);
    eprintln!(
        "V1 threaded callback including GPU poll p50/p95/p99={:.1}/{:.1}/{:.1} us",
        micros[micros.len() / 2],
        micros[micros.len() * 95 / 100],
        micros[micros.len() * 99 / 100]
    );
}

#[expect(
    clippy::print_stderr,
    reason = "record the selected Metal adapter in raw benchmark evidence"
)]
fn report_metal_adapter(info: &wgpu::AdapterInfo) {
    eprintln!("G0 Metal adapter: {} ({:?})", info.name, info.backend);
}

#[expect(
    clippy::print_stderr,
    reason = "the explicit integrated frame benchmark reports its measured CPU stages"
)]
fn report_frame_cpu(
    samples: &[FrameSample],
    callbacks: usize,
    gpu_timestamps: bool,
    render_extent: [u32; 2],
    surface_extent: Option<[u32; 2]>,
) {
    assert!(!samples.is_empty(), "frame CPU needs submitted samples");
    assert!(
        !gpu_timestamps || samples.iter().all(|sample| sample.gpu_pass_ns.is_some()),
        "GPU baseline requires timestamps for every submitted frame"
    );
    let percentile = |value: fn(&FrameSample) -> f64| {
        let mut values: Vec<f64> = samples.iter().map(value).collect();
        values.sort_by(f64::total_cmp);
        (
            values[values.len() / 2],
            values[values.len() * 95 / 100],
            values[values.len() * 99 / 100],
        )
    };
    let tick = percentile(|sample| sample.tick_cpu.as_secs_f64() * 1e6);
    let upload = percentile(|sample| sample.upload_cpu.as_secs_f64() * 1e6);
    let extract = percentile(|sample| sample.extract_cpu.as_secs_f64() * 1e6);
    let view = percentile(|sample| f64::from(sample.view_update_ns) / 1e3);
    let encode = percentile(|sample| sample.encode_cpu.as_secs_f64() * 1e6);
    let combined = percentile(|sample| (sample.upload_cpu + sample.encode_cpu).as_secs_f64() * 1e6);
    let stages = percentile(|sample| {
        (sample.extract_cpu + sample.upload_cpu + sample.encode_cpu).as_secs_f64() * 1e6
    });
    let host_non_tick = percentile(|sample| sample.render_cpu.as_secs_f64() * 1e6);
    let host_wall = percentile(|sample| sample.wall.as_secs_f64() * 1e6);
    eprintln!(
        "V1 offscreen frame CPU submitted={} callbacks={} render={}x{} timestamps={} surface={:?} tick p50/p95={:.1}/{:.1} us extract p50/p95={:.1}/{:.1} us live-view p50/p95={:.1}/{:.1} us upload p50/p95={:.1}/{:.1} us encode p50/p95={:.1}/{:.1} us upload+encode p50/p95={:.1}/{:.1} us stages p50/p95/p99={:.1}/{:.1}/{:.1} us render-thread p50/p95/p99={:.1}/{:.1}/{:.1} us host-wall p50/p95/p99={:.1}/{:.1}/{:.1} us",
        samples.len(),
        callbacks,
        render_extent[0],
        render_extent[1],
        gpu_timestamps,
        surface_extent,
        tick.0,
        tick.1,
        extract.0,
        extract.1,
        view.0,
        view.1,
        upload.0,
        upload.1,
        encode.0,
        encode.1,
        combined.0,
        combined.1,
        stages.0,
        stages.1,
        stages.2,
        host_non_tick.0,
        host_non_tick.1,
        host_non_tick.2,
        host_wall.0,
        host_wall.1,
        host_wall.2,
    );
    let uploaded_rows: u64 = samples
        .iter()
        .map(|sample| u64::from(sample.uploaded_rows))
        .sum();
    let uploaded_bytes: usize = samples.iter().map(|sample| sample.uploaded_bytes).sum();
    let zero_upload_frames = samples
        .iter()
        .filter(|sample| sample.uploaded_rows == 0)
        .count();
    eprintln!(
        "V1 frame uploads rows={uploaded_rows} bytes={uploaded_bytes} zero-upload-frames={zero_upload_frames}"
    );
    if gpu_timestamps {
        let scene_gpu =
            percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11]).iter().sum::<f64>() / 1e6);
        eprintln!(
            "V1 scene GPU pass-sum p50/p95/p99={:.3}/{:.3}/{:.3} ms",
            scene_gpu.0, scene_gpu.1, scene_gpu.2
        );
        let shadow = percentile(|sample| {
            let passes = sample.gpu_pass_ns.unwrap_or([0.0; 11]);
            (passes[0] + passes[1]) / 1e6
        });
        let cluster = percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11])[2] / 1e6);
        let forward = percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11])[3] / 1e6);
        let history = percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11])[4] / 1e6);
        let tone = percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11])[5] / 1e6);
        let atmosphere = percentile(|sample| {
            let passes = sample.gpu_pass_ns.unwrap_or([0.0; 11]);
            passes[6..].iter().sum::<f64>() / 1e6
        });
        let aerial = percentile(|sample| sample.gpu_pass_ns.unwrap_or([0.0; 11])[10] / 1e6);
        eprintln!(
            "V1 GPU p50/p95 ms shadow={:.3}/{:.3} cluster={:.3}/{:.3} forward+sky={:.3}/{:.3} history={:.3}/{:.3} tone={:.3}/{:.3} atmosphere-compute={:.3}/{:.3} aerial={:.3}/{:.3}",
            shadow.0,
            shadow.1,
            cluster.0,
            cluster.1,
            forward.0,
            forward.1,
            history.0,
            history.1,
            tone.0,
            tone.1,
            atmosphere.0,
            atmosphere.1,
            aerial.0,
            aerial.1
        );
    }
}

#[divan::bench]
fn assembled_frame_with_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        true,
        [640, 360],
        None,
        false,
        Duration::from_nanos(16_666_667),
        false,
        1,
        FrameMode::Inline,
    );
}

#[divan::bench]
fn assembled_frame_without_gpu_timestamps(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        false,
        [640, 360],
        None,
        false,
        Duration::from_nanos(16_666_667),
        false,
        1,
        FrameMode::Inline,
    );
}

#[divan::bench]
fn assembled_frame_with_surface_pass(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        true,
        [640, 360],
        Some([1280, 720]),
        false,
        Duration::from_nanos(16_666_667),
        false,
        1,
        FrameMode::Inline,
    );
}

#[divan::bench]
fn assembled_frame_1080p_with_surface_pass(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        true,
        [1920, 1080],
        Some([1920, 1080]),
        false,
        Duration::from_nanos(16_666_667),
        false,
        1,
        FrameMode::Inline,
    );
}

#[divan::bench]
fn assembled_frame_1080p_late_surface(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        true,
        [1920, 1080],
        Some([1920, 1080]),
        true,
        Duration::from_nanos(16_666_667),
        false,
        1,
        FrameMode::Inline,
    );
}

#[divan::bench]
fn g0_120hz_callbacks_at_1080p(bencher: divan::Bencher<'_, '_>) {
    measure(
        Some(bencher),
        true,
        [1920, 1080],
        Some([1920, 1080]),
        false,
        Duration::from_nanos(8_333_333),
        true,
        100,
        FrameMode::Inline,
    );
}
