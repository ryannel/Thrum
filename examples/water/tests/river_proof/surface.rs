use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use thrum_water::query::{BodyQuery, QueryLimits, QueryPoint};
use thrum_water_model::{Field, Map, RiverFile};
use water_scene::{channel, island::Wind, river};

use super::map::{FALL_REACH, HOLLOW, REACHES, WETTING_GAUGES, transect_points, wetting_fixture};
use crate::support::{self, Fixture};

const EXTENT: [u32; 2] = [95, 13];
const STEP: Duration = Duration::from_nanos(16_666_667);
const EPS_INTERIOR: f64 = 0.010;
const EPS_JOIN: f64 = 0.000_04;
fn ray_hit(inverse: glam::DMat4, column: u32, row: u32, depth: f32) -> glam::DVec3 {
    let u = (f64::from(column) + 0.5) / f64::from(EXTENT[0]);
    let v = (f64::from(row) + 0.5) / f64::from(EXTENT[1]);
    inverse.project_point3(glam::dvec3(2.0 * u - 1.0, 1.0 - 2.0 * v, f64::from(depth)))
}

struct TradeDepth {
    scene: Fixture,
    camera: Arc<Mutex<Camera>>,
    query: thrum_water::query::QueryMailbox,
    tap: support::depth::DepthTap,
    tick: u32,
}

impl TradeDepth {
    fn new(gpu: &thrum_test_support::Gpu, map: Map, spectrum: thrum_sea_model::Spectrum) -> Self {
        let mut profile = water_scene::profile(&spectrum);
        profile.optics = Some(thrum_water::Optics::default());
        profile.query = Some(QueryLimits {
            points: 1,
            max_age: 8,
        });
        let water = thrum_water::Water::mapped(Arc::new(Mutex::new(map)), spectrum, profile)
            .expect("mapped rivers");
        let report = water.report();
        let query = water.query_mailbox().expect("body query");
        let initial = river::VIEWS[0].1;
        let camera = Arc::new(Mutex::new(initial));
        let display = Arc::clone(&camera);
        let tap = support::depth::DepthTap::default();
        let game = SkyGame::new(Controls::new(), initial, |_, _| {})
            .with_presentation_camera(move |_| *display.lock().expect("trade depth camera"))
            .with_tick_duration(STEP)
            .expect("elapsed surface clock")
            .with_ground(None)
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
            .with_contribution(Box::new(tap.contribution(EXTENT)), || {
                ContentStatus::Drawn(1)
            });
        let mut scene = Fixture::from_game(gpu, EXTENT, game, report);
        scene.frame_without_timings(0.0);
        Self {
            scene,
            camera,
            query,
            tap,
            tick: 0,
        }
    }

    fn submit_tick(&mut self, tick: u32) {
        self.tick = tick;
        let use_tick = u64::from(self.tick);
        let submitted = self
            .scene
            .frame_without_timings((STEP * (self.tick + 1)).as_secs_f64());
        assert_eq!(submitted.tick, use_tick + 1);
        assert!(submitted.presentation_alpha.abs() < 1e-5);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "query coordinates use GPU metres"
    )]
    fn pair(
        &mut self,
        point: [f32; 2],
        body: usize,
        inverse: glam::DMat4,
        station_radius: f64,
    ) -> Option<(f64, [f32; 3])> {
        self.submit_tick(self.tick + 1);
        let use_tick = u64::from(self.tick);
        let [column, row] = EXTENT.map(|axis| axis / 2);
        let depth = self.tap.read(&self.scene)[(row * EXTENT[0] + column) as usize];
        let hit = (depth > 0.0).then(|| ray_hit(inverse, column, row, depth));
        let position = hit.map_or(point, |p| [p.x as f32, p.z as f32]);
        // Dispatch from the field of the submitted frame. A predicted query
        // from an earlier wave field has its own time extrapolation error.
        self.query.post(
            self.scene.endpoint(use_tick - 1),
            vec![BodyQuery {
                id: 91,
                generation: 1,
                points: vec![QueryPoint {
                    position,
                    radius: 0.001,
                    use_tick,
                }],
            }],
        );
        self.scene.finish_tick_queries();
        let answer = self
            .query
            .drain(self.scene.endpoint(use_tick), STEP)
            .into_iter()
            .flat_map(|body| body.points)
            .find(|answer| answer.point.use_tick == use_tick)
            .expect("query at the submitted use tick");
        if !answer.surface.wet || answer.surface.body != Some(body) {
            return None;
        }
        let hit =
            hit.unwrap_or_else(|| panic!("missing trade depth at {point:?}, tick {use_tick}"));
        assert!(
            (hit.x - f64::from(point[0])).hypot(hit.z - f64::from(point[1])) < station_radius,
            "trade depth ray missed its station at {point:?}, tick {use_tick}: {hit:?}"
        );
        let display_time = (STEP * self.tick).as_secs_f64();
        assert!(
            answer.sources[..4]
                .iter()
                .flatten()
                .all(|time| (time - display_time).abs() < 1e-6),
            "trade depth at {position:?}, tick {use_tick}: stale field sources {:?}",
            answer.sources
        );
        let error = (hit.y - answer.surface.sample.height).abs();
        assert!(
            error <= EPS_INTERIOR,
            "trade depth at {position:?}, tick {use_tick}: {error:.7} m, field sources {:?}",
            answer.sources
        );
        Some((
            error,
            answer.wave_heights[1..]
                .try_into()
                .expect("three wind-wave cascades"),
        ))
    }
}

