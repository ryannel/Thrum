//! Temporal and spatial convergence rejects a cell that switches on or off.
pub mod support;
use support::{field, parameters};

fn largest(step: u32) -> [f64; 3] {
    let field = field(7, 0.3);
    let mut maxima = [0.0_f64; 3];
    for x in [137.0, 1013.0, 2543.0, -1783.0] {
        let mut previous = [0.0; 3];
        for i in 0..=86_400 / step {
            let time = f64::from(i * step);
            // Repeated coverage/type ramps cross births, deaths and generations.
            let coverage = 0.55 + 0.35 * (time / 7000.0).sin();
            let kind = 0.5 + 0.5 * (time / 11000.0).sin();
            let c = field.column([x, x * 0.31], time, parameters(coverage, kind));
            // Clear columns have no geometric top: compare extent above the
            // condensation level (top-base), rather than the zero sentinel.
            let values = [c.optical_depth, c.top - c.base, c.coverage];
            if i > 0 {
                for j in 0..3 {
                    maxima[j] = maxima[j].max((values[j] - previous[j]).abs());
                }
            }
            previous = values;
        }
    }
    maxima
}

#[test]
fn halving_the_step_halves_maximum_change_through_whole_lives() {
    let coarse = largest(4);
    let fine = largest(2);
    let finer = largest(1);
    for i in 0..3 {
        assert!(coarse[i] > 0.0);
        assert!(
            (0.45..0.56).contains(&(fine[i] / coarse[i])),
            "{coarse:?} {fine:?}"
        );
        assert!(
            (0.45..0.56).contains(&(finer[i] / fine[i])),
            "{fine:?} {finer:?}"
        );
    }
}

#[test]
fn overlap_is_continuous_in_space() {
    let field = field(7, 0.3);
    let maximum = |step: f64| {
        let mut max = 0.0_f64;
        let mut previous = field
            .column([0.0, 500.0], 8000.0, parameters(0.8, 0.3))
            .optical_depth;
        let mut x = step;
        while x <= 5200.0 {
            let value = field
                .column([x, 500.0], 8000.0, parameters(0.8, 0.3))
                .optical_depth;
            max = max.max((value - previous).abs());
            previous = value;
            x += step;
        }
        max
    };
    let a = maximum(2.0);
    let b = maximum(1.0);
    assert!(a > 0.0 && (0.45..0.56).contains(&(b / a)));
}
