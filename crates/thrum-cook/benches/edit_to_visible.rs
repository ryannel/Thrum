//! G5 tripwire measurements: source scan latency, watcher-driven edit-to-visible stages, viewer
//! startup and frame hitch during product swaps.
//!
//! Short native run (about ten seconds, light GPU use). Figures are tripwires on a shared machine,
//! not budgets. `cargo bench -p thrum-cook --bench edit_to_visible`; set `THRUM_G5_BENCH_OUT` to also
//! write the report to a file.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use asset_loop::{Viewer, ViewerConfig};
use thrum_asset::Profile;
use thrum_cook::tripwire::{self, Rule};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, StatCache, demo, watch};

const EDITS: usize = 12;
const EDIT_PERIOD: Duration = Duration::from_millis(500);
const FRAME_PERIOD: Duration = Duration::from_micros(8_333);

/// Measured metrics, in milliseconds, checked against their history after the run.
type Metrics = Vec<(String, f64)>;

/// Each tripwire's allowed multiple of its same-load history median, and a floor in ms.
/// Factors are at least 1.25 times the largest max/median spread seen over repeated runs on the
/// shared machine. Startup is bimodal (device creation warm or cold), so its floor sits above the
/// cold runs.
const RULES: &[Rule] = &[
    Rule {
        metric: "scan_1000_warm_ms",
        factor: 3.0,
        floor: 2.0,
    },
    Rule {
        metric: "scan_10000_warm_ms",
        factor: 2.0,
        floor: 5.0,
    },
    Rule {
        metric: "scan_10000_one_change_ms",
        factor: 3.0,
        floor: 5.0,
    },
    Rule {
        metric: "scan_10000_targeted_ms",
        factor: 3.0,
        floor: 0.5,
    },
    Rule {
        metric: "startup_first_resident_ms",
        factor: 3.0,
        floor: 600.0,
    },
    Rule {
        metric: "write_visible_median_ms",
        factor: 2.0,
        floor: 50.0,
    },
    Rule {
        metric: "write_visible_max_ms",
        factor: 3.0,
        floor: 100.0,
    },
    Rule {
        metric: "swap_cpu_max_ms",
        factor: 3.0,
        floor: 5.0,
    },
    Rule {
        metric: "swap_gpu_interval_max_ms",
        factor: 3.0,
        floor: 20.0,
    },
];

