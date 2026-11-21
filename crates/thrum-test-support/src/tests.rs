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

#[global_allocator]
static ALLOCATOR: super::CountingAllocator = super::CountingAllocator;

#[test]
fn allocations_count_calls_and_bytes_on_the_calling_thread_only() {
    let (kept, counted) = super::allocations::count(|| {
        let mut grown = Vec::<u8>::with_capacity(100);
        grown.reserve_exact(200);
        let zeroed = vec![0_u8; 64];
        // Another thread's allocations are not this thread's.
        std::thread::scope(|scope| {
            scope.spawn(|| std::hint::black_box(vec![1_u8; 4096]));
        });
        (grown, zeroed)
    });
    drop(kept);
    // One allocation of 100, its reallocation to 200 (room for 200 more of none held), one zeroed allocation of 64; the spawn's own bookkeeping
    // on this thread is counted too, so the thread's 4096-byte vector is the check.
    assert!(counted.count >= 3, "{counted:?}");
    assert!(counted.bytes >= 364, "{counted:?}");
    assert!(
        counted.bytes < 4096,
        "another thread's allocation counted: {counted:?}"
    );
    assert!(counted.largest >= 200, "{counted:?}");
    assert!(
        counted.largest < 4096,
        "another thread's allocation counted: {counted:?}"
    );
    let ((), alone) = super::allocations::count(|| {
        std::hint::black_box(Vec::<u8>::with_capacity(100));
        std::hint::black_box(vec![0_u8; 64]);
        let mut grown = Vec::<u8>::with_capacity(8);
        grown.reserve_exact(24);
        std::hint::black_box(grown);
    });
    assert_eq!(
        alone,
        super::allocations::Allocations {
            count: 4,
            bytes: 100 + 64 + 8 + 24,
            largest: 100,
        }
    );
    // Outside `count` nothing is counted.
    let ((), idle) = super::allocations::count(|| {});
    std::hint::black_box(vec![0_u8; 10]);
    assert_eq!(idle, super::allocations::Allocations::default());
}

#[test]
fn wait_returns_the_first_value_and_fails_at_its_deadline() {
    let mut calls = 0;
    let value = super::wait::until("third call", std::time::Duration::from_secs(5), || {
        calls += 1;
        (calls == 3).then_some(calls)
    });
    assert_eq!(value, 3);
    let limit = std::time::Duration::from_millis(50);
    let started = std::time::Instant::now();
    let failure = std::panic::catch_unwind(|| super::wait::until::<()>("never", limit, || None))
        .expect_err("a wait that never succeeds fails");
    let waited = started.elapsed();
    assert!(waited >= limit, "failed early, after {waited:?}");
    assert!(waited < limit * 20, "outlived its deadline: {waited:?}");
    let message = failure.downcast_ref::<String>().expect("message");
    assert!(
        message.contains("never: not reached within 50ms"),
        "{message}"
    );
    // A value that arrives at the deadline is still taken.
    let late = super::wait::until("late", std::time::Duration::ZERO, || Some(7));
    assert_eq!(late, 7);
}

#[test]
fn scratch_is_new_and_removed_when_dropped_even_on_panic() {
    let first = super::Scratch::new("kit-test");
    let second = super::Scratch::new("kit-test");
    assert_ne!(first.path(), second.path());
    assert!(std::fs::read_dir(first.path()).unwrap().next().is_none());
    std::fs::create_dir(first.join("nested")).unwrap();
    std::fs::write(first.join("nested/file"), b"x").unwrap();
    let path = first.path().to_owned();
    drop(first);
    assert!(!path.exists());
    let panicked = std::panic::catch_unwind(|| {
        let scratch = super::Scratch::new("kit-test");
        std::fs::write(scratch.join("file"), b"x").unwrap();
        panic!("{}", scratch.path().display());
    })
    .expect_err("panics");
    let path = std::path::PathBuf::from(panicked.downcast_ref::<String>().unwrap());
    assert!(!path.exists());
    drop(second);
}

#[test]
fn ppm_writes_rgb_from_each_channel_order_and_refuses_a_wrong_size() {
    use super::ppm::{Pixels, write};
    let scratch = super::Scratch::new("ppm-test");
    let expected = [b"P6\n2 1\n255\n".as_slice(), &[1, 2, 3, 4, 5, 6]].concat();
    for (order, pixels) in [
        (Pixels::Rgb8, vec![1, 2, 3, 4, 5, 6]),
        (Pixels::Rgba8, vec![1, 2, 3, 255, 4, 5, 6, 0]),
        (Pixels::Bgra8, vec![3, 2, 1, 255, 6, 5, 4, 0]),
    ] {
        let path = scratch.join(format!("{order:?}.ppm"));
        write(&path, [2, 1], &pixels, order).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), expected, "{order:?}");
    }
    let error = write(scratch.join("short.ppm"), [2, 2], &[0; 12], Pixels::Rgba8).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!scratch.join("short.ppm").exists());
}

#[cfg(feature = "render")]
#[test]
fn open_admits_on_the_asked_backend_and_opens_the_restricted_profile() {
    let backend = cfg!(target_os = "macos").then_some(wgpu::Backend::Metal);
    let Some(opened) = super::open(
        "open_admits_on_the_asked_backend_and_opens_the_restricted_profile",
        backend,
        super::Profile::Portable,
        &super::Restriction {
            storage_buffers: Some(4),
            features: Some(wgpu::Features::empty()),
        },
    ) else {
        return;
    };
    if let Some(backend) = backend {
        assert_eq!(opened.gpu.adapter.get_info().backend, backend);
    }
    assert_eq!(opened.device.features(), wgpu::Features::empty());
    assert_eq!(
        opened.device.limits().max_storage_buffers_per_shader_stage,
        4
    );
    assert_eq!(
        opened
            .capabilities
            .limits()
            .max_storage_buffers_per_shader_stage,
        4
    );
    assert_eq!(opened.capabilities.generation(), 1);
}
