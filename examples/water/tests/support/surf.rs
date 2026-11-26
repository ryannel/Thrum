use thrum_sea_model::{LongWaves, Spectrum};
use thrum_water_model::{CutConfig, Field, Map, Sheet, cut};

pub(crate) use water_scene::lens_probe::CHOPPINESS;

pub(crate) fn beach() -> (Map, Spectrum, Field) {
    water_scene::lens_probe::beach(0.0)
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
