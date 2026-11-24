//! Whitewater in the ordinary example, using submitted Metal stocks and depth.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
use support::Fixture;
use thrum_water::{
    foam::Witness,
    query::{BodyQuery, QueryPoint},
};
use water_scene::{
    floating::{WakeDemo, WakeView, WhitewaterDemo, build_wake},
    island::{Wind, build_foam},
};

static DEVICE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn current<'a>(
    witness: &'a Witness,
    stocks: &'a [[f32; 4]],
    level: usize,
) -> impl Iterator<Item = ([f64; 2], [f32; 4])> + use<'a> {
    let plane = (witness.side * witness.side) as usize;
    let start = (witness.banks[1] * 2 + level) * plane;
    let origin = witness.origins[1][level].map(f64::from);
    let scale = f64::from(witness.spacing[level]);
    (0..witness.side * witness.side)
        .zip(stocks[start..start + plane].iter().copied())
        .map(move |(index, values)| {
            let cell = [index % witness.side, index / witness.side];
            (
                std::array::from_fn(|axis| origin[axis] + (f64::from(cell[axis]) + 0.5) * scale),
                values,
            )
        })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "foam cell centres are exact in the query's f32 lattice"
)]
fn shore_queries(
    witness: &Witness,
    map: &thrum_water_model::Map,
    tick: u64,
) -> Vec<(usize, QueryPoint)> {
    let origin = witness.origins[1][0].map(f64::from);
    let spacing = f64::from(witness.spacing[0]);
    let side = witness.side;
    (0..side * side)
        .filter(|&index| (u64::from(index) + tick).is_multiple_of(7))
        .filter_map(|index| {
            let [x, z] = [index % side, index / side];
            let point = [
                origin[0] + (f64::from(x) + 0.5) * spacing,
                origin[1] + (f64::from(z) + 0.5) * spacing,
            ];
            map.sample(point)
                .filter(|row| row.body == 0 && row.depth.abs() < 0.36)
                .map(|_| {
                    (
                        index as usize,
                        QueryPoint {
                            position: point.map(|metres| metres as f32),
                            radius: 0.0,
                            use_tick: tick - 3,
                        },
                    )
                })
        })
        .take(512)
        .collect()
}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: `./dev long-tests` runs it on main, from .config/long-tests.txt"]
fn ten_minutes_moored_on_the_sea_and_the_lake() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    support::island::with_cooked_island("ten_minutes_moored_on_the_sea_and_the_lake", || {
        let Some(gpu) = thrum_test_support::gpu("whitewater calm", Some(wgpu::Backend::Metal)) else {
            return;
        };
        for lake in [false, true] {
            let scene = build_wake(
                Wind::Calm,
                lake,
                0.0,
                0.0,
                WakeDemo {
                    whitewater: WhitewaterDemo::Procedural,
                    view: WakeView::Overhead,
                    body_capacity: Some(1),
                    ..WakeDemo::default()
                },
            )
            .expect("moored foam scene");
            let bodies = scene.observer();
            let tap = scene.foam.expect("foam tap");
            let mut fixture = Fixture::from_game(&gpu, [96, 64], scene.game, scene.report);
            for tick in (0_u32..=36_000).step_by(8) {
                fixture.frame_without_timings(f64::from(tick) * 0.016_666_667);
                if tick < 40 || tick % 600 == 0 {
                    let submitted = tap.latest().expect("submitted calm scene");
                    assert_eq!(submitted.report.ticks[1], u64::from(tick));
                    let stocks = support::foam_stocks(&fixture, &submitted);
                    let peak = stocks.iter().fold([0.0_f32; 4], |peak, cell| {
                        std::array::from_fn(|stock| peak[stock].max(cell[stock]))
                    });
                    assert!(
                        peak.iter().all(|value| *value == 0.0),
                        "calm {lake}, tick {tick}: stocks {peak:?}, craft speed {}, drag power {}, float speed {}",
                        bodies()[0].velocity.with_y(0.0).length(),
                        bodies()[0].drag_power,
                        bodies()
                            .iter()
                            .skip(1)
                            .map(|body| body.velocity.with_y(0.0).length())
                            .fold(0.0, f64::max)
                    );
                }
            }
            eprintln!(
                "Metal moored scene lake={lake}: all four submitted stocks zero through 600 s"
            );
        }
    });
}
}

