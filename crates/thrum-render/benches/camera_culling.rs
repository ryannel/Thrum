//! Paired Direct-versus-Frustum cost of the camera opaque draw at 10k and 100k
//! instances with full, half and 5% of the grid in view. ABBA blocks; GPU
//! timestamps exclude waits. Encode allocations are counted separately in
//! `camera_culling_encode` (divan's allocation profiler).
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::synthetic::{
    CullMode, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView,
};
use thrum_render::timing::GpuTimings;

const GENERATION: u64 = 1;
const WARMUP_BLOCKS: usize = 10;
const BLOCKS: usize = 60;
const EXTENT: [u32; 2] = [1280, 720];
const FOV: f32 = 1.0;
const ASPECT: f32 = 16.0 / 9.0;

#[derive(Clone, Copy, Default)]
struct Sample {
    gpu_ms: f64,
    forward_ms: f64,
    visibility_ms: f64,
    cpu_us: f64,
}

fn wait(device: &wgpu::Device, submitted: wgpu::SubmissionIndex) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("GPU finishes");
}

fn sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    timings: &mut GpuTimings,
) -> Sample {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let started = Instant::now();
    prepared
        .encode(&mut encoder, GENERATION, target, Some(timings))
        .expect("scene encodes");
    let cpu_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    timings
        .finish_render(GENERATION, &encoder)
        .expect("timestamp batch closes");
    wait(device, queue.submit([encoder.finish()]));
    let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert!(
        timings
            .resolve_if_ready(GENERATION, &mut resolve)
            .expect("resolve ready")
    );
    wait(device, queue.submit([resolve.finish()]));
    let passes = timings
        .try_collect(GENERATION)
        .expect("valid timestamps")
        .expect("completed timestamps");
    let named = |name: &str| {
        passes
            .iter()
            .find(|pass| pass.name.as_ref() == name)
            .map_or(0.0, |pass| pass.nanoseconds / 1_000_000.0)
    };
    Sample {
        gpu_ms: passes
            .iter()
            .filter(|pass| pass.name.as_ref() != "synthetic tone encode")
            .map(|pass| pass.nanoseconds / 1_000_000.0)
            .sum(),
        forward_ms: named("synthetic forward opaque"),
        visibility_ms: named("camera visibility"),
        cpu_us,
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn p95(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() * 95 / 100]
}

/// A camera looking straight down the grid normal from `height` above it.
/// The grid spans [-0.9, 0.9]² at z = 0.5, so the covered share is
/// min(1, 0.546·h/0.9) · min(1, 0.971·h/0.9).
fn view(height: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, 0.0, 0.5 + height],
        [0.0, 0.0, 0.5],
        [0.0, 1.0, 0.0],
        FOV,
        ASPECT,
        0.01,
        10.0,
    )
    .expect("finite view")
}

/// Share of instance centres strictly inside the frustum (CPU, reported).
fn in_view(scene: &SyntheticScene) -> f64 {
    let m = scene.view.view_proj_columns();
    let inside = scene
        .instances
        .iter()
        .filter(|instance| {
            let p = instance.current[3];
            let c: [f32; 4] = std::array::from_fn(|row| {
                m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row]
            });
            c[0].abs() < c[3] && c[1].abs() < c[3] && c[2] > 0.0 && c[2] < c[3]
        })
        .count();
    inside as f64 / scene.instances.len() as f64
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
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("timestamp-capable device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, GENERATION);
        println!(
            "adapter={}; {}x{}; shadows on; ABBA {BLOCKS} blocks (A=Direct, B=Frustum); \
             p50 (p95) per frame",
            adapter.get_info().name,
            EXTENT[0],
            EXTENT[1]
        );
        println!(
            "count | view | in view | whole GPU ms D→F | forward ms D→F | visibility ms | \
             within-block F−D GPU ms | CPU encode us D→F"
        );
        for count in [10_000, 100_000] {
            for (label, height) in [("full", 2.0), ("half", 0.874), ("5%", 0.276)] {
                let mut scenes = [CullMode::Direct, CullMode::Frustum].map(|culling| {
                    let mut scene = SyntheticScene::grid(count).expect("grid");
                    scene.view = view(height);
                    scene.sun_shadows = true;
                    scene.camera_culling = culling;
                    scene
                });
                let share = in_view(&scenes[0]);
                let mut prepared = Vec::new();
                for scene in &mut scenes {
                    prepared.push(
                        PreparedSynthetic::prepare(&device, &queue, GENERATION, scene)
                            .await
                            .expect("prepare"),
                    );
                }
                let targets: Vec<_> = prepared
                    .iter()
                    .map(|p| {
                        OffscreenTarget::new(&device, p, EXTENT[0], EXTENT[1]).expect("target")
                    })
                    .collect();
                let mut timings =
                    GpuTimings::new(&capabilities, &queue, prepared[1].scene_pass_count())
                        .expect("timestamp batch");
                assert!(timings.is_enabled(), "device timestamps required");
                let run = |index: usize, timings: &mut GpuTimings| {
                    sample(&device, &queue, &prepared[index], &targets[index], timings)
                };
                for _ in 0..WARMUP_BLOCKS {
                    for index in [0, 1, 1, 0] {
                        std::hint::black_box(run(index, &mut timings));
                    }
                }
                let mut samples = [Vec::new(), Vec::new()];
                let mut delta = Vec::new();
                for _ in 0..BLOCKS {
                    let mut block = [0.0; 4];
                    for (position, index) in [0, 1, 1, 0].into_iter().enumerate() {
                        let s = run(index, &mut timings);
                        block[position] = s.gpu_ms;
                        samples[index].push(s);
                    }
                    delta.push(((block[1] + block[2]) - (block[0] + block[3])) * 0.5);
                }
                let column = |index: usize, field: fn(&Sample) -> f64| -> (f64, f64) {
                    let mut values: Vec<f64> = samples[index].iter().map(field).collect();
                    (median(&mut values), p95(&mut values))
                };
                let gpu = [column(0, |s| s.gpu_ms), column(1, |s| s.gpu_ms)];
                let forward = [column(0, |s| s.forward_ms), column(1, |s| s.forward_ms)];
                let visibility = column(1, |s| s.visibility_ms);
                let cpu = [column(0, |s| s.cpu_us), column(1, |s| s.cpu_us)];
                println!(
                    "{count} | {label} | {:.1}% | {:.3} ({:.3}) → {:.3} ({:.3}) | {:.3} → {:.3} | \
                     {:.3} ({:.3}) | {:+.3} | {:.0} → {:.0}",
                    share * 100.0,
                    gpu[0].0,
                    gpu[0].1,
                    gpu[1].0,
                    gpu[1].1,
                    forward[0].0,
                    forward[1].0,
                    visibility.0,
                    visibility.1,
                    median(&mut delta),
                    cpu[0].0,
                    cpu[1].0,
                );
            }
        }
    });
}
