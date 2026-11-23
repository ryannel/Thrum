use std::f64::consts::{FRAC_1_SQRT_2, TAU};
use std::time::Duration;
use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
use thrum_water_model::{Bed, BodyKind, Field, Map, Stage};

pub(crate) const STEP: Duration = Duration::from_nanos(16_666_667);
pub(crate) const CHOPPINESS: f64 = 0.5;

pub(crate) fn beach() -> (Map, Spectrum, Field) {
    let heights = (0..33).flat_map(|_| (0..49).map(|x| (f64::from(x) * 2.0 - 80.0) / 20.0));
    let bed = Bed::new([-80.0, -32.0], 2.0, [49, 33], heights.collect()).expect("plane beach");
    let map = Map::prepare(
        bed,
        vec![thrum_water_model::Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.5,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        thrum_water_model::Profile {
            margin_height: 1.5,
            swash_ratio: 0.7,
        },
    )
    .expect("beach map");
    let long = [(0.4667, 1.0), (0.2333, -1.0)]
        .map(|(amplitude, side)| Component {
            wave_vector: [
                FRAC_1_SQRT_2 * TAU / 25.0,
                side * FRAC_1_SQRT_2 * TAU / 25.0,
            ],
            amplitude,
            phase: 0.0,
        })
        .to_vec();
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(if i == 0 { long.clone() } else { Vec::new() }, CHOPPINESS)
                .expect("field")
        }),
    };
    let spectrum = Spectrum {
        long,
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm")),
        tail_slope_variance: 0.0,
    };
    (map, spectrum, field)
}
