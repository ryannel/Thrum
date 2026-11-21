//! Small host-only contributor command runner.

use std::{
    env,
    error::Error,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

mod nextest;
mod selection;

use nextest::{
    Rerun, SLOW_TEST_SECONDS, TestTime, junit_times, passes_after_reruns, rerun_filter,
    rerun_lines, reruns, run_nextest, strip_filters,
};

type TaskResult = Result<(), Box<dyn Error>>;

fn run(program: &str, args: &[&str]) -> TaskResult {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(format!("{program} {} failed ({status})", args.join(" ")).into());
    }
    Ok(())
}

fn cargo(args: &[&str]) -> TaskResult {
    run(env!("CARGO"), args)
}

fn python(args: &[&str]) -> TaskResult {
    run(
        &env::var("THRUM_PYTHON").unwrap_or_else(|_| "python3".into()),
        args,
    )
}

/// Script suites that `./dev check` leaves out, each with the reason. Every other
/// `scripts/test_*.py`, and every `scripts/test_*.mjs` where node is installed, runs.
/// Each is named by its stem, without `.py`, so the shape rule `script-caller` does
/// not take an exclusion for a caller of the suite or of the script it tests.
const EXCLUDED_SCRIPT_SUITES: &[(&str, &str)] = &[
    (
        "test_mac_m2_presented",
        "fails at DEAD-TOOLS M1's starting commit 7e30ba7e4 with six errors, each a missing \
         traced-window record of its 11 November capture; its script is \
         PLATFORM-RUNTIME-MAC's, and the cleanup review found it dead (P1-02, P2-04, L5-03)",
    ),
    (
        "test_mac_m3_commission",
        "tests a PLATFORM-RUNTIME-MAC qualification script the cleanup review found dead \
         (P1-02, P2-04, L5-03), left for a second DEAD-TOOLS landing",
    ),
    (
        "test_qualify_macos_display",
        "tests a PLATFORM-RUNTIME-MAC qualification script the cleanup review found dead \
         (P1-02, P2-04, L5-03), left for a second DEAD-TOOLS landing",
    ),
];

/// Runs every script suite under `scripts/` that is not excluded by name above.
fn script_suites() -> TaskResult {
    let mut suites = Vec::new();
    for entry in std::fs::read_dir("scripts")? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let extension = Path::new(&name).extension().and_then(|e| e.to_str());
        if name.starts_with("test_") && matches!(extension, Some("py" | "mjs")) {
            suites.push(name);
        }
    }
    suites.sort();
    let stem = |name: &str| {
        Path::new(name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
    };
    for (name, _) in EXCLUDED_SCRIPT_SUITES {
        if !suites
            .iter()
            .any(|suite| stem(suite).as_deref() == Some(*name))
        {
            return Err(format!(
                "excluded script suite {name} no longer exists; drop its exclusion"
            )
            .into());
        }
    }
    let node = Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    for name in &suites {
        if let Some((_, reason)) = EXCLUDED_SCRIPT_SUITES
            .iter()
            .find(|(excluded, _)| stem(name).as_deref() == Some(*excluded))
        {
            writeln!(
                std::io::stdout().lock(),
                "script suite {name} excluded: {reason}"
            )?;
            continue;
        }
        let path = format!("scripts/{name}");
        if Path::new(name).extension().and_then(|e| e.to_str()) == Some("py") {
            // Discovery imports the module and collects every test case in it, so a test
            // defined after a `__main__` guard, or a suite with no guard, still runs.
            python(&["-m", "unittest", "discover", "-s", "scripts", "-p", name])?;
        } else if node {
            run("node", &["--test", &path])?;
        } else {
            writeln!(
                std::io::stdout().lock(),
                "script suite {name} skipped: node is not installed"
            )?;
        }
    }
    Ok(())
}

fn check_web() -> TaskResult {
    cargo(&[
        "clippy",
        "--locked",
        "--offline",
        "-p",
        "thrum-transport",
        "-p",
        "link-proof",
        "--lib",
        "--features",
        "thrum-transport/browser",
        "--target",
        "wasm32-unknown-unknown",
        "--",
        "-D",
        "warnings",
    ])?;
    let lint = |features: &[&str]| {
        let mut args = vec![
            "clippy",
            "--locked",
            "--offline",
            "--package",
            "v1-game",
            "--package",
            "thrum-app",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
        ];
        args.extend_from_slice(features);
        args.extend_from_slice(&["--", "-D", "warnings"]);
        cargo(&args)
    };
    // The engine's web shell with the default page, and the development page with the
    // material hook reload.
    lint(&[])?;
    lint(&["--features", "dev-material"])?;
    cargo(&[
        "clippy",
        "--locked",
        "--offline",
        "-p",
        "thrum-starter",
        "-p",
        "thrum-app",
        "--all-targets",
        "--target",
        "wasm32-unknown-unknown",
        "--",
        "-D",
        "warnings",
    ])
}

fn scene_package(folder: &str) -> Result<(String, String), Box<dyn Error>> {
    let output = Command::new(env::var("THRUM_PYTHON").unwrap_or_else(|_| "python3".into()))
        .args(["scripts/scene_package.py", folder])
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(
            format!("{folder} is not a runnable scene package; the reason is above").into(),
        );
    }
    let line = String::from_utf8(output.stdout)?;
    let (package, binary) = line
        .trim_end()
        .split_once('\t')
        .ok_or("scripts/scene_package.py printed no package and binary")?;
    Ok((package.to_owned(), binary.to_owned()))
}

