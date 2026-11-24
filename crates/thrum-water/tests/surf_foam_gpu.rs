//! The rates handed to the Metal foam stocks on a small plane beach.
use crate::interaction_fixture::Fixture;
use std::f64::consts::TAU;
use thrum_sea_model::{Component, LongWaves};
use thrum_water_model::{
    Bed, Body, BodyKind, BreakerSection, Field, Map, Sheet, Stage, SurfProfile, shoaling,
};

const MARGIN: f64 = 0.8;
const SWASH: f64 = 0.7;
const SLOPE: f64 = 0.1;

fn beach(gains: [f64; 4], surf: SurfProfile) -> Map {
    let size = [37, 17];
    let origin = [-32.0, -8.0];
    let bed = Bed::new(
        origin,
        1.0,
        size,
        (0..size[1])
            .flat_map(|_| {
                (0..u32::try_from(size[0]).expect("small beach"))
                    .map(|x| SLOPE * (origin[0] + f64::from(x)))
            })
            .collect(),
    )
    .expect("plane bed");
    let mut map = Map::prepare(
        bed,
        vec![Body {
            name: "Plane beach".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains,
        }],
        thrum_water_model::Profile {
            margin_height: MARGIN,
            swash_ratio: SWASH,
        },
    )
    .expect("plane map");
    map.enable_surf(surf).expect("plane shore transform");
    map
}

fn wave(amplitude: f64, phase: f64) -> Component {
    Component {
        wave_vector: [TAU / 40.0, 0.0],
        amplitude,
        phase,
    }
}

#[derive(Debug, Default)]
struct Births {
    baseline: f64,
    film_and_air: f64,
    moved_film: f64,
    added_air: f64,
    maximum_index: f64,
}

#[expect(
    clippy::print_stderr,
    reason = "the guard records the submitted birth integrals"
)]
fn measure(map: &Map, wave: Component) -> Option<Births> {
    let mut fixture = Fixture::new_foam(map.clone(), Some(wave), [0.0; 2])?;
    fixture.draw(0);
    fixture.draw(1);
    let (witness, parcels) = fixture.foam_parcels();
    assert_eq!(witness.report.ticks, [0, 1]);
    let dt = Fixture::STEP.as_secs_f64();
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(if i == 0 { vec![wave] } else { vec![] }, 0.7).expect("regular wave")
        }),
    };
    let sheet = map.surf_sheet(&field, 0, dt).expect("surf sheet");
    let transform = map.shore_transform(0).expect("shoaling");
    let profile = map.surf_profile().expect("surf profile");
    let significant_height = 4.0 * wave.amplitude / 2.0_f64.sqrt();
    let swash = (SWASH * significant_height).min(MARGIN);
    let draw = 0.6 / (2.0 * 2.0_f64.sqrt() - 0.6) * swash;
    let spacing = f64::from(witness.spacing[0]);
    let area = spacing * spacing;
    let mut totals = Births::default();
    for z in 0..witness.side {
        for x in 0..witness.side {
            let point = [x, z].map(f64::from);
            let point = std::array::from_fn(|i| {
                f64::from(witness.origins[0][0][i]) + (point[i] + 0.5) * spacing
            });
            let [stocks, movement] = parcels[(z * witness.side + x) as usize];
            let air = -(-f64::from(stocks[0])).ln_1p() / dt;
            let film = f64::from(movement[3]);
            let Some(bed) = map.bed().height(point) else {
                assert_eq!(film, 0.0, "outside the beach at {point:?}");
                continue;
            };
            let depth = -bed;
            let forward = sheet.forward(point);
            // Exclude the transform's lateral boundary from the independent
            // plane-wave calculation. The direction across the beach is +x.
            if point[1].abs() > 6.0 {
                continue;
            }
            if depth + forward.height <= 0.0 {
                assert_eq!([film, air], [0.0, 0.0], "dry at {point:?}");
                continue;
            }
            let sample_depth = depth.max(profile.lag_floor);
            let fade = ((depth - draw) / (0.5 * swash - draw)).clamp(0.0, 1.0);
            let fade = fade * fade * (3.0 - 2.0 * fade);
            let exposure = transform.exposure(0, 0.0, point);
            let gain = exposure.sqrt()
                * (1.0 + (shoaling(wave.wave_vector[0], sample_depth) - 1.0) * fade);
            let amplitude = wave.amplitude * gain;
            let variance = 0.5 * amplitude * amplitude;
            let limit = (0.6 * (depth + swash).max(0.0) / (4.0 * variance.sqrt())).min(1.0);
            let baseline = 0.39 * variance * (1.0 - limit * limit);
            totals.maximum_index = totals
                .maximum_index
                .max(2.0 * amplitude / (0.78 * sample_depth));
            totals.baseline += area * 1.22 * baseline;
            totals.film_and_air += area * (film + air);
            totals.moved_film += area * (film - baseline).max(0.0);
            totals.added_air += area * (air - 0.22 * baseline).max(0.0);
        }
    }
    eprintln!("Metal surf birth integral: {totals:?}");
    assert!(totals.baseline > 0.001);
    assert!(
        (totals.film_and_air - totals.baseline).abs() < 0.005 * totals.baseline,
        "{totals:?}",
    );
    Some(totals)
}

