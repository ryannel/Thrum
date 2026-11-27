//! Compare opaque panel irradiance on Metal across the provider boundary.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn the_provider_moves_light_only_below_the_waterline() {
    use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water_with_spectrum};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::Profile;

    let Some(gpu) = thrum_test_support::gpu("caustic receivers", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let render = |level: f32, caustics: bool| {
        let view = camera([0.0, level + 0.8, 0.0], [0.0, level, 0.03], 0.8, 1.0);
        let spectrum = Spectrum {
            long: Vec::new(),
            cascades: [
                Cascade::new(160.0, &[]).unwrap(),
                Cascade::new(37.0, &[]).unwrap(),
                Cascade::new(9.0, &[(5, 0, 0.005, 0.0), (0, 4, 0.003, 0.7)]).unwrap(),
            ],
            tail_slope_variance: 0.0,
        };
        let water = water_with_spectrum(None, Profile { caustics, ..Profile::default() }, spectrum);
        let mut rig = Rig::with_scene(
            support::device(&gpu.fresh_adapter()),
            &view,
            sun(1.0, 0.0),
            &[Panel { centre: [0.0, level, 0.0], half_size: [2.0, 0.01, 2.0], radiant: false }],
            None,
            EXTENT,
            |scene| scene.sun_path = water.sun_path_provider(),
        );
        attach(&mut rig, water);
        rig.scene_colour().1
    };
    let above = render(0.35, false);
    assert_eq!(above, render(0.35, true), "an above-water panel changed");
    let below = render(-1.25, false);
    let focused = render(-1.25, true);
    let wider = below.iter().zip(&focused).filter(|(even, net)| net[1] > even[1] * 1.01).count();
    let dimmer = below.iter().zip(&focused).filter(|(even, net)| net[1] < even[1] * 0.99).count();
    assert!(wider > 100 && dimmer > 100, "brightened {wider} and darkened {dimmer} pixels");
}
}

thrum_test_support::macos_gpu_test! {
fn sun_visibility_and_orientation_gate_the_net() {
    use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water_with_spectrum};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::Profile;

    let Some(gpu) = thrum_test_support::gpu("caustic receiver gates", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let panel = Panel { centre: [0.0, -1.15, 0.0], half_size: [2.0, 0.01, 2.0], radiant: false };
    let caster = Panel { centre: [0.0, 0.4, 0.0], half_size: [1.75, 0.1, 1.75], radiant: false };
    let render = |caustics, facing_down, shadowed| {
        let y = if facing_down { -1.8 } else { -0.4 };
        let view = camera([0.0, y, 0.0], [0.0, panel.centre[1], 0.01], 0.45, 1.0);
        let spectrum = Spectrum {
            long: Vec::new(),
            cascades: [
                Cascade::new(160.0, &[]).unwrap(),
                Cascade::new(37.0, &[]).unwrap(),
                Cascade::new(9.0, &[(5, 0, 0.006, 0.0), (0, 4, 0.004, 0.7)]).unwrap(),
            ],
            tail_slope_variance: 0.0,
        };
        let water = water_with_spectrum(None, Profile { caustics, ..Profile::default() }, spectrum);
        let panels = [panel, caster];
        let mut rig = Rig::with_scene(
            support::device(&gpu.fresh_adapter()), &view, sun(1.0, 0.0),
            if shadowed { &panels } else { &panels[..1] },
            None, EXTENT,
            |scene| {
                scene.sun_shadows = shadowed;
                scene.sun_path = water.sun_path_provider();
            },
        );
        attach(&mut rig, water);
        rig.scene_colour().1
    };
    for (facing_down, shadowed) in [(true, false), (false, true)] {
        let with = render(true, facing_down, shadowed);
        let without = render(false, facing_down, shadowed);
        assert_eq!(with[192 * 30..192 * 79], without[192 * 30..192 * 79]);
    }
}
}

thrum_test_support::macos_gpu_test! {
fn gathered_panel_carries_the_same_direct_light_modulation_from_both_sides() {
    use support::optics::{EXTENT, Panel, Rig, attach, camera, sun, water_with_spectrum};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Optics, Profile};

    let Some(gpu) = thrum_test_support::gpu("gathered caustic receiver", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let panel = Panel { centre: [0.0, -0.95, 0.0], half_size: [2.5, 0.01, 2.5], radiant: false };
    let render = |eye_height, caustics| {
        let view = camera([0.0, eye_height, 0.0], [0.0, -0.95, 0.01], 0.7, 1.0);
        let spectrum = Spectrum {
            long: Vec::new(),
            cascades: [
                Cascade::new(160.0, &[]).unwrap(),
                Cascade::new(37.0, &[]).unwrap(),
                Cascade::new(9.0, &[(5, 0, 0.005, 0.0), (0, 4, 0.004, 0.7)]).unwrap(),
            ],
            tail_slope_variance: 0.0,
        };
        let water = water_with_spectrum(Some((-1.17, [-6.0; 2], 12.0)),
            Profile { caustics, underwater: true, optics: Some(Optics::default()), ..Profile::default() }, spectrum);
        let mut rig = Rig::with_scene(
            support::device(&gpu.fresh_adapter()), &view, sun(1.0, 0.0), &[panel], None, EXTENT,
            |scene| scene.sun_path = water.sun_path_provider(),
        );
        let bounds = attach(&mut rig, water);
        assert!(bounds.publish(&[panel.bounds()]));
        rig.scene_colour().1
    };
    for eye_height in [0.55, -0.38] {
        let with = render(eye_height, true);
        let without = render(eye_height, false);
        let (brighter, darker) = with.iter().zip(without).fold((0, 0), |(more, less), (a, b)| {
            (more + usize::from(a[1] > b[1] * 1.01), less + usize::from(a[1] < b[1] * 0.99))
        });
        assert!(brighter > 100 && darker > 100, "lens at {eye_height} m: {brighter} brighter and {darker} darker");
    }
}
}
