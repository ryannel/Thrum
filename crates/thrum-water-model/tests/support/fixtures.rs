use thrum_sea_model::{Component, LongWaves};
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Profile, Stage};

pub(crate) fn profile() -> Profile {
    Profile {
        margin_height: 1.5,
        swash_ratio: 0.7,
    }
}
pub(crate) fn body(name: &str, kind: BodyKind, level: f64) -> Body {
    Body {
        name: name.into(),
        kind,
        level,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    }
}
pub(crate) fn grid(size: [usize; 2], f: impl Fn(f64, f64) -> f64) -> Bed {
    Bed::new(
        [0.0; 2],
        1.0,
        size,
        (0..size[1])
            .flat_map(|z| (0..size[0]).map(move |x| (x, z)))
            .map(|(x, z)| f(x as f64, z as f64))
            .collect(),
    )
    .expect("finite analytic grid")
}
pub(crate) fn basin() -> Map {
    Map::prepare(
        grid([41, 21], |x, z| {
            ((x - 10.0).hypot(z - 10.0)).min((x - 30.0).hypot(z - 10.0))
        }),
        vec![body("hollow", BodyKind::Basin { seed: [10.0, 10.0] }, 3.0)],
        profile(),
    )
    .expect("closed disconnected hollows")
}
pub(crate) fn field(amplitude: f64) -> Field {
    Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(
                if i == 0 {
                    vec![Component {
                        wave_vector: [std::f64::consts::TAU / 60.0, 0.0],
                        amplitude,
                        phase: 0.0,
                    }]
                } else {
                    vec![]
                },
                0.0,
            )
            .expect("contractive regular wave")
        }),
    }
}
pub(crate) fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
