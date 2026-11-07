//! Paired same-build cost of zero versus four local lights. Both variants
//! render the same 10k-instance scene and wait for GPU completion. The ABBA
//! order reduces monotonic host drift, but this is not a display-frame result.
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

use thrum_render::synthetic::{
    OffscreenTarget, PreparedSynthetic, SyntheticPointLight, SyntheticScene,
};
use thrum_render::timing::GpuTimings;

const GENERATION: u64 = 1;
const BLOCKS: usize = 200;
const WARMUP_BLOCKS: usize = 25;
const GPU_BLOCKS: usize = 100;

fn render_wait(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
) -> Duration {
    let started = Instant::now();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, GENERATION, target, None)
        .expect("prepared role encodes");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("GPU submission completes");
    started.elapsed()
}

fn percentiles(values: &mut [Duration]) -> (f64, f64, f64) {
    values.sort_unstable();
    let millis = |value: Duration| value.as_secs_f64() * 1_000.0;
    (
        millis(values[values.len() / 2]),
        millis(values[values.len() * 95 / 100]),
        millis(values[values.len() - 1]),
    )
}

fn gpu_percentiles(values: &mut [f64]) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}

fn paired_delta(samples: [f64; 4]) -> f64 {
    ((samples[1] + samples[2]) - (samples[0] + samples[3])) * 0.5
}

fn delta_percentiles(values: &mut [f64]) -> (f64, f64, f64) {
    values.sort_by(f64::total_cmp);
    (
        values[values.len() * 5 / 100],
        values[values.len() / 2],
        values[values.len() * 95 / 100],
    )
}

fn render_gpu_pass(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    timings: &mut GpuTimings,
) -> f64 {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, GENERATION, target, Some(timings))
        .expect("timestamped view encodes");
    timings
        .finish_render(GENERATION, &encoder)
        .expect("render timestamp batch closes");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("render completes before separate timestamp resolve");
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
        .expect("timestamp readback completes");
    let passes = timings
        .try_collect(GENERATION)
        .expect("valid GPU counter order")
        .expect("completed timestamp batch");
    passes
        .iter()
        .find(|pass| pass.name.as_ref() == "synthetic forward opaque")
        .expect("forward pass timing")
        .nanoseconds
        / 1_000_000.0
}

#[expect(
    clippy::print_stdout,
    reason = "a standalone benchmark reports its paired samples"
)]
fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for the explicit benchmark");
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
            .expect("device");
        let dark_scene = SyntheticScene::grid(10_000).expect("10k fixture");
        let mut lit_scene = dark_scene.clone();
        lit_scene.local_lights = [
            ([-0.55, -0.55, 0.15], [5.0, 0.8, 0.4]),
            ([0.55, -0.55, 0.15], [0.4, 4.0, 0.8]),
            ([-0.55, 0.55, 0.15], [0.7, 1.0, 5.0]),
            ([0.55, 0.55, 0.15], [4.0, 3.0, 0.4]),
        ]
        .map(|(position, candela)| SyntheticPointLight {
            position,
            radius: 0.6,
            candela,
        })
        .to_vec();
        let dark = PreparedSynthetic::prepare(&device, &queue, GENERATION, &dark_scene)
            .await
            .expect("zero-local-light preparation");
        let lit = PreparedSynthetic::prepare(&device, &queue, GENERATION, &lit_scene)
            .await
            .expect("four-local-light preparation");
        let dark_target = OffscreenTarget::new(&device, &dark, 640, 360).expect("dark target");
        let lit_target = OffscreenTarget::new(&device, &lit, 640, 360).expect("lit target");
        let max_scene_passes = dark.scene_pass_count().max(lit.scene_pass_count());
        let variants = [(&dark, &dark_target), (&lit, &lit_target)];
        for _ in 0..WARMUP_BLOCKS {
            for index in [0, 1, 1, 0] {
                let (prepared, target) = variants[index];
                divan::black_box(render_wait(&device, &queue, prepared, target));
            }
        }
        let mut dark_samples = Vec::with_capacity(BLOCKS * 2);
        let mut lit_samples = Vec::with_capacity(BLOCKS * 2);
        let mut wall_deltas = Vec::with_capacity(BLOCKS);
        for _ in 0..BLOCKS {
            let mut block = [0.0; 4];
            for (slot, index) in [0, 1, 1, 0].into_iter().enumerate() {
                let (prepared, target) = variants[index];
                let elapsed = render_wait(&device, &queue, prepared, target);
                block[slot] = elapsed.as_secs_f64() * 1_000.0;
                if index == 0 {
                    dark_samples.push(elapsed);
                } else {
                    lit_samples.push(elapsed);
                }
            }
            wall_deltas.push(paired_delta(block));
        }
        let dark = percentiles(&mut dark_samples);
        let lit = percentiles(&mut lit_samples);
        let wall_delta = delta_percentiles(&mut wall_deltas);
        println!(
            "adapter={adapter_name}; 640x360 10k static cubes; ABBA {BLOCKS} blocks, 400 samples/variant after 50 warmup submissions/variant; no GPU timestamps; encode+submit+GPU-wait wall p50/p95/max ms: zero local={:.3}/{:.3}/{:.3}, four local={:.3}/{:.3}/{:.3}; within-block four-minus-zero delta p05/p50/p95={:.3}/{:.3}/{:.3} ms",
            dark.0, dark.1, dark.2, lit.0, lit.1, lit.2, wall_delta.0, wall_delta.1, wall_delta.2
        );
        let capabilities =
            thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, GENERATION);
        let mut timings = GpuTimings::new(&capabilities, &queue, max_scene_passes)
            .expect("optional GPU timestamp batch");
        if timings.is_enabled() {
            let mut dark_gpu = Vec::with_capacity(GPU_BLOCKS * 2);
            let mut lit_gpu = Vec::with_capacity(GPU_BLOCKS * 2);
            let mut gpu_deltas = Vec::with_capacity(GPU_BLOCKS);
            for _ in 0..GPU_BLOCKS {
                let mut block = [0.0; 4];
                for (slot, index) in [0, 1, 1, 0].into_iter().enumerate() {
                    let (prepared, target) = variants[index];
                    let elapsed = render_gpu_pass(&device, &queue, prepared, target, &mut timings);
                    block[slot] = elapsed;
                    if index == 0 {
                        dark_gpu.push(elapsed);
                    } else {
                        lit_gpu.push(elapsed);
                    }
                }
                gpu_deltas.push(paired_delta(block));
            }
            let dark_gpu = gpu_percentiles(&mut dark_gpu);
            let lit_gpu = gpu_percentiles(&mut lit_gpu);
            let gpu_delta = delta_percentiles(&mut gpu_deltas);
            println!(
                "paired GPU forward pass (cluster compute excluded): ABBA {GPU_BLOCKS} blocks, 200 samples/variant; p50/p95 ms zero={:.3}/{:.3}, four={:.3}/{:.3}; within-block four-minus-zero delta p05/p50/p95={:.3}/{:.3}/{:.3} ms",
                dark_gpu.0, dark_gpu.1, lit_gpu.0, lit_gpu.1, gpu_delta.0, gpu_delta.1, gpu_delta.2
            );
        } else {
            println!("GPU timestamp queries unavailable on this adapter");
        }
    });
}