thrum_test_support::macos_gpu_test! {
fn wind_sweep_raises_the_mean_foam_cover() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    support::island::with_cooked_island("wind_sweep_raises_the_mean_foam_cover", || {
        let Some(gpu) = thrum_test_support::gpu("whitewater wind", Some(wgpu::Backend::Metal)) else {
            return;
        };
        let mut covers = Vec::new();
        for wind in [Wind::Trade, Wind::Strong, Wind::Gale] {
            let scene = build_wake(
                wind,
                false,
                0.0,
                0.0,
                WakeDemo {
                    whitewater: WhitewaterDemo::Procedural,
                    disabled: true,
                    view: WakeView::Waterline,
                    ..WakeDemo::default()
                },
            )
            .expect("whitecap wind scene");
            let tap = scene.foam.expect("foam tap");
            let mut fixture = Fixture::from_game(&gpu, [96, 64], scene.game, scene.report);
            fixture.parameters(r#"{"craft.present":0}"#);
            for tick in (0..=480).step_by(6) {
                fixture.frame_without_timings(f64::from(tick) * 0.016_666_667);
            }
            let witness = tap.latest().expect("submitted whitecaps");
            let stocks = support::foam_stocks(&fixture, &witness);
            let cover = current(&witness, &stocks, 0)
                .map(|(_, c)| f64::from(c[1] + c[2]))
                .sum::<f64>()
                / f64::from(witness.side.pow(2));
            eprintln!("Metal {wind:?} mean fine-level film cover at eight seconds: {cover:.6}");
            covers.push(cover);
        }
        assert!(
            covers[0] > 0.0 && covers.windows(2).all(|c| c[0] < c[1]),
            "{covers:?}"
        );
    });
}
}

#[derive(Debug)]
struct Trail {
    cover: f64,
    peak: f64,
    width: f64,
}

fn trail(witness: &Witness, stocks: &[[f32; 4]]) -> Trail {
    let section = current(witness, stocks, 0).filter(|(p, _)| (125.0..=138.0).contains(&p[0]));
    let (mut mass, mut peak, mut offset, mut radius) = (0.0, 0.0_f64, 0.0, 0.0);
    for ([_, z], c) in section {
        let cover = f64::from(c[1] + c[2]);
        mass += cover;
        peak = peak.max(cover);
        offset += cover * (z - 32.0);
        radius += cover * (z - 32.0).powi(2);
    }
    Trail {
        cover: mass,
        peak,
        width: (radius / mass - (offset / mass).powi(2)).sqrt(),
    }
}

thrum_test_support::macos_gpu_test! {
fn craft_trail_outlives_removed_source_and_spreads() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(gpu) = thrum_test_support::gpu("removed craft's material", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let scene = build_wake(
        Wind::Calm,
        false,
        6.0,
        std::f32::consts::PI,
        WakeDemo {
            whitewater: WhitewaterDemo::Procedural,
            disabled: true,
            view: WakeView::Overhead,
            ..WakeDemo::default()
        },
    )
    .expect("foam course");
    let tap = scene.foam.expect("foam tap");
    let tick_duration = std::time::Duration::from_millis(50);
    let game = scene
        .game
        .with_tick_duration(tick_duration)
        .expect("trail test clock");
    let mut fixture = Fixture::from_game(&gpu, [32, 24], game, scene.report);
    // Run the craft for six seconds, then age its film for 60.1 seconds.
    // The longer test tick reduces submissions within the foam catch-up bound.
    for tick in (0..=120).step_by(4) {
        fixture.frame_without_timings((tick_duration * tick).as_secs_f64());
    }
    fixture.parameters(r#"{"craft.present":0}"#);
    let mut readings: Vec<Trail> = Vec::new();
    for tick in (122..=1322).step_by(8) {
        fixture.frame_without_timings((tick_duration * tick).as_secs_f64());
        if [122, 322, 522, 722, 1322].contains(&tick) {
            let witness = tap.latest().expect("submitted trail");
            assert_eq!(witness.report.ticks[1], u64::from(tick), "submitted trail tick");
            assert_eq!(witness.report.dropped_ticks, 0, "trail ageing at tick {tick}");
            let stocks = support::foam_stocks(&fixture, &witness);
            readings.push(trail(&witness, &stocks));
        }
    }
    eprintln!(
        "Metal removed craft at 6 m/s, trail at 0.1, 10.1, 20.1, 30.1 and 60.1 seconds: {readings:?}"
    );
    assert!(readings[0].peak > 0.14 && readings[2].cover > 0.1);
    for pair in readings.windows(2) {
        assert!(
            pair[1].width > pair[0].width && pair[1].peak < pair[0].peak,
            "{pair:?}"
        );
    }
    assert!(readings[4].peak < 0.025);
}
}

thrum_test_support::macos_gpu_test! {
fn a_calm_beach_births_nothing() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    support::island::with_cooked_island("a_calm_beach_births_nothing", || {
        let Some(gpu) = thrum_test_support::gpu("calm shore power", Some(wgpu::Backend::Metal)) else {
            return;
        };
        assert_eq!(shore_power(&gpu, Wind::Calm, 0), [0.0; 2]);
    });
}
}

