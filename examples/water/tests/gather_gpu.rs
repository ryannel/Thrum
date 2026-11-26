//! Water admission on the Metal frame graph, with and without an opaque body.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
use support::optics::{Panel, Rig, attach, camera, sun, water};

thrum_test_support::macos_gpu_test! {
fn gather_admits_visible_bodies_and_preserves_the_waterless_graph() {
    let Some(gpu) = thrum_test_support::gpu("gather", Some(wgpu::Backend::Metal)) else { return };
    let view = camera([0.0, 2.0, 0.0], [0.0, 0.0, 8.0], 0.7, 1.0/1024.0);
    let panel = Panel { centre: [0.0, 1.0, 8.0], half_size: [1.2, 0.8, 0.08], radiant: true };
    let outside = Panel { centre: [20.0, 2.0, 0.0], half_size: [1.0; 3], radiant: true };
    let mut rig = Rig::new(&gpu, &view, sun(2.0, 0.000_01), &[panel, outside], None);
    let graph = |rig: &Rig| rig.host.scene_graph().expect("graph").pass_names()
        .map(str::to_owned).collect::<Vec<_>>();
    let baseline = graph(&rig);
    let storage = rig.host.storage_report();
    let opaque = attach(&mut rig, water(None, true));
    assert!(graph(&rig).iter().any(|p| p.contains("water gather")));
    assert!(graph(&rig).iter().all(|p| !p.contains("effect refraction copy")));
    rig.host.set_timing_requested(true);
    let timing = rig.frame().gpu_pass_ns.expect("Metal pass timings");
    assert!(timing.passes().iter().all(|pass| !pass.name.contains("water gather")));
    let (_, empty) = rig.scene_colour();

    assert!(opaque.publish(&[outside.bounds()]));
    let timing = rig.frame().gpu_pass_ns.expect("Metal timings with an off-screen body");
    assert!(timing.passes().iter().all(|pass| !pass.name.contains("water gather")));
    assert_eq!(rig.scene_colour().1, empty);
    assert!(opaque.publish(&[outside.bounds(), panel.bounds()]));
    let timing = rig.frame().gpu_pass_ns.expect("Metal pass timings with a visible receiver");
    assert!(timing.passes().iter().any(|pass| pass.name.contains("water gather")));
    let (_, reflected) = rig.scene_colour();
    let changed = empty.iter().zip(&reflected)
        .filter(|(unregistered, registered)| registered[0] > unregistered[0] + 0.02)
        .count();
    assert!(changed > 14, "a registered panel must appear below its source rectangle: {changed} pixels");
    assert!(opaque.publish(&[]));
    assert_eq!(rig.scene_colour().1, empty, "no source may read the previous frame's capture");
    rig.host.set_scene_contribution(Box::<thrum_render::contribution::SceneContributions>::default()).expect("detach");
    assert_eq!(graph(&rig), baseline);
    assert_eq!(rig.host.storage_report(), storage);
}
}

