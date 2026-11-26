//! Time delivery of the example's actual craft and float station requests.
use super::Fixture;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use thrum_sea_model::{Cascade, Spectrum};
use thrum_water::{Water, query::QueryLimits};
use thrum_water_model::{
    Bed, Body, BodyKind, Chart, Map, OpenSky, RainBands, RainBandsProfile, RateHistory, Stage,
};
use water_scene::floating::{Body as HullBody, Hull};

const STEP: Duration = Duration::from_nanos(16_666_667);
const TICKS: u32 = 64;
const STATIONS: u32 = 54;

pub(super) fn drains(gpu: &thrum_test_support::Gpu) -> Value {
    let schedule = RainBandsProfile {
        count: 256.0,
        ..Default::default()
    };
    let table = Arc::new(
        thrum_rain_model::RingProfile {
            count: schedule.count,
            ..Default::default()
        }
        .table(),
    );
    let rain = RainBands::new(
        schedule,
        table,
        RateHistory::dry(STEP.as_secs_f64(), -1, 64),
        Arc::new(OpenSky),
    )
    .expect("prepared rain model");
    let mut map = channel();
    let start = Instant::now();
    map.set_rain(Some(rain));
    let footprint_prepare_ms = start.elapsed().as_secs_f64() * 1000.0;
    let arms = [0.0, 50.0].map(|rate| measure(gpu, map.clone(), rate));
    json!({"stations": STATIONS, "step_ns": STEP.as_nanos(), "arms": arms,
        "footprint_prepare_ms": footprint_prepare_ms})
}

fn measure(gpu: &thrum_test_support::Gpu, mut map: Map, rate: f64) -> Value {
    let mut history = map
        .rain()
        .expect("prepared fleet rain model")
        .history()
        .clone();
    history.rates.fill(rate / 3600.0);
    map.set_rate_history(history.clone())
        .expect("initial rain history");
    let map = Arc::new(Mutex::new(map));
    let spectrum = Spectrum {
        long: Vec::new(),
        tail_slope_variance: 0.0,
        cascades: [160.0, 37.0, 9.0]
            .map(|length| Cascade::new(length, &[]).expect("empty cascade")),
    };
    let mut profile = water_scene::profile(&spectrum);
    profile.query = Some(QueryLimits {
        points: STATIONS,
        max_age: 3,
    });
    let water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("fleet surface");
    let mailbox = water.query_mailbox().expect("fleet mailbox");
    let report = water.report();
    let game = SkyGame::new(
        Controls::new(),
        Camera::look_at([3.0, 3.0, 4.0], [6.0, 1.0, 4.0], 60.0),
        |_, _| {},
    )
    .with_tick_duration(STEP)
    .expect("fleet tick duration")
    .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    let mut fixture = Fixture::from_game(gpu, [96, 64], game, report);
    fixture.frame_without_timings(0.0);
    let fleet: Vec<_> = std::iter::once(HullBody::new(0, Hull::Craft, [5.0, 3.0], 1.0, 0.0))
        .chain((1..=8).map(|id| {
            HullBody::new(
                id,
                Hull::Float,
                [3.0 + 0.7 * f64::from(id), 4.0 + 0.3 * f64::from(id % 3)],
                1.0,
                0.0,
            )
        }))
        .collect();
    let mut nanos = Vec::new();
    for posted in 0..TICKS + 2 {
        let posted = u64::from(posted);
        let endpoint = fixture.endpoint(posted);
        history.commit(rate / 3600.0);
        map.lock()
            .expect("fleet map")
            .set_rate_history(history.clone())
            .expect("committed fleet rain history");
        let start = Instant::now();
        let answers = mailbox.drain(endpoint, STEP);
        let elapsed = start.elapsed();
        if posted >= 2 {
            assert_eq!(answers.len(), fleet.len());
            assert!(answers.iter().all(|answer| !answer.held
                && answer.points.len() == 6
                && answer.points.iter().all(|point| point.surface.wet)));
            nanos.push(elapsed.as_nanos());
        }
        mailbox.post(
            endpoint,
            fleet
                .iter()
                .map(|body| body.request(posted, STEP))
                .collect(),
        );
        fixture.frame_without_timings(STEP.mul_f64(posted as f64).as_secs_f64());
        fixture.finish_tick_queries();
    }
    json!({"rate_mm_h": rate, "drain_ns": nanos})
}

fn channel() -> Map {
    let size = [25, 17];
    let footprint: Vec<_> = (0..size[1])
        .flat_map(|z| {
            (0..size[0]).map(move |x| x > 0 && x + 1 < size[0] && z > 0 && z + 1 < size[1])
        })
        .collect();
    let heights = footprint
        .iter()
        .map(|&wet| if wet { 0.0 } else { 1.1 })
        .collect();
    let bed = Bed::new([0.0; 2], 0.5, size, heights).expect("fleet bed");
    let chart = Chart {
        levels: vec![1.0; footprint.len()],
        currents: (0..footprint.len())
            .map(|i| {
                if i % size[0] < 12 {
                    [0.5, 0.0]
                } else {
                    [1.5, 0.0]
                }
            })
            .collect(),
        footprint,
        ends: Vec::new(),
    };
    Map::prepare(
        bed,
        vec![Body {
            name: "Fleet cost channel".into(),
            kind: BodyKind::Chart(chart),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 10.0,
            gains: [0.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 0.3,
            swash_ratio: 0.0,
        },
    )
    .expect("fleet channel")
}
