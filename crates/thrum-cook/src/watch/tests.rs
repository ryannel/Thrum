//! Exercise the command's production loop with silent or controlled notice delivery.

use std::collections::BTreeMap;
use std::fs::{File, FileTimes};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant, SystemTime};

use thrum_asset::{Manifest, Profile};

use crate::{CookConfig, Cooker, Durability, Quality, demo};

struct Fixture {
    root: thrum_test_support::Scratch,
    source: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = thrum_test_support::Scratch::new(&format!("content-loop-{name}"));
        let source = root.join("source");
        std::fs::create_dir_all(&source).expect("source");
        Self {
            root,
            source: source.canonicalize().expect("canonical root"),
        }
    }

    fn cooker(&self, store: &str) -> Cooker {
        Cooker::open(CookConfig {
            source: self.source.clone(),
            store: self.root.join(store),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker")
    }

    fn write(&self, path: &str, seed: u32) {
        let path = self.source.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        demo::write_cloud_volume(&path, 8, seed).expect("source");
    }

    fn matches_fresh(&self, manifest: &Manifest, stage: u64) {
        let mut fresh = self.cooker(&format!("fresh-{stage}"));
        let head = fresh
            .cook_all()
            .expect("fresh cook")
            .published
            .expect("fresh HEAD");
        let expected = fresh.store().load_manifest(&head).expect("fresh manifest");
        // Generation is publication history; compare every binding, kind, profile, hash and size.
        assert_eq!(
            manifest.entries().collect::<Vec<_>>(),
            expected.entries().collect::<Vec<_>>()
        );
    }
}

fn watchdog(
    stop: &Arc<AtomicBool>,
    duration: Duration,
) -> (mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    let (done, finished) = mpsc::channel();
    let stop = Arc::clone(stop);
    let thread = std::thread::spawn(move || {
        finished.recv_timeout(duration).ok();
        stop.store(true, Ordering::Relaxed);
    });
    (done, thread)
}

#[test]
fn initial_cook_and_sweep_publish_while_notice_registration_is_stalled() {
    let fixture = Fixture::new("registration");
    fixture.write("original.tvol", 1);
    let mut cooker = fixture.cooker("store");
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_secs(10));
    let (started, registering) = mpsc::channel();
    let (release, blocked) = mpsc::channel::<()>();
    let mut generation = 0;
    super::watch_with_registration(
        &mut cooker,
        &fixture.source,
        &stop,
        |event| {
            let head = event
                .report
                .as_ref()
                .expect("cycle")
                .published
                .expect("HEAD");
            generation = head.generation;
            match generation {
                1 => {
                    registering
                        .recv_timeout(Duration::from_secs(5))
                        .expect("worker started");
                    fixture.write("original.tvol", 2);
                }
                2 => {
                    let manifest = thrum_asset::AssetStore::new(fixture.root.join("store"))
                        .load_manifest(&head)
                        .expect("manifest");
                    fixture.matches_fresh(&manifest, generation);
                    stop.store(true, Ordering::Relaxed);
                }
                _ => panic!("unexpected generation {generation}"),
            }
        },
        move |_, _| {
            started.send(()).expect("registration began");
            blocked
                .recv_timeout(Duration::from_secs(15))
                .expect("registration released");
            Ok(())
        },
    )
    .expect("watch loop");
    release
        .send(())
        .expect("registration was still pending when the loop stopped");
    done.send(()).ok();
    timer.join().expect("timer");
    assert_eq!(generation, 2);
}

#[test]
fn missed_notices_publish_edits_nested_additions_and_deletions() {
    let fixture = Fixture::new("missed");
    fixture.write("original.tvol", 1);
    let mut cooker = fixture.cooker("store");
    let store = thrum_asset::AssetStore::new(fixture.root.join("store"));
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_secs(15));
    let (sender, receiver) = mpsc::channel();
    // Keep this sender alive but never send a notice, even for additions and deletion.
    let mut generation = 0;
    super::run_cycles(
        &mut cooker,
        &fixture.source,
        &stop,
        &receiver,
        Some(super::SWEEP_FLOOR),
        |event| {
            let report = event.report.as_ref().expect("cycle");
            assert!(report.failed.is_empty(), "{report:?}");
            let head = report.published.expect("changed sources publish");
            generation = head.generation;
            let manifest = store.load_manifest(&head).expect("manifest");
            fixture.matches_fresh(&manifest, generation);
            match generation {
                1 => fixture.write("original.tvol", 2),
                2 => fixture.write("new/puff.tvol", 3),
                3 => std::fs::remove_file(fixture.source.join("original.tvol")).expect("delete"),
                4 => stop.store(true, Ordering::Relaxed),
                _ => panic!("unexpected generation {generation}"),
            }
        },
    )
    .expect("watch loop");
    done.send(()).ok();
    timer.join().expect("timer");
    drop(sender);
    assert_eq!(generation, 4);
}