thrum_test_support::macos_gpu_test! {
fn gather_skips_a_deep_rough_basin_and_keeps_visible_readers() {
    use std::sync::{Arc, Mutex};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Optics, Profile, Water};
    use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

    let Some(gpu) = thrum_test_support::gpu("basin gather", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let map = Arc::new(Mutex::new(Map::prepare(
        Bed::new(
            [-128.0; 2],
            8.0,
            [33, 33],
            (0..33)
                .flat_map(|z| (0..33).map(move |x| match (x, z) {
                    (0 | 32, _) | (_, 0 | 32) => 3.0,
                    (1..=7, _) => -0.2,
                    _ => -45.0,
                }))
                .collect(),
        )
        .expect("bed"),
        vec![Body {
            name: "test lake".into(),
            kind: BodyKind::Basin { seed: [0.0; 2] },
            level: 0.0,
            stage: Stage { min: 0.0, max: 0.0, value: 0.0 },
            fetch: 100.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile { margin_height: 1.0, swash_ratio: 0.7 },
    )
    .expect("map")));
    let assert_gather = |rig: &mut Rig, expected| {
        let times = rig.frame().gpu_pass_ns.expect("Metal pass timings");
        assert!(times.passes().iter().any(|p| p.name.contains("water surface")));
        assert_eq!(
            times.passes().iter().any(|p| p.name.contains("water gather")),
            expected
        );
    };
    let make_water = |variance, shortest_wave| {
        let modes = [(1, 0, 0.015, 0.0)];
        let spectrum = Spectrum {
            long: vec![],
            cascades: [
                Cascade::new(160.0, &[]).expect("long cascade"),
                Cascade::new(37.0, &[]).expect("middle cascade"),
                Cascade::new(9.0, if shortest_wave { &modes } else { &[] }).expect("short cascade"),
            ],
            tail_slope_variance: variance,
        };
        let mut profile = Profile { optics: Some(Optics::default()), ..Profile::default() };
        profile.waves.choppiness = 0.0;
        Water::mapped(Arc::clone(&map), spectrum, profile).expect("basin water")
    };
    let view = |x| camera([x, 10.0, 0.0], [x, 0.0, 1.0], 60_f32.to_radians(), 1.0 / 1024.0);
    let panel = Panel { centre: [0.0, 1.0, 1.0], half_size: [0.5; 3], radiant: true };
    let mut deep = Rig::new(&gpu, &view(0.0), sun(10.0, 0.000_01), &[panel], None);
    deep.host.set_timing_requested(true);
    for shortest_wave in [false, true] {
        let sources = attach(&mut deep, make_water(0.01, shortest_wave));
        assert_gather(&mut deep, false);
        if shortest_wave {
            assert!(sources.publish(&[panel.bounds()]));
            assert_gather(&mut deep, true);
            assert!(sources.publish(&[]));
            assert_gather(&mut deep, false);
        }
    }
    let _calm_sources = attach(&mut deep, make_water(0.0, false));
    assert_gather(&mut deep, true);
    let mut shelf = Rig::new(&gpu, &view(-96.0), sun(10.0, 0.000_01), &[], None);
    shelf.host.set_timing_requested(true);
    let _shelf_sources = attach(&mut shelf, make_water(0.01, true));
    assert_gather(&mut shelf, true);
}
}

thrum_test_support::macos_gpu_test! {
fn calm_deep_chart_and_basin_reflect_an_unregistered_panel() {
    use std::sync::{Arc, Mutex};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Optics, Profile, Water};
    use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Stage};

    let Some(gpu) = thrum_test_support::gpu("calm inland gather", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let wet: Vec<_> = (0..65)
        .flat_map(|z| (0..65).map(move |x| (1..64).contains(&x) && (1..64).contains(&z)))
        .collect();
    let bed = Bed::new(
        [-80.0; 2],
        2.5,
        [65, 65],
        wet.iter().map(|&inside| if inside { -3.0 } else { 2.0 }).collect(),
    )
    .expect("deep bed");
    let kinds = [
        BodyKind::Basin { seed: [0.0; 2] },
        BodyKind::Chart(Chart {
            levels: vec![0.0; wet.len()],
            currents: vec![[0.0; 2]; wet.len()],
            footprint: wet,
            ends: vec![],
        }),
    ];
    let spectrum = Spectrum {
        long: vec![],
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("empty band")),
        tail_slope_variance: 0.0,
    };
    let optics = Optics { extinction: [4.0; 3], ..Optics::default() };
    // At 3 m the bed transmits exp(-4 * 3), below the 1% sight threshold.
    let profile = Profile { optics: Some(optics), ..Profile::default() };
    let view = camera([0.0, 10.0, 0.0], [0.0, 0.0, 7.0], 46_f32.to_radians(), 1.0 / 16384.0);
    let [width, _] = support::optics::EXTENT.map(|size| usize::try_from(size).expect("extent"));

    for (name, kind) in ["basin", "chart"].into_iter().zip(kinds) {
        let map = Arc::new(Mutex::new(Map::prepare(
            bed.clone(),
            vec![Body {
                name: name.into(),
                kind,
                level: 0.0,
                stage: Stage { min: 0.0, max: 0.0, value: 0.0 },
                fetch: 150.0,
                gains: [1.0; 4],
            }],
            thrum_water_model::Profile { margin_height: 0.3, swash_ratio: 0.0 },
        ).expect("level water map")));
        let measure = |radiant| {
            let panel = Panel { centre: [0.0, 1.5, 7.0], half_size: [1.8, 1.0, 0.25], radiant };
            let mut rig = Rig::new(&gpu, &view, sun(10.0, 0.000_01), &[panel], None);
            let dry = rig.scene_colour().1;
            // The panel is ordinary opaque scene geometry. No optics body is published.
            let water = Water::mapped(Arc::clone(&map), spectrum.clone(), profile).expect("mapped water");
            let _bodies = attach(&mut rig, water);
            rig.host.set_timing_requested(true);
            let times = rig.frame().gpu_pass_ns.expect("Metal pass timings");
            assert!(times.passes().iter().any(|pass| pass.name.contains("water surface")));
            let gather = times.passes().iter().any(|pass| pass.name.contains("water gather"));
            (gather, dry, rig.scene_colour().1)
        };
        let (dark_gather, dry_dark, wet_dark) = measure(false);
        let (bright_gather, dry_bright, wet_bright) = measure(true);
        let changed = |dark: &[f32; 3], bright: &[f32; 3]| bright[0] - dark[0] > 0.0001;
        let last_panel_row = dry_dark.iter().zip(&dry_bright)
            .rposition(|(dark, bright)| changed(dark, bright))
            .expect("radiant panel visible with water detached") / width;
        let reflected = wet_dark.iter().zip(&wet_bright)
            .skip((last_panel_row + 1) * width)
            .filter(|(dark, bright)| changed(dark, bright))
            .count();
        println!("{name}: gather={dark_gather}/{bright_gather}, {reflected} changed water pixels below row {last_panel_row}");
        assert!(dark_gather && bright_gather, "{name} must gather for the scene mirror");
        assert!(reflected > 14, "{name} panel reflection: {reflected} pixels");
    }
}
}