/// Build a scene package and the cooker, cook the scene's `assets/` into a store under the
/// build directory, keep G5's watcher cooking edits while the scene's window is open, and stop
/// the watcher when the window closes or the scene fails. The scene gets `THRUM_ASSET_STORE`
/// and, unless already set, `THRUM_PARAMS` for `<folder>/<folder name>.params.json` and
/// `THRUM_MATERIAL` for `<folder>/material.wgsl` when they exist; the window reloads both on
/// `L`. `THRUM_SCENE_ASSETS` cooks another source folder, such as a copy to experiment on.
fn scene(root: &Path, folder: &str) -> TaskResult {
    if !cfg!(target_os = "macos") {
        return Err("scene windows run on macOS only".into());
    }
    let folder = folder.trim_end_matches('/');
    let directory = root.join(folder);
    let (package, binary) = scene_package(folder)?;
    // `./dev` has already added a new scene package to Cargo.lock.
    cargo(&[
        "build",
        "--locked",
        "--offline",
        "--release",
        "--package",
        &package,
        "--bin",
        &binary,
        "--package",
        "thrum-cook",
        "--bins",
    ])?;
    let target = env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let release = target.join("release");
    let cook = release.join("thrum-cook");
    let source =
        env::var_os("THRUM_SCENE_ASSETS").map_or_else(|| directory.join("assets"), PathBuf::from);
    if !source.is_dir() {
        return Err(format!(
            "{} does not exist; put the scene's glTF meshes and PNG textures there",
            source.display()
        )
        .into());
    }
    let store = target.join("scenes").join(&package).join("store");
    let status = Command::new(&cook)
        .arg("cook")
        .arg(&source)
        .arg(&store)
        .status()?;
    if !status.success() {
        return Err(format!(
            "cooking {} into {} failed ({status}); fix the asset the cooker names and rerun ./dev scene {folder}",
            source.display(),
            store.display()
        )
        .into());
    }
    let mut run = Command::new(release.join(&binary));
    run.env("THRUM_ASSET_STORE", &store);
    let name = Path::new(folder).file_name().unwrap_or_default();
    let params = directory.join(format!("{}.params.json", name.to_string_lossy()));
    if env::var_os("THRUM_PARAMS").is_none() && params.is_file() {
        run.env("THRUM_PARAMS", params);
    }
    let material = directory.join("material.wgsl");
    if env::var_os("THRUM_MATERIAL").is_none() && material.is_file() {
        run.env("THRUM_MATERIAL", material);
    }
    let mut watcher = Command::new(&cook)
        .arg("watch")
        .arg(&source)
        .arg(&store)
        .spawn()?;
    let ran = run.status();
    // Stop the watcher however the scene ended.
    let stopped = watcher.kill().and_then(|()| watcher.wait());
    let ran = ran?;
    stopped.map_err(|error| format!("stopping thrum-cook watch: {error}"))?;
    if !ran.success() {
        return Err(format!("{package} failed ({ran}); its message is above").into());
    }
    Ok(())
}

/// One test binary of the main-tier list: `<package> <test binary> <build>`.
struct LongTestBinary {
    package: String,
    binary: String,
    release: bool,
}

/// One feature line of the main-tier list: `<package> features=<feature>[,...] [<test binary>...]`.
/// With no binary named it covers every test binary of the package; `lib` names the library's tests.
struct FeatureTests {
    package: String,
    features: Vec<String>,
    binaries: Vec<String>,
}

/// The main-tier list, `.config/long-tests.txt`.
struct MainTierList {
    long: Vec<LongTestBinary>,
    features: Vec<FeatureTests>,
    /// `<package> <test binary>` of each not-run line: it runs in no tier until its owner fixes it.
    not_run: Vec<String>,
}

fn main_tier_list(text: &str) -> Result<MainTierList, String> {
    let mut list = MainTierList {
        long: Vec::new(),
        features: Vec::new(),
        not_run: Vec::new(),
    };
    for (index, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        // A binary that failed when it was listed; `./dev check-docs` checks the line's shape.
        if let [package, "not-run", binary, ..] = fields.as_slice() {
            list.not_run.push(format!("{package} {binary}"));
            continue;
        }
        if let [package, features, binaries @ ..] = fields.as_slice()
            && let Some(features) = features.strip_prefix("features=")
        {
            list.features.push(FeatureTests {
                package: (*package).into(),
                features: features
                    .split(',')
                    .filter(|feature| !feature.is_empty())
                    .map(Into::into)
                    .collect(),
                binaries: binaries.iter().map(|&binary| binary.into()).collect(),
            });
            continue;
        }
        let release = match fields.as_slice() {
            [_, _, "release"] => true,
            [_, _, "debug"] => false,
            _ => {
                return Err(format!(
                    ".config/long-tests.txt:{}: expected `<package> <test binary> release|debug` \
                     or `<package> features=<feature>[,...] [<test binary>...]`",
                    index + 1
                ));
            }
        };
        list.long.push(LongTestBinary {
            package: fields[0].into(),
            binary: fields[1].into(),
            release,
        });
    }
    Ok(list)
}

