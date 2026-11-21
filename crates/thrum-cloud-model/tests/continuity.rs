//! Temporal and spatial convergence rejects a cell that switches on or off.
use crate::support;
use support::parameters;
use thrum_cloud_model::{Column, Field, Profile, Type};

fn field() -> Field {
    let row = Type {
        base: 700.0,
        depth: [220.0, 950.0],
        liquid_water: 0.0003,
        droplet_radius: 9e-6,
        deck: 0.0,
        profile: [0.2, 0.3],
    };
    Field::new(
        7,
        Profile {
            pitch: 1300.0,
            turret_pitch: 340.0,
            turret_least: 0.18,
            turret_fade: 0.12,
            period: 1600.0,
            detail_mean: 0.5,
            erosion: 0.3,
        },
        vec![
            row,
            Type {
                base: 2300.0,
                deck: 1.0,
                ..row
            },
        ],
    )
    .expect("synthetic rows")
}

fn values(c: Column) -> [f64; 4] {
    [c.optical_depth, c.coverage, c.top, c.top - c.base]
}

fn accumulate(maxima: &mut [f64; 4], previous: [f64; 4], next: [f64; 4]) {
    for j in 0..4 {
        maxima[j] = maxima[j].max((next[j] - previous[j]).abs());
    }
}

// A rim's depth is proportional to distance^0.7, so halving need not halve
// its increment. Require at least 20% shrink; a step's ratio stays one.
fn converges(coarse: f64, fine: f64) -> bool {
    coarse > 0.0 && fine / coarse < 0.8
}

fn largest(step: u32, ramp: bool) -> [f64; 4] {
    let field = field();
    let mut maxima = [0.0_f64; 4];
    let mut births = 0;
    let mut deaths = 0;
    for x in [137.0, 1013.0, 2543.0, -1783.0] {
        let mut previous = [0.0; 4];
        for i in 0..=86_400 / step {
            let time = f64::from(i * step);
            // Repeated coverage/type ramps cross births, deaths and generations.
            let coverage = if ramp {
                0.55 + 0.35 * (time / 7000.0).sin()
            } else {
                0.8
            };
            let kind = if ramp {
                0.5 + 0.5 * (time / 11000.0).sin()
            } else {
                0.0
            };
            let c = field.column([x, x * 0.31], time, parameters(coverage, kind));
            assert_eq!(c.base, 700.0 + 1600.0 * kind);
            let next = values(c);
            if i > 0 {
                births += usize::from(previous[1] == 0.0 && next[1] > 0.0);
                deaths += usize::from(previous[1] > 0.0 && next[1] == 0.0);
                accumulate(&mut maxima, previous, next);
            }
            previous = next;
        }
    }
    assert!(
        births > 0 && deaths > 0,
        "must cross clear/cloud boundaries"
    );
    maxima
}

#[test]
fn halving_the_step_shrinks_changes_through_whole_lives_and_type_ramps() {
    for ramp in [false, true] {
        let coarse = largest(4, ramp);
        let fine = largest(2, ramp);
        let finer = largest(1, ramp);
        for i in 0..4 {
            assert!(converges(coarse[i], fine[i]), "{coarse:?} {fine:?}");
            assert!(converges(fine[i], finer[i]), "{fine:?} {finer:?}");
        }
    }
}

#[test]
fn overlap_is_continuous_in_space() {
    let field = field();
    let maximum = |step: f64| {
        let mut maxima = [0.0; 4];
        let mut previous = values(field.column([0.0, 500.0], 8000.0, parameters(0.8, 0.3)));
        let mut x = step;
        while x <= 5200.0 {
            let next = values(field.column([x, 500.0], 8000.0, parameters(0.8, 0.3)));
            accumulate(&mut maxima, previous, next);
            previous = next;
            x += step;
        }
        maxima
    };
    let a = maximum(2.0);
    let b = maximum(1.0);
    let c = maximum(0.5);
    for j in 0..4 {
        assert!(converges(a[j], b[j]), "{a:?} {b:?}");
        assert!(converges(b[j], c[j]), "{b:?} {c:?}");
    }
}

#[test]
fn birth_and_death_are_resolved_and_a_switched_cell_fails_each_quantity() {
    let field = field();
    let cell = field.cell(0, 0, 0);
    let clear = field.column(cell.centre, 0.0, parameters(0.0, 0.0));
    let mature = field.column(
        cell.centre,
        cell.birth + 0.5 * cell.life,
        parameters(0.8, 0.0),
    );
    assert!(mature.optical_depth > 0.0);
    for phase in [-0.1, 1.1] {
        let c = field.column(
            cell.centre,
            cell.birth + phase * cell.life,
            parameters(0.8, 0.0),
        );
        assert_eq!(
            c.optical_depth, 0.0,
            "isolated cell's endpoints must be clear"
        );
        assert_eq!(c.top, 700.0);
    }
    let measure = |step: f64, switched: bool| {
        let mut maxima = [0.0; 4];
        let mut previous = values(clear);
        let mut birth_samples = 0;
        let mut death_samples = 0;
        for i in 0..=1200 {
            let phase = -0.1 + f64::from(i) * step;
            if phase > 1.1 {
                break;
            }
            let time = cell.birth + phase * cell.life;
            let c = if switched {
                // Inject a single cell switched on between samples, with
                // positive optical depth, coverage, top and thickness jumps.
                if (0.12345..0.87654).contains(&phase) {
                    mature
                } else {
                    clear
                }
            } else {
                field.column(cell.centre, time, parameters(0.8, 0.0))
            };
            // Turrets reach the clearing share after the life envelope opens.
            birth_samples += usize::from(phase > 0.0 && phase < 0.25 && c.coverage > 0.0);
            death_samples += usize::from(phase > 0.75 && phase < 1.0 && c.coverage > 0.0);
            if i > 0 {
                accumulate(&mut maxima, previous, values(c));
            }
            previous = values(c);
        }
        if !switched {
            assert!(birth_samples > 0 && death_samples > 0);
        }
        maxima
    };
    for switched in [false, true] {
        let a = measure(0.004, switched);
        let b = measure(0.002, switched);
        let c = measure(0.001, switched);
        for j in 0..4 {
            assert_eq!(converges(a[j], b[j]), !switched, "{a:?} {b:?}");
            assert_eq!(converges(b[j], c[j]), !switched, "{b:?} {c:?}");
        }
    }
}
