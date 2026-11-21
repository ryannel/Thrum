//! A growing turret's clearing is continuous in the public point query.
use crate::support;

use thrum_cloud_model::Column;

#[test]
fn halving_an_envelope_step_shrinks_every_change_at_the_clearing() {
    let field = support::field(7, 0.0);
    let cell = field.cell(0, 0, 0);
    let sample = |envelope: f64| {
        let phase = envelope.sqrt().asin() / std::f64::consts::PI;
        let time = cell.birth + phase * cell.life;
        let mut state = support::state(0.8, 0.0);
        state
            .step(1, time)
            .expect("sample on the growing half of the cell");
        field.query([cell.centre[0], 0.0, cell.centre[1]], &state)
    };
    assert_eq!(sample(0.0).coverage, 0.0);
    assert!(sample(1.0).coverage > 0.0);
    // Locate the transition with the public query. The probe must cross it.
    let (mut clear, mut cloudy) = (0.0_f64, 1.0_f64);
    for _ in 0..40 {
        let mid = clear.midpoint(cloudy);
        if sample(mid).coverage == 0.0 {
            clear = mid;
        } else {
            cloudy = mid;
        }
    }
    let clearing = clear.midpoint(cloudy);
    let threshold = ((0.95 - cell.threshold) / 0.15).clamp(0.0, 1.0);
    let mature_dome = threshold * threshold * (3.0 - 2.0 * threshold);
    let values = |c: Column| [c.coverage, c.top - c.base, c.top, c.optical_depth];
    let mut previous = [f64::INFINITY; 4];
    let mut previous_switch = f64::INFINITY;
    for step in [0.02, 0.01, 0.005, 0.0025] {
        let a = sample(clearing - step);
        let b = sample(clearing + step);
        assert_eq!(a.coverage, 0.0);
        assert!(b.coverage > 0.0);
        let changes = std::array::from_fn(|i| (values(b)[i] - values(a)[i]).abs());
        for (change, last) in changes.into_iter().zip(previous) {
            assert!(
                change > 0.0 && change < 0.55 * last,
                "{changes:?} vs {previous:?}"
            );
        }
        // At the isolated centre the dome's coverage is proportional to its
        // envelope. A switch to full coverage at the same clearing has a jump.
        let switched = mature_dome * (clearing + step);
        if previous_switch.is_finite() {
            assert!(switched >= 0.55 * previous_switch);
        }
        println!("step {step}: {changes:?}, switched coverage {switched}");
        previous = changes;
        previous_switch = switched;
    }
}
