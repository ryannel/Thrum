//! The real game selects Weather and Shade through public engine interfaces.
/// Shared native diagnostic device policy.
pub mod support;

use std::time::{Duration, Instant};
use thrum_render::runtime::FrameError;
use v1_game::{CloudSelection, FrameConfig, FrameHost, FrameSample, GameSetup};

fn completed(host: &mut FrameHost, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = host.poll_completed().expect("frame completion") {
            return sample;
        }
        assert!(Instant::now() < deadline, "frame completion timeout");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal completion");
    }
}

#[test]
fn cloud_attachment_reports_the_device_limit_as_a_gpu_error() {
    use thrum_test_support::{Profile, Restriction};

    let Some(gpu) = thrum_test_support::gpu("cloud GPU limits", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue, caps) = gpu
        .device_with_hardware_adjusted(Profile::Portable, &Restriction::default(), 1, &[], |desc| {
            desc.required_limits.max_storage_textures_per_shader_stage = 2;
        })
        .expect("restricted Metal device");
    let config = FrameConfig {
        render_extent: [160, 90],
        surface: None,
    };
    let prepare = |clouds| {
        pollster::block_on(FrameHost::new_with_rows(
            &device,
            &queue,
            &caps,
            config,
            GameSetup {
                clouds,
                ..GameSetup::default()
            },
            None,
        ))
    };
    assert!(prepare(CloudSelection::Off).is_ok());
    match prepare(CloudSelection::Shade) {
        Err(FrameError::Gpu(reason)) => assert!(reason.contains("weather map texture limits")),
        Err(other) => panic!("expected a GPU device limit, got {other:?}"),
        Ok(_) => panic!("Shade prepared without enough storage textures"),
    }
}

#[test]
#[cfg(target_os = "macos")]
fn selection_is_off_by_default_and_on_uses_only_weather_and_shade() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "selection_is_off_by_default_and_on_uses_only_weather_and_shade",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_instance, device, queue, caps) = support::device(&test_gpu, "portable", true);
        let (view, profile) = support::presentation(&device, [160, 90]);
        let surface = v1_game::SurfaceOutput {
            view: &view,
            extent: [160, 90],
        };
        let config = FrameConfig {
            render_extent: [160, 90],
            surface: Some(profile),
        };
        assert_eq!(GameSetup::default().clouds, CloudSelection::Off);
        let mut baseline =
            FrameHost::new_with_setup(&device, &queue, &caps, config, GameSetup::default())
                .await
                .expect("default game");
        let mut off = FrameHost::new_with_setup(
            &device,
            &queue,
            &caps,
            config,
            GameSetup {
                clouds: CloudSelection::Off,
                ..GameSetup::default()
            },
        )
        .await
        .expect("explicitly absent clouds");
        assert_eq!(baseline.storage_report(), off.storage_report());
        assert_eq!(
            baseline.capture_pixels().expect("default pixels"),
            off.capture_pixels().expect("clouds-off pixels")
        );
        let mut on = FrameHost::new_with_setup(
            &device,
            &queue,
            &caps,
            config,
            GameSetup {
                clouds: CloudSelection::Shade,
                ..GameSetup::default()
            },
        )
        .await
        .expect("clouds selected");
        on.drive(Duration::ZERO, Some(&surface))
            .expect("clock origin");
        completed(&mut on, &device);
        assert!(
            on.drive(Duration::from_nanos(16_666_667), Some(&surface))
                .expect("new weather tick")
                .submitted
        );
        let first = completed(&mut on, &device);
        let timings = first.gpu_pass_ns.expect("real named pass timings");
        for name in [
            "cloud weather map",
            "cloud Sun path near",
            "cloud Sun path middle",
            "cloud Sun path far",
            "cloud ground field",
            "cloud far transmittance",
        ] {
            assert!(
                timings.passes().iter().any(|p| p.name.ends_with(name)),
                "missing pass {name}"
            );
        }
        assert!(!timings.passes().iter().any(|p| {
            ["trace", "overlay", "detail noise", "cloud layer"]
                .iter()
                .any(|name| p.name.contains(name))
        }));
        let report = on
            .drive(Duration::from_millis(25), Some(&surface))
            .expect("same tick at another display time");
        assert_eq!(report.tick, 1);
        assert!(report.submitted);
        let repeated = completed(&mut on, &device);
        let repeated = repeated.gpu_pass_ns.expect("repeated-frame timings");
        assert!(
            !repeated.passes().iter().any(|p| p.name.contains("cloud ")),
            "repeated display must write no cloud texel"
        );
    });
}
