//! Provider fault injection: values and support disagreements remain observable.
use crate::support;
use support::fixtures::{close, field};
use thrum_water_model::{MapSample, compare};

#[test]
fn model_conforms_and_centimetre_shift_exposes_crossed_shoreline() {
    let map = support::fixtures::basin();
    let field = field(0.0);
    let reference = |p, t| map.surface(&field, p, t as f64 / 60.0);
    let shifted = |p, t| {
        let old = map.sample(p).unwrap();
        let mut row = old.row;
        row.level += 0.01;
        map.surface_on_row(
            &field,
            p,
            t as f64 / 60.0,
            MapSample {
                row,
                depth: old.depth + 0.01,
                ..old
            },
        )
    };
    let points = [[10.0, 10.0], [12.999, 10.0], [13.005, 10.0], [13.02, 10.0]];
    let same = compare(&reference, &reference, &points, &[0, 1, 50]).unwrap();
    close(same.surface.height, 0.0);
    assert_eq!(same.wet_mismatches.len(), 0);
    assert_eq!(same.body_mismatches.len(), 0);
    let bad = compare(&reference, &shifted, &points, &[0, 1, 50]).unwrap();
    close(bad.surface.height, 0.01);
    close(bad.depth, 0.01);
    assert_eq!(bad.wet_mismatches.len(), 3);
    assert!(
        bad.wet_mismatches
            .iter()
            .all(|(p, _)| (p[0] - 13.005).abs() < 1e-12 && (p[1] - 10.0).abs() < 1e-12)
    );
    let wrong = |p, t| {
        let mut s = reference(p, t)?;
        s.body = Some(99);
        s.sample.slope = [0.2, -0.3];
        s.sample.velocity = [1.0, 2.0, 3.0];
        s.current = [0.1, -0.2];
        Ok(s)
    };
    let bad = compare(&reference, &wrong, &points, &[0]).unwrap();
    assert_eq!(bad.body_mismatches.len(), 4);
    assert_eq!(bad.surface.slope, [0.2, 0.3]);
    assert_eq!(bad.surface.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(bad.current, [0.1, 0.2]);
}
