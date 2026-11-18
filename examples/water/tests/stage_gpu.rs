//! Raise and lower the lake through its sill on the live renderer.
use std::{cmp::Ordering, time::Duration};
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
            let mut cpu = water_scene::island::map(water_scene::island::Wind::Calm)
                .expect("CPU stage oracle");
            scene.parameters(camera);
            let step = Duration::from_secs_f64(1.0 / 60.0);
            let before = scene.frame((step * 60 + step / 2).as_secs_f64());
            assert!(
                scene.witness()[centre][2].abs() >= 65_000.0,
                "multi-tick advance invalidates history"
            );
            assert_eq!(before.tick, 60);
            let px = [
                u32::try_from(centre % (island::EXTENT[0] as usize)).expect("x"),
                u32::try_from(centre / (island::EXTENT[0] as usize)).expect("y"),
            ];
            let mut from = 0.0_f64;
            let mut last_stage = 0.0;
            println!(
                "tick alpha target sampled_stage submitted_lake_y hollow_wet bank_wet motion_y"
            );
            for (index, target) in [1.0, 0.0, 0.3, 0.52, 0.71, 0.83, 1.0, 0.71, 0.3, 0.0]
                .into_iter()
                .enumerate()
            {
                scene.parameters(&format!(r#"{{"lake.stage":{target}}}"#));
                let tick = 61 + u32::try_from(index).expect("sweep ticks");
                for phase in 1..=4 {
                    let offset =
                        (step * phase / 4).min(step.saturating_sub(Duration::from_nanos(1)));
                    let frame = scene.frame((step * tick + offset).as_secs_f64());
                    assert_eq!(frame.tick, u64::from(tick));
                    assert!((frame.presentation_alpha - phase as f32 / 4.0).abs() < 2e-6);
                    let alpha = f64::from(frame.presentation_alpha);
                    let stage = from + (target - from) * alpha;
                    let witness = scene.witness();
                    let endpoint = map
                        .lock()
                        .expect("map")
                        .body(1)
                        .expect("lake body")
                        .stage
                        .value;
                    assert!((endpoint - target).abs() < 1e-6);
                    cpu.set_stage(1, stage).expect("oracle stage");
                    let flooded = coverage(&cpu, [235.0, 5.0], witness[hollow][1], hollow_bed);
                    let bank_wet = coverage(&cpu, [183.0, 35.0], witness[bank][1], bank_bed);
                    if stage <= 0.3 || stage >= 0.71 {
                        assert_eq!(flooded, stage >= 0.71, "the sill holds back the hollow");
                    }
                    assert!(island::water(witness[centre]) && witness[centre][1] > centre_bed);
                    let position =
                        support::world_at(matrix, island::EXTENT, px, witness[centre][1]);
                    let level = cpu.row(1, [183.0, 5.0]).level;
                    assert!(
                        (f64::from(position.y) - level).abs() < 0.065,
                        "submitted lake level {} vs {level} at tick {tick} alpha {alpha}",
                        position.y
                    );
                    check_motion(matrix, position, last_stage - stage, witness[centre]);
                    last_stage = stage;
                    println!(
                        "{tick} {alpha:.6} {target:.2} {stage:.6} {:.4} {flooded} {bank_wet} {:.5}",
                        position.y, witness[centre][3]
                    );
                }
                from = target;
            }
        },
    );
}

fn coverage(map: &thrum_water_model::Map, point: [f64; 2], depth: f32, opaque: f32) -> bool {
    let wet = map
        .sample(point)
        .is_some_and(|p| p.body == 1 && p.depth > 0.0);
    assert_eq!(
        depth.total_cmp(&opaque),
        if wet {
            Ordering::Greater
        } else {
            Ordering::Equal
        },
        "coverage at {point:?} at level {}: depth {depth}, opaque {opaque}",
        map.row(1, point).level
    );
    wet
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the projected displacement is in GPU metres"
)]
fn check_motion(matrix: glam::Mat4, position: glam::Vec3, delta: f64, witness: [f32; 4]) {
    let current = matrix.project_point3(position);
    let previous = matrix.project_point3(position + glam::Vec3::Y * delta as f32);
    let motion = (previous - current).truncate()
        * glam::Vec2::new(island::EXTENT[0] as f32, -(island::EXTENT[1] as f32))
        * 0.5;
    let actual = glam::Vec2::new(witness[2], witness[3]);
    assert!(
        (actual - motion).length() < 0.025,
        "submitted motion {actual:?} vs {motion:?}"
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
