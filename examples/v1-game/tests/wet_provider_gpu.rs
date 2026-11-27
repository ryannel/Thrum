//! Run the default V1 game through the actual forward shader in both dry arms.
thrum_test_support::macos_gpu_test! {
fn registered_dry_wetness_preserves_default_game_pixels() {
    use thrum_render::{runtime::FrameConfig, present::SurfaceProfile};
    use thrum_water::wet_ground::{GroundClass, WetGround};
    use v1_game::GameSetup;

    let Some(gpu) = thrum_test_support::gpu(
        "registered_dry_wetness_preserves_default_game_pixels",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (device, queue, caps) = gpu.device_with_hardware(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
            1,
            &[],
        ).expect("Metal portable device");
        let extent = [224, 126];
        let surface = SurfaceProfile { format: wgpu::TextureFormat::Bgra8Unorm, extent };
        let config = FrameConfig { render_extent: extent, surface: Some(surface) };
        let dry = WetGround::new(None, None, &[GroundClass {
            relief: 0.001, steepest: 0.44, share: 0.5, porosity: 0.7,
            film_roughness: 0.09, ..GroundClass::default()
        }], 0).expect("dry rain state");
        let mut ordinary = GameSetup::default().prepare(&device, &queue, &caps, config)
            .await.expect("default game");
        let mut with_dry = GameSetup::default().prepare_with_wetness(
            &device, &queue, &caps, config, dry.provider()
        ).await.expect("default game with dry provider");
        assert!(!ordinary.scene_shader_source().expect("forward source").contains("fn ground_wetness("));
        assert!(with_dry.scene_shader_source().expect("forward source").contains("fn ground_wetness("));
        let pixels = ordinary.capture_pixels().expect("default submitted pixels");
        assert_eq!(pixels, with_dry.capture_pixels().expect("dry submitted pixels"));
    });
}
}
