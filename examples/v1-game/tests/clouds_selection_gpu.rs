//! The real game selects Weather and Shade through public engine interfaces.
/// Shared native diagnostic device policy.
pub mod support;

use std::time::{Duration, Instant};
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
#[cfg(target_os = "macos")]
fn selection_is_off_by_default_and_on_uses_only_weather_and_shade() {
    pollster::block_on(async {
        let (_instance, device, queue, caps) = support::device("portable", true).await;
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
