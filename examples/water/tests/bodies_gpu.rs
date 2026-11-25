//! Classify submitted pixels against the f64 map and the opaque arm.
use std::sync::{Arc, Mutex};
#[cfg(target_os = "macos")]
use support::island;
use thrum_app::sky::{Camera, ContentStatus, Controls, MeshPlacement, SkyGame};
use thrum_water_model::Map;
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn submitted_island_has_the_maps_bodies_and_no_bridge_over_the_ridge() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_island_has_the_maps_bodies_and_no_bridge_over_the_ridge",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    island::with_cooked_island(
        "submitted_island_has_the_maps_bodies_and_no_bridge_over_the_ridge",
        || {
            let (mut dry, _) = island::fixture(&test_gpu, false);
            dry.frame(0.5);
            let opaque = dry.witness();
            drop(dry);
            let (mut scene, map) = island::fixture(&test_gpu, true);
            scene.frame(0.5);
            let drawn = scene.witness();
            let map = map.lock().expect("map");
            let clip = island::view_proj(4);
            let mut counts = [0; 4];
            let mut boundary = 0;
            let mut errors = Vec::new();
            let stride = island::EXTENT[0] as usize;
            // At 400 m the ray footprint is roughly 0.65 m on the island.
            // The pixel allowance is measured in screen space with its neighbours,
            // rather than a depth band, which would admit large errors on flat sand.
            let mut expected = Vec::with_capacity(opaque.len());
            for (i, &p) in opaque.iter().enumerate() {
                let xy = [
                    u32::try_from(i % stride).expect("column"),
                    u32::try_from(i / stride).expect("row"),
                ];
                let position = support::world_at(
                    clip,
                    island::EXTENT,
                    xy,
                    if p[1] > 0.0 { p[1] } else { 0.1 / 400.0 },
                );
                expected.push(
                    map.sample([f64::from(position.x), f64::from(position.z)])
                        .filter(|a| {
                            // A wet column can be hidden by a rock or tree above its sheet.
                            a.depth > 0.0 && (p[1] == 0.0 || f64::from(position.y) <= a.row.level)
                        })
                        .map_or(3, |a| a.body),
                );
            }
            for (i, &p) in drawn.iter().enumerate() {
                let y = i / stride;
                let x = i % stride;
                let target = expected[i];
                let at_edge = (y.saturating_sub(1)..=(y + 1).min(island::EXTENT[1] as usize - 1))
                    .any(|row| {
                        (x.saturating_sub(1)..=(x + 1).min(stride - 1))
                            .any(|col| expected[row * stride + col] != target)
                    });
                if at_edge {
                    boundary += 1;
                    continue;
                }
                let actual = if island::water(p) {
                    let xy = [
                        u32::try_from(x).expect("column"),
                        u32::try_from(y).expect("row"),
                    ];
                    let point = support::world_at(clip, island::EXTENT, xy, p[1]);
                    [0.0, 12.0, 10.6]
                        .into_iter()
                        .position(|level| (point.y - level).abs() < 0.42)
                        .unwrap_or(4)
                } else {
                    3
                };
                if (actual != target || (actual < 3 && p[1] < opaque[i][1])) && errors.len() < 20 {
                    errors.push((x, y, target, actual, p, opaque[i]));
                }
                counts[target] += 1;
            }
            assert!(errors.is_empty(), "pixel ownership/depth {errors:?}");
            assert!(
                counts.into_iter().all(|n| n >= 300) && boundary > 100,
                "{counts:?} / {boundary}"
            );
            let sealed = island::project(clip, [235.0, 10.7, 5.0]);
            assert_eq!(expected[sealed], 3, "lower hollow is beyond the sill");
            assert!(
                !island::water(drawn[sealed]),
                "no interpolated sheet reaches into the dry hollow"
            );
            assert_eq!(drawn[sealed][1], opaque[sealed][1]);
            let report = scene.report.lock().expect("mesh report");
            assert_eq!(report.bodies.len(), 3);
            assert_eq!(
                report.bodies.iter().map(|(_, n)| n).sum::<u32>(),
                report.vertices
            );
        },
    );
}
}

thrum_test_support::macos_gpu_test! {
fn no_lower_sea_sheet_is_hidden_under_the_lake() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "no_lower_sea_sheet_is_hidden_under_the_lake",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    island::with_cooked_island("no_lower_sea_sheet_is_hidden_under_the_lake", || {
        let dry = water_scene::island::map(water_scene::island::Wind::Calm).expect("island map");
        let mut bodies = (0..3)
            .map(|id| dry.body(id).expect("island body").clone())
            .collect::<Vec<_>>();
        // Both meshes span this column and the dry bed is below both levels.
        // From below, an unmasked sea would win the reverse-Z test.
        bodies[0].level = 9.25;
        let map = Map::prepare(dry.bed().clone(), bodies, dry.profile()).expect("overlap fixture");
        let centre = map.sample([183.0, 5.0]).expect("water");
        assert_eq!(centre.body, 1);
        assert!(centre.bed.expect("bed") < 8.7);
        let spectrum = thrum_sea_model::Spectrum::build(&water_scene::island::Wind::Calm.forcing())
            .expect("zero waves");
        let water = thrum_water::Water::mapped(
            Arc::new(Mutex::new(map)),
            spectrum,
            thrum_water::Profile::default(),
        )
        .expect("mapped surface");
        let report = water.report();
        let eye = [183.0, 8.7, 5.0];
        let target = [183.0, 17.0, 4.999];
        let camera = Camera::look_at(eye, target, 50.0);
        let game = SkyGame::new(Controls::new(), camera, |_, sky| {
            sky.temporal_inputs = true;
            sky.sun_elevation_degrees = 60.0;
        })
        .with_ground(None)
        .with_meshes(vec![MeshPlacement::new("meshes/island", "textures/white")])
        .expect("opaque island")
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
        let mut fixture = support::Fixture::from_game(&test_gpu, [160, 96], game, report);
        fixture.warm_cooked();
        fixture.frame(0.2);
        let depth = fixture.witness()[48 * 160 + 80][1];
        assert!(depth > 0.0, "a surface writes the centre depth");
        let view = thrum_render::synthetic::SyntheticView::perspective(
            eye,
            target,
            [0.0, 1.0, 0.0],
            50_f32.to_radians(),
            160.0 / 96.0,
            0.1,
            30_000.0,
        )
        .expect("overlap view");
        let view = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
        let level = support::world_at(view, [160, 96], [80, 48], depth).y;
        assert!(
            (level - 12.0).abs() < 0.015,
            "only the lake may write here; a second sea sheet would read 9.25 m, got {level}"
        );
    });
}
}
