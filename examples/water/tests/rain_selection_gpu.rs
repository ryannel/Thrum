//! Render the optional yard and the same island with the selection off.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn selected_yard_steps_without_changing_the_default_island() {
    let Some(gpu) = thrum_test_support::gpu(
        "selected_yard_steps_without_changing_the_default_island",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    support::island::with_cooked_island("selected_yard_steps_without_changing_the_default_island", || {
        use thrum_water_model::BodyKind;
        use water_scene::{island::{self, Wind}, settings::Settings};
        assert_eq!(Settings::from_query("?island=calm"), Settings::Island { wind: Wind::Calm, foam: false });
        assert_eq!(Settings::from_query("?rain=calm"), Settings::RainYard(Wind::Calm));
        let (base, base_report, base_map) = island::build(Wind::Calm, true).expect("ordinary island");
        let (yard, yard_report, yard_map, _) = island::build_rain_yard(Wind::Calm, true).expect("rain yard");
        let extent = [112, 64];
        let mut base = support::Fixture::from_game(&gpu, extent, base, base_report);
        let mut yard = support::Fixture::from_game(&gpu, extent, yard, yard_report);
        base.warm_cooked();
        yard.warm_cooked();
        yard.parameters(r#"{"camera.island":3,"rain.rate":100,"rain.time_scale":600}"#);
        base.frame(2.0);
        yard.frame(2.0);
        assert!(base_map.lock().expect("island map").body(3).is_none());
        assert_eq!(base.host.scene_shader_source(), yard.host.scene_shader_source());
        let yard_map = yard_map.lock().expect("yard map");
        let BodyKind::Hollows(hollows) = &yard_map.body(3).expect("yard body").kind else {
            panic!("the selected yard uses one hollows body");
        };
        assert!(hollows.optical_types().iter().flatten().any(|optics| optics.extinction <= 3.0));
        assert!(hollows.optical_types().iter().flatten().any(|optics| optics.extinction > 10.0));
        for point in [[147.2, 66.2], [147.1, 62.1]] {
            let hit = yard_map.sample(point).expect("puddle column");
            assert_eq!(hit.body, 3);
            assert!(hit.depth > 0.01, "rain stands at {point:?}");
        }
        let has_yard_resources = |scene: &support::Fixture| scene.report
            .lock().expect("water report").resources.iter().any(|(name, bytes)| name == "water hollows" && *bytes > 0);
        assert!(!has_yard_resources(&base));
        assert!(has_yard_resources(&yard));
    });
}
}
