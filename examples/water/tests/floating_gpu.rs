//! Game-owned forces on real Metal and long-duration fixed-tick energy bounds.
#[cfg(target_os = "macos")]
mod support;
use glam::DVec3;
use std::sync::{Arc, Mutex};
use thrum_sea_model::{Component, LongWaves, Sample};
use thrum_water_model::Surface;
use water_scene::{
    floating::{Body, Hull},
    island::Wind,
};

thrum_test_support::macos_gpu_test! {
fn running_scene_drains_metal_answers_and_draws_the_fleet() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "running_scene_drains_metal_answers_and_draws_the_fleet",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    for wind in [Wind::Calm, Wind::Gale] {
        let scene = water_scene::floating::build(wind, false, 0.0, 0.0, false).expect("scene");
        let bodies = scene.observer();
        let mailbox = scene.mailbox.clone();
        let initial = bodies();
        let mut host = support::Fixture::from_game(&test_gpu, [128, 96], scene.game, scene.report);
        for frame in 0..80 {
            host.frame(f64::from(frame) / 120.0);
        }
        let stats = mailbox.stats();
        assert!(stats.age_one + stats.age_two > 200, "{stats:?}");
        assert_eq!(stats.blocking_reads, 0);
        let bodies = bodies();
        assert_eq!(bodies.len(), 9);
        for (body, old) in bodies.iter().zip(&initial) {
            if matches!(wind, Wind::Calm) {
                assert!((body.position.y + body.draft()).abs() < 0.001);
                assert!(body.velocity.length() < 0.001);
            } else {
                assert!((body.position.y - old.position.y).abs() > 0.015);
                assert!(body.rotation.is_finite());
            }
        }
    }
}
}

#[derive(Default)]
struct FleetRecord {
    tick: u64,
    bodies: [BodyRecord; 9],
}

