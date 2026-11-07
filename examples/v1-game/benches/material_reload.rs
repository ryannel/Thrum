//! G6 in the game: what an explicit material reload costs the frames around it on this machine.
//!
//! Short native run (light GPU use). Each frame is driven and then waited for, so a frame's time
//! is its whole CPU-plus-GPU latency, not pipelined throughput. The replacement compiles on its
//! own thread while frames keep running; the worst frame during a reload shows whether that
//! compile stalls the render thread. Every hook source is distinct, so no reload is served from
//! an earlier compile of the same source by this process. Figures are contended observations on
//! a shared machine, not budgets. `cargo bench -p v1-game --bench material_reload`;
//! `THRUM_G6_BENCH_OUT` also writes the report.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::Profile;
use thrum_asset_gpu::{CookedAssets, CookedStats, MaterialHook, MaterialOutcome};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::capabilities::DeviceCapabilities;
use v1_game::{FrameConfig, FrameHost};

const EXTENT: [u32; 2] = [1280, 720];
const RELOADS: u32 = 6;
const QUIET_FRAMES: usize = 60;

fn main() {
    let root = std::env::temp_dir().join(format!("thrum-g6-material-bench-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let source = root.join("source");
    for directory in ["meshes", "textures"] {
        std::fs::create_dir_all(source.join(directory)).expect("source tree");
    }
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 128, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("textures/checker.png"), 512, [200, 200, 200])
        .expect("texture");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 32, 1).expect("volume");
    let cooked = Cooker::open(CookConfig {
        source,
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker")
    .cook_all()
    .expect("cook");
    assert!(cooked.failed.is_empty(), "{:?}", cooked.failed);
    let report = pollster::block_on(run(&root));
    std::fs::remove_dir_all(&root).ok();
    #[expect(clippy::print_stderr, reason = "a bench reports to its terminal")]
    {
        eprint!("{report}");
    }
    if let Some(path) = std::env::var_os("THRUM_G6_BENCH_OUT") {
        std::fs::write(path, &report).expect("write report");
    }
}

/// Distinct sources: a banded tint whose band frequency differs per reload and per run, so
/// neither this process nor the system's shader cache has compiled it before.
fn hook(index: u32, run: u32) -> MaterialHook {
    MaterialHook {
        name: format!("bench/hook-{index}.wgsl"),
        source: format!(
            "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {{
    var out = surface;
    let band = step(0.5, fract(at.world.x * {}.{run:04}));
    out.base_colour = vec4(surface.base_colour.rgb * mix(vec3(1.0), vec3(0.3), band), 1.0);
    return out;
}}
",
            index + 2
        ),
    }
}

async fn run(root: &std::path::Path) -> String {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_content_features(&adapter)
                | DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .expect("device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
    let mut host = FrameHost::new_with_capabilities(
        &device,
        &queue,
        &capabilities,
        FrameConfig {
            render_extent: EXTENT,
            surface: None,
        },
    )
    .await
    .expect("game");
    let assets = CookedAssets::new(v1_game::cooked::cooked_config(root.join("store")))
        .expect("rigid placements");
    let stats: Arc<Mutex<CookedStats>> = assets.stats();
    let material = assets.material();
    host.set_scene_contribution(Box::new(assets))
        .expect("attach");
    let mut clock = Duration::ZERO;
    let deadline = Instant::now() + Duration::from_secs(60);
    while stats.lock().expect("stats").drawn.is_none() {
        assert!(Instant::now() < deadline, "cooked set never drawn");
        frame(&mut host, &device, &mut clock);
    }

    let mut report = String::new();
    writeln!(
        report,
        "# G6 material reload: frames around an explicit reload"
    )
    .ok();
    writeln!(
        report,
        "release bench, {} {}, {}x{}, each frame waited for; contended shared GPU, one run",
        std::env::consts::OS,
        std::env::consts::ARCH,
        EXTENT[0],
        EXTENT[1]
    )
    .ok();
    let mut quiet: Vec<f64> = (0..QUIET_FRAMES)
        .filter_map(|_| frame(&mut host, &device, &mut clock))
        .collect();
    writeln!(
        report,
        "\nquiet frames before any reload: {} frames, p50 {:.2} ms, p95 {:.2} ms, worst {:.2} ms",
        quiet.len(),
        percentile(&mut quiet, 0.5),
        percentile(&mut quiet, 0.95),
        percentile(&mut quiet, 1.0)
    )
    .ok();
    writeln!(
        report,
        "\n| reload | call | prepare (compile thread) | eligible | drawn | frames during | during p50 | during worst | after worst |\n|---|---|---|---|---|---|---|---|---|"
    )
    .ok();
    let mut worst_during = 0.0_f64;
    let run = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos() % 10_000);
    for index in 0..RELOADS {
        let serial = material.request(hook(index, run));
        let reload_deadline = Instant::now() + Duration::from_secs(60);
        let mut during = Vec::new();
        let timing = loop {
            assert!(
                Instant::now() < reload_deadline,
                "reload {serial} never finished"
            );
            if let Some(latency) = frame(&mut host, &device, &mut clock) {
                during.push(latency);
            }
            let outcomes = material.take_outcomes();
            if let Some(outcome) = outcomes
                .iter()
                .find(|outcome| outcome.serial() == Some(serial))
            {
                match outcome {
                    MaterialOutcome::Promoted { timing, .. } => break *timing,
                    other => panic!("bench hook did not draw: {other}"),
                }
            }
        };
        let mut after: Vec<f64> = (0..QUIET_FRAMES)
            .filter_map(|_| frame(&mut host, &device, &mut clock))
            .collect();
        let worst = percentile(&mut during, 1.0);
        worst_during = worst_during.max(worst);
        writeln!(
            report,
            "| {serial} | {:.2} ms | {:.1} ms | {:.1} ms | {:.1} ms | {} | {:.2} ms | {:.2} ms | {:.2} ms |",
            ms(timing.call),
            ms(timing.prepare),
            ms(timing.eligible),
            ms(timing.drawn),
            during.len(),
            percentile(&mut during, 0.5),
            worst,
            percentile(&mut after, 1.0)
        )
        .ok();
    }
    writeln!(
        report,
        "\nworst frame during any reload: {worst_during:.2} ms (quiet worst {:.2} ms)",
        percentile(&mut quiet, 1.0)
    )
    .ok();
    report
}

fn percentile(values: &mut [f64], fraction: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "index into a short sample list"
    )]
    let index = ((values.len() - 1) as f64 * fraction).round() as usize;
    values[index]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

/// Drive one frame and wait for it; returns its latency, or `None` when nothing new rendered.
fn frame(host: &mut FrameHost, device: &wgpu::Device, clock: &mut Duration) -> Option<f64> {
    *clock += Duration::from_micros(16_667);
    let start = Instant::now();
    if !host.drive(*clock, None).expect("frame").submitted {
        return None;
    }
    while host.pending_frames() > 0 {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
        host.poll_completed().expect("GPU progress");
    }
    Some(ms(start.elapsed()))
}
