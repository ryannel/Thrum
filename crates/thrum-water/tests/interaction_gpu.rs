//! Submitted Metal interaction endpoints and the public body query.
mod interaction_fixture;

use interaction_fixture::{Fixture, TERRAIN_WIDTH, current, shore, terrain};
use thrum_water::{
    interaction::{BodyPressure, Limits, Witness},
    query::{BodyQuery, QueryPoint},
};
use thrum_water_model::Impulse;

fn pressure(id: u32, place: [f64; 2], heading: f64) -> BodyPressure {
    BodyPressure {
        id,
        place,
        heading,
        footprint: [1.45, 0.58],
        force: 36_000.0,
    }
}

fn impact(tick: u64, id: u64, x: f64, vertical: f64) -> Impulse {
    Impulse {
        id,
        place: [x, 0.0],
        vertical,
        radius: 0.45,
        tick,
    }
}

#[test]
fn moving_pressure_answers_from_its_submitted_endpoints() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -4.0), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    let probes = [[-7.25, 2.5], [-5.375, 2.75], [-3.0, 3.0], [-10.0, -2.5]];
    let mut peak = 0.0_f64;
    let mut error = 0.0_f64;
    let mut final_place = [0.0; 2];
    for tick in (8..=240).step_by(8) {
        for tick in (tick - 7)..=tick {
            let time = tick as f64 / 60.0;
            let (place, heading) = if time < 1.3 {
                ([-10.0 + 6.0 * time, 0.0], 0.0)
            } else if time < 2.5 {
                let angle = (time - 1.3) * 1.3;
                ([-2.2 + 4.6 * angle.sin(), 4.6 * (1.0 - angle.cos())], angle)
            } else if time < 3.2 {
                (final_place, 1.56)
            } else {
                ([final_place[0], 4.55 + (time - 3.2) * 10.0], 1.56)
            };
            final_place = place;
            fixture
                .mailbox()
                .post_bodies(tick, Some(1), vec![pressure(1, place, heading)]);
        }
        fixture.query().post(
            tick - 1,
            vec![BodyQuery {
                id: 99,
                generation: 1,
                points: probes
                    .map(|position| QueryPoint {
                        position,
                        radius: 0.0,
                        use_tick: tick,
                    })
                    .to_vec(),
            }],
        );
        fixture.draw(tick);
        let witness = fixture.read();
        assert_eq!(
            witness.endpoint.endpoints.map(|endpoint| endpoint.tick),
            [tick - 8, tick]
        );
        let answers = fixture.query().drain(tick, Fixture::STEP);
        for (answer, point) in answers[0].points.iter().zip(probes) {
            let actual = answer.surface.sample.height;
            let expected = f64::from(witness.height(point));
            peak = peak.max(expected.abs());
            error = error.max((actual - expected).abs());
            assert!(
                (actual - expected).abs() < 0.000_02,
                "tick {tick}, {point:?}: {actual} != {expected}"
            );
        }
    }
    assert!(
        peak > 0.015,
        "proof requires a wave far above its 20 micrometre tolerance"
    );
    eprintln!("Metal endpoint query peak {peak:.6} m, maximum error {error:.7} m");

    // Move more than either entire window on one frame, with no source.
    // Old water must disappear from both levels, and cannot wrap.
    fixture.mailbox().post_bodies(
        241,
        Some(1),
        vec![BodyPressure {
            force: 0.0,
            ..pressure(1, [850.0, 900.0], 0.0)
        }],
    );
    fixture.draw(241);
    let witness = fixture.read();
    assert!(
        witness
            .level(0)
            .iter()
            .chain(witness.level(1))
            .copied()
            .all(on_rest)
    );
}

