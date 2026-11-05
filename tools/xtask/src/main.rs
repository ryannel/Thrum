//! Small host-only contributor command runner.

use std::{env, error::Error, io::Write, path::Path, process::Command};

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
    if args.len() > 1 {
        return Err("expected one command; see ./dev help".into());
    }
    match args.first().map_or("help", String::as_str) {
        "help" | "--help" | "-h" => println!(
            "Thrum contributor commands:\n  check                  Repository guard, guard tests, fmt, Clippy, Rust tests (including Mac Metal), game Wasm compile\n  check-docs             Links, metadata, design hashes and crate policy\n  baseline-inventory     Run G0 fixed budget baselines and list absent consumers on the M-series Mac\n  check-web              Compile the V1 game for wasm32-unknown-unknown\n  check-mobile-core      Compile the portable core for ARM64 iOS, iOS simulator and Android\n  fmt                    Apply rustfmt\n  refresh-design-hashes  Refresh current digests after reviewing design changes"
        ),
        "check-docs" => python(&["scripts/repo_check.py"])?,
        "baseline-inventory" => baseline_inventory()?,
        "refresh-design-hashes" => python(&["scripts/design_hashes.py", "--write"])?,
        "fmt" => cargo(&["fmt", "--all"])?,
        "check-web" => cargo(&[
            "check",
            "--locked",
            "--offline",
            "--package",
            "v1-game",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
        ])?,
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
            cargo(&[
                "check",
                "--locked",
                "--offline",
                "--package",
                "v1-game",
                "--lib",
                "--target",
                "wasm32-unknown-unknown",
            ])?;
            cargo(&["test", "--locked", "--offline", "--workspace"])?;
        }
        other => return Err(format!("unknown command {other:?}; see ./dev help").into()),
    }
    Ok(())
}
