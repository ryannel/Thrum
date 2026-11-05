//! Paired GPU cost of drawing 10k versus 1k active rows from the same
//! 10k-slot scene. Shadow and forward pass timestamps exclude GPU waits.
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

use thrum_render::synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView};
use thrum_render::timing::GpuTimings;

const GENERATION: u64 = 1;
const WARMUP_BLOCKS: usize = 20;
const BLOCKS: usize = 100;

fn sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    timings: &mut GpuTimings,
) -> (f64, f64) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let started = Instant::now();
    prepared
        .encode(&mut encoder, GENERATION, target, Some(timings))
        .expect("prepared scene encodes");
    let cpu_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    timings
        .finish_render(GENERATION, &encoder)
        .expect("timestamp batch closes");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("scene finishes");
    let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert!(
        timings
            .resolve_if_ready(GENERATION, &mut resolve)
            .expect("timestamp resolve ready")
    );
    let submitted = queue.submit([resolve.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("timestamps finish");
    let passes = timings
        .try_collect(GENERATION)
        .expect("valid timestamps")
        .expect("completed timestamps");
    let gpu_ms = passes
        .iter()
        .filter(|pass| pass.name.as_ref() != "synthetic tone encode")
        .map(|pass| pass.nanoseconds / 1_000_000.0)
        .sum();
    (gpu_ms, cpu_us)
}

fn percentiles(values: &mut [f64]) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit device benchmark reports paired CPU and GPU samples"
)]
fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real native adapter required");
        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features:
                    thrum_render::capabilities::DeviceCapabilities::requested_timing_features(
                        &adapter,
                    ),
                ..Default::default()
            })
            .await
            .expect("timestamp-capable device");
        let mut full_scene = SyntheticScene::grid(10_000).expect("10k fixture scene");
        full_scene.view = SyntheticView::perspective(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            640.0 / 360.0,
            0.1,
            20.0,
        )
        .expect("finite perspective camera");
        full_scene.sun_shadows = true;
        let mut sparse_scene = full_scene.clone();
        for (slot, row) in sparse_scene.instances.iter_mut().enumerate() {
            if slot % 10 != 0 {
                row.ids[3] = 0;
            }
        }
        let full = PreparedSynthetic::prepare(&device, &queue, GENERATION, &full_scene)
            .await
            .expect("full scene preparation");
        let sparse = PreparedSynthetic::prepare(&device, &queue, GENERATION, &sparse_scene)
            .await
            .expect("sparse scene preparation");
        assert_eq!(full.active_instance_count(), 10_000);
        assert_eq!(sparse.active_instance_count(), 1_000);
        let full_target =
            OffscreenTarget::new(&device, &full, 640, 360).expect("full render target");
        let sparse_target =
            OffscreenTarget::new(&device, &sparse, 640, 360).expect("sparse render target");
        let variants = [(&full, &full_target), (&sparse, &sparse_target)];
        let capabilities =
            thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, GENERATION);
        let mut timings = GpuTimings::new(&capabilities, &queue, full.scene_pass_count())
            .expect("timestamp batch");
        assert!(timings.is_enabled(), "device timestamps required");
        for _ in 0..WARMUP_BLOCKS {
            for index in [0, 1, 1, 0] {
                let (prepared, target) = variants[index];
                divan::black_box(sample(&device, &queue, prepared, target, &mut timings));
            }
        }
        let mut gpu = [
            Vec::with_capacity(BLOCKS * 2),
            Vec::with_capacity(BLOCKS * 2),
        ];
        let mut cpu = [
            Vec::with_capacity(BLOCKS * 2),
            Vec::with_capacity(BLOCKS * 2),
        ];
        let mut gpu_delta = Vec::with_capacity(BLOCKS);
        for _ in 0..BLOCKS {
            let mut block = [0.0; 4];
            for (position, index) in [0, 1, 1, 0].into_iter().enumerate() {
                let (prepared, target) = variants[index];
                let (gpu_ms, cpu_us) = sample(&device, &queue, prepared, target, &mut timings);
                gpu[index].push(gpu_ms);
                cpu[index].push(cpu_us);
                block[position] = gpu_ms;
            }
            gpu_delta.push(((block[1] + block[2]) - (block[0] + block[3])) * 0.5);
        }
        let full_gpu = percentiles(&mut gpu[0]);
        let sparse_gpu = percentiles(&mut gpu[1]);
        let full_cpu = percentiles(&mut cpu[0]);
        let sparse_cpu = percentiles(&mut cpu[1]);
        gpu_delta.sort_by(f64::total_cmp);
        println!(
            "adapter={adapter_name}; 640x360 10k slots, shadows on; ABBA {BLOCKS} blocks; active rows 10000→1000; shadow+forward GPU p50/p95 ms {:.3}/{:.3}→{:.3}/{:.3}; within-block sparse-minus-full GPU p50 ms {:.3}; CPU encode p50/p95 us {:.1}/{:.1}→{:.1}/{:.1}",
            full_gpu.0,
            full_gpu.1,
            sparse_gpu.0,
            sparse_gpu.1,
            gpu_delta[BLOCKS / 2],
            full_cpu.0,
            full_cpu.1,
            sparse_cpu.0,
            sparse_cpu.1
        );
    });
}
