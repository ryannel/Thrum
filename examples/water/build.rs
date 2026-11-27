//! Embed the river files selected by the browser packager.
use std::{env, fs, path::PathBuf};

fn main() {
    const FILES: [&str; 3] = [
        "THRUM_WATER_STREAM_FILE",
        "THRUM_WATER_OUTLET_FILE",
        "THRUM_WATER_FALL_FILE",
    ];
    for name in FILES {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        return;
    }
    let entries = FILES.map(|name| {
        let Some(path) = env::var_os(name).map(PathBuf::from) else {
            return "None".into();
        };
        println!("cargo:rerun-if-changed={}", path.display());
        let key = path
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("river file key");
        assert_eq!(key.len(), 64, "expected a 32-byte river file key");
        let bytes: [u8; 32] = std::array::from_fn(|i| {
            u8::from_str_radix(&key[2 * i..2 * i + 2], 16).expect("hexadecimal river file key")
        });
        format!("Some(({bytes:?}, include_bytes!(env!({name:?}))))")
    });
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output"));
    fs::write(
        output.join("river_package.rs"),
        format!(
            "const FILES: [Option<([u8; 32], &[u8])>; 3] = [{}];",
            entries.join(", ")
        ),
    )
    .expect("write river package declarations");
}
