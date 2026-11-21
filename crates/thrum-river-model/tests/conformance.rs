//! M3.6: `WATERS`' conformance harness, `thrum_water_model::compare`, run over a river with three
//! samples, a fall and both flow bands. The lake's outlet is solved at three levels of its lake;
//! the channel leaves over a cliff into the tarn in each. Its map holds the flow bands, under a
//! wind. The harness reports nothing for the model against itself, at gauges on and between the
//! samples. A provider that reads only the middle sample, the same world but with the river's
//! charts at that sample whatever the gauge, is reported between the samples by the points and
//! the difference, which this test works out for itself and matches; on the middle sample's own
//! gauge it agrees.
#![expect(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::lake::{CENTRE, FOOT, HIGH, RANGE, basin, bed_of, chart_body, sampled};
use thrum_river_model::Grid;
use thrum_sea_model::{Component, LongWaves};
use thrum_water_model::{
    Field, FlowBands, FlowBandsProfile, Gauge, Map, Profile as WaterProfile, RiverFile, Stage,
    Surface, WaterError, compare,
};

const WATER: WaterProfile = WaterProfile {
    margin_height: 0.3,
    swash_ratio: 0.7,
};

fn wind() -> Field {
    Field {
        layers: std::array::from_fn(|i| {
            let k = std::f64::consts::TAU / [40.0, 6.0, 1.5, 0.4][i];
            LongWaves::new(
                vec![Component {
                    wave_vector: [k * 0.6, -k * 0.8],
                    amplitude: 0.003 / [1.0, 3.0, 9.0, 27.0][i],
                    phase: 0.7 * i as f64,
                }],
                0.0,
            )
            .unwrap()
        }),
    }
}

/// The lake (body 0, its stage over the range, at `gauge`), the tarn (body 1) and the file's
/// reaches as one river's charts (bodies 2 and on), with `gauge_of` the river's gauge, and both
/// flow bands.
fn world(grid: &Grid, file: &RiverFile, gauge: f64, river_gauge: Option<Stage>) -> Map {
    let bed = bed_of(grid);
    let lookup = |name: &str| match name {
        "lake" => Some(0),
        "tarn" => Some(1),
        _ => None,
    };
    let mut bodies = vec![
        basin(
            "lake",
            [3.5, CENTRE],
            0.0,
            Stage {
                min: RANGE[0],
                max: RANGE[1],
                value: gauge,
            },
        ),
        basin(
            "tarn",
            [30.0, CENTRE],
            -1.0,
            Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
        ),
    ];
    for r in &file.reaches {
        bodies.push(chart_body(&r.name, r.chart(&bed, lookup).unwrap()));
    }
    let charts: Vec<usize> = (2..bodies.len()).collect();
    let mut map = Map::prepare(bed.clone(), bodies, WATER).unwrap();
    let g = river_gauge.map_or(Gauge::Body(0), Gauge::Stage);
    map.add_river(file.river(&bed, "outlet", &charts, g).unwrap())
        .unwrap();
    map.set_flow_bands(Some(FlowBands::new(FlowBandsProfile::default()).unwrap()));
    map
}

/// The file with only its middle sample: each reach's rows and the falls are that sample's.
fn middle_only(file: &RiverFile) -> RiverFile {
    let mut m = file.clone();
    m.gauge_level = file.upper[0].gauge_level;
    m.falls.clone_from(&file.upper[0].falls);
    m.upper.clear();
    for r in &mut m.reaches {
        let s = r.upper.remove(0);
        r.levels = s.levels;
        r.currents = s.currents;
        r.loss = s.loss;
        r.gains = s.gains;
        r.upper.clear();
    }
    m
}

fn answer(map: &Map, field: &Field) -> impl Fn([f64; 2], u64) -> Result<Surface, WaterError> {
    move |p, t| map.surface(field, p, t as f64 / 60.0)
}

#[test]
fn the_harness_reports_nothing_for_the_model_and_the_middle_sample_alone_by_points_and_difference()
{
    let (grid, samples, file) = sampled(HIGH);
    let gauges = samples.gauges.clone();
    assert_eq!(gauges.len(), 3);
    assert!(file.upper.iter().all(|u| !u.falls.is_empty()) && !file.falls.is_empty());
    let field = wind();
    let middle = middle_only(&file);
    let g1 = gauges[1];
    // Points over the channel, off the cell centres, from the lake to the tarn below the cliff.
    let mut points = Vec::new();
    let mut x = 6.3;
    while x < FOOT + 3.0 {
        for dz in [-1.6, -0.9, -0.2, 0.45, 1.15, 1.7] {
            points.push([x, CENTRE + dz]);
        }
        x += 0.55;
    }
    let ticks = [0, 17, 600, 3601];
    for g in [gauges[0], 3.25, g1, 3.55, gauges[2]] {
        let model = world(&grid, &file, g, None);
        let same = compare(
            &answer(&model, &field),
            &answer(&model, &field),
            &points,
            &ticks,
        )
        .unwrap();
        assert_eq!(same.surface_differences.len(), 0);
        assert!(same.body_mismatches.is_empty() && same.wet_mismatches.is_empty());
        assert_eq!(same.surface.height, 0.0);
        assert_eq!((same.depth, same.current), (0.0, [0.0; 2]));
        // The provider that reads only the middle sample, whatever the gauge.
        let provider = world(
            &grid,
            &middle,
            g,
            Some(Stage {
                min: g1,
                max: g1,
                value: g1,
            }),
        );
        let report = compare(
            &answer(&model, &field),
            &answer(&provider, &field),
            &points,
            &ticks,
        )
        .unwrap();
        // This test's own account of where they differ, and by how much.
        let (mut differ, mut height, mut wet) = (Vec::new(), 0.0f64, Vec::new());
        for &t in &ticks {
            for &p in &points {
                let a = answer(&model, &field)(p, t).unwrap();
                let b = answer(&provider, &field)(p, t).unwrap();
                if a.sample != b.sample {
                    differ.push((p, t));
                }
                if a.wet != b.wet {
                    wet.push((p, t));
                }
                height = height.max((a.sample.height - b.sample.height).abs());
            }
        }
        assert_eq!(report.surface_differences, differ);
        assert_eq!(report.wet_mismatches, wet);
        assert_eq!(report.surface.height, height);
        println!(
            "gauge {g:.2} m: the model against itself reports nothing; the middle sample alone \
             differs at {} of {} answers, by up to {:.1} mm in height, {:.1} mm in depth and \
             {:.3} m/s in current, wet in one and not the other at {}",
            report.surface_differences.len(),
            points.len() * ticks.len(),
            report.surface.height * 1e3,
            report.depth * 1e3,
            report.current[0].max(report.current[1]),
            report.wet_mismatches.len()
        );
        if g == g1 {
            assert!(report.surface_differences.is_empty() && report.wet_mismatches.is_empty());
        } else {
            assert!(report.surface.height > 0.01);
            assert_ne!(report.surface_differences.len(), 0);
        }
    }
}
