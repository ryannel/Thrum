//! G5 in the game at realistic sizes: edit-to-visible stages and per-frame cost while cooked
//! products upload under the per-frame budget, against the single-submit `write_texture` path.
//!
//! Short native run (light GPU use). Each frame is driven and then waited for, so a frame's time
//! is its whole CPU-plus-GPU latency, not pipelined throughput: a conservative per-frame cost.
//! Figures are tripwires on a shared machine, not budgets.
//! `cargo bench -p v1-game --bench cooked_edit`; `THRUM_G5_BENCH_OUT` also writes the report.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::{AssetStore, Ktx2, Profile};
use thrum_asset_gpu::{CookedAssets, CookedStats};
use thrum_cook::tripwire::{self, Rule};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use v1_game::{FrameConfig, FrameHost};

const BUDGET: u64 = 8 << 20;

/// Measured metrics (milliseconds, frames or MiB), checked against their history after the run.
type Metrics = Vec<(String, f64)>;

/// Each tripwire's allowed multiple of its same-load history median, and a floor. Staging peaks
/// are deterministic and may not grow at all, so they are judged against every load class; upload
/// frames are paced by chunks returning, so they depend on load like the timings.
/// Timing factors are at least 1.25 times the largest max/median spread seen over repeated runs.
/// Startup is bimodal (device creation warm or cold), so its floor sits above the cold runs.
const RULES: &[Rule] = &[
    Rule {
        metric: "leased_upload_cpu_p99_ms",
        factor: 3.0,
        floor: 5.0,
        any_load: false,
    },
    Rule {
        metric: "leased_staging_peak_mib",
        factor: 1.0,
        floor: 0.0,
        any_load: true,
    },
    Rule {
        metric: "realistic_startup_ms",
        factor: 3.0,
        floor: 750.0,
        any_load: false,
    },
    Rule {
        metric: "realistic_write_swapped_median_ms",
        factor: 2.0,
        floor: 100.0,
        any_load: false,
    },
    Rule {
        metric: "realistic_upload_cpu_p99_ms",
        factor: 3.0,
        floor: 5.0,
        any_load: false,
    },
    Rule {
        metric: "realistic_staging_peak_mib",
        factor: 1.0,
        floor: 0.0,
        any_load: true,
    },
    Rule {
        metric: "large_startup_ms",
        factor: 3.0,
        floor: 1000.0,
        any_load: false,
    },
    Rule {
        metric: "large_upload_frames_max",
        factor: 1.6,
        floor: 0.0,
        any_load: false,
    },
    Rule {
        metric: "large_upload_cpu_p99_ms",
        factor: 3.0,
        floor: 5.0,
        any_load: false,
    },
    Rule {
        metric: "large_staging_peak_mib",
        factor: 1.0,
        floor: 0.0,
        any_load: true,
    },
];

struct Scenario {
    leased: bool,
    key: &'static str,
    name: &'static str,
    volume: u32,
    profiles: Vec<Profile>,
    edits: usize,
}