#[derive(Default)]
struct BodyRecord {
    energy: Option<f64>,
    input: f64,
    correlation: [f64; 3],
    least_contact: Option<f64>,
    gpu_points: u64,
    worst_energy_overage: f64,
    max_draft_error: f64,
}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: `./dev long-tests` runs it on main, from .config/long-tests.txt"]
fn actual_gale_keeps_the_fleet_in_contact_on_metal() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "actual_gale_keeps_the_fleet_in_contact_on_metal",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let seconds = 600_u32;
    let final_tick = seconds * 60;
    // Cover the old detachment interval at a frame per tick. Then use the
    // scene's normal 30 Hz capture cadence while physics stays at 60 Hz.
    let dense_ticks = 60 * 60;
    for speed in [15.0, 5.0, 0.0] {
        let scene =
            water_scene::floating::build(Wind::Gale, false, speed, std::f32::consts::PI, false)
                .expect("head-sea gale scene");
        let mailbox = scene.mailbox.clone();
        let observe_pose = scene.observer();
        let record = Arc::new(Mutex::new(FleetRecord::default()));
        let write = Arc::clone(&record);
        scene.observe_steps(move |step| {
            let mut record = write.lock().expect("step record");
            record.tick = step.tick;
            let body = &mut record.bodies[step.id as usize];
            body.least_contact = Some(
                body.least_contact
                    .map_or(step.contact, |old| old.min(step.contact)),
            );
            assert!(
                step.contact > 0.0,
                "body {} lost all contact at tick {} at {speed} m/s: {:.4} m",
                step.id,
                step.tick,
                step.contact
            );
            assert!(
                step.up > 0.62,
                "body {} listed or capsized at tick {}, speed {speed}",
                step.id,
                step.tick
            );
            body.correlation[0] += step.heave * step.waterline;
            body.correlation[1] += step.heave * step.heave;
            body.correlation[2] += step.waterline * step.waterline;
            body.gpu_points += u64::from(step.gpu_points);
            body.max_draft_error = body
                .max_draft_error
                .max((step.heave - step.waterline).abs());
            if step.tick % 120 == 1 || step.tick == u64::from(final_tick) {
                if let Some(begin) = body.energy {
                    let overage = step.energy - begin - body.input;
                    body.worst_energy_overage = body.worst_energy_overage.max(overage);
                    assert!(
                        overage <= 0.8,
                        "body {} at {speed} m/s, tick {}: energy gained {}, supplied {}",
                        step.id,
                        step.tick,
                        step.energy - begin,
                        body.input
                    );
                }
                body.energy = Some(step.energy);
                body.input = 0.0;
            }
            body.input += step.work.water + step.work.motor;
        });
        let mut host = support::Fixture::from_game(&test_gpu, [96, 64], scene.game, scene.report);
        let begin = std::time::Instant::now();
        let baseline = observe_pose()[0].position.x;
        for tick in (0..=dense_ticks).chain(((dense_ticks + 2)..=final_tick).step_by(2)) {
            host.frame_without_timings(f64::from(tick) * 0.016_666_667);
            if tick > 0 && tick % 3600 == 0 {
                eprintln!(
                    "gale {speed} m/s: {} simulated seconds, {:.1} wall seconds",
                    tick / 60,
                    begin.elapsed().as_secs_f64()
                );
            }
        }
        let final_poses = observe_pose();
        assert!(
            final_poses
                .iter()
                .all(|body| (body.rotation * DVec3::Y).y > 0.62)
        );
        let actual_speed = (baseline - final_poses[0].position.x) / f64::from(seconds);
        assert!(
            (actual_speed - f64::from(speed)).abs() < 0.3,
            "actual speed {actual_speed}"
        );
        let record = record.lock().expect("step record");
        assert_eq!(record.tick, u64::from(final_tick));
        for (id, body) in record.bodies.iter().enumerate() {
            let [cross, heave, waves] = body.correlation;
            let tracking = cross / (heave * waves).sqrt();
            assert!(
                tracking > 0.88,
                "body {id} at {speed} m/s tracks at {tracking}"
            );
            assert!(
                body.max_draft_error < 1.6,
                "body {id} at {speed} m/s departed from its draft by {} m",
                body.max_draft_error,
            );
            assert!(
                body.gpu_points >= (record.tick - 2) * 6,
                "missing GPU samples for body {id}: {}",
                body.gpu_points
            );
            eprintln!(
                "gale {speed} m/s, body {id}: least contact {:.4} m, peak draft error {:.4} m, tracking {tracking:.4}, GPU stations {}, maximum unaccounted energy {:.4} J/kg",
                body.least_contact.expect("observed contact"),
                body.max_draft_error,
                body.gpu_points,
                body.worst_energy_overage
            );
        }
        let stats = mailbox.stats();
        assert_eq!(stats.dispatches, u64::from(final_tick) + 1);
        assert_eq!(stats.blocking_reads, 0);
        eprintln!("gale {speed} m/s: actual {actual_speed:.3} m/s, {stats:?}");
    }
}
}

const DT: f64 = 1.0 / 60.0;

#[test]
fn displaced_hulls_settle_back_to_hydrostatic_draft() {
    let sea = Surface {
        body: Some(0),
        wet: true,
        ..Surface::default()
    };
    for (id, hull) in (0_u32..).zip([Hull::Craft].into_iter().chain([Hull::Float; 8])) {
        let mut body = Body::new(id, hull, [0.0; 2], 0.0, 0.0);
        body.position.y += if id % 2 == 0 { 0.24 } else { -0.31 };
        body.rotation = glam::DQuat::from_rotation_z(0.15);
        for _ in 0..600 {
            body.advance(DT, |_, _| (sea, None));
        }
        assert!((body.position.y + body.draft()).abs() < 0.001);
        assert!(body.velocity.length() < 0.001);
        assert!((body.rotation * DVec3::Y).distance(DVec3::Y) < 0.001);
    }
}

