//! M3.1: a river with a source body is solved at levels across that body's stage range no more
//! than 0.3 m apart, and a spring-fed one at the discharges declared. Each sample records its
//! gauge level, and the river reports the range of its gauge. Below a lake with a sill the
//! discharge rises with the level at every sample, and between the lowest and the highest it
//! grows as the head over the sill to a power between 1.3 and 2.
use crate::support;
use support::lake::{HIGH, SILL, lake};
use support::valley::DISCHARGE;
use thrum_river_model::{Profile, key_for_samples, sample_levels, solve_samples, write};
use thrum_water_model::RiverFile;

#[test]
fn levels_across_a_range_are_never_more_than_the_spacing_apart() {
    for (range, count) in [
        ([3.1, 3.7], 3),
        ([3.1, 3.75], 4),
        ([0.0, 0.3], 2),
        ([1.0, 1.0], 1),
        ([0.0, 2.0], 8),
    ] {
        let l = sample_levels(range, 0.3);
        assert_eq!(l.len(), count, "{range:?}: {l:?}");
        assert_eq!(l[0], range[0]);
        assert_eq!(l[l.len() - 1], range[1]);
        assert!(
            l.windows(2)
                .all(|w| w[1] > w[0] && w[1] - w[0] <= 0.3 + 1e-12)
        );
    }
}

#[test]
fn a_lake_s_outlet_is_solved_across_its_range_and_its_discharge_follows_the_head() {
    let (grid, declaration) = lake(0.25, HIGH, [3.1, 3.7]);
    let profile = Profile::default();
    let s = solve_samples(&grid, &declaration, &profile, &[]).unwrap();
    assert_eq!(s.gauges, sample_levels([3.1, 3.7], 0.3));
    assert_eq!(s.gauges.len(), 3);
    assert!((s.gauges[1] - 3.4).abs() < 1e-12);
    assert_eq!(s.gauge_range(), [3.1, 3.7]);
    let q: Vec<f64> = s.solutions.iter().map(|x| x.report.discharge).collect();
    for (k, x) in s.solutions.iter().enumerate() {
        let r = &x.report;
        println!(
            "lake at {:.2} m, head {:.2} m: {:.4} m³/s; {} steps, {:.2} s, {} wet cells",
            s.gauges[k],
            s.gauges[k] - SILL,
            q[k],
            r.steps,
            r.seconds,
            r.wet_cells
        );
    }
    assert!(q.windows(2).all(|w| w[1] > w[0]), "{q:?}");
    let (h0, h1) = (s.gauges[0] - SILL, s.gauges[2] - SILL);
    let power = (q[2] / q[0]).ln() / (h1 / h0).ln();
    println!("discharge grows as the head to the power {power:.3}");
    assert!((1.3..=2.0).contains(&power), "{power}");
    // The river file records each sample's gauge level and reports the range.
    let k = key_for_samples(&grid, &declaration, &profile, &[]);
    let file = RiverFile::read(&write(&s.file(k).unwrap()), &k).unwrap();
    assert_eq!(file.gauges(), s.gauges);
    assert_eq!(file.gauge_range(), [3.1, 3.7]);
    assert_eq!(file.upper.len(), 2);
    assert!(file.reaches.iter().all(|r| r.upper.len() == 2));
}

#[test]
fn a_spring_fed_stream_is_solved_at_its_declared_discharges_and_gauged_at_its_spring() {
    let (grid, declaration) = support::section::short_reach();
    let flows = vec![vec![0.5 * DISCHARGE], vec![DISCHARGE]];
    let s = solve_samples(&grid, &declaration, &Profile::default(), &flows).unwrap();
    let spring = grid.cell(declaration.springs[0].point).unwrap();
    for (k, x) in s.solutions.iter().enumerate() {
        println!(
            "spring at {:.3} m³/s: gauge {:.4} m, inflow {:.4} m³/s",
            flows[k][0], s.gauges[k], x.report.discharge
        );
        assert_eq!(s.gauges[k], x.level(spring));
        assert!((x.report.discharge / flows[k][0] - 1.0).abs() < 1e-9);
    }
    assert!(s.gauges[1] > s.gauges[0]);
    assert_eq!(s.gauge_range(), [s.gauges[0], s.gauges[1]]);
    // A spring-fed river takes no levels, and a lake-fed one no spring samples.
    let (lake_grid, lake_declaration) = lake(0.5, HIGH, [3.1, 3.7]);
    assert!(solve_samples(&lake_grid, &lake_declaration, &Profile::default(), &flows).is_err());
}
