//! Monotonic weather coverage across the supplied type axis.
pub mod support;
use support::{field, parameters};
use thrum_cloud_model::Column;

#[test]
fn coverage_is_monotone_at_both_ends_and_middle_of_type_axis() {
    let field = field(7, 0.3);
    for kind in [0.0, 0.5, 1.0] {
        let mut previous = vec![Column::default(); 400];
        for step in 0..=20 {
            let p = parameters(f64::from(step) / 20.0, kind);
            let mut count = 0;
            let old_count = previous.iter().filter(|c| c.optical_depth > 0.0).count();
            for i in 0..20 {
                for j in 0..20 {
                    let index = i * 20 + j;
                    let c = field.column(
                        [i as f64 * 1300.0 + 137.0, j as f64 * 1300.0 + 287.0],
                        18_000.0,
                        p,
                    );
                    if step == 0 {
                        assert_eq!(c, Column::default());
                    }
                    assert!(c.optical_depth >= previous[index].optical_depth);
                    assert!(c.coverage.is_finite() && c.top.is_finite());
                    count += usize::from(c.optical_depth > 0.0);
                    previous[index] = c;
                }
            }
            assert!(count >= old_count);
            if step == 20 && kind > 0.99 {
                assert_eq!(count, 400);
            }
        }
    }
}

#[test]
fn extreme_finite_coordinates_do_not_overflow_lattice_neighbours() {
    let field = field(7, 0.3);
    for x in [-1e300, -1e22, 1e22, 1e300] {
        for coverage in [0.0, 1.0] {
            let c = field.column([x, -x], 1e25, parameters(coverage, 1.0));
            assert!(c.optical_depth.is_finite() && c.top.is_finite());
            if coverage == 0.0 {
                assert_eq!(c, Column::default());
            }
        }
    }
}