fn wave_field(phase: f64) -> LongWaves {
    let components = [
        (105.0, 0.03, 1.12, 0.7),
        (76.0, 0.19, 0.57, 1.9),
        (53.0, 0.44, 0.46, 0.9),
        (44.0, -0.52, 0.33, 2.3),
        (28.0, -0.24, 0.22, 0.2),
    ]
    .map(
        |(length, bearing, amplitude, offset): (f64, f64, f64, f64)| {
            let k = std::f64::consts::TAU / length;
            Component {
                wave_vector: [k * bearing.cos(), k * bearing.sin()],
                amplitude,
                phase: phase + offset,
            }
        },
    );
    LongWaves::new(components.to_vec(), 0.45).expect("gale waves")
}

#[test]
fn ten_minutes_of_gale_stay_bounded_at_rest() {
    gale_bounds(0.0);
}

#[test]
fn ten_minutes_of_gale_stay_bounded_at_five_metres_per_second() {
    gale_bounds(5.0);
}

#[test]
fn ten_minutes_of_gale_stay_bounded_at_fifteen_metres_per_second() {
    gale_bounds(15.0);
}

fn gale_bounds(speed: f64) {
    let field = wave_field(speed * 0.127);
    let mut craft = Body::new(0, Hull::Craft, [0.0; 2], 0.0, std::f64::consts::PI);
    craft.velocity = -speed * DVec3::X;
    craft.propulsion = craft.velocity;
    run(&field, craft, speed);
    for id in 1..=8 {
        let float = Body::new(
            id,
            Hull::Float,
            [f64::from(id) * 2.3, f64::from(id % 4) * 2.4],
            0.0,
            0.0,
        );
        run(&field, float, 0.0);
    }
}

fn run(field: &LongWaves, mut body: Body, speed: f64) {
    let mut input_work = 0.0;
    let wave_height = |p: DVec3, time| field.evaluate([p.x, p.z], time).expect("waterline").height;
    let mut interval_start_energy = body.mechanical_energy(|p| wave_height(p, 0.0));
    let mut correlation = [0.0; 3];
    let mut maximum_error = 0.0_f64;
    let mut worst = (0.0, 0.0, 0.0);
    let mut least_immersion = f64::INFINITY;
    for tick in 1..=36_000 {
        let time = f64::from(tick) * DT;
        let work = body.advance(DT, |_, p| {
            let sample = field.evaluate([p.x, p.z], time).expect("wind wave");
            (
                Surface {
                    body: Some(0),
                    wet: true,
                    sample,
                    ..Surface::default()
                },
                None,
            )
        });
        input_work += work.water + work.motor;
        let Sample { height, .. } = field
            .evaluate([body.position.x, body.position.z], time)
            .expect("waterline");
        let heave = body.position.y + body.draft();
        let error = (height - heave).abs();
        if tick > 60 {
            let immersion = (0..6)
                .map(|i| {
                    let point = body.station(i);
                    wave_height(point, time) - point.y
                })
                .fold(f64::NEG_INFINITY, f64::max);
            least_immersion = least_immersion.min(immersion);
            correlation[0] += heave * height;
            correlation[1] += heave * heave;
            correlation[2] += height * height;
            if error > maximum_error {
                maximum_error = error;
                worst = (time, heave, height);
            }
        }
        assert!(
            (body.rotation * DVec3::Y).y > 0.62,
            "body {} capsized at {time}s",
            body.id
        );
        if tick % 120 == 0 {
            let energy = body.mechanical_energy(|p| wave_height(p, time));
            // This envelope covers first-order integration error over two
            // seconds. Water and thrust must pay for any larger gain.
            assert!(
                energy - interval_start_energy <= input_work + 0.8,
                "energy at {time}s: gain {}, input {input_work}",
                energy - interval_start_energy
            );
            interval_start_energy = energy;
            input_work = 0.0;
        }
    }
    let tracking = correlation[0] / (correlation[1] * correlation[2]).sqrt();
    assert!(
        tracking > 0.88,
        "body {} at {speed} m/s tracks at {tracking}",
        body.id
    );
    assert!(
        maximum_error < 1.6 && least_immersion > -body.hull.keel_depth(),
        "body {} at {speed} m/s error {maximum_error}, least immersion {least_immersion}, time/heave/height {worst:?}",
        body.id
    );
}
