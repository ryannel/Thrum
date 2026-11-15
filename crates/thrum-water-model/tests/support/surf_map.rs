use std::f64::consts::{FRAC_1_SQRT_2, TAU};
use thrum_sea_model::{Component, LongWaves};
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Profile, Stage, SurfProfile};

pub(crate) fn fixture(case: usize) -> (Map, Field) {
    let bed = Bed::new(
        [-120.0, -80.0],
        2.0,
        [81, 81],
        (0..81)
            .flat_map(|_| {
                (0..81).map(|x| {
                    if case == 2 {
                        (-120.0 + f64::from(x) * 2.0) / 20.0
                    } else {
                        -1.8
                    }
                })
            })
            .collect(),
    )
    .expect("finite analytic bed");
    let body = Body {
        name: "still water".into(),
        kind: BodyKind::Sea,
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [1.0; 4],
    };
    let mut map = Map::prepare(
        bed,
        vec![body],
        Profile {
            margin_height: 1.5,
            swash_ratio: 0.7,
        },
    )
    .expect("valid still water");
    map.enable_surf(SurfProfile::default())
        .expect("valid surf profile");
    let components = match case {
        0 => vec![Component {
            wave_vector: [TAU / 25.0, 0.0],
            amplitude: 0.7,
            phase: 0.0,
        }],
        1 => vec![
            Component {
                wave_vector: [TAU / 25.0, 0.0],
                amplitude: 0.4667,
                phase: 0.0,
            },
            Component {
                wave_vector: [0.0, TAU / 25.0],
                amplitude: 0.2333,
                phase: 0.0,
            },
        ],
        _ => [
            (60.0, [1.0, 0.0], 0.42, 0.3),
            (45.0, [FRAC_1_SQRT_2; 2], 0.3, 2.1),
            (80.0, [FRAC_1_SQRT_2, -FRAC_1_SQRT_2], 0.25, 4.4),
        ]
        .map(|(length, dir, amplitude, phase)| Component {
            wave_vector: dir.map(|v| v * TAU / length),
            amplitude,
            phase,
        })
        .to_vec(),
    };
    let long = LongWaves::new(components, 0.5).expect("contractive original components");
    let field = Field {
        layers: [
            long,
            LongWaves::new(vec![], 0.0).expect("empty"),
            LongWaves::new(vec![], 0.0).expect("empty"),
            LongWaves::new(vec![], 0.0).expect("empty"),
        ],
    };
    (map, field)
}