fn main() {
    let mut report = String::new();
    let mut metrics = Metrics::new();
    writeln!(report, "# G5 in-game edit-to-visible and upload cost").ok();
    writeln!(
        report,
        "release bench, {} {}, budget {} MiB/frame, 640x360, each frame waited for",
        std::env::consts::OS,
        std::env::consts::ARCH,
        BUDGET >> 20
    )
    .ok();
    for scenario in [
        Scenario {
            leased: false,
            key: "realistic",
            name: "realistic: 256-grid mesh, 1024² texture, 256³ volume, all profiles",
            volume: 256,
            profiles: Profile::ALL.to_vec(),
            edits: 6,
        },
        Scenario {
            leased: false,
            key: "large",
            name: "large: 512³ R8 volume (portable only)",
            volume: 512,
            profiles: vec![Profile::Portable],
            edits: 2,
        },
        Scenario {
            leased: true,
            key: "leased",
            name: "leased: twelve 128³ volumes beside placements; one-volume edits",
            volume: 128,
            profiles: vec![Profile::Portable, Profile::DesktopBc],
            edits: 3,
        },
    ] {
        run(&scenario, &mut report, &mut metrics);
    }
    // Checked against earlier runs at the same load; THRUM_TRIPWIRES_RECORD=1 appends a passing run.
    let history = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/evidence/integration/tripwires/g5-cooked-edit.tsv");
    let record = std::env::var_os("THRUM_TRIPWIRES_RECORD").is_some();
    let verdict = tripwire::gate(&history, &tripwire::this_run(metrics), RULES, record);
    report.push_str(verdict.as_ref().map_or_else(|error| error, |ok| ok));
    #[expect(clippy::print_stderr, reason = "a bench reports to its terminal")]
    {
        eprint!("{report}");
    }
    if let Some(path) = std::env::var_os("THRUM_G5_BENCH_OUT") {
        std::fs::write(path, &report).expect("write report");
    }
    if verdict.is_err() {
        std::process::exit(1);
    }
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

/// Drive one frame and wait for it; returns (frame latency, render-thread CPU), or `None` when
/// the engine had nothing new to render.
fn frame(host: &mut FrameHost, device: &wgpu::Device, clock: &mut Duration) -> Option<(f64, f64)> {
    *clock += Duration::from_micros(8_333);
    let start = Instant::now();
    if !host.drive(*clock, None).expect("frame").submitted {
        return None;
    }
    let mut render_cpu = 0.0;
    while host.pending_frames() > 0 {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
        let polled = host.poll_completed().expect("GPU progress");
        if let Some(sample) = polled {
            render_cpu = ms(sample.render_cpu);
        }
    }
    Some((ms(start.elapsed()), render_cpu))
}

#[expect(
    clippy::too_many_lines,
    reason = "one bench scenario reads top to bottom: build, edit, measure, report"
)]
fn run(scenario: &Scenario, report: &mut String, metrics: &mut Metrics) {
    let root = std::env::temp_dir().join(format!(
        "thrum-g5-game-bench-{}-{}",
        std::process::id(),
        scenario.volume
    ));
    std::fs::remove_dir_all(&root).ok();
    let source = root.join("source");
    for directory in ["meshes", "textures"] {
        std::fs::create_dir_all(source.join(directory)).expect("source tree");
    }
    let generate = Instant::now();
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 256, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("textures/checker.png"), 1024, [200, 40, 40])
        .expect("png");
    demo::write_cloud_volume(&source.join("cloud.tvol"), scenario.volume, 1).expect("volume");
    if scenario.leased {
        for index in 1..12 {
            demo::write_cloud_volume(&source.join(format!("cloud{index}.tvol")), 128, index + 1)
                .expect("leased volume");
        }
    }
    let generated = generate.elapsed();
    let mut cooker = Cooker::open(CookConfig {
        source: source.clone(),
        store: root.join("store"),
        profiles: scenario.profiles.clone(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    let first = cooker.cook_all().expect("cook");
    assert!(first.failed.is_empty(), "{:?}", first.failed);
    writeln!(
        report,
        "\n## {}\n\nsource generation {:.0} ms, first cook {:.0} ms",
        scenario.name,
        ms(generated),
        ms(first.cook_time)
    )
    .ok();

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("adapter");
        let features = adapter.features()
            & (wgpu::Features::TEXTURE_COMPRESSION_ASTC
                | wgpu::Features::TEXTURE_COMPRESSION_BC
                | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: features,
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .expect("device");
        let capabilities =
            thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("game");
        let mut config = v1_game::cooked::cooked_config(root.join("store"));
        config.upload_budget = BUDGET;
        config.poll_interval = Duration::from_millis(1);
        if scenario.leased {
            config.leased = (0..12)
                .map(|index| thrum_asset_gpu::LeaseRequest {
                    name: if index == 0 {
                        "cloud".into()
                    } else {
                        format!("cloud{index}")
                    },
                    dimension: wgpu::TextureDimension::D3,
                })
                .collect();
            config.leased.push(thrum_asset_gpu::LeaseRequest {
                name: "textures/checker".into(),
                dimension: wgpu::TextureDimension::D2,
            });
        }
        let assets = CookedAssets::new(config).expect("rigid placements");
        let stats: Arc<Mutex<CookedStats>> = assets.stats();
        let attach = Instant::now();
        if scenario.leased {
            let (consumer, _) = v1_game::lent::LentDiagnostic::new(assets.lease());
            host.set_scene_contribution(Box::new(
                thrum_render::contribution::SceneContributions::new(vec![
                    Box::new(assets),
                    Box::new(consumer),
                ]),
            ))
            .expect("attach leased scenario");
        } else {
            host.set_scene_contribution(Box::new(assets))
                .expect("attach");
        }
        assert_eq!(
            stats.lock().expect("stats").draw_path,
            Some(thrum_asset_gpu::CookedDrawPath::Indirect)
        );
        writeln!(
            report,
            "selected cooked path: Indirect (adapter-backed record)"
        )
        .ok();
        let mut clock = Duration::ZERO;
        let mut steady = Vec::new();
        let mut upload = Vec::new();
        let mut cpu_steady = Vec::new();
        let mut cpu_upload = Vec::new();
        let mut startup = None;
        let mut edits_made = 0;
        let mut edit_started: Vec<(u64, Instant, Instant)> = Vec::new();
        let mut last_swaps = 0;
        let mut frames_since_swap = 0;
        let mut trace = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(120);
        while Instant::now() < deadline {
            let uploading = {
                let stats = stats.lock().expect("stats");
                assert!(stats.errors.is_empty(), "{:?}", stats.errors);
                stats.swaps.len() < 1 + edits_made
            };
            let Some((latency, cpu)) = frame(&mut host, &device, &mut clock) else {
                continue;
            };
            let (swaps, copied) = {
                let stats = stats.lock().expect("stats");
                (stats.swaps.len(), stats.last_frame_copy_bytes)
            };
            if copied > 0 || uploading {
                trace.push((copied, latency, cpu, swaps));
            }
            if swaps > last_swaps {
                last_swaps = swaps;
                frames_since_swap = 0;
                if startup.is_none() {
                    startup = Some(attach.elapsed());
                }
            }
            frames_since_swap += 1;
            if copied > 0 || uploading {
                upload.push(latency);
                cpu_upload.push(cpu);
            } else if startup.is_some() {
                steady.push(latency);
                cpu_steady.push(cpu);
            }
            // Edit after 30 quiet frames, alternating texture and volume.
            if !uploading && frames_since_swap > 30 {
                if edits_made == scenario.edits {
                    break;
                }
                let volume_edit = scenario.leased || scenario.volume >= 512 || edits_made % 2 == 1;
                let write = Instant::now();
                let path = if volume_edit {
                    // The leased library already contains seeds 1–11: use new content so
                    // this measures an upload rather than a switch to another resident volume.
                    let seed = u32::try_from(edits_made).expect("seed")
                        + if scenario.leased { 100 } else { 2 };
                    demo::write_cloud_volume(&source.join("cloud.tvol"), scenario.volume, seed)
                        .expect("edit volume");
                    PathBuf::from("cloud.tvol")
                } else {
                    let shade = u8::try_from(40 + edits_made * 30).expect("shade");
                    demo::write_checker_png(
                        &source.join("textures/checker.png"),
                        1024,
                        [shade, 60, 210],
                    )
                    .expect("edit texture");
                    PathBuf::from("textures/checker.png")
                };
                let written = Instant::now();
                let report = cooker.cook_changed(&[path]).expect("cook");
                let generation = report.published.expect("published").generation;
                edit_started.push((generation, write, written));
                edits_made += 1;
            }
        }
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.swaps.len(), 1 + scenario.edits, "every edit swapped");
        let key = scenario.key;
        let mut write_swapped: Vec<f64> = stats
            .swaps
            .iter()
            .filter_map(|swap| {
                let (_, write, _) = edit_started
                    .iter()
                    .find(|(generation, _, _)| *generation == swap.generation)?;
                Some(ms(swap.swapped - *write))
            })
            .collect();
        metrics.push((format!("{key}_startup_ms"), ms(startup.unwrap_or_default())));
        if key == "realistic" {
            metrics.push((
                format!("{key}_write_swapped_median_ms"),
                percentile(&mut write_swapped, 0.5),
            ));
        } else {
            let frames = stats.swaps.iter().map(|swap| swap.upload_frames).max();
            metrics.push((
                format!("{key}_upload_frames_max"),
                f64::from(frames.unwrap_or_default()),
            ));
        }
        metrics.push((
            format!("{key}_upload_cpu_p99_ms"),
            percentile(&mut cpu_upload, 0.99),
        ));
        metrics.push((
            format!("{key}_staging_peak_mib"),
            stats.staging_peak_bytes as f64 / f64::from(1 << 20),
        ));
        writeln!(
            report,
            "startup (attach → first set drawn): {:.1} ms; profiles drawn: texture {:?}, volume {:?}",
            ms(startup.unwrap_or_default()),
            stats.swaps[0].texture_profile,
            stats.swaps[0].volume_profile
        )
        .ok();
        writeln!(
            report,
            "\n| generation | uploaded | frames | write+cook → HEAD seen | → staged | → first copy | → swapped | write → swapped |\n|---|---|---|---|---|---|---|---|"
        )
        .ok();
        for swap in &stats.swaps {
            let edit = edit_started
                .iter()
                .find(|(generation, _, _)| *generation == swap.generation);
            let origin = edit.map_or(swap.head_seen, |(_, write, _)| *write);
            writeln!(
                report,
                "| {} | {:.2} MiB | {} | {:.1} ms | {:.1} ms | {:.1} ms | {:.1} ms | {} |",
                swap.generation,
                swap.uploaded_bytes as f64 / f64::from(1 << 20),
                swap.upload_frames,
                ms(swap.head_seen.saturating_duration_since(origin)),
                ms(swap.staged - swap.head_seen),
                ms(swap.first_copy.saturating_duration_since(swap.staged)),
                ms(swap.swapped - swap.first_copy),
                edit.map_or_else(
                    || "startup".to_owned(),
                    |_| format!("{:.1} ms", ms(swap.swapped - origin))
                )
            )
            .ok();
        }
        writeln!(
            report,
            "\nlargest copy bytes recorded in one frame: {:.2} MiB",
            stats.max_frame_copy_bytes as f64 / f64::from(1 << 20)
        )
        .ok();
        writeln!(
            report,
            "staging peak: {:.2} MiB (bounded ring of {} chunks)",
            stats.staging_peak_bytes as f64 / f64::from(1 << 20),
            thrum_asset_gpu::STAGING_CHUNKS
        )
        .ok();
        writeln!(
            report,
            "\n| frames | n | latency p50 | p99 | max | render CPU p50 | p99 | max |\n|---|---|---|---|---|---|---|---|"
        )
        .ok();
        for (label, latency, cpu) in [
            ("steady", &mut steady, &mut cpu_steady),
            ("uploading", &mut upload, &mut cpu_upload),
        ] {
            writeln!(
                report,
                "| {label} | {} | {:.2} | {:.2} | {:.2} | {:.3} | {:.3} | {:.3} |",
                latency.len(),
                percentile(latency, 0.5),
                percentile(latency, 0.99),
                percentile(latency, 1.0),
                percentile(cpu, 0.5),
                percentile(cpu, 0.99),
                percentile(cpu, 1.0)
            )
            .ok();
        }
        if std::env::var_os("THRUM_G5_BENCH_TRACE").is_some() {
            for (copied, latency, cpu, swaps) in &trace {
                writeln!(report, "upload-frame after {swaps} swaps, {copied} bytes: latency {latency:.2} ms, render CPU {cpu:.3} ms").ok();
            }
        }
        // Release the stats lock: the contribution updates them every frame.
        drop(stats);
        contrast(
            &root.join("store"),
            &device,
            &queue,
            &mut host,
            &mut clock,
            report,
        );
    });
    std::fs::remove_dir_all(&root).ok();
}

