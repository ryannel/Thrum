use super::{RowBounds, bound_joins, segment_distances};

fn row(level: f64, gain: f64) -> RowBounds {
    RowBounds {
        level: [level; 2],
        depth: level + 3.0,
        gain,
    }
}

#[test]
fn overlapping_partial_joins_bound_the_normalised_row() {
    let own = row(0.0, 0.2);
    let source = row(2.0, 1.0);
    let mouth = row(-2.0, 0.6);
    let bounds = bound_joins(own, [(source, [0.25, 0.5]), (mouth, [0.5, 0.75])]);

    // Sample the independent weights with the direct normalised expression
    // the incremental mix represents, including all four corners.
    for a in 0..=16 {
        for b in 0..=16 {
            let a = (0.25 + f64::from(a) / 64.0) / (0.75 - f64::from(a) / 64.0);
            let b = (0.5 + f64::from(b) / 64.0) / (0.5 - f64::from(b) / 64.0);
            let total = 1.0 + a + b;
            let level = (2.0 * a - 2.0 * b) / total;
            let gain = (0.2 + a + 0.6 * b) / total;
            assert!(level >= bounds.level[0] - 1e-12 && level <= bounds.level[1] + 1e-12);
            assert!(level + 3.0 >= bounds.depth - 1e-12);
            assert!(gain >= bounds.gain - 1e-12);
        }
    }
    assert!((bounds.gain - 17.0 / 35.0).abs() < 1e-12);
    assert!((bounds.depth - 23.0 / 13.0).abs() < 1e-12);
}

#[test]
fn a_full_join_keeps_only_earlier_possible_full_outcomes() {
    let own = row(0.0, 0.2);
    let source = row(2.0, 0.5);
    let mouth = row(-1.0, 0.7);
    let bounds = bound_joins(own, [(source, [0.5, 1.0]), (mouth, [1.0; 2])]);
    assert_eq!(bounds.level, [-1.0, 2.0]);
    assert_eq!(bounds.gain, 0.5);
    let bounds = bound_joins(own, [(source, [1.0; 2]), (mouth, [1.0; 2])]);
    assert_eq!(bounds.level, [2.0; 2]);
    assert_eq!(bounds.gain, 0.5);
}

#[test]
fn a_tile_uses_the_join_distance_at_its_nearest_and_farthest_points() {
    let low = [0.0, 0.0];
    let high = [2.0, 2.0];
    let [near, far] = segment_distances([[-1.0, 1.0], [3.0, 1.0]], low, high);
    assert_eq!([near, far], [0.0, 1.0]);
    let [near, far] = segment_distances([[3.0, -2.0], [3.0, 4.0]], low, high);
    assert_eq!([near, far], [1.0, 3.0]);
    let [near, far] = segment_distances([[3.0, 3.0], [4.0, 3.0]], low, high);
    assert_eq!([near, far], [2.0_f64.sqrt(), 18.0_f64.sqrt()]);
}
