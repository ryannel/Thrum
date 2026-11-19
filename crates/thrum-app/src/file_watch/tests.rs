use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use super::{FileWatch, Kinds, MISSING_GRACE, Reload, TICK, deliver};

const TEST_TICK: Duration = Duration::from_millis(25);

/// A fresh folder under the system temporary directory, removed when dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "thrum-file-watch-{name}-{}-{:?}",
            std::process::id(),
            Instant::now()
        ));
        std::fs::create_dir_all(&path).expect("temporary folder");
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

struct Watching {
    watch: FileWatch,
    wakes: Arc<AtomicUsize>,
    tick: Duration,
}

fn watch(files: &[(&Path, Kinds)]) -> Watching {
    watch_every(files, TEST_TICK)
}

fn watch_every(files: &[(&Path, Kinds)], tick: Duration) -> Watching {
    let wakes = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&wakes);
    let watch = FileWatch::start(
        files
            .iter()
            .map(|(path, kind)| (path.to_path_buf(), *kind))
            .collect(),
        tick,
        Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .expect("spawn")
    .expect("files to watch");
    Watching { watch, wakes, tick }
}

impl Watching {
    /// Wait until `count` wakes have arrived, then a further six ticks for any extra one.
    fn settled(&self, count: usize) -> usize {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.wakes.load(Ordering::SeqCst) < count && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(self.tick * 6);
        self.wakes.load(Ordering::SeqCst)
    }
}

