//! An admitted dry provider preserves the island's submitted output.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn dry_wetness_provider_is_pixel_exact() {
    use std::time::Duration;
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
            f
        };
        let mut absent = make(false);
        let mut dry = make(true);
        assert!(!absent.host.scene_shader_source().expect("scene shader").contains("fn ground_wetness("));
        assert!(dry.host.scene_shader_source().expect("scene shader").contains("fn ground_wetness("));
        let capture = |f: &mut support::Fixture| {
            thrum_test_support::wait::until("cooked yard textures", Duration::from_secs(30), || {
                f.frame_without_timings(2.0);
                let pixels = f.image();
                // The island draws its untextured ground before the grass streams in.
                // The fixed grass sample witnesses that the full scene is ready.
                let grass = (30 * 192 + 16) * 4;
                (pixels[grass + 1].saturating_sub(pixels[grass + 2]) > 15).then_some(pixels)
            })
        };
        let ordinary = capture(&mut absent);
        let registered = capture(&mut dry);
        let changed = ordinary.chunks_exact(4).zip(registered.chunks_exact(4))
            .filter(|(left, right)| left != right).count();
        assert_eq!(changed, 0, "the dry provider changed submitted pixels");
    });
}
}