fn read_main_tier_list() -> Result<MainTierList, Box<dyn Error>> {
    Ok(main_tier_list(&std::fs::read_to_string(
        ".config/long-tests.txt",
    )?)?)
}

/// Builds and lints every target of each package the list's feature lines name, with all their
/// features on at once, under the workspace's lints. One invocation, so the dependency graph is
/// resolved once and an unchanged crate's build is reused from the last run.
fn lint_feature_tests() -> TaskResult {
    let list = read_main_tier_list()?;
    let mut args: Vec<String> = ["clippy", "--locked", "--offline", "--all-targets"]
        .map(Into::into)
        .into();
    let mut features = Vec::new();
    for entry in &list.features {
        if !args.contains(&entry.package) {
            args.extend(["--package".into(), entry.package.clone()]);
        }
        // With several packages selected, a feature is named through its package.
        features.extend(entry.features.iter().map(|feature| {
            if feature.contains('/') {
                feature.clone()
            } else {
                format!("{}/{feature}", entry.package)
            }
        }));
    }
    if features.is_empty() {
        return Ok(());
    }
    args.extend(["--features".into(), features.join(",")]);
    args.extend(["--", "-D", "warnings"].map(Into::into));
    cargo(&args.iter().map(String::as_str).collect::<Vec<_>>())
}

fn main_tier_cargo(args: &[&str]) -> Result<std::process::ExitStatus, std::io::Error> {
    Command::new(env!("CARGO"))
        .args(args)
        .env("THRUM_REQUIRE_GPU", "1")
        .env("CARGO_INCREMENTAL", "0")
        // Release keeps overflow checks; debug assertions stay off, as the flythroughs
        // choose their length by them.
        .env("CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS", "true")
        .status()
}

/// Runs the main tier: the ignored tests of every long-test binary on the list, each binary once,
/// then the tests that are not ignored behind each feature line, in the debug build with its
/// features on. Fails if any fails. A GPU is required, so a test that finds no adapter fails
/// rather than skips.
#[expect(
    clippy::print_stderr,
    reason = "the main-branch check's log names each binary listed as not run"
)]
fn long_tests() -> TaskResult {
    let list = read_main_tier_list()?;
    let mut failed = Vec::new();
    for entry in &list.long {
        let mut args = vec!["test", "--locked", "--offline"];
        if entry.release {
            args.push("--release");
        }
        args.extend([
            "-p",
            &entry.package,
            "--test",
            &entry.binary,
            "--",
            "--ignored",
        ]);
        if !main_tier_cargo(&args)?.success() {
            failed.push(format!("{} {}", entry.package, entry.binary));
        }
    }
    for entry in &list.features {
        let features = entry.features.join(",");
        let mut args = vec![
            "test",
            "--locked",
            "--offline",
            "-p",
            &entry.package,
            "--features",
            &features,
        ];
        if entry.binaries.is_empty() {
            // Every test binary, the library's own tests included.
            args.push("--tests");
        }
        for binary in &entry.binaries {
            // `lib` names the library's own tests.
            if binary == "lib" {
                args.push("--lib");
            } else {
                args.extend(["--test", binary]);
            }
        }
        if !main_tier_cargo(&args)?.success() {
            failed.push(format!("{} with {features}", entry.package));
        }
    }
    for binary in &list.not_run {
        eprintln!(
            "not run: {binary}, listed as not run in .config/long-tests.txt with its failure"
        );
    }
    if !failed.is_empty() {
        return Err(format!("long tests failed: {}", failed.join(", ")).into());
    }
    Ok(())
}

/// The oldest cargo-nextest `./dev check` accepts; `.config/nextest.toml` names the same.
const NEXTEST_MIN: (u64, u64, u64) = (0, 9, 150);