#[test]
fn an_in_place_save_is_delivered_once_with_its_kind() {
    let folder = Folder::new("in-place");
    let params = folder.file("starter.params.json");
    std::fs::write(&params, "{}").expect("write");
    let watching = watch(&[(&params, Kinds::PARAMS)]);
    std::fs::write(&params, r#"{"sun.elevation": 30}"#).expect("save");
    assert_eq!(watching.settled(1), 1);
    assert_eq!(watching.watch.take(), Kinds::PARAMS);
    assert_eq!(watching.watch.take(), Kinds::default(), "taking clears");
}

#[test]
fn a_rename_save_is_delivered_once() {
    let folder = Folder::new("rename");
    let material = folder.file("material.wgsl");
    std::fs::write(&material, "// a").expect("write");
    let watching = watch(&[(&material, Kinds::GAME)]);
    let temporary = folder.file(".material.wgsl.tmp");
    std::fs::write(&temporary, "// b").expect("write temporary");
    std::fs::rename(&temporary, &material).expect("rename");
    assert_eq!(watching.settled(1), 1);
    assert_eq!(watching.watch.take(), Kinds::GAME);
}

#[test]
fn a_burst_of_writes_is_delivered_once() {
    let folder = Folder::new("burst");
    let params = folder.file("p.json");
    std::fs::write(&params, "0").expect("write");
    // The window's own tick: an editor's burst lands well inside it.
    let watching = watch_every(&[(&params, Kinds::PARAMS)], TICK);
    for value in 1..=5 {
        std::fs::write(&params, value.to_string()).expect("write");
        std::thread::sleep(Duration::from_millis(3));
    }
    assert_eq!(watching.settled(1), 1);
}

#[test]
fn a_touch_or_an_identical_rewrite_is_never_delivered() {
    let folder = Folder::new("same");
    let params = folder.file("p.json");
    std::fs::write(&params, "{}").expect("write");
    let watching = watch(&[(&params, Kinds::PARAMS)]);
    std::thread::sleep(TEST_TICK * 2);
    std::fs::File::options()
        .append(true)
        .open(&params)
        .and_then(|file| file.set_modified(std::time::SystemTime::now()))
        .expect("touch");
    std::fs::write(&params, "{}").expect("rewrite");
    let temporary = folder.file("p.tmp");
    std::fs::write(&temporary, "{}").expect("write temporary");
    std::fs::rename(&temporary, &params).expect("rename the same bytes");
    assert_eq!(watching.settled(0), 0);
}

#[test]
fn a_delete_then_create_save_is_delivered_once_and_a_lasting_absence_is_delivered() {
    let folder = Folder::new("missing");
    let params = folder.file("p.json");
    std::fs::write(&params, "a").expect("write");
    let watching = watch(&[(&params, Kinds::PARAMS)]);
    std::fs::remove_file(&params).expect("delete");
    std::thread::sleep(TEST_TICK * 3);
    std::fs::write(&params, "b").expect("create");
    assert_eq!(watching.settled(1), 1, "the new bytes only");
    std::fs::remove_file(&params).expect("delete");
    // Past the grace the reload runs, so it can report the missing file.
    std::thread::sleep(MISSING_GRACE);
    assert_eq!(watching.settled(2), 2);
    std::fs::write(&params, "b").expect("restore");
    assert_eq!(
        watching.settled(3),
        3,
        "bytes after an absence are a change"
    );
}

#[test]
fn a_file_missing_at_start_is_delivered_when_created() {
    let folder = Folder::new("late");
    let material = folder.file("material.wgsl");
    let watching = watch(&[(&material, Kinds::GAME)]);
    assert_eq!(watching.settled(0), 0);
    std::fs::write(&material, "// new").expect("create");
    assert_eq!(watching.settled(1), 1);
}

#[test]
fn saves_to_both_kinds_before_the_event_loop_runs_coalesce_into_one_take() {
    let folder = Folder::new("both");
    let params = folder.file("p.json");
    let material = folder.file("m.wgsl");
    std::fs::write(&params, "a").expect("write");
    std::fs::write(&material, "a").expect("write");
    let watching = watch(&[(&params, Kinds::PARAMS), (&material, Kinds::GAME)]);
    std::fs::write(&params, "b").expect("save");
    std::fs::write(&material, "b").expect("save");
    watching.settled(1);
    assert_eq!(watching.watch.take(), Kinds::ALL);
}

#[test]
fn stopping_joins_without_waiting_for_a_tick_and_no_wake_follows() {
    let folder = Folder::new("stop");
    let params = folder.file("p.json");
    std::fs::write(&params, "a").expect("write");
    let wakes = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&wakes);
    let mut watch = FileWatch::start(
        vec![(params.clone(), Kinds::PARAMS)],
        Duration::from_secs(10),
        Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .expect("spawn")
    .expect("files to watch");
    let started = Instant::now();
    watch.stop();
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
    std::fs::write(&params, "b").expect("save after stop");
    std::thread::sleep(TEST_TICK * 4);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn nothing_to_watch_starts_no_thread() {
    let watch = FileWatch::start(Vec::new(), TEST_TICK, Box::new(|| {})).expect("no spawn");
    assert!(watch.is_none());
}

/// Records the reload body's calls in order.
#[derive(Default)]
struct Recorder {
    stopped: bool,
    calls: Vec<&'static str>,
}

impl Reload for Recorder {
    fn stopped(&self) -> bool {
        self.stopped
    }

    fn reload_params(&mut self, saved: bool) -> Result<(), String> {
        self.calls
            .push(if saved { "params saved" } else { "params key" });
        Ok(())
    }

    fn reload_game(&mut self) {
        self.calls.push("game");
    }

    fn redraw(&self) {}
}

#[test]
fn a_parameter_save_never_reloads_the_game_and_l_reloads_both_in_order() {
    let mut host = Recorder::default();
    deliver(&mut host, Kinds::PARAMS, true).expect("params");
    assert_eq!(host.calls, ["params saved"]);
    host.calls.clear();
    deliver(&mut host, Kinds::GAME, true).expect("game");
    assert_eq!(
        host.calls,
        ["game"],
        "a game save leaves the parameter file unread"
    );
    host.calls.clear();
    deliver(&mut host, Kinds::ALL, false).expect("L");
    assert_eq!(host.calls, ["params key", "game"]);
}

#[test]
fn a_save_landing_during_shutdown_is_dropped() {
    let mut host = Recorder {
        stopped: true,
        ..Recorder::default()
    };
    deliver(&mut host, Kinds::ALL, true).expect("dropped");
    assert_eq!(host.calls, Vec::<&str>::new());
}