/// The single-submit alternative: `Queue::write_texture` of the drawn volume, flushed by the
/// next frame's submission.
fn contrast(
    store: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    host: &mut FrameHost,
    clock: &mut Duration,
    report: &mut String,
) {
    let store = AssetStore::new(store);
    let head = store.read_head().expect("head").expect("published");
    let manifest = store.load_manifest(&head).expect("manifest");
    let entry = manifest
        .resolve("cloud", &[Profile::Portable])
        .expect("portable volume");
    let volume = Ktx2::open(store.map(&entry.product).expect("map")).expect("ktx2");
    let level = volume.levels()[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("contrast volume"),
        size: wgpu::Extent3d {
            width: level.width,
            height: level.height,
            depth_or_array_layers: level.depth,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let mut latency = Vec::new();
    let mut write_cpu = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        queue.write_texture(
            texture.as_image_copy(),
            volume.level_data(&level),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(level.width),
                rows_per_image: Some(level.height),
            },
            wgpu::Extent3d {
                width: level.width,
                height: level.height,
                depth_or_array_layers: level.depth,
            },
        );
        write_cpu.push(ms(start.elapsed()));
        let mut sample = None;
        while sample.is_none() {
            sample = frame(host, device, clock);
        }
        latency.push(sample.map_or(0.0, |(latency, _)| latency));
    }
    writeln!(
        report,
        "\nsingle-submit contrast (render thread `write_texture` of the {:.1} MiB portable volume level 0, then that frame): write CPU p50 {:.2} ms, frame latency p50 {:.2} ms, max {:.2} ms",
        volume.level_data(&level).len() as f64 / f64::from(1 << 20),
        percentile(&mut write_cpu, 0.5),
        percentile(&mut latency.clone(), 0.5),
        percentile(&mut latency, 1.0)
    )
    .ok();
}
