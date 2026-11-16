use std::process::Command;

#[cfg(feature = "render")]
#[test]
fn restrictions_only_remove_capabilities() {
    let limits = wgpu::Limits::default();
    let features = wgpu::Features::TIMESTAMP_QUERY;
    let restriction = super::Restriction {
        storage_buffers: Some(u32::MAX),
        features: Some(wgpu::Features::all()),
    };
    let (actual_limits, actual_features) = restriction.apply(limits.clone(), features);
    assert_eq!(actual_limits, limits);
    assert_eq!(actual_features, features);
    let restriction = super::Restriction {
        storage_buffers: Some(7),
        features: Some(wgpu::Features::empty()),
    };
    let (actual_limits, actual_features) = restriction.apply(limits.clone(), features);
    assert_eq!(
        actual_limits,
        wgpu::Limits {
            max_storage_buffers_per_shader_stage: 7,
            ..limits
        }
    );
    assert!(actual_features.is_empty());
}

#[test]
fn absent_adapter_child() {
    if std::env::var_os("THRUM_GATE_CHILD").is_some() {
        assert!(super::admit("absent_adapter_child", wgpu::Backends::empty()).is_none());
    }
}

#[test]
fn ordinary_harness_reports_skip_and_required_failure() {
    for required in [false, true] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "tests::absent_adapter_child"])
            .env("THRUM_GATE_CHILD", "1")
            .env_remove("THRUM_REQUIRE_GPU");
        if required {
            command.env("THRUM_REQUIRE_GPU", "1");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.success(), !required);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            combined.contains("thrum-gpu-skip: absent_adapter_child:"),
            "{combined}"
        );
        assert!(combined.contains("adapter unavailable:"), "{combined}");
        if !required {
            assert!(String::from_utf8_lossy(&output.stderr).contains("thrum-gpu-skip:"));
        }
    }
}
