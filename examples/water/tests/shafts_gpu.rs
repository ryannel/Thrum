//! The signed march contributes zero to an unblocked calm.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn calm_water_stores_the_same_light_with_or_without_the_march() {
    use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water_configured};
    use thrum_water::{Optics, Profile};

    let Some(gpu) = thrum_test_support::gpu("calm underwater Sun", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, -1.2, -1.0], [0.0, -2.6, 0.0], 0.66, 1.0);
    let panel = Panel { centre: [0.0, -3.0, 0.0], half_size: [2.5, 0.01, 2.5], radiant: true };
    let render = |caustics, shafts, steps| {
        let water = water_configured(None, Profile {
            caustics,
            shafts,
            optics: Some(Optics::default()),
            underwater: true,
            deep_scatter: [0.02, 0.07, 0.13],
            ..Profile::default()
        });
        let mut rig = Rig::with_scene(
            support::device(&gpu.fresh_adapter()),
            &view,
            sun(1.0, 0.0),
            &[panel],
            None,
            EXTENT,
            |scene| scene.sun_path = water.sun_path_provider(),
        );
        attach(&mut rig, water);
        if let Some(steps) = steps {
            rig.host.set_quality_knob("water.shaft_steps", steps).expect("shaft steps knob");
        }
        rig.scene_colour().1
    };
    let marched = render(true, true, None);
    let directional = render(true, true, Some(0));
    assert_eq!(marched, directional, "a calm unblocked path gained a signed remainder");
    let wp15 = render(false, false, None);
    assert_eq!(wp15, render(true, false, None), "a calm caustic field altered WP15's composition");
    assert_ne!(marched, wp15, "the scene never exercised directional beam scattering");
}
}
