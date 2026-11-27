//! An admitted dry provider preserves the island's submitted output.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn dry_wetness_provider_is_pixel_exact() {
    use water_scene::island::{Wind, build_rain_ground};

    let name = "dry_wetness_provider_is_pixel_exact";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let make = |enabled| {
            let (game, report, _map, _replies) = build_rain_ground(Wind::Calm, true, enabled).expect("yard");
            let mut f = support::Fixture::from_game(&gpu, [192, 108], game, report);
            f.parameters(r#"{"camera.island":0,"rain.rate":0,"rain.drying":0}"#);
            f.warm_cooked();
            f.frame_without_timings(2.0);
            f
        };
        let absent = make(false);
        let dry = make(true);
        assert!(!absent.host.scene_shader_source().expect("scene shader").contains("fn ground_wetness("));
        assert!(dry.host.scene_shader_source().expect("scene shader").contains("fn ground_wetness("));
        let pixels = absent.image();
        assert!(pixels.windows(2).any(|pair| pair[0] != pair[1]));
        assert_eq!(dry.image(), pixels, "the dry provider changed submitted pixels");
    });
}
}
