//! Raise and lower the lake through its sill on the live renderer.
use std::cmp::Ordering;
use support::island;
mod support;

#[test]
fn lake_stage_floods_and_drains_the_hollow_without_resetting_history() {
    island::with_cooked_island(
        "lake_stage_floods_and_drains_the_hollow_without_resetting_history",
        || {
            let camera = r#"{"camera.island":5}"#;
            let matrix = island::view_proj(5);
            let centre = island::project(matrix, [183.0, 12.0, 5.0]);
            let hollow = island::project(matrix, [235.0, 10.7, 5.0]);
            let bank = island::project(matrix, [183.0, 12.35, 35.0]);
            let [centre_bed, hollow_bed, bank_bed] = opaque_depths(camera, [centre, hollow, bank]);
            let (mut scene, map) = island::fixture(true);
            scene.parameters(camera);
            let mut elapsed = 1.0;
            scene.frame(elapsed);
            assert!(
                scene.witness()[centre][2].abs() >= 65_000.0,
                "multi-tick advance invalidates history"
            );
            let mut coverage = Vec::new();
            for (i, stage) in [
                0.0, 0.1, 0.3, 0.52, 0.71, 0.83, 1.0, 1.0, 0.83, 0.71, 0.52, 0.3, 0.1, 0.0,
            ]
            .into_iter()
            .enumerate()
            {
                scene.parameters(&format!(r#"{{"lake.stage":{stage}}}"#));
                elapsed += 1.0 / 60.0;
                scene.frame(elapsed);
                let witness = scene.witness();
                let cpu = map.lock().expect("map");
                let flooded = cpu
                    .sample([235.0, 5.0])
                    .is_some_and(|p| p.body == 1 && p.depth > 0.0);
                assert_eq!(
                    witness[hollow][1].total_cmp(&hollow_bed),
                    if flooded {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    },
                    "hollow connectivity at stage {stage}: {:?}, opaque depth {hollow_bed}",
                    witness[hollow]
                );
                if stage <= 0.3 || stage >= 0.71 {
                    assert_eq!(
                        flooded,
                        stage >= 0.71,
                        "hollow crosses its sill between stages 0.3 and 0.71"
                    );
                }
                assert!(
                    island::water(witness[centre]) && witness[centre][1] > centre_bed,
                    "lake centre draws in front of the bed"
                );
                assert!(
                    witness[centre][2].abs() < 100.0 && witness[centre][3].abs() < 100.0,
                    "continuous stage {stage} reset history at frame {i}: {:?}",
                    witness[centre]
                );
                let bank_wet = cpu
                    .sample([183.0, 35.0])
                    .is_some_and(|p| p.body == 1 && p.depth > 0.0);
                assert_eq!(
                    witness[bank][1].total_cmp(&bank_bed),
                    if bank_wet {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    },
                    "shore moves up bank at stage {stage}: {:?}, opaque depth {bank_bed}",
                    witness[bank]
                );
                if stage <= 0.0 || stage >= 1.0 {
                    assert_eq!(bank_wet, stage >= 1.0, "bank lies inside the stage range");
                }
                let level = cpu.row(1, [183.0, 5.0]).level;
                let px = [
                    u32::try_from(centre % (island::EXTENT[0] as usize)).expect("x"),
                    u32::try_from(centre / (island::EXTENT[0] as usize)).expect("y"),
                ];
                let position = support::world_at(matrix, island::EXTENT, px, witness[centre][1]);
                assert!(
                    (f64::from(position.y) - level).abs() < 0.065,
                    "submitted lake level {} vs {level}",
                    position.y
                );
                coverage.push(witness[hollow][1]);
            }
            assert_eq!(
                coverage.first(),
                coverage.last(),
                "original hollow depth restored"
            );
            assert_ne!(
                coverage[0], coverage[7],
                "flooded hollow writes the raised surface"
            );
        },
    );
}

fn opaque_depths(camera: &str, landmarks: [usize; 3]) -> [f32; 3] {
    let (mut dry, _) = island::fixture(false);
    dry.parameters(camera);
    dry.host.invalidate_history();
    dry.frame(1.0);
    let witness = dry.witness();
    // Opaque geometry writes a reactive value on a history reset. Its depth
    // still identifies the bed at each landmark.
    let hollow = witness[landmarks[1]];
    assert!(
        island::water(hollow) && hollow[2].abs() >= 65_000.0,
        "opaque reset control at the hollow: {hollow:?}"
    );
    landmarks.map(|i| {
        assert!(witness[i][1] > 0.0, "missing opaque ground at pixel {i}");
        witness[i][1]
    })
}