#[test]
fn notice_alone_uses_named_paths_and_loss_rescans_everything() {
    let fixture = Fixture::new("notices");
    fixture.write("a.tvol", 1);
    fixture.write("b.tvol", 2);
    let mut cooker = fixture.cooker("store");
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_secs(10));
    let (sender, receiver) = mpsc::channel();
    let mut cycles = 0;
    super::run_cycles(
        &mut cooker,
        &fixture.source,
        &stop,
        &receiver,
        None,
        |event| {
            let report = event.report.as_ref().expect("cycle");
            assert!(report.failed.is_empty(), "{report:?}");
            cycles += 1;
            assert_eq!(report.published.expect("publication").generation, cycles);
            match cycles {
                1 => {
                    fixture.write("a.tvol", 3);
                    sender
                        .send(super::Notice::Event(Ok(notify::Event::new(
                            notify::EventKind::Any,
                        )
                        .add_path(fixture.source.join("a.tvol")))))
                        .expect("notice");
                }
                2 => {
                    assert_eq!(event.paths, 1);
                    assert_eq!(
                        report.scan.hashed + report.scan.reused,
                        1,
                        "only named file"
                    );
                    fixture.write("b.tvol", 4);
                    sender
                        .send(super::Notice::Event(Ok(notify::Event::new(
                            notify::EventKind::Other,
                        )
                        .set_flag(notify::event::Flag::Rescan))))
                        .expect("loss notice");
                }
                3 => {
                    assert_eq!(event.paths, 0);
                    assert_eq!(report.scan.hashed + report.scan.reused, 2, "whole tree");
                    stop.store(true, Ordering::Relaxed);
                }
                _ => panic!("unexpected cycle"),
            }
        },
    )
    .expect("watch loop");
    done.send(()).ok();
    timer.join().expect("timer");
    assert_eq!(cycles, 3);
}

type StoreSnapshot = BTreeMap<PathBuf, (Vec<u8>, SystemTime)>;

