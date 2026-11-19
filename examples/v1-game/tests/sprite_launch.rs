//! Native startup rejects provider requests that only the 3D scene can run.

#[cfg(target_os = "macos")]
#[test]
fn sprite_startup_refuses_each_native_witness_selection() {
    let root = std::env::temp_dir().join(format!("thrum-sprite-launch-{}", std::process::id()));

    for selection in [
        "metal",
        "wgsl",
        "rejected",
        "no-commands",
        "no-tile",
        "no-reset",
        "stale-source",
        "stale-history",
    ] {
        let path = root.join(selection);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_v1-game"))
            .env_clear()
            .env("THRUM_CAPTURE_SCENE", &path)
            .env("THRUM_SCENE", "sprites")
            .env("THRUM_NATIVE_WITNESS", selection)
            .output()
            .expect("launch the V1 game");
        let created_capture = path.exists();
        if created_capture {
            std::fs::remove_dir_all(&path).expect("remove unexpected capture");
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{selection}: {stderr}");
        assert_eq!(
            stderr.trim(),
            format!("capture failed: 2D scene refuses THRUM_NATIVE_WITNESS={selection}")
        );
        assert!(!created_capture, "{selection} created a capture");
    }
}