thrum_test_support::macos_gpu_test! {
fn beach_births_rise_with_the_wind() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    support::island::with_cooked_island("beach_births_rise_with_the_wind", || {
        let Some(gpu) = thrum_test_support::gpu("shore power by wind", Some(wgpu::Backend::Metal)) else {
            return;
        };
        let means = [Wind::Trade, Wind::Gale].map(|wind| shore_power(&gpu, wind, 0));
        assert!(
            means[1][0] > means[0][0] && means[1][1] > means[0][1],
            "{means:?}"
        );
    });
}
}

thrum_test_support::macos_gpu_test! {
fn lake_beach_births_follow_its_share_of_lost_power() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(gpu) = thrum_test_support::gpu("lake shore power", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let means = [0, 1].map(|body| compact_shore_power(&gpu, body));
    let lake_foam_share = means[1][0] / means[0][0];
    let lake_power_share = means[1][1] / means[0][1];
    assert!(
        lake_foam_share > 0.0 && lake_power_share > 0.0 && lake_foam_share < 0.32,
        "{means:?}"
    );
    // Exposure of wet cells and the bounded cover cap affect the accumulated
    // ratio. The lake must stay in the same order of magnitude as its power share.
    assert!(
        (0.1..8.0).contains(&(lake_foam_share / lake_power_share)),
        "{means:?}"
    );
    eprintln!(
        "Lake vs beach share of film {lake_foam_share:.5}, of depth-limited power {lake_power_share:.5}"
    );
}
}

thrum_test_support::macos_gpu_test! {
fn the_film_leaves_ground_depth() {
    let _serial = DEVICE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    support::island::with_cooked_island("the_film_leaves_ground_depth", || {
        let Some(gpu) = thrum_test_support::gpu("shore film", Some(wgpu::Backend::Metal)) else {
            return;
        };
        film_depth_and_query(&gpu);
    });
}
}

#[cfg(target_os = "macos")]
fn compact_shore_power(gpu: &thrum_test_support::Gpu, body: usize) -> [f64; 2] {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_water_model::{Bed, Body, BodyKind, Map, Profile, Stage};

    const LAKE_CENTRE: f64 = 49.0;
    const LAKE_LEVEL: f64 = 8.0;
    let origin = [-32.0, -32.0];
    let size = [117_u32, 65];
    let heights = (0..size[1])
        .flat_map(|z| {
            (0..size[0]).map(move |x| {
                let x = origin[0] + f64::from(x);
                let z = origin[1] + f64::from(z);
                let beach = (0.14 * x).clamp(-8.0, LAKE_LEVEL + 3.0);
                if x < 20.0 {
                    beach
                } else {
                    LAKE_LEVEL + 0.22 * ((x - LAKE_CENTRE).hypot(z) - 23.0)
                }
            })
        })
        .collect();
    let bed = Bed::new(origin, 1.0, size.map(|n| n as usize), heights).expect("shore bed");
    let wind = Wind::Trade;
    let forcing = wind.forcing();
    let stage = Stage {
        min: 0.0,
        max: 0.0,
        value: 0.0,
    };
    let mut map = Map::prepare(
        bed,
        vec![
            Body {
                name: "Beach".into(),
                kind: BodyKind::Sea,
                level: 0.0,
                stage,
                fetch: forcing.fetch,
                gains: [1.0; 4],
            },
            Body {
                name: "Lake".into(),
                kind: BodyKind::Basin {
                    seed: [LAKE_CENTRE, 0.0],
                },
                level: LAKE_LEVEL,
                stage,
                fetch: 1_000.0,
                gains: [0.0; 4],
            },
        ],
        Profile {
            margin_height: 3.5,
            swash_ratio: 0.7,
        },
    )
    .expect("beach and enclosed basin");
    map.set_forcing(&forcing).expect("sheltered gains");
    let map = Arc::new(Mutex::new(map));
    let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("wind forcing");
    let mut profile = water_scene::profile(&spectrum);
    profile.query = Some(thrum_water::query::QueryLimits {
        points: 512,
        max_age: 8,
    });
    let mut water =
        thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile).expect("mapped shoreline");
    water.enable_foam([8.0, 0.0]).expect("shore film");
    let probes = water_scene::island::FoamProbes {
        foam: water.tap_foam(),
        query: water.query_mailbox().expect("shore queries"),
    };
    let report = water.report();
    let camera = if body == 0 {
        Camera::look_at([1.0, 2.0, 3.0], [-6.0, -0.5, 0.0], 65.0)
    } else {
        Camera::look_at([70.0, 10.0, 3.0], [63.0, 7.5, 0.0], 65.0)
    };
    let game = SkyGame::new(Controls::new(), camera, |_, sky| sky.temporal_inputs = true)
        .with_ground(None)
        .with_tick_duration(Duration::from_nanos(16_666_667))
        .expect("supported tick")
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    let mut fixture = Fixture::from_game(gpu, [64, 48], game, report);
    measure_shore_power(&mut fixture, &map, &probes, wind, body, 120..=360)
}