fn snapshot(root: &Path) -> StoreSnapshot {
    fn walk(root: &Path, directory: &Path, out: &mut StoreSnapshot) {
        for item in std::fs::read_dir(directory).expect("directory") {
            let path = item.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).expect("relative").to_path_buf(),
                    (
                        std::fs::read(&path).expect("bytes"),
                        path.metadata()
                            .expect("metadata")
                            .modified()
                            .expect("mtime"),
                    ),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn quiet_tree_sweeps_write_nothing_and_never_call_the_callback() {
    let fixture = Fixture::new("quiet");
    fixture.write("a.tvol", 1);
    let mut cooker = fixture.cooker("store");
    cooker.cook_all().expect("initial");
    let before = snapshot(cooker.store().root());
    for _ in 0..4 {
        assert!(
            cooker
                .sweep(SystemTime::now() + Duration::from_secs(2))
                .expect("sweep")
                .is_none()
        );
    }
    assert_eq!(snapshot(cooker.store().root()), before);
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_millis(1800));
    let (_sender, receiver) = mpsc::channel();
    let mut calls = 0;
    let mut initial = None;
    let sweeps = super::run_cycles(
        &mut cooker,
        &fixture.source,
        &stop,
        &receiver,
        Some(super::SWEEP_FLOOR),
        |event| {
            calls += 1;
            assert!(event.report.as_ref().expect("cycle").published.is_none());
            initial = Some(snapshot(&fixture.root.join("store")));
        },
    )
    .expect("watch loop");
    done.send(()).ok();
    timer.join().expect("timer");
    assert_eq!(calls, 1, "only startup calls back");
    assert!(sweeps >= 3, "observed {sweeps} actual idle sweeps");
    assert_eq!(
        snapshot(cooker.store().root()),
        initial.expect("startup snapshot")
    );
}

fn modified(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .expect("file")
        .set_times(FileTimes::new().set_modified(time))
        .expect("set mtime");
}

#[test]
fn quiet_tree_with_a_bad_save_does_not_repeat_the_error_or_write() {
    let fixture = Fixture::new("quiet-bad");
    fixture.write("a.tvol", 1);
    let mut cooker = fixture.cooker("store");
    cooker.cook_all().expect("initial");
    std::fs::write(fixture.source.join("a.tvol"), b"invalid volume").expect("bad save");
    let now = SystemTime::now() + Duration::from_secs(2);
    let bad = cooker
        .sweep(now)
        .expect("sweep")
        .expect("report bad save once");
    assert_eq!(bad.failed.len(), 1);
    assert!(bad.published.is_none());
    let before = snapshot(cooker.store().root());
    for _ in 0..4 {
        assert!(cooker.sweep(now).expect("idle sweep").is_none());
    }
    assert_eq!(snapshot(cooker.store().root()), before);
    fixture.write("a.tvol", 2);
    assert!(
        cooker
            .sweep(SystemTime::now() + Duration::from_secs(2))
            .expect("fixed save")
            .expect("cycle")
            .published
            .is_some()
    );
}

#[test]
fn settling_save_uses_supplied_time_and_never_delays_startup_or_one_shot() {
    let fixture = Fixture::new("settling");
    fixture.write("a.tvol", 1);
    let time = SystemTime::now();
    let path = fixture.source.join("a.tvol");
    modified(&path, time);
    let mut cooker = fixture.cooker("store");
    assert!(
        cooker
            .cook_all()
            .expect("one-shot recent source")
            .published
            .is_some()
    );
    fixture.write("a.tvol", 2);
    modified(&path, time);
    assert!(
        cooker
            .sweep(time + Duration::from_millis(249))
            .expect("recent")
            .is_none()
    );
    assert_eq!(cooker.generation(), 1);
    assert!(
        cooker
            .sweep(time + super::MAX_BATCH)
            .expect("first eligible sweep")
            .expect("cycle")
            .published
            .is_some()
    );
    fixture.write("nested/new.tvol", 3);
    modified(&fixture.source.join("nested/new.tvol"), time);
    assert!(
        cooker
            .sweep(time + Duration::from_millis(249))
            .expect("new recent source")
            .is_none()
    );
    assert!(
        cooker
            .sweep(time + super::MAX_BATCH)
            .expect("new settled source")
            .expect("cycle")
            .published
            .is_some()
    );
    fixture.write("a.tvol", 4);
    modified(&path, time + Duration::from_secs(30));
    assert!(
        cooker
            .sweep(time)
            .expect("future mtime")
            .expect("cycle")
            .published
            .is_some()
    );
    let mut startup = fixture.cooker("startup");
    modified(&path, SystemTime::now());
    let stop = AtomicBool::new(false);
    let (_sender, receiver) = mpsc::channel();
    super::run_cycles(
        &mut startup,
        &fixture.source,
        &stop,
        &receiver,
        Some(super::SWEEP_FLOOR),
        |event| {
            assert_eq!(event.report.as_ref().expect("startup").scan.hashed, 2);
            assert!(event.report.as_ref().expect("startup").published.is_some());
            stop.store(true, Ordering::Relaxed);
        },
    )
    .expect("startup loop");
}

#[test]
fn sweep_wait_respects_floor_and_measured_cost() {
    assert_eq!(
        super::sweep_wait(super::SWEEP_FLOOR, Duration::from_millis(2)),
        super::SWEEP_FLOOR
    );
    assert_eq!(
        super::sweep_wait(super::SWEEP_FLOOR, Duration::from_millis(80)),
        Duration::from_millis(800)
    );
}

#[test]
fn unrelated_notice_traffic_cannot_postpone_a_sweep() {
    let fixture = Fixture::new("traffic");
    fixture.write("a.tvol", 1);
    fixture.write("b.tvol", 2);
    let mut cooker = fixture.cooker("store");
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_secs(10));
    let (sender, receiver) = mpsc::channel();
    let feeder_stop = Arc::clone(&stop);
    let path = fixture.source.join("b.tvol");
    let feeder = std::thread::spawn(move || {
        while !feeder_stop.load(Ordering::Relaxed) {
            if sender
                .send(super::Notice::Event(Ok(notify::Event::new(
                    notify::EventKind::Any,
                )
                .add_path(path.clone()))))
                .is_err()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let mut generation = 0;
    super::run_cycles(
        &mut cooker,
        &fixture.source,
        &stop,
        &receiver,
        Some(super::SWEEP_FLOOR),
        |event| {
            let report = event.report.as_ref().expect("cycle");
            if let Some(head) = report.published {
                generation = head.generation;
                if generation == 1 {
                    fixture.write("a.tvol", 3);
                } else {
                    assert_eq!(event.paths, 0, "the independent sweep discovered a");
                    stop.store(true, Ordering::Relaxed);
                }
            }
        },
    )
    .expect("watch loop");
    done.send(()).ok();
    timer.join().expect("timer");
    feeder.join().expect("feeder");
    assert_eq!(generation, 2);
}

#[test]
#[ignore = "manual baseline records host load and warm sweep duration; not a timing gate"]
fn quiet_tree_sweep_cost_baseline() {
    for count in [1_000, 10_000] {
        let fixture = Fixture::new(&format!("cost-{count}"));
        let time = SystemTime::now() - Duration::from_secs(3);
        for index in 0..count {
            let path = fixture.source.join(format!("file-{index}.txt"));
            std::fs::write(&path, b"source dependency").expect("file");
            modified(&path, time);
        }
        let mut cooker = fixture.cooker("store");
        cooker.cook_all().expect("warm cache");
        for _ in 0..5 {
            let start = Instant::now();
            assert!(
                cooker
                    .sweep(SystemTime::now())
                    .expect("idle sweep")
                    .is_none()
            );
            eprintln!(
                "files={count} sweep_us={} floor_ms={} multiplier={}",
                start.elapsed().as_micros(),
                super::SWEEP_FLOOR.as_millis(),
                super::SWEEP_MULTIPLIER
            );
        }
    }
}

#[test]
fn missing_notifications_reconcile_edits_additions_and_removals_without_republishing_idle_sources()
{
    let fixture = Fixture::new("reconcile-idle");
    let source = fixture.source.clone();
    let original = source.join("cloud.tvol");
    fixture.write("cloud.tvol", 1);
    let mut cooker = fixture.cooker("store");
    let store = cooker.store().clone();
    // This live channel deliberately delivers no hints. The real cycle loop must recover anyway.
    let (_events, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let (done, timer) = watchdog(&stop, Duration::from_secs(10));
    let mut old_product = None;
    let mut generation = 0;
    super::run_cycles(
        &mut cooker,
        &source,
        &stop,
        &receiver,
        Some(super::SWEEP_FLOOR),
        |event| {
            let report = event.report.as_ref().expect("cook cycle");
            assert!(report.failed.is_empty(), "{:?}", report.failed);
            assert_eq!(event.paths, 0, "no filesystem hints were sent");
            let head = store.read_head().expect("head").expect("published");
            let manifest = store.load_manifest(&head).expect("manifest");
            generation = head.generation;
            match report.published.map(|head| head.generation) {
                Some(1) => {
                    old_product = Some(
                        manifest
                            .resolve("cloud", &[Profile::Portable])
                            .unwrap()
                            .product,
                    );
                    demo::write_cloud_volume(&original, 8, 2).expect("edit without hint");
                    demo::write_cloud_volume(&source.join("new.tvol"), 8, 3)
                        .expect("add without hint");
                }
                Some(2) => {
                    assert_ne!(
                        manifest
                            .resolve("cloud", &[Profile::Portable])
                            .unwrap()
                            .product,
                        old_product.unwrap()
                    );
                    assert!(manifest.resolve("new", &[Profile::Portable]).is_some());
                    std::fs::remove_file(&original).expect("remove without hint");
                }
                Some(3) => {
                    assert!(manifest.resolve("cloud", &[Profile::Portable]).is_none());
                    assert!(manifest.resolve("new", &[Profile::Portable]).is_some());
                    stop.store(true, Ordering::Relaxed);
                }
                generation => panic!("unexpected generation {generation:?}"),
            }
        },
    )
    .expect("watch cycles");
    done.send(()).ok();
    timer.join().expect("timer");
    assert_eq!(
        generation, 3,
        "missing notifications recover before the watchdog"
    );
    assert!(
        cooker
            .sweep(SystemTime::now())
            .expect("idle sweep")
            .is_none()
    );
    assert_eq!(
        store
            .read_head()
            .expect("head")
            .expect("published")
            .generation,
        3
    );
}
