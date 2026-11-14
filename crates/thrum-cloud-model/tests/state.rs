//! Tick authority, replay, response and accumulated-travel precision.
pub mod support;
use support::{close, parameters};
use thrum_cloud_model::{Error, State};

// Error-free TwoSum stores an extended-precision sum as unevaluated hi+lo.
// This is test-only, independent of the state's plain f64 integration.
fn wide_add(sum: &mut [f64; 2], value: f64) {
    let hi = sum[0] + value;
    let recovered = hi - sum[0];
    let error = (sum[0] - (hi - recovered)) + (value - recovered);
    sum[1] += error;
    sum[0] = hi;
}

fn bytes(state: &State) -> Vec<u8> {
    let p = state.parameters();
    [
        p.coverage,
        p.kind,
        p.wind[0],
        p.wind[1],
        p.time_scale,
        state.time(),
        state.travel()[0],
        state.travel()[1],
        state.previous_travel()[0],
        state.previous_travel()[1],
    ]
    .into_iter()
    .flat_map(f64::to_le_bytes)
    .collect()
}

#[test]
fn ticks_integrate_wind_and_replay_exactly() {
    for scale in [1.0, 60.0] {
        let mut p = parameters(0.3, 0.2);
        p.wind = [40.0, -13.0];
        p.time_scale = scale;
        let mut a = State::new(p, 5.0, 16).expect("state");
        let mut b = a.clone();
        let mut sums = [[0.0; 2]; 2];
        let mut expected_wind = p.wind;
        let dt = 1.0 / 60.0;
        for tick in 1..=5_184_000 {
            if tick == 2_592_000 {
                p.wind = [-7.0, 23.0];
                p.coverage = 0.8;
                p.kind = 0.9;
                let before = a.travel();
                a.set_target(p).expect("target");
                b.set_target(p).expect("target");
                assert_eq!(a.travel(), before);
            }
            let previous = a.travel();
            a.step(tick, dt).expect("step");
            b.step(tick, dt).expect("replay");
            assert_eq!(a, b);
            if tick % 60 == 0 {
                assert_eq!(bytes(&a), bytes(&b));
            }
            for i in 0..2 {
                let target = p.wind[i];
                let distance = (target - expected_wind[i])
                    .abs()
                    .min(40.0 * dt * scale / 5.0);
                expected_wind[i] += (target - expected_wind[i]).signum() * distance;
                close(a.parameters().wind[i], expected_wind[i], 1e-11);
                wide_add(&mut sums[i], expected_wind[i] * dt * scale);
                close(
                    a.display_travel(0.25, false)[i],
                    a.travel()[i] - 0.75 * (a.travel()[i] - previous[i]),
                    0.0,
                );
            }
        }
        // 5.2 million additions: bound by n * epsilon * total magnitude.
        for (actual, expected) in a.travel().into_iter().zip(sums.map(|s| s[0] + s[1])) {
            close(
                actual,
                expected,
                5_184_000.0 * f64::EPSILON * 40.0 * 86_400.0 * scale,
            );
        }
        let before = a.clone();
        assert_eq!(a.step(5_184_000, dt), Err(Error::Tick));
        assert_eq!(a, before);
        assert_eq!(a.step(5_184_001, f64::NAN), Err(Error::Invalid));
        assert_eq!(a, before);
        assert_eq!(a.display_travel(0.0, true), a.travel());
    }
}

#[test]
fn controls_ramp_at_the_response_rate() {
    let mut state = State::new(parameters(0.0, 0.0), 10.0, 0).expect("state");
    state.set_target(parameters(1.0, 1.0)).expect("target");
    state.step(1, 2.0).expect("step");
    close(state.parameters().coverage, 0.2, 1e-15);
    close(state.parameters().kind, 0.2, 1e-15);
}
