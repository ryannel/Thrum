use std::f64::consts::{FRAC_1_SQRT_2, TAU};
use std::time::Duration;
use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
use thrum_water_model::{Bed, BodyKind, CutConfig, Field, Map, Sheet, Stage, cut};

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

#[expect(
    clippy::print_stderr,
    reason = "record both failing controls in the target receipt"
)]
pub(crate) fn check_lip(
    map: &Map,
    plain: &Map,
    field: &Field,
    column: [f64; 2],
    time: f64,
    height: f64,
) {
    let sheet = map.surf_sheet(field, 0, time).expect("breaker sheet");
    let lip = cut(&sheet, column, CutConfig::default()).expect("breaker cut");
    assert!(
        lip.hits >= 3 && lip.converged,
        "three-surface witness: {lip:?}"
    );
    assert!((lip.height - height).abs() < 0.001);
    for &train in field.layers[0].components() {
        let mut alone = Field {
            layers: field.layers.clone(),
        };
        alone.layers[0] = LongWaves::new(vec![train], CHOPPINESS).expect("one train");
        let alone = map.surf_sheet(&alone, 0, time).expect("one-train sheet");
        let column = cut(&alone, column, CutConfig::default()).expect("one-train cut");
        assert_eq!(column.hits, 1, "the overhang needs both trains: {column:?}");
    }
    let mut rest = column;
    for _ in 0..128 {
        let p = sheet.forward(rest);
        rest = std::array::from_fn(|a| column[a] - (p.world[a] - rest[a]));
    }
    let trough = sheet.forward(rest);
    let residual = (trough.world[0] - column[0]).hypot(trough.world[1] - column[1]);
    assert!(
        residual < 0.001 && height - trough.height > 0.10,
        "plain iteration must converge on the trough: {trough:?}"
    );
    let no_transform = plain
        .surface(field, column, time)
        .expect("untransformed wave");
    assert!((height - no_transform.sample.height).abs() > 0.10);
    eprintln!(
        "fold: {} crossings, lip {:.6} m, trough {:.6} m, without transform {:.6} m",
        lip.hits, height, trough.height, no_transform.sample.height
    );
}
