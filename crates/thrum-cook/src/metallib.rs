//! Offline compilation of the MSL and stage records exported by the Metal backend.

use std::{
    fs, io,
    path::Path,
    process::{Command, Output},
};

use serde::Deserialize;

#[derive(Deserialize)]
struct CompileOptions {
    language_version: u32,
    preserve_invariance: bool,
}

impl CompileOptions {
    fn standard(&self) -> String {
        let major = self.language_version >> 16;
        let minor = self.language_version & 0xffff;
        format!("-std=metal{major}.{minor}")
    }
}

fn require_success(tool: &str, output: &Output) -> io::Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{tool} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

/// Compile one backend-exported stage and prefix its library with the stage record.
///
/// The input directory holds `source.metal`, `options.json` and `stage.record`.
/// The compiler receives the language version and invariance used at translation.
/// The intermediate files belong to the caller's build directory.
///
/// # Errors
/// Returns invalid input, missing Metal toolchain, I/O or compiler errors.
pub fn cook_stage(input: &Path, output: &Path) -> io::Result<()> {
    let options: CompileOptions = serde_json::from_slice(&fs::read(input.join("options.json"))?)?;
    let air = input.join("stage.air");
    let library = input.join("stage.metallib");
    let mut command = Command::new("xcrun");
    command
        .args(["--toolchain", "Metal", "-sdk", "macosx", "metal"])
        .arg(options.standard());
    if options.preserve_invariance {
        command.arg("-fpreserve-invariance");
    }
    let compiled = command
        .arg("-c")
        .arg(input.join("source.metal"))
        .arg("-o")
        .arg(&air)
        .output()?;
    require_success("metal", &compiled)?;
    let linked = Command::new("xcrun")
        .args(["--toolchain", "Metal", "-sdk", "macosx", "metallib"])
        .arg(air)
        .arg("-o")
        .arg(&library)
        .output()?;
    require_success("metallib", &linked)?;
    let mut bytes = fs::read(input.join("stage.record"))?;
    bytes.extend(fs::read(library)?);
    fs::write(output, bytes)
}

/// Compile a stage directory named for its key, and stamp the blob with that key
/// and the digest of its native payload.
///
/// # Errors
/// Returns an invalid stage key, compiler or I/O error.
pub fn cook_keyed_stage(input: &Path, output: &Path) -> io::Result<()> {
    let key = input
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|key| {
            key.len() == 64
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
        .ok_or_else(|| io::Error::other("expected an offline stage directory named for its key"))?;
    let native = input.join("stage.blob");
    cook_stage(input, &native)?;
    let payload = fs::read(&native)?;
    let mut blob = format!("{key}\n{}\n", blake3::hash(&payload).to_hex()).into_bytes();
    blob.extend(payload);
    fs::write(output, blob)
}
