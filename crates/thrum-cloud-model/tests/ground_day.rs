//! Three deterministic days driven by supplied solar elevation.
pub mod ground_support;
pub mod support;
use thrum_cloud_model::{GroundState, State};

fn days(dt: f64, coverage: f64, wind: f64, moisture: f64) -> (Vec<(f64, f64)>, State) {
    let mut row = ground_support::row();
    row.radiative = true;
    let ground = ground_support::table(vec![row, ground_support::row()]);
    let mut p = support::parameters(coverage, 0.0);
    p.wind = [wind, 0.0];
    let mut state = State::new(p, 10.0, 0).expect("state");
    state.set_ground(GroundState::new(vec![0.0, 0.42], moisture, 40_000.0).expect("controls"));
    let mut values = Vec::new();
    let mut tick = 0;
    let mut time = 0.0;
    while time < 3.0 * 86400.0 {
        let hour = (time / 3600.0) % 24.0;
        let sun = 45.0 * (std::f64::consts::TAU * (hour - 6.0) / 24.0).sin();
        tick += 1;
        state.step_day(tick, dt, sun, &ground).expect("day");
        if time >= 2.0 * 86400.0 {
            values.push((hour, state.ground().amount(0, true)));
        }
        assert_eq!(state.ground().amount(1, false), 0.42);
        time += dt;
    }
    (values, state)
}

#[test]
fn radiative_day_is_deterministic_suppressed_and_tick_continuous() {
    let (values, state) = days(60.0, 0.0, 1.0, 0.5);
    assert_eq!((values.clone(), state), days(60.0, 0.0, 1.0, 0.5));
    let peak = values
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("samples");
    assert!(peak.1 > 0.8);
    assert!((5.0..6.0).contains(&peak.0), "peak at {peak:?}");
    assert!(
        values
            .iter()
            .filter(|v| (11.0..18.0).contains(&v.0))
            .all(|v| v.1 == 0.0)
    );
    let at = |hour: f64| values.iter().find(|v| v.0 >= hour).expect("hour").1;
    assert!(at(22.0) > at(20.0) && at(20.0) > at(18.0));
    for (c, w, m) in [
        (1.0, 1.0, 0.5),
        (1.0, 1.0, 0.7),
        (1.0, 1.0, 1.0),
        (0.0, 9.0, 1.0),
        (0.0, 1.0, 0.0),
    ] {
        assert!(days(60.0, c, w, m).0.iter().all(|v| v.1 < 0.1 * peak.1));
    }
    let max_delta = |v: &[(f64, f64)]| {
        v.windows(2)
            .map(|w| (w[1].1 - w[0].1).abs())
            .fold(0.0_f64, f64::max)
    };
    support::close(
        max_delta(&days(30.0, 0.0, 1.0, 0.5).0) / max_delta(&values),
        0.5,
        0.005,
    );
}

#[test]
fn invalid_day_and_targets_preserve_state() {
    let ground = ground_support::table(vec![ground_support::row()]);
    let mut s = support::state(0.0, 0.0);
    ground_support::controls(&mut s, vec![0.5], 0.7);
    let before = s.clone();
    assert!(s.step_day(1, 1.0, f64::NAN, &ground).is_err());
    assert_eq!(s, before);
    assert!(s.step_day(2, 1.0, -10.0, &ground).is_err());
    assert_eq!(s, before);
    assert!(
        s.ground_mut()
            .set_targets(&[0.1, 0.2], 0.4, 10_000.0)
            .is_err()
    );
    assert_eq!(s, before);
}