fn main() {
    let root = std::env::temp_dir().join(format!("thrum-g5-bench-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("bench root");
    let mut report = String::new();
    writeln!(report, "# G5 edit-to-visible tripwires").ok();
    writeln!(
        report,
        "build: release bench, {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
    .ok();
    let mut metrics = Metrics::new();
    for files in [1_000, 10_000] {
        scan(
            &root.join(format!("scan-{files}")),
            files,
            &mut report,
            &mut metrics,
        );
    }
    edit_loop(&root.join("loop"), &mut report, &mut metrics);
    make_writable(&root);
    std::fs::remove_dir_all(&root).ok();
    // Checked against earlier runs at the same load; THRUM_TRIPWIRES_RECORD=1 appends a passing run.
    let history = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/evidence/integration/tripwires/g5-edit-to-visible.tsv");
    let record = std::env::var_os("THRUM_TRIPWIRES_RECORD").is_some();
    let verdict = tripwire::gate(&history, &tripwire::this_run(metrics), RULES, record);
    report.push_str(verdict.as_ref().map_or_else(|error| error, |ok| ok));
    std::io::stdout().write_all(report.as_bytes()).ok();
    if let Some(path) = std::env::var_os("THRUM_G5_BENCH_OUT") {
        std::fs::write(path, &report).expect("write report");
    }
    if verdict.is_err() {
        std::process::exit(1);
    }
}

fn scan(directory: &Path, files: usize, report: &mut String, metrics: &mut Metrics) {
    let old = std::time::SystemTime::now() - Duration::from_secs(60);
    for index in 0..files {
        let path = directory.join(format!("d{:03}/f{index:05}.bin", index / 100));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, vec![u8::try_from(index % 251).expect("byte"); 4096]).expect("file");
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open")
            .set_modified(old)
            .expect("age");
    }
    let mut cache = StatCache::default();
    let start = Instant::now();
    cache.scan_all(directory).expect("cold scan");
    let cold = start.elapsed();
    let warm = median((0..7).map(|_| {
        let start = Instant::now();
        let counts = cache.scan_all(directory).expect("warm scan");
        assert_eq!(counts.hashed, 0);
        start.elapsed()
    }));
    let edited = directory.join("d000/f00000.bin");
    std::fs::write(&edited, vec![7; 4096]).expect("edit");
    let start = Instant::now();
    let counts = cache.scan_all(directory).expect("one-change scan");
    let one_change = start.elapsed();
    assert_eq!(counts.hashed, 1);
    let start = Instant::now();
    cache
        .scan_paths(directory, &[PathBuf::from("d000/f00000.bin")])
        .expect("targeted scan");
    let targeted = start.elapsed();
    if files >= 10_000 {
        metrics.push((format!("scan_{files}_one_change_ms"), ms(one_change)));
        metrics.push((format!("scan_{files}_targeted_ms"), ms(targeted)));
    }
    metrics.push((format!("scan_{files}_warm_ms"), ms(warm)));
    writeln!(
        report,
        "scan {files} files x 4 KiB: cold {:.2} ms, warm no-change median {:.2} ms, one change {:.2} ms, watcher-targeted {:.3} ms",
        ms(cold),
        ms(warm),
        ms(one_change),
        ms(targeted)
    )
    .ok();
}

struct Publication {
    generation: u64,
    first_event: Instant,
    published: Instant,
    cook: Duration,
}

fn edit_loop(root: &Path, report: &mut String, metrics: &mut Metrics) {
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("dir");
    std::fs::create_dir_all(source.join("textures")).expect("dir");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 128, 0.3).expect("gltf");
    demo::write_checker_png(&source.join("textures/checker.png"), 1024, [200, 60, 40])
        .expect("png");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 64, 1).expect("volume");
    let config = CookConfig {
        source: source.clone(),
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    let publications = Arc::new(Mutex::new(Vec::<Publication>::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let (publications, stop, source) =
            (Arc::clone(&publications), Arc::clone(&stop), source.clone());
        std::thread::spawn(move || {
            let mut cooker = Cooker::open(config).expect("cooker");
            watch(&mut cooker, &source, &stop, |event| {
                if let Ok(report) = &event.report
                    && let Some(head) = report.published
                {
                    publications.lock().expect("log").push(Publication {
                        generation: head.generation,
                        first_event: event.first_event,
                        published: Instant::now(),
                        cook: report.cook_time,
                    });
                }
            })
            .expect("watch");
        })
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    while publications.lock().expect("log").is_empty() {
        assert!(Instant::now() < deadline, "initial cook never published");
        std::thread::sleep(Duration::from_millis(5));
    }

    let start = Instant::now();
    let mut viewer = Viewer::new(ViewerConfig::demo(root.join("store"))).expect("GPU device");
    let device_ready = start.elapsed();
    let mut first_resident = None;
    while first_resident.is_none() {
        if viewer.frame(0.0).generation > 0 {
            first_resident = Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    viewer.settle();

    // Editor thread: alternate texture colours with periodic mesh edits while the viewer renders.
    let writes = Arc::new(Mutex::new(Vec::<(Instant, &'static str)>::new()));
    let editor = {
        let (writes, source) = (Arc::clone(&writes), source.clone());
        std::thread::spawn(move || {
            for edit in 0..EDITS {
                std::thread::sleep(EDIT_PERIOD);
                let at = Instant::now();
                let kind = if edit % 3 == 2 {
                    demo::write_grid_gltf(
                        &source.join("meshes"),
                        "terrain",
                        128,
                        0.3 + 0.05 * edit as f32,
                    )
                    .expect("mesh edit");
                    "mesh"
                } else {
                    // Every edit is distinct content; identical content would (correctly) not republish.
                    let step = u8::try_from(edit).expect("edit");
                    let colour = [40 + 12 * step, 60, 200 - 12 * step];
                    demo::write_checker_png(&source.join("textures/checker.png"), 1024, colour)
                        .expect("texture edit");
                    "texture"
                };
                writes.lock().expect("log").push((at, kind));
            }
        })
    };
    let run_start = Instant::now();
    let mut next = Instant::now();
    let mut swap_frames = Vec::new();
    let mut last_generation = viewer.generation();
    while run_start.elapsed() < EDIT_PERIOD * u32::try_from(EDITS + 2).expect("count") {
        let frame = viewer.frame(run_start.elapsed().as_secs_f32());
        if frame.generation != last_generation {
            swap_frames.push(frame.index);
            last_generation = frame.generation;
        }
        next += FRAME_PERIOD;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    editor.join().expect("editor");
    viewer.settle();
    stop.store(true, Ordering::Relaxed);
    watcher.join().expect("watcher");

    let records = viewer.records();
    let records = records.lock().expect("records");
    let publications = publications.lock().expect("log");
    let writes = writes.lock().expect("log");
    writeln!(report, "startup: device {:.1} ms, first resident frame {:.1} ms (includes adapter/device creation)", ms(device_ready), ms(first_resident.expect("resident"))).ok();
    metrics.push((
        "startup_first_resident_ms".into(),
        ms(first_resident.expect("resident")),
    ));
    if let Some(swap) = records.swaps.first() {
        writeln!(
            report,
            "drawn profiles: texture {:?}, volume {:?}; upload chunk {} KiB",
            swap.texture_profile,
            swap.volume_profile,
            ViewerConfig::demo("").upload_chunk / 1024
        )
        .ok();
    }
    writeln!(
        report,
        "edits {} at {} ms spacing; publications after initial {}; loader errors {}",
        writes.len(),
        EDIT_PERIOD.as_millis(),
        publications.len() - 1,
        records.errors.len()
    )
    .ok();
    writeln!(report, "| edit | kind | write->event | event->publish (cook) | publish->head seen | map+upload | ->swap | ->GPU done | write->visible | uploaded |").ok();
    writeln!(report, "|---|---|---|---|---|---|---|---|---|---|").ok();
    let mut totals = Vec::new();
    for (index, (written, kind)) in writes.iter().enumerate() {
        // Edits are spaced well beyond the batch window: an edit's publication is the first whose
        // batch started after the write.
        let Some(publication) = publications.iter().find(|p| {
            p.first_event >= *written
                && writes
                    .get(index + 1)
                    .is_none_or(|(next, _)| p.first_event < *next)
        }) else {
            writeln!(report, "| {index} | {kind} | no publication |").ok();
            continue;
        };
        let generation = publication.generation;
        let (Some(swap), Some(visible)) = (
            records.swaps.iter().find(|s| s.generation == generation),
            records.visible(generation),
        ) else {
            writeln!(
                report,
                "| {index} | {kind} | no swap for generation {generation} |"
            )
            .ok();
            continue;
        };
        let total = visible.duration_since(*written);
        totals.push(total);
        writeln!(
            report,
            "| {index} | {kind} | {:.1} | {:.1} ({:.1}) | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} | {} KiB |",
            ms(publication.first_event.saturating_duration_since(*written)),
            ms(publication.published.duration_since(publication.first_event)),
            ms(publication.cook),
            ms(swap.head_seen.saturating_duration_since(publication.published)),
            ms(swap.prepared.duration_since(swap.head_seen)),
            ms(swap.swapped.duration_since(swap.prepared)),
            ms(visible.saturating_duration_since(swap.swapped)),
            ms(total),
            swap.uploaded_bytes / 1024,
        )
        .ok();
    }
    totals.sort();
    if !totals.is_empty() {
        writeln!(report, "write->visible: median {:.1} ms, max {:.1} ms (milliseconds in table; times include the 8 ms debounce)", ms(totals[totals.len() / 2]), ms(totals[totals.len() - 1])).ok();
        metrics.push((
            "write_visible_median_ms".into(),
            ms(totals[totals.len() / 2]),
        ));
        metrics.push(("write_visible_max_ms".into(), ms(totals[totals.len() - 1])));
    }
    hitch(report, &records, &swap_frames, run_start, metrics);
}

/// Hitch tripwire: render-thread CPU and GPU completion cadence on swap frames vs all others.
fn hitch(
    report: &mut String,
    records: &asset_loop::Records,
    swap_frames: &[u64],
    run_start: Instant,
    metrics: &mut Metrics,
) {
    let (mut swap_cpu, mut steady_cpu) = (Vec::new(), Vec::new());
    for frame in records.frames.iter().filter(|f| f.submitted >= run_start) {
        if swap_frames.contains(&frame.index) {
            swap_cpu.push(frame.cpu);
        } else {
            steady_cpu.push(frame.cpu);
        }
    }
    let mut completions: Vec<_> = records
        .completions
        .iter()
        .filter(|(_, at)| *at >= run_start)
        .map(|(_, at)| *at)
        .collect();
    completions.sort();
    let intervals: Vec<Duration> = completions
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect();
    let near_swap = |at: Instant| {
        records
            .swaps
            .iter()
            .any(|s| at >= s.swapped && at.duration_since(s.swapped) < Duration::from_millis(50))
    };
    let (mut swap_gpu, mut steady_gpu) = (Vec::new(), Vec::new());
    for (pair, interval) in completions.windows(2).zip(&intervals) {
        if near_swap(pair[1]) {
            swap_gpu.push(*interval);
        } else {
            steady_gpu.push(*interval);
        }
    }
    for (name, values) in [
        ("swap_cpu_max_ms", &swap_cpu),
        ("swap_gpu_interval_max_ms", &swap_gpu),
    ] {
        if let Some(max) = values.iter().max() {
            metrics.push((name.into(), ms(*max)));
        }
    }
    for (label, values) in [
        ("render CPU steady", &mut steady_cpu),
        ("render CPU swap frames", &mut swap_cpu),
        ("GPU completion interval steady", &mut steady_gpu),
        (
            "GPU completion interval within 50 ms of swap",
            &mut swap_gpu,
        ),
    ] {
        values.sort();
        if values.is_empty() {
            continue;
        }
        let at = |p: usize| ms(values[(values.len() - 1) * p / 100]);
        writeln!(
            report,
            "{label}: n {} p50 {:.3} p99 {:.3} max {:.3} ms",
            values.len(),
            at(50),
            at(99),
            at(100)
        )
        .ok();
    }
}

fn median(values: impl Iterator<Item = Duration>) -> Duration {
    let mut values: Vec<_> = values.collect();
    values.sort();
    values[values.len() / 2]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn make_writable(path: &Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut permissions = meta.permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "bench cleanup of read-only store files"
        )]
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions).ok();
        if meta.is_dir() {
            for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
                make_writable(&entry.path());
            }
        }
    }
}
