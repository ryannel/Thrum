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

fn cargo_verified(args: &[&str], markers: &[(&str, usize)]) -> TaskResult {
    let output = Command::new(env!("CARGO")).args(args).output()?;
    std::io::stdout().lock().write_all(&output.stdout)?;
    std::io::stderr().lock().write_all(&output.stderr)?;
    if !output.status.success() {
        return Err(format!("cargo {} failed ({})", args.join(" "), output.status).into());
    }
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for &(marker, minimum) in markers {
        if combined.matches(marker).count() < minimum {
            return Err(format!("cargo bench omitted required row {marker:?}").into());
        }
    }
    Ok(())
}

fn python(args: &[&str]) -> TaskResult {
    run(
        &env::var("THRUM_PYTHON").unwrap_or_else(|_| "python3".into()),
        args,
    )
}

fn check_web() -> TaskResult {
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

fn baseline_inventory() -> TaskResult {
    if !cfg!(target_os = "macos") {
        return Err("G0 baseline inventory requires the named M-series Mac".into());
    }
    python(&["scripts/repo_check.py"])?;
    for (row, marker, count) in [
        ("--g0-s1", "\"G0,S1,", 6),
        ("--g0-s1-paired", "\"G0,S1_paired,", 8),
        ("--g0-s2", "\"G0,S2,", 1),
        (
            "--g0-burst",
            "\"G0,S1,10000,Serial,shuffled,damage1,sethealth1,spawn200\"",
            1,
        ),
    ] {
        cargo_verified(
            &[
                "bench",
                "--locked",
                "--offline",
                "--package",
                "v1-game",
                "--bench",
                "g2r",
                "--",
                row,
            ],
            &[(marker, count)],
        )?;
    }
    cargo_verified(
        &[
            "bench",
            "--locked",
            "--offline",
            "--package",
            "v1-game",
            "--bench",
            "frame",
            "--",
            "assembled_frame_1080p_late_surface",
            "g0_120hz_callbacks_at_1080p",
            "--sample-count",
            "600",
            "--sample-size",
            "1",
        ],
        &[
            ("assembled_frame_1080p_late_surface", 1),
            ("g0_120hz_callbacks_at_1080p", 1),
            ("V1 offscreen frame CPU submitted=600 callbacks=600", 2),
            ("V1 scene GPU pass-sum", 2),
        ],
    )?;
    cargo_verified(
        &[
            "bench",
            "--locked",
            "--offline",
            "--package",
            "v1-game",
            "--bench",
            "frame",
            "--",
            "--g0-inventory",
        ],
        &[
            ("fixed_row,baseline_state", 1),
            ("G7 remaining iPhone CPU/GPU proxy rows", 1),
        ],
    )?;
    Ok(())
}

/// The workspace package and window binary for a scene folder, as Cargo's own metadata reads
/// them (`scripts/scene_package.py`, which `./dev` also runs before touching the lockfile).
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
        "help" | "--help" | "-h" => println!(
            "Thrum contributor commands:\n  check                  Repository guard, guard tests, fmt, Clippy, Rust tests (including Mac Metal), game Wasm Clippy\n  check-docs             Links, metadata, design hashes and crate policy\n  baseline-inventory     Run G0 fixed budget baselines and list absent consumers on the M-series Mac\n  check-web              Lint/compile the V1 game and starter for wasm32-unknown-unknown\n  check-mobile-core      Compile the portable core for ARM64 iOS, iOS simulator and Android\n  fmt                    Apply rustfmt\n  scene <folder> [--web]  Build a scene package, cook and watch its assets/ and open its window (macOS)\n  starter [--web]        The same for examples/starter; --web serves Chrome\n  refresh-design-hashes  Refresh current digests after reviewing design changes"
        ),
        "check-docs" => python(&["scripts/repo_check.py"])?,
        "baseline-inventory" => baseline_inventory()?,
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
            }
        }
        "check" => {
            python(&["scripts/repo_check.py"])?;
            python(&[
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_repo_check.py",
            ])?;
            python(&[
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_scene_package.py",
            ])?;
            python(&[
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_t2_capture.py",
            ])?;
            python(&[
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_mac_m2_window.py",
            ])?;
            cargo(&["fmt", "--all", "--", "--check"])?;
            cargo(&[
                "clippy",
                "--locked",
                "--offline",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])?;
            check_web()?;
            cargo(&[
                "clippy",
                "--locked",
                "--offline",
                "-p",
                "v1-game",
                "--all-targets",
                "--features",
                "draw-probe",
                "--",
                "-D",
                "warnings",
            ])?;
            cargo(&["test", "--locked", "--offline", "--workspace"])?;
            // Qualification-only observations inspect the real bound buffers and commands.
            cargo(&[
                "test",
                "--locked",
                "--offline",
                "-p",
                "v1-game",
                "--features",
                "draw-probe",
                "--test",
                "cooked_assets_gpu",
            ])?;
        }
        other => return Err(format!("unknown command {other:?}; see ./dev help").into()),
    }
    Ok(())
}
