//! Small host-only contributor command runner.

use std::{env, error::Error, path::Path, process::Command};

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
            "Thrum contributor commands:\n  check                  Repository guard, guard tests, fmt, Clippy, Rust tests\n  check-docs             Links, metadata, design hashes and crate policy\n  check-web              Compile the portable core for wasm32-unknown-unknown\n  check-mobile-core      Compile the portable core for ARM64 iOS, iOS simulator and Android\n  fmt                    Apply rustfmt\n  refresh-design-hashes  Refresh current digests after reviewing design changes"
        ),
        "check-docs" => python(&["scripts/repo_check.py"])?,
        "refresh-design-hashes" => python(&["scripts/design_hashes.py", "--write"])?,
        "fmt" => cargo(&["fmt", "--all"])?,
        "check-web" => cargo(&[
            "check",
            "--locked",
            "--offline",
            "--package",
            "thrum-core",
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
                "test_*.py",
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
            cargo(&["test", "--locked", "--offline", "--workspace"])?;
        }
        other => return Err(format!("unknown command {other:?}; see ./dev help").into()),
    }
    Ok(())
}