#[test]
fn barrier_keeps_disconnected_water_at_exact_rest_and_counts_refused_sources() {
    // With one-metre bed spacing the two zero crossings are -0.5 m and 1.5 m.
    let bar = terrain(|x| if (0.0..2.0).contains(&x) { 4.0 } else { -4.0 });
    let Some(mut fixture) = Fixture::new(
        bar,
        Limits {
            solves: 3,
            impulses: 2,
            bodies: 2,
        },
    ) else {
        return;
    };
    fixture.draw(0);
    fixture
        .mailbox()
        .post_impulse(impact(1, 90, -4.0, 64_000.0));
    fixture.mailbox().post_impulse(impact(1, 10, -4.0, 400.0));
    fixture.mailbox().post_impulse(impact(1, 20, 6.0, 400.0));
    fixture.mailbox().post_bodies(
        1,
        Some(1),
        vec![pressure(1, [-4.0, 0.0], 0.0), pressure(2, [6.0, 0.0], 0.0)],
    );
    fixture.query().post(
        0,
        vec![BodyQuery {
            id: 2,
            generation: 1,
            points: vec![QueryPoint {
                position: [6.0, 0.0],
                radius: 0.0,
                use_tick: 1,
            }],
        }],
    );
    fixture.draw(1);
    let report = fixture
        .report
        .lock()
        .expect("water report")
        .interaction
        .expect("interaction");
    assert_eq!(report.refused_impulses, 1);
    assert_eq!(report.refused_regions, [2, 2]);
    assert_eq!(report.regions, [2, 2]);
    assert_eq!(report.solves, [3, 2]);
    let witness = fixture.read();
    let answer = &fixture.query().drain(1, Fixture::STEP)[0].points[0].surface;
    assert!(answer.wet, "the refused body must be over water");
    assert_eq!(answer.sample.height.to_bits(), 0);
    eprintln!(
        "Metal refused body height over wet cells: {} m",
        answer.sample.height
    );
    for level in 0..2 {
        let [origin, _] = witness.endpoint.endpoints[1].origins[level];
        let spacing = Witness::CELL_METRES[level];
        for (i, value) in witness.level(level).iter().enumerate() {
            let x = (i % Witness::SIDE) as f32 * spacing + origin;
            if (3.0..TERRAIN_WIDTH / 2.0).contains(&x) {
                assert!(
                    on_rest(*value),
                    "wet region opposite the bar: {x}, {value:?}"
                );
            }
        }
    }

    drop(witness);
    drop(fixture);

    // Repeat with both wet regions admitted. No dropped solve can explain
    // the exact zero beyond the bar in this second run.
    let Some(mut fixture) = Fixture::new(
        terrain(|x| if (0.0..2.0).contains(&x) { 4.0 } else { -4.0 }),
        Limits::default(),
    ) else {
        return;
    };
    fixture.draw(0);
    fixture.mailbox().post_impulse(impact(1, 10, -4.0, 400.0));
    for tick in (1..=89).step_by(8) {
        fixture.draw(tick);
    }
    let witness = fixture.read();
    let mut peak = 0.0_f32;
    for level in 0..2 {
        let [origin, _] = witness.endpoint.endpoints[1].origins[level];
        let spacing = Witness::CELL_METRES[level];
        for (i, value) in witness.level(level).iter().enumerate() {
            let x = (i % Witness::SIDE) as f32 * spacing + origin;
            if (3.0..TERRAIN_WIDTH / 2.0).contains(&x) {
                assert!(
                    on_rest(*value),
                    "wet region opposite the bar: {x}, {value:?}"
                );
            } else if x < -1.0 {
                peak = peak.max(value[0].abs());
            }
        }
    }
    assert!(peak > 0.000_01);
}

#[test]
fn impact_is_applied_once_at_the_named_tick_and_scales_with_impulse() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -4.0), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    fixture.mailbox().post_impulse(impact(2, 8, 0.0, 200.0));
    fixture.mailbox().post_impulse(impact(2, 8, 0.0, 5_000.0));
    fixture.draw(1);
    assert_eq!(fixture.read().height([0.0; 2]).to_bits(), 0);
    fixture.draw(2);
    let small = fixture.read();
    assert!(small.height([0.0; 2]) < -0.002);
    fixture.draw(2);
    assert!(
        small.cells == fixture.read().cells,
        "repeat changed the wake"
    );
    fixture.draw(3);
    let evolved = fixture.read().height([1.0, 0.0]);
    fixture.timeline(2);
    fixture.draw(4);
    assert_eq!(fixture.read().height([0.0; 2]).to_bits(), 0);
    fixture.draw(5);
    fixture.mailbox().post_impulse(impact(6, 8, 0.0, 400.0));
    fixture.draw(6);
    assert!((fixture.read().height([0.0; 2]) - 2.0 * small.height([0.0; 2])).abs() < 0.000_001);
    fixture.draw(7);
    assert!((fixture.read().height([1.0, 0.0]) - 2.0 * evolved).abs() < 0.000_001);

    // A drop can name a tick caught up between presentations. Its near
    // window follows the drop, even away from the scene's original origin.
    fixture.mailbox().post_impulse(impact(10, 9, 120.0, 400.0));
    fixture.draw(12);
    let witness = fixture.read();
    assert!(witness.height([120.0, 0.0]) < -0.005);
    assert!(witness.endpoint.endpoints[1].origins[0][0] > 80.0);
}

