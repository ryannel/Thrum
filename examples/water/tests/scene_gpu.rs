//! Runtime proof through the public game and water interfaces.
#[cfg(target_os = "macos")]
use crate::support;
use thrum_render::runtime::{FrameWorld, SceneSource};
thrum_test_support::macos_gpu_test! {
fn scene_ticks_and_declares_one_water_contribution() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "scene_ticks_and_declares_one_water_contribution",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::new(&test_gpu, [64, 36]);
    let sample = fixture.frame_without_timings(0.1);
    assert_eq!(fixture.host.world().source().tick, 5);
    assert_eq!(sample.tick, 5);
    let graph = fixture.host.scene_graph().expect("graph");
    let names: Vec<_> = graph.pass_names().collect();
    assert_eq!(
        names.iter().filter(|n| n.contains("water surface")).count(),
        1
    );
    assert_eq!(
        names
            .iter()
            .filter(|n| n.starts_with("effect ") && !n.starts_with("effect after"))
            .count(),
        21
    );
    let witness = fixture.witness();
    let water = witness.iter().filter(|p| p[0] > 0.1 && p[1] > 0.0).count();
    assert!(water > witness.len() / 4, "surface coverage {water}");
}
}

thrum_test_support::macos_gpu_test! {
fn optics_sun_edits_keep_the_submitted_camera_independent() {
    let Some(gpu) = thrum_test_support::gpu("optics camera", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (game, report) = water_scene::build_optics_sea(water_scene::island::Wind::Calm, false, true)
        .expect("optics example");
    let mut fixture = support::Fixture::from_game(&gpu, [96, 54], game, report);
    let mut inspect = |settings, at| {
        fixture.parameters(settings);
        let sample = fixture.frame_without_timings(at);
        fixture.host.source().frame(fixture.host.world().source(),
            sample.presentation_alpha, fixture.extent, None).expect("submitted camera")
    };
    let before = inspect(r#"{ "sun.elevation": 49 }"#, 0.1);
    let after = inspect(r#"{ "sun.elevation": 50 }"#, 0.2);
    assert_eq!(before.view.view_proj_columns(), after.view.view_proj_columns());
    let direction = |frame: thrum_render::runtime::SceneFrame| {
        let thrum_render::synthetic::FrameLighting::Atmosphere(air) = frame.lighting else {
            panic!("optics uses the physical sky");
        };
        air.toward_sun()
    };
    let fixed_view = after.view.view_proj_columns();
    assert_ne!(direction(before), direction(after));
    let pitched = inspect(r#"{ "camera.pitch": -33 }"#, 0.3);
    assert_ne!(fixed_view, pitched.view.view_proj_columns());
}
}