#[cfg(target_os = "macos")]
fn shore_power(gpu: &thrum_test_support::Gpu, wind: Wind, body: usize) -> [f64; 2] {
    let (game, report, map, probes) = build_foam(wind, true, true).expect("island whitewater");
    let mut fixture = Fixture::from_game(gpu, [128, 96], game, report);
    fixture.parameters(if body == 0 {
        r#"{"camera.island":0}"#
    } else {
        r#"{"camera.island":1}"#
    });
    measure_shore_power(
        &mut fixture,
        &map,
        &probes.expect("shore probes"),
        wind,
        body,
        240..=720,
    )
}

#[expect(
    clippy::print_stderr,
    reason = "keep shoreline power and stock measurements in target evidence"
)]
#[cfg(target_os = "macos")]
fn measure_shore_power(
    fixture: &mut Fixture,
    map: &water_scene::SharedMap,
    probes: &water_scene::island::FoamProbes,
    wind: Wind,
    body: usize,
    samples: std::ops::RangeInclusive<u32>,
) -> [f64; 2] {
    let tap = &probes.foam;
    let spectrum = thrum_sea_model::Spectrum::build(&wind.forcing()).expect("wind forcing");
    let significant_variance = [
        spectrum
            .long
            .iter()
            .map(|wave| 0.5 * wave.amplitude.powi(2))
            .sum(),
        spectrum.cascades[0].variance(),
        spectrum.cascades[1].variance(),
        spectrum.cascades[2].variance(),
    ];
    let mut deposited = 0.0;
    let mut expected_loss = 0.0;
    let mut reported_loss = 0.0;
    let mut wet_loss = 0.0;
    let mut shoreline_cells = 0_u32;
    let mut swash_cells = 0_u32;
    let mut newly_dry_stocks = 0;
    for tick in (0..=*samples.end()).step_by(6) {
        let sampled = samples.contains(&tick) && tick % 30 == 0;
        let columns = if sampled && body == 0 && !matches!(wind, Wind::Calm) {
            let previous = tap.latest().expect("previous foam endpoint");
            let columns =
                shore_queries(&previous, &map.lock().expect("shore map"), u64::from(tick));
            assert!(!columns.is_empty());
            probes.query.post(
                u64::from(tick - 4),
                vec![BodyQuery {
                    id: 33,
                    generation: 1,
                    points: columns.iter().map(|(_, point)| *point).collect(),
                }],
            );
            columns
        } else {
            Vec::new()
        };
        fixture.frame_without_timings(f64::from(tick) * 0.016_666_667);
        if sampled {
            let witness = tap.latest().expect("submitted shore");
            let stocks = support::foam_stocks(fixture, &witness);
            let parcels: Vec<[[f32; 4]; 2]> = support::foam_buffer(fixture, &witness.parcels);
            if !columns.is_empty() {
                let answers = probes.query.drain(
                    u64::from(tick - 3),
                    std::time::Duration::from_nanos(16_666_667),
                );
                assert_eq!(answers.len(), 1, "submitted midpoint query");
                assert_eq!(answers[0].points.len(), columns.len());
                let source = witness.banks[0] * 2 * (witness.side * witness.side) as usize;
                for ((index, _), query) in columns.into_iter().zip(&answers[0].points) {
                    if !query.surface.wet
                        && query.bed.is_some_and(|bed| {
                            f64::from(bed) - query.surface.sample.height > 0.000_1
                        })
                    {
                        assert_eq!(
                            parcels[index][1], [0.0; 4],
                            "{wind:?} tick {tick}: dry column {:?} must neither move nor create foam",
                            query.point.position
                        );
                        let old = stocks[source + index];
                        let aged = parcels[index][0];
                        if old[1] + old[2] > 0.001 && old[0] > 0.001 {
                            assert!(aged[0] < old[0]);
                            newly_dry_stocks += 1;
                        }
                    }
                }
            }
            assert_eq!(
                witness.origins[0], witness.origins[1],
                "fixed-view source alignment"
            );
            let map = map.lock().expect("shore map");
            for ((point, stock), parcel) in current(&witness, &stocks, 0).zip(parcels) {
                let Some(row) = map
                    .sample(point)
                    .filter(|p| p.body == body && (-0.45..=3.0).contains(&p.depth))
                else {
                    continue;
                };
                let limited = thrum_water_model::limit(
                    row.row,
                    row.depth,
                    significant_variance,
                    map.profile(),
                );
                let lost = limited.lost_power.into_iter().sum::<f64>();
                let rate = f64::from(parcel[1][3]);
                if row.depth + limited.swash_depth < 0.0 {
                    assert!(rate == 0.0, "birth at {point:?} beyond wave support");
                }
                if rate > 0.0 {
                    reported_loss += rate / 0.39;
                    wet_loss += lost;
                }
                deposited += f64::from(stock[1] + stock[2]);
                expected_loss += lost;
                shoreline_cells += 1;
                swash_cells += u32::from(row.depth < 0.0 && stock[1] > 1e-6);
            }
        }
    }
    assert!(shoreline_cells > 0);
    let divisor = f64::from(shoreline_cells);
    let means_for_view = [deposited / divisor, expected_loss / divisor];
    assert!((reported_loss - wet_loss).abs() <= 0.002 * wet_loss + 0.000_01);
    eprintln!(
        "Metal {wind:?}, body {body}: cover and mean depth loss {means_for_view:?}, above-level stocked samples {swash_cells}, sampled birth-rate loss {reported_loss:.6} vs map {wet_loss:.6}"
    );
    if body == 0 && !matches!(wind, Wind::Calm) {
        assert!(swash_cells > 0);
        assert!(
            newly_dry_stocks > 0,
            "previously stocked cells must be checked after drainage"
        );
        eprintln!(
            "Metal {wind:?}: {newly_dry_stocks} stocked dry midpoint columns aged without movement or birth"
        );
    }
    means_for_view
}