#[test]
fn admitted_impulses_survive_future_pages_and_discard() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -4.0), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    for tick in 1..=33 {
        fixture.mailbox().post_impulse(impact(tick, 8, 0.0, 400.0));
        fixture.mailbox().post_bodies(tick, None, Vec::new());
    }
    fixture.discard(1);
    assert_eq!(fixture.read().endpoint.endpoints.map(|e| e.tick), [0, 0]);
    fixture.draw(1);
    let height = fixture.read().height([0.0; 2]);
    let report = fixture
        .report
        .lock()
        .expect("water report")
        .interaction
        .expect("interaction");
    eprintln!("Metal tick-1 impulse with 32 later pages queued: {height:.6} m");
    assert!(height < -0.005);
    assert_eq!(report.refused_impulses, 0);
    assert_eq!(report.dropped_ticks, 0);
}

#[test]
fn endpoint_clock_keeps_discarded_and_capture_frames_out_of_history() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -4.0), Limits::default()) else {
        return;
    };
    fixture.discard(0);
    assert!(
        fixture.witness().is_none(),
        "discard published initial banks"
    );
    assert!(
        fixture
            .report
            .lock()
            .expect("water report")
            .interaction
            .is_none()
    );
    fixture.draw(1);
    let initial = fixture.read();
    assert_eq!(initial.endpoint.endpoints.map(|e| e.tick), [1, 1]);
    assert!(initial.cells.iter().copied().all(on_rest));
    fixture.mailbox().post_impulse(impact(2, 1, 0.0, 300.0));
    fixture.draw(2);
    let original = fixture.read();
    fixture.discard(4);
    fixture.capture();
    let retained = fixture.read();
    assert!(
        original.cells == retained.cells,
        "discard or capture changed the wake"
    );
    assert_eq!(retained.endpoint.endpoints.map(|e| e.tick), [1, 2]);
    fixture.draw(16);
    let report = fixture
        .report
        .lock()
        .expect("water report")
        .interaction
        .expect("interaction");
    assert_eq!(report.endpoints, Some([2, 16]));
    assert_eq!(report.dropped_ticks, 6);
    fixture.draw(7);
    let backwards = fixture.read();
    assert_eq!(backwards.endpoint.endpoints.map(|e| e.tick), [7, 7]);
    assert!(backwards.cells.iter().copied().all(on_rest));
    fixture.timeline(3);
    fixture.draw(8);
    assert_eq!(fixture.read().endpoint.endpoints.map(|e| e.tick), [8, 8]);
    fixture.mailbox().post_impulse(impact(9, 7, 0.0, 300.0));
    fixture.draw(9);
    assert!(fixture.read().height([0.0; 2]) < -0.002);
    fixture.replace_device();
    fixture.draw(9);
    let replaced = fixture.read();
    assert_eq!(replaced.endpoint.endpoints.map(|e| e.tick), [9, 9]);
    assert!(replaced.cells.iter().copied().all(on_rest));
}

fn on_rest(cell: [f32; 2]) -> bool {
    cell.map(|v| v.to_bits() & 0x7fff_ffff) == [0; 2]
}

#[test]
fn a_shared_frequency_is_emitted_by_its_own_level() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -1000.0), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    fixture.mailbox().post_impulse(impact(1, 4, 0.0, 800.0));
    fixture.draw(1);
    let witness = fixture.read();
    let amplitude = |level, wavelength: f64| {
        let spacing = f64::from(Witness::CELL_METRES[level]);
        let [real, imaginary] = witness.level(level).iter().enumerate().fold(
            [0.0, 0.0],
            |[real, imaginary], (index, cell)| {
                let x = (index % Witness::SIDE) as f64 * spacing;
                let (sin, cos) = (x * std::f64::consts::TAU / wavelength).sin_cos();
                let value = f64::from(cell[1]);
                [real + value * cos, imaginary + value * sin]
            },
        );
        real.hypot(imaginary) * spacing * spacing
    };
    let long = [amplitude(0, 32.0), amplitude(1, 32.0)];
    let short = [amplitude(0, 8.0), amplitude(1, 8.0)];
    eprintln!("Metal potential amplitudes (near/far), 32 m {long:?}, 8 m {short:?}");
    assert!(long[1] > 0.4 && long[0] < long[1] * 0.15);
    assert!(short[0] > 0.4 && short[1] < short[0] * 0.15);
}

