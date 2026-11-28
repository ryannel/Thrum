use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::mistakes::{compiler_dependencies, sdk_library};

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

struct LinkedGame(PathBuf);
impl Drop for LinkedGame {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("clean the linked game fixture");
    }
}

pub(super) fn rustc(source: &str, link: bool) -> Output {
    let (sdk_directory, dependencies) = compiler_dependencies();
    let sdk = sdk_library(&sdk_directory, &dependencies);
    let directory = std::env::current_exe()
        .expect("test binary")
        .parent()
        .expect("test output directory")
        .join(format!(
            "prefab-fixture-{}-{}",
            std::process::id(),
            NEXT_RUN.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir(&directory).expect("make fixture output directory");
    let directory = LinkedGame(directory);
    let binary = directory.0.join("game");
    let mut rustc = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
    rustc.args(["--edition=2024", "--crate-name", "linked_game"]);
    if !link {
        rustc.arg("--emit=metadata");
    }
    for dependency in dependencies {
        rustc
            .arg("-L")
            .arg(format!("dependency={}", dependency.display()));
    }
    let mut child = rustc
        .arg("--extern")
        .arg(format!("thrum={}", sdk.display()))
        .arg("-o")
        .arg(&binary)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run rustc for the linked game");
    child
        .stdin
        .take()
        .expect("rustc input")
        .write_all(source.as_bytes())
        .expect("send fixture source");
    let output = child.wait_with_output().expect("collect compiler output");
    if link {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Command::new(binary).output().expect("run the linked game")
    } else {
        output
    }
}

#[test]
fn defaults_variants_required_inputs_and_definition_children() {
    let output = rustc(include_str!("fixtures/prefab_game.rs"), true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
