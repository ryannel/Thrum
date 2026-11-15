use super::fixtures::{body, grid};
use thrum_water_model::{Bed, Body, BodyKind, Chart, End, Stage};

pub(crate) fn declarations(source: f64, tide: f64, depth: f64) -> (Bed, Vec<Body>) {
    let mut lake = body("source", BodyKind::Basin { seed: [10.0, 10.0] }, 5.0);
    lake.stage = Stage {
        min: -1.0,
        max: 1.0,
        value: source,
    };
    lake.gains = [0.0, 0.2, 0.7, 1.0];
    let mut sea = body("receiver", BodyKind::Sea, 0.0);
    sea.stage = Stage {
        min: -1.0,
        max: 1.0,
        value: tide,
    };
    sea.gains = [1.0; 4];
    let bed = grid([61, 21], |x, z| {
        if x >= 45.0 {
            return -4.0;
        }
        if (20.0..=44.0).contains(&x) && (9.0..=11.0).contains(&z) {
            return (4.0 - depth) * (44.0 - x) / 24.0 + (-1.0 - depth) * (x - 20.0) / 24.0;
        }
        if (5.0..=19.0).contains(&x) && (5.0..=15.0).contains(&z) {
            return 2.0;
        }
        10.0
    });
    let chart = Chart {
        levels: (0..1281)
            .map(|i| 4.0 - 6.0 * (f64::from(i % 61) - 20.0) / 24.0)
            .collect(),
        currents: vec![[1.2, 0.0]; 1281],
        footprint: (0..1281)
            .map(|i| (20..=44).contains(&(i % 61)) && (9..=11).contains(&(i / 61)))
            .collect(),
        ends: vec![
            End {
                body: 0,
                segment: [[20.0, 9.0], [20.0, 11.0]],
                length: 5.0,
                mouth: false,
            },
            End {
                body: 1,
                segment: [[44.0, 9.0], [44.0, 11.0]],
                length: 5.0,
                mouth: true,
            },
        ],
    };
    let mut flow = body("flow", BodyKind::Chart(chart), 0.0);
    flow.gains = [0.0, 0.1, 0.6, 1.0];
    (bed, vec![lake, sea, flow])
}