#[test]
fn a_hitch_puts_pressure_along_the_path() {
    let Some(mut fixture) = Fixture::new(terrain(|_| -1000.0), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    for tick in 1..=8 {
        fixture.mailbox().post_bodies(
            tick,
            Some(1),
            vec![pressure(1, [tick as f64 - 4.0, 0.0], 0.0)],
        );
    }
    fixture.draw(8);
    let wake = fixture.read();
    assert_eq!(wake.endpoint.endpoints.map(|e| e.tick), [0, 8]);
    let heights = [-3.0, 0.0, 4.0].map(|x| wake.height([x, 0.0]));
    eprintln!("Metal pressure along the eight-tick hitch: {heights:?}");
    assert!(heights.into_iter().all(|height| height < -0.006));
}

#[test]
fn a_uniform_current_advects_the_ring() {
    let mut centroids = [0.0; 2];
    for (speed, centroid) in [0.0, 1.25].into_iter().zip(&mut centroids) {
        let Some(mut fixture) = Fixture::new(current(speed), Limits::default()) else {
            return;
        };
        fixture.draw(0);
        fixture.mailbox().post_impulse(impact(1, 0, 0.0, 400.0));
        for tick in (1..=97).step_by(8) {
            fixture.draw(tick);
        }
        let field = fixture.read();
        let origin = f64::from(field.endpoint.endpoints[1].origins[0][0]);
        let mut sum = 0.0;
        let mut energy = 0.0;
        let mut moment = 0.0;
        for (index, cell) in field.level(0).iter().enumerate() {
            let x = origin + (index % Witness::SIDE) as f64 * 0.25;
            let square = f64::from(cell[0]).powi(2);
            sum += x * square;
            moment += x * x * square;
            energy += square;
        }
        *centroid = sum / energy;
        assert!(
            moment / energy - centroid.powi(2) > 1.0,
            "ring did not spread"
        );
    }
    let travelled = centroids[1] - centroids[0];
    eprintln!("Metal ring advanced {travelled:.4} m with uniform 1.25 m/s current in 1.6 s");
    assert!((travelled - 2.0).abs() < 0.12);
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "seed the production field in its f32 format"
)]
fn a_half_dry_window_absorbs_an_incoming_packet() {
    let Some(mut fixture) = Fixture::new(shore(), Limits::default()) else {
        return;
    };
    fixture.draw(0);
    let k = std::f64::consts::TAU / 2.5;
    let dispersion = k * (1.8 * k).tanh();
    let potential_amplitude = (9.81 / dispersion).sqrt();
    fixture.seed_near(|x, z| {
        let along = x + 9.0;
        let envelope = 0.025 * (-0.5 * ((along / 1.8).powi(2) + (z / 3.0).powi(2))).exp();
        let (sin, cos) = (k * along).sin_cos();
        if x >= 0.0 {
            [0.0; 2]
        } else {
            [
                (envelope * cos) as f32,
                (envelope * potential_amplitude * sin) as f32,
            ]
        }
    });
    let initial = fixture.read();
    let arriving: f64 = initial
        .level(0)
        .iter()
        .map(|cell| f64::from(cell[0]).powi(2))
        .sum();
    let mut returning = 0.0_f64;
    for tick in (8..=1_312).step_by(8) {
        fixture.draw(tick);
        if (832..=1_312).contains(&tick) && tick % 32 == 0 {
            let wake = fixture.read();
            let energy = wake
                .level(0)
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| {
                    let x = (index % Witness::SIDE) as f32 * 0.25 - 32.0;
                    assert!(x < 0.0 || on_rest(*cell), "dry half has a wave at {x}");
                    (x < -2.0).then_some(f64::from(cell[0]).powi(2))
                })
                .sum::<f64>();
            returning = returning.max(energy);
        }
    }
    let ratio = returning / arriving;
    eprintln!("Metal half-dry sloping shore returned {ratio:.3} of incident squared height");
    assert!(
        arriving > 0.03 && ratio < 0.1,
        "returning shore energy {ratio:.4}"
    );
}
