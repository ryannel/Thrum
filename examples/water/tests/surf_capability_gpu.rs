//! Creation and draw witnesses for profiles without and with the surf capability.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn surf_products_and_pass_exist_only_when_enabled() {
    use std::sync::{Arc, Mutex};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Profile, Water};
    use thrum_water_model::{Bed, Body, BodyKind, Map, Stage, SurfProfile};

    let Some(test_gpu) = thrum_test_support::gpu(
        "surf products and pass are opt-in",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::new(&test_gpu, [96, 64]);
    let bed = Bed::new([-4.0, -4.0], 1.0, [9, 9], vec![-4.0; 81]).expect("bed");
    let mut map = Map::prepare(bed, vec![Body {
        name: "Sea".into(),
        kind: BodyKind::Sea,
        level: 0.0,
        stage: Stage { min: 0.0, max: 0.0, value: 0.0 },
        fetch: 100_000.0,
        gains: [1.0; 4],
    }], thrum_water_model::Profile { margin_height: 1.0, swash_ratio: 0.7 }).expect("map");
    let spectrum = Spectrum {
        long: vec![support::component(0.35)],
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm")),
        tail_slope_variance: 0.0,
    };
    let expected = ["surf lags", "surf exposures", "breaker section", "surf bed pyramid", "surf field cache"];
    for (enabled, elapsed) in [(false, 0.05), (true, 0.10)] {
        if enabled {
            map.enable_surf(SurfProfile::default()).expect("prepared surf");
        }
        let water = Water::mapped(Arc::new(Mutex::new(map.clone())), spectrum.clone(), Profile::default())
            .expect("mapped water");
        let report = water.report();
        fixture.host.set_scene_contribution(Box::new(water)).expect("contribution");
        let frame = fixture.frame(elapsed);
        assert!(frame.submission_order > 0);
        let report = report.lock().expect("resources");
        for label in expected {
            let product = report.resources.iter().find(|(name, bytes)| name == label && *bytes > 0);
            assert_eq!(product.is_some(), enabled, "{label}");
        }
        let graph = fixture.host.scene_graph().expect("scene graph");
        assert_eq!(graph.pass_names().any(|label| label.contains("surf field cache")), enabled);
        assert!(graph.pass_names().any(|label| label.contains("water surface")));
    }
}
}
