//! Reject MetalFX after readiness through the frame host.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
    fn rejection_uses_prepared_taau_and_reports_why() {
        use std::{io::Write, time::Duration};
        use thrum_render::{
            present::SurfaceProfile,
            runtime::{CreationPhase, FrameConfig, SurfaceOutput},
            synthetic::{FrameLighting, NativeOutcome, Reconstruction, SyntheticLight, SyntheticScene, TwinReason},
        };
        let Some(opened) = thrum_test_support::open(
            "MetalFX prepared fallback",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
        ) else {
            return;
        };
        let capabilities = opened.capabilities.with_recipe_override("apple-tile").expect("Apple tile");
        let profile = SurfaceProfile { format: wgpu::TextureFormat::Rgba8Unorm, extent: [64, 64] };
        let mut scene = SyntheticScene::grid(1).expect("fixture");
        scene.sun_shadows = false;
        scene.view = scene.view.with_metalfx();
        let mut host = pollster::block_on(support::host_with_config_exact(
            &capabilities, &opened.queue, scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            FrameConfig { render_extent: profile.extent, surface: Some(profile) },
        ));
        assert_eq!(host.reconstruction(), Reconstruction::MetalFx);
        let before = host.creation_report();
        assert_eq!(before.native_events(CreationPhase::Build), 1);
        assert!(host.reject_native_provider(), "coordinator rejected the optional provider");
        assert_eq!(host.reconstruction(), Reconstruction::Taau);
        let provider = host.native_provider_report().expect("provider report");
        let twin = provider.twin.expect("resolved twin");
        assert_eq!((twin.name, twin.reason), ("TAAU", TwinReason::Rejected));
        let view = support::drawable(&opened.device, profile.format, profile.extent);
        let surface = SurfaceOutput { view: &view, extent: profile.extent };
        assert!(host.drive(Duration::ZERO, Some(&surface)).expect("TAAU submit").surface_presented);
        let sample = thrum_test_support::wait::until(
            "twin completion",
            Duration::from_secs(10),
            || {
                opened.device.poll(wgpu::PollType::Poll).expect("poll");
                host.poll_completed().expect("completed frame")
            },
        );
        assert_eq!(sample.native_segment.expect("twin receipt").outcome(), NativeOutcome::Unmeasured);
        assert_eq!(sample.history_identity[1], 0);
        let after = host.creation_report();
        assert_eq!(before.pipelines, after.pipelines);
        assert_eq!(after.native_events(CreationPhase::Play), 0);
        writeln!(std::io::stdout().lock(), "resolved {:?}, {twin:?}, {after}", host.reconstruction()).expect("write target proof");
    }
}