/// Fails before any step when cargo-nextest is missing or older than [`NEXTEST_MIN`].
fn require_nextest() -> TaskResult {
    let (major, minor, patch) = NEXTEST_MIN;
    let install = format!(
        "./dev check runs its tests through cargo-nextest {major}.{minor}.{patch} or newer; \
         install it with `cargo install --locked cargo-nextest`"
    );
    let found = Command::new(env!("CARGO"))
        .args(["nextest", "--version"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| nextest_version(&String::from_utf8_lossy(&output.stdout)));
    match found {
        Some(version) if version >= NEXTEST_MIN => Ok(()),
        Some((a, b, c)) => Err(format!("{install} (found {a}.{b}.{c})").into()),
        None => Err(format!("{install} (it is not installed)").into()),
    }
}

/// The version in `cargo-nextest 0.9.150 (d225b1daf 2026-10-26)`.
fn nextest_version(text: &str) -> Option<(u64, u64, u64)> {
    let version = text
        .lines()
        .next()?
        .strip_prefix("cargo-nextest ")?
        .split_whitespace()
        .next()?;
    let mut parts = version.split('-').next()?.split('.').map(str::parse::<u64>);
    Some((
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

/// What `./dev check` has done so far: each step's wall time, and each test's time.
#[derive(Default)]
struct CheckRecord {
    steps: Vec<(String, std::time::Duration, bool)>,
    tests: Vec<TestTime>,
    /// Each test the limit stopped and its second run alone.
    reruns: Vec<Rerun>,
    /// When the check started, before the cargo-nextest version check.
    started: Option<std::time::Instant>,
}

impl CheckRecord {
    fn step(&mut self, name: &str, work: impl FnOnce(&mut Self) -> TaskResult) -> TaskResult {
        writeln!(std::io::stdout().lock(), "==> {name}")?;
        let start = std::time::Instant::now();
        let result = work(self);
        self.steps
            .push((name.to_owned(), start.elapsed(), result.is_ok()));
        result
    }

    /// Runs cargo-nextest with `args` after `cargo nextest run`, then reads its JUnit report.
    /// `probe` marks the draw-probe pass, whose tests are counted apart from the workspace's. A test
    /// the limit stopped is run once more after the others, alone, with the same build and limit,
    /// and the pass fails unless each such test passes then (GATE-TIME mechanism 13).
    fn nextest(&mut self, junit: &Path, probe: bool, args: &[&str]) -> TaskResult {
        let (ran, xml) = run_nextest(junit, args)?;
        let Some(xml) = xml else { return ran };
        let mut times = junit_times(&xml);
        for test in &mut times {
            test.probe = probe;
        }
        let stopped: Vec<&TestTime> = times.iter().filter(|test| test.timed_out).collect();
        let mut again = Vec::new();
        if !stopped.is_empty() {
            {
                let mut out = std::io::stdout().lock();
                for test in &stopped {
                    writeln!(
                        out,
                        "test stopped at {SLOW_TEST_SECONDS} s side by side: {}; it runs once more, alone, \
                         after the others",
                        test.name
                    )?;
                }
            }
            // The same build and arguments, with only the stopped tests selected, one at a time.
            let filter = rerun_filter(&stopped);
            let mut alone: Vec<&str> = strip_filters(args);
            alone.extend_from_slice(&["-E", &filter, "--test-threads", "1"]);
            let (_, xml) = run_nextest(junit, &alone)?;
            let alone_times = xml.as_deref().map_or_else(Vec::new, junit_times);
            again = reruns(&stopped, &alone_times);
            let mut out = std::io::stdout().lock();
            for rerun in again.iter().filter(|rerun| !rerun.passed) {
                writeln!(
                    out,
                    "test stopped: {} ran for over {SLOW_TEST_SECONDS} s side by side and did not pass \
                     within {SLOW_TEST_SECONDS} s alone. A test in a merge's check takes about ten \
                     seconds or less alone: make it fast, or mark it #[ignore = \"long: <reason>\"] and \
                     list its binary in .config/long-tests.txt (ORCHESTRATE.md, Test tiers)",
                    rerun.name
                )?;
            }
        }
        let passed = ran.is_ok() || passes_after_reruns(&times, &again);
        let failed_alone: Vec<String> = again
            .iter()
            .filter(|rerun| !rerun.passed)
            .map(|rerun| rerun.name.clone())
            .collect();
        self.tests.extend(times);
        self.reruns.extend(again);
        if passed {
            Ok(())
        } else if !failed_alone.is_empty() && ran.is_err() {
            Err(format!(
                "{} (and {} test(s) stopped at {SLOW_TEST_SECONDS} s failed again alone: {})",
                ran.err()
                    .map_or_else(String::new, |error| error.to_string()),
                failed_alone.len(),
                failed_alone.join(", ")
            )
            .into())
        } else {
            ran
        }
    }

    /// The closing summary, printed whether the check passed or failed.
    fn summary(&mut self) -> TaskResult {
        let mut out = std::io::stdout().lock();
        writeln!(out, "\n./dev check step times (wall):")?;
        for (name, elapsed, passed) in &self.steps {
            let mark = if *passed { "" } else { "  FAILED" };
            writeln!(out, "  {:>8.1} s  {name}{mark}", elapsed.as_secs_f64())?;
        }
        if let Some(started) = self.started {
            writeln!(
                out,
                "  {:>8.1} s  total, the whole check from its cargo-nextest version check",
                started.elapsed().as_secs_f64()
            )?;
        }
        self.tests.sort_by(|a, b| b.seconds.total_cmp(&a.seconds));
        let label = |test: &TestTime| {
            if test.probe {
                format!("{} (draw-probe pass)", test.name)
            } else {
                test.name.clone()
            }
        };
        if !self.tests.is_empty() {
            writeln!(out, "ten slowest tests:")?;
            for test in self.tests.iter().take(10) {
                writeln!(out, "  {:>8.1} s  {}", test.seconds, label(test))?;
            }
            let workspace = self.tests.iter().filter(|test| !test.probe).count();
            let slow: Vec<&TestTime> = self
                .tests
                .iter()
                .filter(|test| test.seconds > SLOW_TEST_SECONDS)
                .collect();
            writeln!(
                out,
                "{} of the workspace pass's {workspace} tests and {} of the draw-probe pass's {} ran for \
                 over {SLOW_TEST_SECONDS} s, the merge tier's bound for one test, at which one is stopped \
                 unless .config/nextest.toml exempts it by name:",
                slow.iter().filter(|test| !test.probe).count(),
                slow.iter().filter(|test| test.probe).count(),
                self.tests.len() - workspace,
            )?;
            for test in slow {
                writeln!(out, "  {:>8.1} s  {}", test.seconds, label(test))?;
            }
        }
        if !self.tests.is_empty() {
            for line in rerun_lines(&self.reruns) {
                writeln!(out, "{line}")?;
            }
        }
        for line in summed_times(&self.tests) {
            writeln!(out, "{line}")?;
        }
        Ok(())
    }
}

/// Whether a test counts as a GPU test in the closing lines: its binary or its name says `gpu` or
/// `metal`, as GATE-TIME's measurement of 21 November counted them. A GPU test named otherwise is not
/// counted.
fn gpu_named(test: &TestTime) -> bool {
    let name = test.name.to_ascii_lowercase();
    name.contains("gpu") || name.contains("metal")
}

/// The two closing lines on the tests' time: the summed time of every test the run ran, and of its
/// GPU tests. Reports only; nothing fails on either.
fn summed_times(tests: &[TestTime]) -> [String; 2] {
    let all = tests.iter().fold(0.0_f64, |sum, test| sum + test.seconds);
    let gpu: Vec<&TestTime> = tests.iter().filter(|test| gpu_named(test)).collect();
    let gpu_seconds = gpu.iter().fold(0.0_f64, |sum, test| sum + test.seconds);
    let share = if all > 0.0 {
        100.0 * gpu_seconds / all
    } else {
        0.0
    };
    [
        format!(
            "tests' summed time: {all:.1} s over the {} tests cargo-nextest ran in this check, each \
             test's own time added up; doc tests are not counted (a report, never a gate)",
            tests.len()
        ),
        format!(
            "GPU tests' summed time: {gpu_seconds:.1} s over {} tests, {share:.0}% of the summed time; a GPU \
             test here is one whose binary or name says `gpu` or `metal`, so one named otherwise is not counted",
            gpu.len()
        ),
    ]
}

/// cargo-nextest's store, relative to the workspace root, as `[store] dir` in `.config/nextest.toml`
/// names it. Named there because cargo-nextest otherwise picks its own place whatever
/// `CARGO_TARGET_DIR` says, and the check reads its JUnit report from here.
const NEXTEST_STORE: &str = "target/nextest";

/// The draw-probe pass runs only the cooked-asset tests that assert on the draw probe; the
/// workspace pass runs every cooked-asset test against the production build.
const DRAW_PROBE_TESTS: &[&str] = &[
    "a_fetched_edit_swaps_in_and_reuses_unchanged_products",
    "a_publication_larger_than_staging_streams_through_the_bounded_ring",
    "a_newer_publication_supersedes_an_upload_still_being_filled",
    "adapter_less_public_host_selects_direct",
];

/// `./dev check`: every step in turn, stopping at the first that fails, then the time of each
/// step that ran and the slowest tests. With `all`, as `./dev check-all`, every test binary of the
/// workspace runs; without it, those of the packages the change can reach (GATE-TIME mechanism 4).
fn check(root: &Path, all: bool) -> TaskResult {
    let started = std::time::Instant::now();
    require_nextest()?;
    let junit = root.join(NEXTEST_STORE).join("default/junit.xml");
    let mut record = CheckRecord {
        started: Some(started),
        ..CheckRecord::default()
    };
    let mut chosen = None;
    let result = record
        .step("selection", |_| {
            let (selection, lines) = if all {
                let selection = selection::Selection::Whole(
                    "`./dev check-all` runs every test binary, whatever changed".to_owned(),
                );
                let lines = selection::report(&selection, None);
                (selection, lines)
            } else {
                selection::for_checkout(root)
            };
            let mut out = std::io::stdout().lock();
            for line in lines {
                writeln!(out, "{line}")?;
            }
            chosen = Some(selection);
            Ok(())
        })
        .and_then(|()| {
            let chosen = chosen.as_ref().ok_or("the selection step chose nothing")?;
            check_steps(&mut record, &junit, chosen)
        });
    record.summary()?;
    result
}

/// Whether the game's package, whose draw-probe pass is a second build of one of its test
/// binaries, is selected.
fn game_selected(chosen: &selection::Selection) -> bool {
    match chosen {
        selection::Selection::Whole(_) => true,
        selection::Selection::Packages(packages) => packages.contains_key("v1-game"),
    }
}

fn check_steps(
    record: &mut CheckRecord,
    junit: &Path,
    chosen: &selection::Selection,
) -> TaskResult {
    record.step("2D profile guard", |_| {
        python(&["scripts/sim2d_profile.py", "check"])?;
        python(&["scripts/sim2d_profile.py", "controls"])
    })?;
    record.step("exact-baseline reference guard", |_| {
        python(&["scripts/exact_baselines.py", "guard"])
    })?;
    record.step("repository guards", |_| python(&["scripts/repo_check.py"]))?;
    record.step("script suites", |_| script_suites())?;
    record.step("rustfmt", |_| cargo(&["fmt", "--all", "--", "--check"]))?;
    record.step("Clippy, workspace", |_| {
        cargo(&[
            "clippy",
            "--locked",
            "--offline",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
    })?;
    record.step("Clippy, wasm", |_| check_web())?;
    // Tests behind a Cargo feature, from the main-tier list; `./dev long-tests` runs them.
    record.step("Clippy, tests behind a feature", |_| lint_feature_tests())?;
    // Every test binary of the workspace, built once with the workspace's features, each test in
    // a process of its own; the selection decides only which packages' binaries run.
    record.step(
        "tests, side by side",
        |record| match selection::nextest_filter(chosen) {
            None => record.nextest(junit, false, &["--workspace"]),
            Some(filter) => record.nextest(
                junit,
                false,
                &["--workspace", "-E", &filter, "--no-tests=pass"],
            ),
        },
    )?;
    record.step("doc tests", |_| {
        cargo(&["test", "--locked", "--offline", "--workspace", "--doc"])
    })?;
    if !game_selected(chosen) {
        writeln!(
            std::io::stdout().lock(),
            "==> draw-probe tests: not run, because the selection leaves out v1-game"
        )?;
        return Ok(());
    }
    // Qualification-only observations inspect the real bound buffers and commands.
    record.step("draw-probe tests", |record| {
        let filter = DRAW_PROBE_TESTS
            .iter()
            .map(|name| format!("test(={name})"))
            .collect::<Vec<_>>()
            .join(" | ");
        let before = record.tests.len();
        record.nextest(
            junit,
            true,
            &[
                "-p",
                "v1-game",
                "--features",
                "draw-probe",
                "--test",
                "cooked_assets_gpu",
                "-E",
                &filter,
            ],
        )?;
        let ran = record.tests.len() - before;
        // The report was read, so a count other than the list's is a renamed or removed test.
        if ran != DRAW_PROBE_TESTS.len() {
            return Err(format!(
                "the draw-probe pass ran {ran} tests where it names {}; a test it names was renamed or removed",
                DRAW_PROBE_TESTS.len()
            )
            .into());
        }
        Ok(())
    })
}

/// `./dev selection`: what `./dev check` selects for this checkout's change and why, or with
/// `--paths`, for the paths on standard input, one to a line, against the checkout's graph.
fn print_selection(root: &Path, flag: Option<&str>) -> TaskResult {
    let lines = match flag {
        None => selection::for_checkout(root).1,
        Some("--paths") => {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            let paths: Vec<String> = input
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            let graph = selection::Graph::read()?;
            let chosen = selection::select(&graph, &paths);
            selection::report(
                &chosen,
                Some((&graph, "were given on standard input", &paths)),
            )
        }
        Some(other) => return Err(format!("unknown selection flag {other:?}").into()),
    };
    let mut out = std::io::stdout().lock();
    for line in lines {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// `./dev help`, one command to a line so that edits to different commands merge cleanly.
const HELP: &[&str] = &[
    "Thrum contributor commands:",
    "  check                  Exact-baseline reference guard (CPU pins and refreshes), repository guards, fmt, Clippy, game Wasm Clippy, the Rust tests of the packages the change since the merge base with origin/main can reach, side by side through cargo-nextest (including Mac Metal; one stopped at 10 s runs once more alone and fails unless it passes then; it prints what it selected and why), then each step's time, the slowest tests, and the summed time of the tests and of the GPU-named ones",
    "  Exact images are verified on main by the main-branch check exact-images; python3 scripts/exact_baselines.py verify asks it",
    "  Refresh: commit regenerated references, verify --full under gpu, update pins and refresh (anchor/module/milestone), commit and merge together. Comparison edits and added/redefined baselines need pins and refreshes; retirement moves id/owner to retired with a ruling refresh. guard prints pins from HEAD.",
    "  check-all              Everything check runs, with every test binary of the workspace whatever changed; the main-branch check workspace-tests runs it",
    "  selection [--paths]    What check would select for this checkout's change, or for the paths on standard input, and why",
    "  long-tests             The main tier from .config/long-tests.txt (long tests, and tests behind a Cargo feature), a GPU required; the main-branch check long-tests runs it",
    "  check-docs             Links, metadata, design hashes and crate policy",
    "  check-web              Lint/compile the V1 game and starter for wasm32-unknown-unknown",
    "  check-mobile-core      Compile the portable core for ARM64 iOS, iOS simulator and Android",
    "  fmt                    Apply rustfmt",
    "  scene <folder> [--web]  Build a scene package, cook and watch its assets/ and open its window (macOS)",
    "  starter [--web]        The same for examples/starter; --web serves Chrome",
    "  refresh-design-hashes  Refresh current digests after reviewing design changes",
];

#[expect(
    clippy::print_stdout,
    reason = "the contributor CLI's help command intentionally writes to stdout"
)]
fn main() -> TaskResult {
    // Locate the checkout at runtime; no build-machine path is embedded in the tool.
    let root = env::current_dir()?;
    if !Path::new(".config/architecture.toml").is_file() {
        return Err(format!(
            "run from the Thrum repository root (current: {})",
            root.display()
        )
        .into());
    }
    let args: Vec<String> = env::args().skip(1).collect();
    let valid_args = match args.as_slice() {
        [] | [_] => true,
        [command, flag] if command == "starter" && flag == "--web" => true,
        [command, _] if command == "scene" => true,
        [command, _, flag] if command == "scene" && flag == "--web" => true,
        [command, flag] if command == "selection" && flag == "--paths" => true,
        _ => false,
    };
    if !valid_args {
        return Err("expected one command; see ./dev help".into());
    }
    match args.first().map_or("help", String::as_str) {
        "help" | "--help" | "-h" => println!("{}", HELP.join("\n")),
        "check-docs" => python(&["scripts/repo_check.py"])?,
        "long-tests" => long_tests()?,
        "refresh-design-hashes" => python(&["scripts/design_hashes.py", "--write"])?,
        "fmt" => cargo(&["fmt", "--all"])?,
        "starter" if args.get(1).map(String::as_str) == Some("--web") => {
            python(&["scripts/scene_web.py", "examples/starter"])?;
        }
        "scene" if args.get(2).map(String::as_str) == Some("--web") => {
            python(&["scripts/scene_web.py", &args[1]])?;
        }
        "starter" => scene(&root, "examples/starter")?,
        "scene" => match args.get(1) {
            Some(folder) => scene(&root, folder)?,
            None => {
                return Err(
                    "usage: ./dev scene <folder>, such as ./dev scene examples/starter".into(),
                );
            }
        },
        "check-web" => check_web()?,
        "check-mobile-core" => {
            for target in [
                "aarch64-apple-ios",
                "aarch64-apple-ios-sim",
                "aarch64-linux-android",
            ] {
                cargo(&[
                    "check",
                    "--locked",
                    "--offline",
                    "--package",
                    "thrum-core",
                    "--target",
                    target,
                ])?;
                if target == "aarch64-apple-ios-sim" {
                    cargo(&[
                        "check",
                        "--locked",
                        "--offline",
                        "--package",
                        "thrum-starter",
                        "--target",
                        target,
                    ])?;
                }
            }
        }
        "check" => check(&root, false)?,
        "check-all" => check(&root, true)?,
        "selection" => print_selection(&root, args.get(1).map(String::as_str))?,
        other => return Err(format!("unknown command {other:?}; see ./dev help").into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        NEXTEST_STORE, TestTime, junit_times, main_tier_list, nextest_version, passes_after_reruns,
        rerun_filter, rerun_lines, reruns, strip_filters, summed_times,
    };

    /// The closing lines add every test's time, and the times of those named `gpu` or `metal`.
    #[test]
    fn summed_times_add_every_test_and_the_gpu_named_ones() {
        let test = |name: &str, seconds: f64, probe: bool| TestTime {
            name: name.to_owned(),
            seconds,
            timed_out: false,
            failed: false,
            probe,
        };
        let tests = [
            test("thrum-sea::frames_gpu wind_change", 3.0, false),
            test("thrum-render metal::pipeline_cache_survives", 1.5, false),
            test("v1-game::cooked_assets_gpu adapter_less", 0.5, true),
            test("thrum-core::clock advances", 5.0, false),
            test("thrum-render::contracts GPU_upper_case", 2.0, false),
        ];
        let [all, gpu] = summed_times(&tests);
        assert!(
            all.starts_with("tests' summed time: 12.0 s over the 5 tests cargo-nextest ran"),
            "{all}"
        );
        assert!(
            gpu.starts_with("GPU tests' summed time: 7.0 s over 4 tests, 58% of"),
            "{gpu}"
        );
        assert!(
            gpu.contains("binary or name says `gpu` or `metal`"),
            "{gpu}"
        );
        let [none, none_gpu] = summed_times(&[]);
        assert!(
            none.starts_with("tests' summed time: 0.0 s over the 0 tests cargo-nextest ran"),
            "{none}"
        );
        assert!(none_gpu.contains("0%"), "{none_gpu}");
    }

    /// The check reads cargo-nextest's report from the store `.config/nextest.toml` names.
    #[test]
    fn the_runner_configuration_names_the_store_the_check_reads() {
        // xtask is the repository's own tool, run from the root; its test reads the root's file.
        let config = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.config/nextest.toml"),
        )
        .expect(".config/nextest.toml");
        let store = config
            .split("\n[store]\n")
            .nth(1)
            .and_then(|table| table.lines().find(|line| line.starts_with("dir = ")))
            .expect("a [store] table with a dir in .config/nextest.toml");
        assert_eq!(store, format!("dir = \"{NEXTEST_STORE}\""));
    }

    #[test]
    fn nextest_version_reads_the_first_line() {
        assert_eq!(
            nextest_version("cargo-nextest 0.9.150 (d225b1daf 2026-10-26)\nrelease: 0.9.150\n"),
            Some((0, 9, 150))
        );
        assert_eq!(
            nextest_version("cargo-nextest 0.10.2-rc.1\n"),
            Some((0, 10, 2))
        );
        assert_eq!(nextest_version("error: no such command: `nextest`"), None);
    }

    #[test]
    fn junit_times_reads_each_case_and_marks_timeouts() {
        let xml = r#"<testsuites><testsuite name="game::cooked">
<testcase name="fast" classname="game::cooked" time="1.5"></testcase>
<testcase name="a &amp; b" classname="game::cooked" time="61.000">
<failure type="test timeout"/><system-out>test timed out</system-out></testcase>
<testcase name="fails" classname="game::cooked" time="2.0">
<failure type="test failure"/><system-out>waited until it timed out</system-out></testcase>
<testcase name="slow" classname="model::report" time="12.25"/>
</testsuite></testsuites>"#;
        let times = junit_times(xml);
        let names: Vec<_> = times
            .iter()
            .map(|t| (t.name.as_str(), t.seconds, t.timed_out, t.failed))
            .collect();
        assert_eq!(
            names,
            [
                ("game::cooked fast", 1.5, false, false),
                ("game::cooked a & b", 61.0, true, true),
                ("game::cooked fails", 2.0, false, true),
                ("model::report slow", 12.25, false, false)
            ]
        );
    }

    fn case(name: &str, seconds: f64, timed_out: bool, failed: bool) -> TestTime {
        TestTime {
            name: name.to_owned(),
            seconds,
            timed_out,
            failed,
            probe: false,
        }
    }

    /// A stopped test runs again by its exact binary and name, with the pass's other arguments.
    #[test]
    fn a_stopped_test_runs_again_alone_by_its_exact_name() {
        let first = case("water-scene::foam_scene_gpu beach_births", 10.0, true, true);
        let second = case("thrum-core clock::advances", 10.0, true, true);
        assert_eq!(
            rerun_filter(&[&first, &second]),
            "(binary_id(=water-scene::foam_scene_gpu) & test(=beach_births)) | \
             (binary_id(=thrum-core) & test(=clock::advances))"
        );
        assert_eq!(
            strip_filters(&["--workspace", "-E", "package(a)", "--no-tests=pass"]),
            ["--workspace"]
        );
        assert_eq!(
            strip_filters(&[
                "-p",
                "v1-game",
                "--features",
                "draw-probe",
                "-E",
                "test(=x)"
            ]),
            ["-p", "v1-game", "--features", "draw-probe"]
        );
    }

    /// The three cases of GATE-TIME M13.1: stopped twice fails, stopped once and fast alone passes
    /// and is named with its time alone, and a plain failure is not run again and fails.
    #[test]
    fn a_stopped_test_fails_the_pass_unless_it_passes_alone() {
        let always = case("p::t always_slow", 10.0, true, true);
        let once = case("p::t slow_once", 10.0, true, true);
        let fails = case("p::t fails", 0.1, false, true);
        let fast = case("p::t fast", 0.1, false, false);

        let alone = [case("p::t slow_once", 0.2, false, false)];
        let again = reruns(&[&once], &alone);
        assert!(again[0].passed);
        assert!(passes_after_reruns(
            &[
                case("p::t slow_once", 10.0, true, true),
                case("p::t fast", 0.1, false, false)
            ],
            &again
        ));
        let lines = rerun_lines(&again);
        assert_eq!(lines.len(), 2);
        assert!(
            lines[1].contains("10.0 s side by side, 0.2 s alone, passed  p::t slow_once"),
            "{}",
            lines[1]
        );

        let alone = [case("p::t always_slow", 10.0, true, true)];
        let again = reruns(&[&always], &alone);
        assert!(!again[0].passed);
        assert!(!passes_after_reruns(
            &[case("p::t always_slow", 10.0, true, true)],
            &again
        ));
        assert!(rerun_lines(&again)[1].contains("FAILED  p::t always_slow"));

        // A failure of another kind is not run again, so the pass fails whatever the stopped test did.
        let again = reruns(&[&once], &[case("p::t slow_once", 0.2, false, false)]);
        let times = [case("p::t slow_once", 10.0, true, true), fails, fast];
        assert!(!passes_after_reruns(&times, &again));
        // A second run that reports nothing for the test, or one over the limit that passed, fails it.
        assert!(!reruns(&[&once], &[]).iter().any(|rerun| rerun.passed));
        assert!(!reruns(&[&once], &[case("p::t slow_once", 10.5, false, false)])[0].passed);
        // A failed pass with no failed test, such as a build error, is not passed by its reruns.
        assert!(!passes_after_reruns(&[], &[]));
        assert_eq!(rerun_lines(&[]).len(), 1);
    }

    #[test]
    fn main_tier_list_reads_long_tests_and_feature_lines() {
        let list = main_tier_list(
            "# comment\nwater floating_gpu release\ngame field_gpu debug # why\n\
             render features=counters\ngame features=probe,asset/fault one two\n\
             game not-run three features=probe owner=OWNER # fails\n",
        )
        .unwrap();
        let long: Vec<_> = list
            .long
            .iter()
            .map(|entry| (entry.package.as_str(), entry.binary.as_str(), entry.release))
            .collect();
        assert_eq!(
            long,
            [
                ("water", "floating_gpu", true),
                ("game", "field_gpu", false)
            ]
        );
        let features: Vec<_> = list
            .features
            .iter()
            .map(|entry| (entry.package.as_str(), &entry.features, &entry.binaries))
            .collect();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0], ("render", &vec!["counters".into()], &vec![]));
        assert_eq!(
            features[1],
            (
                "game",
                &vec!["probe".into(), "asset/fault".into()],
                &vec!["one".into(), "two".into()]
            )
        );
        assert_eq!(list.not_run, ["game three"]);
        assert!(main_tier_list("water floating_gpu fast\n").is_err());
    }
}