fn trade_depth(gpu: &thrum_test_support::Gpu, stage: f64) -> (TradeDepth, Map) {
    let mut forcing = channel::forcing(Wind::Trade);
    forcing.wind_direction = 0.42;
    let (mut map, files) =
        river::map(&forcing, Some(&support::river_store())).expect("island rivers");
    map.set_stage(1, stage).expect("lake stage");
    let [low, middle, high] = files[0].gauges().try_into().expect("three spring samples");
    let spring = if stage.to_bits() == 0.5_f64.to_bits() {
        middle
    } else {
        (high - low).mul_add(stage, low)
    };
    map.set_gauge(0, spring).expect("spring gauge");
    let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    (TradeDepth::new(gpu, map.clone(), spectrum), map)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "proof camera uses GPU metres and logs retain the measured residuals"
)]
pub(crate) fn check_live_depth(gpu: &thrum_test_support::Gpu, stage: f64, tidal_station: bool) {
    let (mut probe, map) = trade_depth(gpu, stage);
    let sites = if tidal_station {
        vec![(5, [337.83, 5.23])]
    } else {
        transect_points(&map)
    };
    let mut count = [0; 6];
    let mut maximum_error = 0.0_f64;
    let mut waves = [0.0_f32; 3];
    for (body, site) in sites {
        let [x, z] = site.map(|v| v as f32);
        let level = map.sample(site).expect("wet river station").row.level as f32;
        let view = Camera::look_at([x + 0.001, level + 2.0, z], [x, level, z], 12.0);
        *probe.camera.lock().expect("trade depth camera") = view;
        let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
        let mut observed = 0;
        for _ in 0..if tidal_station { 180 } else { 240 } {
            if let Some((error, short_waves)) = probe.pair([x, z], body, inverse, 0.002) {
                maximum_error = maximum_error.max(error);
                for (peak, height) in waves.iter_mut().zip(short_waves) {
                    *peak = peak.max(height.abs());
                }
                observed += 1;
                if !tidal_station {
                    break;
                }
            }
        }
        assert!(
            observed >= if tidal_station { 135 } else { 1 },
            "only {observed} wet pairs at {site:?}, stage {stage}"
        );
        count[body] += observed;
    }
    assert!(
        waves.iter().all(|&height| height > 0.001),
        "inactive trade cascades: {waves:?}"
    );
    if !tidal_station {
        assert!(
            count[3] >= 46 && count[4] >= 3 && count[5] >= 90,
            "{count:?}"
        );
    }
    println!(
        "Metal full-trade stage {stage}: {} spring, {} plunge reach, {} outlet pairs, maximum residual {maximum_error:.7} m, short-wave peaks {waves:?} m",
        count[3], count[4], count[5]
    );
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "proof camera uses GPU metres and logs retain the measured residuals"
)]
pub(crate) fn check_bank_depth(gpu: &thrum_test_support::Gpu) {
    let (mut probe, map) = trade_depth(gpu, 0.0);
    let site = [340.71, 5.8];
    let [x, z] = site.map(|v| v as f32);
    let level = map.sample(site).expect("wet tidal station").row.level as f32;
    let view = Camera::look_at([x + 4.5, level + 2.0, z + 3.6], [x, level, z], 55.0);
    *probe.camera.lock().expect("trade depth camera") = view;
    let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
    // Start before the wave train that crosses this bank ray, then submit
    // each display tick so the short-wave cascades reach their normal cadence.
    for tick in 376..380 {
        probe.submit_tick(tick);
    }
    let mut maximum_error = 0.0_f64;
    for _ in 380..=400 {
        // The ray moves across the reach as the level rises. Use the actual
        // reconstructed hit for the query and require it to belong to the outlet.
        let (error, _) = probe
            .pair([x, z], 5, inverse, 2.0)
            .expect("wet depth at the oblique tidal station");
        maximum_error = maximum_error.max(error);
    }
    println!("Metal full-trade bank view, ticks 380–400: maximum residual {maximum_error:.7} m");
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "fixture camera uses GPU metres and the proof records submitted depths"
)]
pub(crate) fn check_wetting(gpu: &thrum_test_support::Gpu) {
    let map = Arc::new(Mutex::new(wetting_fixture()));
    let spectrum = thrum_sea_model::Spectrum::build(&channel::forcing(Wind::Calm))
        .expect("calm fixture spectrum");
    let profile = water_scene::profile(&spectrum);
    let field = water_scene::floating::fallback(&spectrum, profile.waves.choppiness)
        .expect("calm model field");
    let water =
        thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile).expect("sampled water");
    let report = water.report();
    let initial = Camera::look_at([5.02, 5.0, 5.0], [5.0, 2.9, 5.0], 4.0);
    let camera = Arc::new(Mutex::new(initial));
    let on_tick = Arc::clone(&camera);
    let tap = support::depth::DepthTap::default();
    // With no opaque ground, an early water write cannot hide behind it.
    // The chart's own bed still determines which part of a ray is above ground.
    let game = SkyGame::new(Controls::new(), initial, |_, _| {})
        .with_presentation_camera(move |_| *on_tick.lock().expect("wetting camera"))
        .with_tick_duration(STEP)
        .expect("elapsed surface clock")
        .with_ground(None)
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
        .with_contribution(Box::new(tap.contribution(EXTENT)), || {
            ContentStatus::Drawn(1)
        });
    let mut scene = Fixture::from_game(gpu, EXTENT, game, report);
    scene.frame_without_timings(0.0);
    let mut time = 0.0;
    for gauge in WETTING_GAUGES {
        map.lock()
            .expect("wetting map")
            .set_gauge(0, gauge)
            .expect("fixture gauge");
        for (feature, point, threshold) in std::iter::once(("hollow", HOLLOW, 0.47))
            .chain(FALL_REACH.map(|[x, z]| ("fall reach", [x, z.clamp(5.02, 7.98)], 0.34)))
        {
            let bed = map
                .lock()
                .expect("wetting map")
                .bed()
                .height(point.map(f64::from))
                .expect("fixture ground");
            let [x, z] = point;
            let y = bed as f32;
            let view = Camera::look_at([x + 0.02, y + 2.0, z], [x, y, z], 4.0);
            *camera.lock().expect("wetting camera") = view;
            time += 0.25;
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(&scene);
            let [column, row] = EXTENT.map(|axis| axis / 2);
            let written = depth[(row * EXTENT[0] + column) as usize];
            let map = map.lock().expect("wetting map");
            let expected = map
                .surface(&field, point.map(f64::from), time)
                .expect("model column");
            assert_eq!(
                expected.wet,
                gauge > threshold,
                "{feature} {point:?}, gauge {gauge}"
            );
            if written == 0.0 {
                assert!(
                    !expected.wet,
                    "{feature} {point:?} missing wet depth at gauge {gauge}"
                );
                println!(
                    "Metal wetting draw {feature} {point:?}, gauge {gauge:.2}, t {time:.2}: clear"
                );
                continue;
            }
            let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
            let hit = ray_hit(inverse, column, row, written);
            let place = [hit.x, hit.z];
            let model = map
                .surface(&field, place, time)
                .expect("model at submitted hit");
            assert_eq!(
                model.wet, expected.wet,
                "{feature} {point:?}, gauge {gauge}"
            );
            if expected.wet {
                let error = (hit.y - model.sample.height).abs();
                assert!(
                    error <= EPS_INTERIOR,
                    "{feature} {point:?}, gauge {gauge}: depth residual {error:.7} m"
                );
                println!(
                    "Metal wetting draw {feature} {point:?}, gauge {gauge:.2}, t {time:.2}: depth residual {error:.7} m"
                );
            } else {
                let ground = map.bed().height(place).expect("ground at submitted hit");
                assert!(
                    hit.y <= ground,
                    "{feature} {point:?} wrote above ground at dry gauge {gauge}: {:.7} m above bed",
                    hit.y - ground
                );
                println!(
                    "Metal wetting draw {feature} {point:?}, gauge {gauge:.2}, t {time:.2}: depth below ground"
                );
            }
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "authored camera coordinates use GPU metres and logs retain target measurements"
)]
pub(crate) fn check(gpu: &thrum_test_support::Gpu, upper: bool) {
    let forcing = channel::forcing(Wind::Trade);
    let (map, files) = river::map(&forcing, Some(&support::river_store())).expect("island rivers");
    let mut spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    spectrum.cascades = [160.0, 37.0, 9.0].map(|length| {
        thrum_sea_model::Cascade::new(length, &[]).expect("empty calibration cascade")
    });
    let profile = water_scene::profile(&spectrum);
    let field =
        water_scene::floating::fallback(&spectrum, profile.waves.choppiness).expect("column field");
    let map = Arc::new(Mutex::new(map));
    let water =
        thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile).expect("mapped rivers");
    let report = water.report();
    let camera = Arc::new(Mutex::new(river::VIEWS[0].1));
    let on_tick = Arc::clone(&camera);
    let tap = support::depth::DepthTap::default();
    let game = SkyGame::new(Controls::new(), river::VIEWS[0].1, |_, _| {})
        .with_presentation_camera(move |_| *on_tick.lock().expect("proof camera"))
        .with_tick_duration(STEP)
        .expect("elapsed surface clock")
        .with_ground(None)
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
        .with_contribution(Box::new(tap.contribution(EXTENT)), || {
            ContentStatus::Drawn(1)
        });
    let mut scene = Fixture::from_game(gpu, EXTENT, game, report);
    scene.frame_without_timings(0.0);
    let mut time = 0.0;
    let [low, middle, high] = files[0].gauges().try_into().expect("three spring samples");
    let mut count = [0; 6];
    let mut max_error: f64 = 0.0;
    let stages = if upper {
        [(middle, 0.5), (high, 1.0)]
    } else {
        [(low, 0.0), ((high - low).mul_add(0.37, low), 0.37)]
    };
    for (spring, stage) in stages {
        {
            let mut map = map.lock().expect("river stage");
            map.set_gauge(0, spring).expect("spring stage");
            map.set_stage(1, stage).expect("lake stage");
        }
        let sites = transect_points(&map.lock().expect("river map"));
        for (body, axis) in REACHES {
            let stations: Vec<_> = sites
                .iter()
                .filter_map(|&(owner, point)| (owner == body).then_some(point[axis]))
                .collect();
            let first = *stations.first().expect("wet reach start");
            let last = *stations.last().expect("wet reach end");
            assert!(
                stations.windows(2).all(|p| p[1] - p[0] <= 2.0),
                "{stations:?}"
            );
            println!(
                "Metal stage {stage}, body {body}: {first:.2} to {last:.2} m, {} stations at 1.5 m",
                stations.len()
            );
        }
        let close = sites
            .into_iter()
            .chain([(5, [320.87, 6.17])])
            .map(|(owner, point)| (owner, point, [0.14, 0.25, 0.08]));
        let from_bank = [
            (3, [179.35, -61.97], [7.0, 2.0, 0.8]),
            (5, [237.98, 6.17], [7.0, 2.0, 0.8]),
        ];
        for (owner, point, [dx, dy, dz]) in close.chain(from_bank) {
            time += 0.25;
            let height = {
                let map = map.lock().expect("river map");
                // At the tidal end choose a passing wave that leaves the
                // sampled column wet. The advancing field time is still
                // the exact time read back from the submitted frame.
                let surface = loop {
                    let sample = map.surface(&field, point, time).expect("model surface");
                    if sample.wet && sample.body == Some(owner) {
                        break sample;
                    }
                    time += 0.125;
                    assert!(time < 1024.0, "no wet column at {point:?}");
                };
                surface.sample.height as f32
            };
            let [x, z] = point.map(|p| p as f32);
            let view = Camera::look_at(
                [x + dx, (height + dy).max(0.12), z + dz],
                [x, height, z],
                4.0,
            );
            *camera.lock().expect("proof camera") = view;
            let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(&scene);
            let map = map.lock().expect("river map");
            let row = EXTENT[1] / 2;
            let column = EXTENT[0] / 2;
            let d = depth[(row * EXTENT[0] + column) as usize];
            assert!(d > 0.0, "submitted depth at {point:?}, stage {stage}");
            let p = ray_hit(inverse, column, row, d);
            let place = [p.x, p.z];
            let expected = map.surface(&field, place, time).expect("model surface");
            assert!(expected.wet, "model water at {place:?}, stage {stage}");
            assert_eq!(expected.body, Some(owner), "model river at {place:?}");
            let error = (p.y - expected.sample.height).abs();
            assert!(
                error <= EPS_INTERIOR,
                "depth at {place:?}, stage {stage}, t {time:.2}: {error:.8} m"
            );
            count[owner] += 1;
            max_error = max_error.max(error);
        }
    }
    assert!(
        count[3] >= 92 && count[4] >= 6 && count[5] >= 180,
        "{count:?}"
    );
    println!(
        "Metal river transects: {} spring, {} plunge reach, {} outlet hits, maximum residual {max_error:.7} m",
        count[3], count[4], count[5],
    );
    if upper {
        check_joins(&map, &camera, &mut scene, &tap, &field, &files, time);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "join cameras use GPU metres and the log retains the measured step"
)]
fn check_joins(
    map: &Mutex<Map>,
    camera: &Mutex<Camera>,
    scene: &mut Fixture,
    tap: &support::depth::DepthTap,
    field: &Field,
    files: &[RiverFile; 2],
    mut time: f64,
) {
    let joins: Vec<_> = files
        .iter()
        .flat_map(|file| &file.reaches)
        .flat_map(|reach| &reach.ends)
        .filter(|end| end.kind != thrum_water_model::EndKind::Brink)
        .collect();
    assert_eq!(joins.len(), 3);
    let mut max_swell: f64 = 0.0;
    for (lake, tide) in [(0.0, -0.5), (1.0, -0.5), (0.0, 0.5), (1.0, 0.5)] {
        {
            let mut map = map.lock().expect("river stage");
            map.set_stage(1, lake).expect("lake stage");
            map.set_stage(0, tide).expect("tide");
        }
        for join in &joins {
            time += 0.25;
            let [a, b] = join.segment;
            let point = [a[0].midpoint(b[0]), a[1].midpoint(b[1])];
            let centre = {
                let map = map.lock().expect("river map");
                let bed = map.bed().height(point).expect("join bed");
                let (at, surface) = (0_u16..=90)
                    .map(|offset| time + f64::from(offset) * 0.20)
                    .find_map(|at| {
                        let surface = map.surface(field, point, at).expect("join surface");
                        let high = surface.sample.height;
                        (surface.wet
                            && high > bed + 0.03
                            && (join.body != "Sea" || high > tide + 0.02))
                            .then_some((at, surface))
                    })
                    .expect("a submerged join under the passing swell");
                time = at;
                surface
            };
            let [x, z] = point.map(|p| p as f32);
            let y = centre.sample.height as f32;
            let offset = if (a[0] - b[0]).abs() > 1e-8 {
                [0.002, 0.0]
            } else {
                [0.0, 0.002]
            };
            let view = Camera::look_at(
                [x + offset[0], (y + 0.25).max(0.12), z + offset[1]],
                [x, y, z],
                4.0,
            );
            *camera.lock().expect("proof camera") = view;
            let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
            // The displayed sample lies one fixed step behind the observation clock.
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(scene);
            let map = map.lock().expect("river map");
            let row = EXTENT[1] / 2;
            // An odd target samples the seam line through the centre of the
            // middle row. Adjacent columns then resolve the two ownership
            // sides within millimetres of the shared mesh edge.
            let [left, right] = [EXTENT[0] / 2 - 1, EXTENT[0] / 2 + 1].map(|column| {
                let d = depth[(row * EXTENT[0] + column) as usize];
                assert!(
                    d > 0.0,
                    "rendered join at {point:?}, lake {lake}, tide {tide}, column {column}"
                );
                let p = ray_hit(inverse, column, row, d);
                let place = [p.x, p.z];
                let expected = map.surface(field, place, time).expect("model surface");
                assert!(expected.wet);
                let error = (p.y - expected.sample.height).abs();
                assert!(
                    error < EPS_INTERIOR,
                    "depth residual at {place:?}: {error:.7} m"
                );
                (expected.body, p.y, expected.sample.height)
            });
            assert_ne!(
                left.0, right.0,
                "the pixels must straddle ownership at {point:?}"
            );
            // Sea wave height is a common displacement across this join.
            // Compare the step itself, after accounting for the reference
            // wave slope between the two distinct ray hits.
            let residual = ((right.1 - left.1) - (right.2 - left.2)).abs();
            println!(
                "Metal join {point:?}, lake {lake}, tide {tide}, t {time:.3}: owners {:?}/{:?}, GPU {:.7}/{:.7} m, model {:.7}/{:.7} m, seam residual {residual:.7} m",
                left.0, right.0, left.1, right.1, left.2, right.2,
            );
            assert!(residual < EPS_JOIN, "step at {point:?}: {residual:.7} m");
            if join.body == "Sea" {
                assert!([left.1, right.1].iter().all(|y| y - tide > 0.01));
                max_swell = max_swell.max((centre.sample.height - tide).abs());
            }
        }
    }
    assert!(max_swell > 0.01, "the sea join was crossed with a swell");
}
