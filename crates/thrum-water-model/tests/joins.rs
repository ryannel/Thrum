//! Joined rows, stages, reach and the complete limited surface at shallow/deep ends.
use crate::support;
use support::fixtures::{close, field, grid, profile};
use thrum_water_model::{BodyKind, Map, MapSample, limit};

use support::joined::declarations;

fn joined(source: f64, tide: f64, depth: f64) -> Map {
    let (bed, bodies) = declarations(source, tide, depth);
    Map::prepare(bed, bodies, profile()).expect("held channel between contained bodies")
}

#[test]
fn preparation_refuses_a_low_source_and_a_stage_raised_blend_over_its_bank() {
    let (bed, mut bodies) = declarations(0.0, 0.0, 0.25);
    bodies[0].stage.min = -2.0;
    let err = Map::prepare(bed, bodies, profile()).err().unwrap();
    assert_eq!(err.name, "flow");
    assert!(err.reason.contains("sill"));
    let (old, bodies) = declarations(0.0, 0.0, 0.25);
    // x=21,z=8 borders the chart in its source mix. Its own level is 3.75 m,
    // but the source at its maximum stage raises it above a bank at 5 m.
    let bed = grid([61, 21], |x, z| {
        if (x - 21.0).abs() < 0.1 && (z - 8.0).abs() < 0.1 {
            5.0
        } else {
            old.height([x, z]).unwrap()
        }
    });
    let err = Map::prepare(bed, bodies, profile()).err().unwrap();
    assert_eq!(err.name, "flow");
    assert!(err.reason.contains("bank"));
}

#[test]
fn every_end_equals_its_receiver_after_the_limit_for_both_stage_extremes() {
    // The long component represents a swell; the other bands represent wind waves.
    let mut f = field(0.5);
    f.layers[1] = thrum_sea_model::LongWaves::new(
        vec![thrum_sea_model::Component {
            wave_vector: [1.0, 0.0],
            amplitude: 0.04,
            phase: 0.3,
        }],
        0.0,
    )
    .unwrap();
    for source in [-1.0, 1.0] {
        for tide in [-1.0, 1.0] {
            for depth in [0.25, 4.0] {
                let map = joined(source, tide, depth);
                for (x, other) in [(20.0, 0), (44.0, 1)] {
                    for z in [9.0, 9.3, 10.0, 10.7, 11.0] {
                        let p = [x, z];
                        let chart = map.row(2, p);
                        let receiver = map.row(other, p);
                        assert_eq!(chart, receiver);
                        let lim = limit(chart, depth, f.variances(), profile());
                        let expected = limit(receiver, depth, f.variances(), profile());
                        close(lim.swash_depth, expected.swash_depth);
                        assert_eq!(lim.gains, expected.gains);
                        for time in [0.0, 0.7, 2.3] {
                            let answer = |row, body| {
                                map.surface_on_row(
                                    &f,
                                    p,
                                    time,
                                    MapSample {
                                        row,
                                        body,
                                        bed: Some(receiver.level - depth),
                                        depth,
                                    },
                                )
                                .unwrap()
                            };
                            let a = answer(chart, 2);
                            let b = answer(receiver, other);
                            assert_eq!(a.sample, b.sample);
                            assert_eq!(a.wet, b.wet);
                            // Actual map queries on either side must converge to the same height.
                            let left = map.surface(&f, [x - 1e-7, z], time).unwrap();
                            let right = map.surface(&f, [x + 1e-7, z], time).unwrap();
                            assert!(
                                (left.sample.height - right.sample.height).abs() < 1e-5,
                                "end {x} depth {depth}: {left:?} {right:?}"
                            );
                        }
                    }
                }
                let own = map.row(2, [29.0, 10.0]);
                close(own.level, 1.75);
                assert_eq!(own.current, [1.2, 0.0]);
                assert_eq!(own.gains, [0.0, 0.1, 0.6, 1.0]);
            }
        }
    }
}

#[test]
fn backwater_uses_reach_and_the_quarter_metre_smooth_transition() {
    let map = joined(0.0, 0.0, 0.25);
    let reach = 0.175 * profile().margin_height;
    // Invert the chart's own level 4 - (x-20)/4 to choose exact elevations.
    for above in [-0.1, 0.0, 0.125, 0.25, 0.3] {
        let level = reach + above;
        let x = 20.0 + 4.0 * (4.0 - level);
        let row = map.row(2, [x, 10.0]);
        let u = (above / 0.25).clamp(0.0, 1.0);
        let own_weight = u * u * (3.0 - 2.0 * u);
        close(row.level, level * own_weight);
        close(row.current[0], 1.2 * own_weight);
        close(row.gains[0], 1.0 - own_weight);
    }
}

#[test]
fn overlapping_join_lengths_keep_both_end_rows_and_their_limits_continuous() {
    let (bed, mut bodies) = declarations(1.0, -1.0, 0.25);
    let BodyKind::Chart(chart) = &mut bodies[2].kind else {
        panic!("chart fixture");
    };
    for end in &mut chart.ends {
        end.length = 100.0;
    }
    let map = Map::prepare(bed, bodies, profile()).unwrap();
    for (x, other, direction) in [(20.0, 0, 1.0), (44.0, 1, -1.0)] {
        let endpoint = map.row(other, [x, 10.0]);
        assert_eq!(map.row(2, [x, 10.0]), endpoint);
        let near = map.row(2, [x + direction * 1e-4, 10.0]);
        assert!((near.level - endpoint.level).abs() < 1e-7);
        for (a, b) in near.gains.into_iter().zip(endpoint.gains) {
            assert!((a - b).abs() < 1e-7);
        }
    }
}

#[test]
fn preparation_refuses_receiving_reach_that_contradicts_the_source_end() {
    let (bed, mut bodies) = declarations(0.0, 0.0, 0.25);
    let BodyKind::Chart(chart) = &mut bodies[2].kind else {
        panic!("chart fixture");
    };
    // Backwater would demand the sea's row at the source edge, while that edge
    // simultaneously demands the raised basin's row. There is no consistent mix.
    chart.levels.fill(0.0);
    let error = Map::prepare(bed, bodies, profile()).err().unwrap();
    assert_eq!(error.name, "flow");
    assert!(error.reason.contains("conflicting"));
}
