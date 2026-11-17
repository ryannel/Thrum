//! Display prediction agrees with actual ticks, including ramps reaching a target.
use thrum_cloud_model::{Parameters, State};

fn controls(wind: f64, scale: f64) -> Parameters {
    Parameters {
        coverage: 0.4,
        kind: 0.5,
        wind: [wind, -wind],
        time_scale: scale,
    }
}

fn close(a: [f64; 2], b: [f64; 2]) {
    for i in 0..2 {
        // At most two differently associated endpoint interpolations; allow
        // 16 ulps of the travel magnitude, including cancellation near zero.
        assert!(
            (a[i] - b[i]).abs() <= 16.0 * f64::EPSILON * a[i].abs().max(b[i].abs()).max(1.0),
            "{a:?} != {b:?}"
        );
    }
}

#[test]
fn predicted_ticks_match_real_steps_without_committing() {
    let mut pairs = 0;
    for hz in [30, 60, 240] {
        let dt = 1.0 / f64::from(hz);
        for (start, target, response) in [
            (controls(40.0, 60.0), controls(40.0, 60.0), 10.0),
            (controls(10.0, 60.0), controls(40.0, 60.0), 10.0),
            (controls(10.0, 8.0), controls(40.0, 60.0), 10.0),
            (controls(39.0, 59.0), controls(40.0, 60.0), 2.0),
            (controls(40.0, 60.0), controls(-10.0, 1.0), 10.0),
        ] {
            let mut state = State::new(start, response, 0).unwrap();
            state.set_target(target).unwrap();
            for tick in 1..=40 {
                state.step(tick, dt).unwrap();
                let before = state.clone();
                for part in 0..=200 {
                    let p = (1.0 + 0.05 / dt) * f64::from(part) / 200.0;
                    let travel = state.display_travel(p, dt, false);
                    let mut later = state.clone();
                    for k in 1..=hz / 20 + 1 {
                        if p < f64::from(k) {
                            break;
                        }
                        later.step(later.tick() + 1, dt).unwrap();
                        close(travel, later.display_travel(p - f64::from(k), dt, false));
                        pairs += 1;
                    }
                    assert_eq!(state.display_travel(p, dt, true), state.travel());
                }
                assert_eq!(state, before);
                assert_eq!(state.display_travel(1.0, dt, false), state.travel());
            }
        }
        let mut steady = State::new(controls(40.0, 60.0), 10.0, 0).unwrap();
        steady.step(1, dt).unwrap();
        for p in [0.0, 0.25, 1.0, 1.5, 1.0 + 0.05 / dt] {
            close(
                steady.display_travel(p, dt, false),
                [40.0 * 60.0 * dt * p, -40.0 * 60.0 * dt * p],
            );
        }
    }
    println!("{pairs} same-display-time pairs; 16-ulp relative f64 rounding bound");
}

#[test]
fn ramp_rejects_one_increment_and_changed_target_has_only_its_correction() {
    let dt = 1.0 / 60.0;
    let mut state = State::new(controls(10.0, 60.0), 10.0, 0).unwrap();
    state.set_target(controls(40.0, 60.0)).unwrap();
    // Tick zero is a valid committed endpoint; first predicted winds are 14,18,22.
    close(state.display_travel(3.5, dt, false), [43.0, -43.0]);
    let one_increment = 14.0 * 2.5;
    assert_eq!(43.0 - one_increment, 8.0);
    let before = state.display_travel(3.5, dt, false);
    state.step(1, dt).unwrap();
    close(before, state.display_travel(2.5, dt, false));
    // New target at the next commit: winds 10,6 versus old 18,22.
    state.set_target(controls(-40.0, 60.0)).unwrap();
    state.step(2, dt).unwrap();
    let changed = state.display_travel(1.5, dt, false);
    close(changed, [14.0 + 10.0 + 0.5 * 6.0, -27.0]);
    close(before, [14.0 + 18.0 + 0.5 * 22.0, -43.0]);
    assert_eq!(before[0] - changed[0], (18.0 - 10.0) + 0.5 * (22.0 - 6.0));
    println!(
        "Single-increment prediction misses 8 m; changed-target correction is 16 m from independent wind arithmetic"
    );
}