#[expect(
    clippy::print_stderr,
    reason = "keep the ground-depth sample count in target evidence"
)]
#[cfg(target_os = "macos")]
fn film_depth_and_query(gpu: &thrum_test_support::Gpu) {
    let extent = support::island::EXTENT;
    let (control, report, _) = water_scene::island::build(Wind::Gale, false).expect("opaque beach");
    let mut control = Fixture::from_game(gpu, extent, control, report);
    control.parameters(r#"{"camera.island":0}"#);
    control.warm_cooked();
    control.frame_without_timings(8.0);
    let ground = control.witness();
    let ground_pixels = control.image();
    drop(control);

    let (game, report, map, probes) = build_foam(Wind::Gale, true, true).expect("beach film");
    let probes = probes.expect("beach probes");
    let mut fixture = Fixture::from_game(gpu, extent, game, report);
    fixture.parameters(r#"{"camera.island":0}"#);
    fixture.warm_cooked();
    for tick in (0..480).step_by(8) {
        fixture.frame_without_timings(f64::from(tick) * 0.016_666_667);
    }
    let before = probes.foam.latest().expect("submitted foam source window");
    let matrix = support::island::view_proj(0);
    let mut candidates = Vec::new();
    let mut points = Vec::new();
    for (pixel, values) in ground.iter().enumerate().step_by(11) {
        if values[1] <= 0.0 {
            continue;
        }
        let pixel = u32::try_from(pixel).expect("pixel index");
        let at = support::world_at(
            matrix,
            extent,
            [pixel % extent[0], pixel / extent[0]],
            values[1],
        );
        let point = [f64::from(at.x), f64::from(at.z)];
        let depth = map
            .lock()
            .expect("map")
            .sample(point)
            .map_or(f64::INFINITY, |p| p.depth);
        if (-0.20..=-0.035).contains(&depth) && points.len() < 480 {
            points.push(QueryPoint {
                position: [at.x, at.z],
                radius: 0.0,
                use_tick: 480,
            });
            candidates.push(pixel as usize);
        }
    }
    assert!(!points.is_empty(), "visible dry beach margin");
    let sample_start = points.len();
    let [ox, oz] = before.origins[1][0].map(f64::from);
    let side = before.side;
    let cell = f64::from(before.spacing[0]);
    let map = map.lock().expect("shore map");
    let mut parcel_indices = Vec::new();
    'rows: for j in 0..side {
        for i in 0..side {
            if (i + 3 * j) % 7 != 0 {
                continue;
            }
            let position = [
                ox + (f64::from(i) + 0.5) * cell,
                oz + (f64::from(j) + 0.5) * cell,
            ];
            if map
                .sample(position)
                .is_some_and(|column| column.body == 0 && (-0.30..=-0.07).contains(&column.depth))
            {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the sampled beach lattice fits the query's metre records"
                )]
                let position = position.map(|metres| metres as f32);
                points.push(QueryPoint {
                    position,
                    radius: 0.0,
                    use_tick: 476,
                });
                parcel_indices.push((j * side + i) as usize);
                if points.len() == 512 {
                    break 'rows;
                }
            }
        }
    }
    drop(map);
    probes.query.post(
        479,
        vec![BodyQuery {
            id: 9,
            generation: 1,
            points,
        }],
    );
    fixture.frame_without_timings(8.000_000_16);
    let answer = probes
        .query
        .drain(481, std::time::Duration::from_nanos(16_666_667));
    let output = fixture.witness();
    let image = fixture.image();
    assert_eq!(answer.len(), 1, "submitted public shoreline query");
    let mut dry_film = 0;
    let mut reactive_foam = 0;
    for (&pixel, query) in candidates.iter().zip(&answer[0].points[..sample_start]) {
        if query.surface.wet {
            continue;
        }
        if output[pixel][1].to_bits() == ground[pixel][1].to_bits()
            && image[pixel * 4..pixel * 4 + 3] != ground_pixels[pixel * 4..pixel * 4 + 3]
        {
            dry_film += 1;
            assert_eq!(
                output[pixel][2..],
                ground[pixel][2..],
                "ground motion below the film"
            );
            assert!(
                output[pixel][0] >= ground[pixel][0],
                "film preserves existing reactive coverage"
            );
            reactive_foam += u32::from(output[pixel][0] > ground[pixel][0]);
        }
    }
    assert!(
        dry_film > 0,
        "dry film must colour pixels at unchanged opaque depth"
    );
    assert!(
        reactive_foam > 0,
        "stranded foam must raise reactive coverage at ground depth"
    );
    let witness = probes.foam.latest().expect("submitted foam parcels");
    assert_eq!(before.origins[1], witness.origins[0]);
    let parcels: Vec<[[f32; 4]; 2]> = support::foam_buffer(&fixture, &witness.parcels);
    let mut dry_births = 0;
    for (&index, query) in parcel_indices.iter().zip(&answer[0].points[sample_start..]) {
        if !query.surface.wet
            && query
                .bed
                .is_some_and(|bed| f64::from(bed) > query.surface.sample.height + 0.04)
        {
            assert_eq!(
                parcels[index][1][3], 0.0,
                "dry column reports a shore birth at {:?}",
                query.point.position
            );
            dry_births += 1;
        }
    }
    assert!(
        dry_births > 0,
        "the public query must identify dry source cells"
    );
    eprintln!(
        "Metal shore film: {dry_film} changed samples with unchanged ground depth and motion and dry public query, including {reactive_foam} with raised reactive coverage. {dry_births} queried dry source cells had zero shore birth."
    );
}
