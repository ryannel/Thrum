//! Classify submitted pixels against the f64 map and the opaque arm.
use support::island;
mod support;

#[test]
fn submitted_island_has_the_maps_bodies_and_no_bridge_over_the_ridge() {
    island::with_cooked_island(
        "submitted_island_has_the_maps_bodies_and_no_bridge_over_the_ridge",
        || {
            let (mut dry, _) = island::fixture(false);
            dry.frame(0.5);
            let opaque = dry.witness();
            drop(dry);
            let (mut scene, map) = island::fixture(true);
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
                        .filter(|a| a.depth > 0.0)
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