#[test]
fn subcritical_wet_loss_is_distributed() {
    let map = beach([1.0, 0.0, 0.0, 0.0], SurfProfile::default());
    if let Some(totals) = measure(&map, wave(0.12, 0.25)) {
        assert!(totals.maximum_index < 1.0 && totals.moved_film < 0.001 * totals.baseline);
        assert!(totals.added_air < 0.005 * totals.baseline);
    }
}

#[test]
fn breaking_face_receives_a_share_of_the_wet_loss() {
    let map = beach([1.0, 0.0, 0.0, 0.0], SurfProfile::default());
    if let Some(totals) = measure(&map, wave(0.85, 0.5)) {
        assert!(totals.maximum_index > 1.0 && totals.moved_film > 0.005 * totals.baseline);
    }
}

#[test]
fn resolved_lip_receives_submerged_air() {
    let section = BreakerSection::first();
    let (indices, rows) = section
        .rows()
        .map(|(index, row)| {
            let mut row = row.to_vec();
            for sample in &mut row {
                sample.throw *= 4.0;
            }
            (index, row)
        })
        .unzip();
    let profile = SurfProfile {
        section: BreakerSection::new(indices, rows, section.bore_index())
            .expect("test lip resolves on the foam grid"),
        ..SurfProfile::default()
    };
    let map = beach([1.0, 0.0, 0.0, 0.0], profile);
    // Put the falling face in the narrow span that throws. A deeper crest
    // would have no breaking face, and a shallower one would be a bore.
    let point = [-19.125, 0.125];
    let lag = map
        .shore_transform(0)
        .expect("shoaling")
        .sample(0, 40.0, 0.0, point)
        .0;
    let phase = 0.56 - TAU / 40.0 * (point[0] + lag);
    if let Some(totals) = measure(&map, wave(0.85, phase)) {
        assert!(totals.added_air > 0.0002 * totals.baseline, "{totals:?}");
    }
}

#[test]
fn calm_and_omitted_waves_make_no_shore_birth() {
    for (gain, component) in [(1.0, None), (0.0, Some(wave(0.85, 0.5)))] {
        let map = beach([gain, 0.0, 0.0, 0.0], SurfProfile::default());
        let Some(mut fixture) = Fixture::new_foam(map, component, [0.0; 2]) else {
            return;
        };
        fixture.draw(0);
        fixture.draw(1);
        let (_, parcels) = fixture.foam_parcels();
        assert!(
            parcels
                .iter()
                .all(|[stock, movement]| stock.iter().all(|&value| value == 0.0)
                    && movement[3] == 0.0)
        );
    }
}
