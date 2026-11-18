//! Build the reference game's portable identity from actual rule inputs.

use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn sources(directory: &Path, paths: &mut Vec<PathBuf>) {
    for item in fs::read_dir(directory).expect("rule source directory exists") {
        let path = item.expect("read directory entry").path();
        if path.is_dir() {
            sources(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("../..");
    let mut inputs = [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        ".cargo/config.toml",
        "crates/thrum-core/Cargo.toml",
        "crates/thrum-sim2d/Cargo.toml",
        "crates/thrum-network/src/input_batch.rs",
        "examples/crowd-2d/Cargo.toml",
        // This is where the reference game's ActionWire<()> is defined.
        "examples/crowd-2d-run/src/lib.rs",
        "examples/match-proof/build.rs",
    ]
    .map(|path| root.join(path))
    .to_vec();
    for directory in [
        "crates/thrum-core/src",
        "crates/thrum-sim2d/src",
        "examples/crowd-2d/src",
    ] {
        let directory = root.join(directory);
        println!("cargo:rerun-if-changed={}", directory.display());
        sources(&directory, &mut inputs);
    }
    inputs.sort();
    let mut hash = Sha256::new();
    for path in inputs {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .strip_prefix(root)?
            .to_str()
            .expect("source paths are UTF-8");
        let bytes = fs::read(&path)?;
        hash.update(u64::try_from(name.len())?.to_le_bytes());
        hash.update(name);
        hash.update(u64::try_from(bytes.len())?.to_le_bytes());
        hash.update(bytes);
    }
    let compiler = Command::new(std::env::var_os("RUSTC").expect("Cargo supplies RUSTC"))
        .args(["--version", "--verbose"])
        .output()?;
    assert!(compiler.status.success(), "read compiler identity");
    for line in std::str::from_utf8(&compiler.stdout)?.lines() {
        if !line.starts_with("host: ") {
            hash.update(line);
            hash.update(b"\n");
        }
    }
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"));
    fs::write(output.join("rule-set-id"), hash.finalize())?;
    Ok(())
}