thrum_test_support::macos_gpu_test! {
fn scenery_in_front_of_a_registered_reflection_stays_in_front() {
    let Some(gpu) = thrum_test_support::gpu("reflection layering", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, 2.0, 0.0], [0.0, 0.0, 8.0], 0.7, 1.0 / 1024.0);
    // The centre ray reflects from z=8. Its slope is 1:4, so it meets
    // the low unregistered panel before the registered panel. The direct
    // camera ray to the taller panel passes above the low one.
    let near = Panel { centre: [0.0, 0.5, 10.0], half_size: [1.5, 0.16, 0.06], radiant: false };
    let far = Panel { centre: [0.0, 1.0, 12.0], half_size: [1.2, 0.35, 0.06], radiant: true };
    let reflected_radiance = |panels: &[Panel]| {
        let mut rig = Rig::new(&gpu, &view, sun(2.0, 0.000_01), panels, None);
        let bodies = attach(&mut rig, water(Some((-0.5, [-16.0, -2.0], 32.0)), true));
        assert!(bodies.publish(&[far.bounds()]));
        let (scale, image) = rig.scene_colour();
        let [width, height] = support::optics::EXTENT.map(|size| usize::try_from(size).expect("extent"));
        image[(height / 2) * width + width / 2][0] / scale
    };
    let mut dark = far;
    dark.radiant = false;
    let unobstructed = reflected_radiance(&[far]) - reflected_radiance(&[dark]);
    let obstructed = reflected_radiance(&[near, far]) - reflected_radiance(&[near, dark]);
    assert!(unobstructed > 100.0, "registered panel signal: {unobstructed}");
    assert!(
        obstructed.abs() < 0.15 * unobstructed,
        "nearer scenery must cover the registered reflection: {obstructed} of {unobstructed}"
    );
}
}
