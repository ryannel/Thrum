//! Small host-only contributor command runner.

use std::{
    env,
    error::Error,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
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

/// One test's result from cargo-nextest's JUnit report.
struct TestTime {
    /// `<package>::<binary> <test>`, as cargo-nextest names it.
    name: String,
    seconds: f64,
    timed_out: bool,
}

/// The value of `attribute="..."` in one XML start tag, unescaped.
fn xml_attribute(tag: &str, attribute: &str) -> Option<String> {
    let start = tag.find(&format!(" {attribute}=\""))? + attribute.len() + 3;
    let value = &tag[start..start + tag[start..].find('"')?];
    Some(
        value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

/// Every test case of a cargo-nextest JUnit report. A case whose failure says it timed out was
/// stopped by the slow-test limit of `.config/nextest.toml`.
fn junit_times(xml: &str) -> Vec<TestTime> {
    let mut times = Vec::new();
    for case in xml.split("<testcase ").skip(1) {
        // The leading space lets the first attribute match like the others.
        let tag = format!(" {}", &case[..case.find('>').unwrap_or(case.len())]);
        let body = &case[..case.find("</testcase>").unwrap_or(case.len())];
        let (Some(name), Some(class)) = (
            xml_attribute(&tag, "name"),
            xml_attribute(&tag, "classname"),
        ) else {
            continue;
        };
        times.push(TestTime {
            name: format!("{class} {name}"),
            seconds: xml_attribute(&tag, "time")
                .and_then(|time| time.parse().ok())
                .unwrap_or(0.0),
            timed_out: body.contains("timed out"),
        });
    }
    times
}

/// Above this a test is reported as slow; ten seconds is the merge tier's bound for one test.
const SLOW_TEST_SECONDS: f64 = 10.0;

/// What `./dev check` has done so far: each step's wall time, and each test's time.
#[derive(Default)]
struct CheckRecord {
    steps: Vec<(String, std::time::Duration, bool)>,
    tests: Vec<TestTime>,
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
    fn nextest(&mut self, junit: &Path, args: &[&str]) -> TaskResult {
        // A report left by an earlier run must not be read as this one's.
        match std::fs::remove_file(junit) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        let mut full = vec!["nextest", "run", "--locked", "--offline"];
        full.extend_from_slice(args);
        let ran = cargo(&full);
        if let Ok(xml) = std::fs::read_to_string(junit) {
            let times = junit_times(&xml);
            let stopped: Vec<&TestTime> = times.iter().filter(|test| test.timed_out).collect();
            if !stopped.is_empty() {
                let mut out = std::io::stdout().lock();
                for test in stopped {
                    writeln!(
                        out,
                        "test stopped: {} ran for over 60 s. A test in a merge's check takes about ten \
                         seconds or less: make it fast, or mark it #[ignore = \"long: <reason>\"] and \
                         list its binary in .config/long-tests.txt (ORCHESTRATE.md, Test tiers)",
                        test.name
                    )?;
                }
            }
            self.tests.extend(times);
        }
        ran
    }

    /// The closing summary, printed whether the check passed or failed.
    fn summary(&mut self) -> TaskResult {
        let mut out = std::io::stdout().lock();
        writeln!(out, "\n./dev check step times (wall):")?;
        let mut total = std::time::Duration::ZERO;
        for (name, elapsed, passed) in &self.steps {
            total += *elapsed;
            let mark = if *passed { "" } else { "  FAILED" };
            writeln!(out, "  {:>8.1} s  {name}{mark}", elapsed.as_secs_f64())?;
        }
        writeln!(out, "  {:>8.1} s  total", total.as_secs_f64())?;
        self.tests.sort_by(|a, b| b.seconds.total_cmp(&a.seconds));
        if !self.tests.is_empty() {
            writeln!(out, "ten slowest tests:")?;
            for test in self.tests.iter().take(10) {
                writeln!(out, "  {:>8.1} s  {}", test.seconds, test.name)?;
            }
            let slow: Vec<&TestTime> = self
                .tests
                .iter()
                .filter(|test| test.seconds > SLOW_TEST_SECONDS)
                .collect();
            writeln!(
                out,
                "{} of {} tests ran for over {SLOW_TEST_SECONDS} s, the merge tier's bound for one test; \
                 one over 60 s is stopped unless .config/nextest.toml exempts it by name:",
                slow.len(),
                self.tests.len()
            )?;
            for test in slow {
                writeln!(out, "  {:>8.1} s  {}", test.seconds, test.name)?;
            }
        }
        Ok(())
    }
}

/// The target directory cargo-nextest writes its store under.
fn target_dir(root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// The draw-probe pass runs only the cooked-asset tests that assert on the draw probe; the
/// workspace pass runs every cooked-asset test against the production build.
const DRAW_PROBE_TESTS: &[&str] = &[
    "a_fetched_edit_swaps_in_and_reuses_unchanged_products",
    "a_publication_larger_than_staging_streams_through_the_bounded_ring",
    "a_newer_publication_supersedes_an_upload_still_being_filled",
    "adapter_less_public_host_selects_direct",
];

/// `./dev check`: every step in turn, stopping at the first that fails, then the time of each
/// step that ran and the slowest tests.
fn check(root: &Path) -> TaskResult {
    require_nextest()?;
    let junit = target_dir(root).join("nextest/default/junit.xml");
    let mut record = CheckRecord::default();
    let result = check_steps(&mut record, &junit);
    record.summary()?;
    result
}

fn check_steps(record: &mut CheckRecord, junit: &Path) -> TaskResult {
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
    // Every test binary of the workspace, built once, each test in a process of its own.
    record.step("tests, side by side", |record| {
        record.nextest(junit, &["--workspace"])
    })?;
    record.step("doc tests", |_| {
        cargo(&["test", "--locked", "--offline", "--workspace", "--doc"])
    })?;
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

/// `./dev help`, one command to a line so that edits to different commands merge cleanly.
const HELP: &[&str] = &[
    "Thrum contributor commands:",
    "  check                  Exact-baseline reference guard (CPU pins and refreshes), repository guards, fmt, Clippy, Rust tests (including Mac Metal), game Wasm Clippy",
    "  Exact images are verified on main by the main-branch check exact-images; python3 scripts/exact_baselines.py verify asks it",
    "  Refresh: commit regenerated references, verify --full under gpu, update pins and refresh (anchor/module/milestone), commit and merge together. Comparison edits and added/redefined baselines need pins and refreshes; retirement moves id/owner to retired with a ruling refresh. guard prints pins from HEAD.",
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
        "check" => check(&root)?,
        other => return Err(format!("unknown command {other:?}; see ./dev help").into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{junit_times, main_tier_list, nextest_version};

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
<failure type="test failure">test timed out</failure></testcase>
<testcase name="slow" classname="model::report" time="12.25"/>
</testsuite></testsuites>"#;
        let times = junit_times(xml);
        let names: Vec<_> = times
            .iter()
            .map(|t| (t.name.as_str(), t.seconds, t.timed_out))
            .collect();
        assert_eq!(
            names,
            [
                ("game::cooked fast", 1.5, false),
                ("game::cooked a & b", 61.0, true),
                ("model::report slow", 12.25, false)
            ]
        );
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
