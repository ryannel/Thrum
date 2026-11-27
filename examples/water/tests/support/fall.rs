use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use thrum_water::{
    FallsProfile, Water, foam,
    interaction::{self, BodyPressure, Limits},
};
use thrum_water_model::{
    Bed, Below, Body, BodyKind, Chart, ChartSamples, Fall, FallSegment, FlowSample, Gauge, Map,
    PathPoint, River, RiverFile, Stage, UpperSample,
};
use water_scene::{channel, island::Wind};

pub(crate) const STEP: Duration = Duration::from_nanos(16_666_667);
pub(crate) const POOL: [f64; 2] = [0.0, 2.0];
const WIDTH: usize = 13;
const HEIGHT: usize = 17;
const ORIGIN: [f64; 2] = [-6.0, -5.0];

pub(crate) fn map(gauge: f64) -> Map {
    let closed = |min, max, value| Stage { min, max, value };
    let cell = |i: usize| (i % WIDTH, i / WIDTH);
    let top_foot = |x, z| (4..=8).contains(&x) && (1..=3).contains(&z);
    let pool_foot = |x, z| (1..=11).contains(&x) && (5..=15).contains(&z);
    let heights: Vec<_> = (0..WIDTH * HEIGHT)
        .map(|i| {
            let (x, z) = cell(i);
            if top_foot(x, z) {
                2.75
            } else if pool_foot(x, z) {
                -0.5
            } else if z < 5 {
                3.5
            } else {
                1.0
            }
        })
        .collect();
    let make_chart = |name: &str, level, stage, foot: fn(usize, usize) -> bool, current| Body {
        name: name.into(),
        level: 0.0,
        fetch: 100.0,
        gains: [0.0; 4],
        stage,
        kind: BodyKind::Chart(Chart {
            levels: (0..WIDTH * HEIGHT)
                .map(|i| {
                    let (x, z) = cell(i);
                    if foot(x, z) { level } else { heights[i] - 0.05 }
                })
                .collect(),
            footprint: (0..WIDTH * HEIGHT)
                .map(|i| {
                    let (x, z) = cell(i);
                    foot(x, z)
                })
                .collect(),
            currents: (0..WIDTH * HEIGHT)
                .map(|i| {
                    let (x, z) = cell(i);
                    if foot(x, z) { current } else { [0.0; 2] }
                })
                .collect(),
            ends: Vec::new(),
        }),
    };
    let top = make_chart(
        "Lip",
        3.0,
        closed(0.0, 0.0, 0.0),
        |x, z| (4..=8).contains(&x) && (1..=3).contains(&z),
        [0.0, 0.7],
    );
    let pool = make_chart(
        "Plunge pool",
        0.0,
        closed(0.0, 0.33, 0.0),
        |x, z| (1..=11).contains(&x) && (5..=15).contains(&z),
        [0.0, 0.80],
    );
    let BodyKind::Chart(lip) = &top.kind else {
        unreachable!()
    };
    let sample = FlowSample {
        levels: lip.levels.clone(),
        currents: lip.currents.clone(),
        loss: vec![0.0; WIDTH * HEIGHT],
        gains: vec![[0.0; 2]; WIDTH * HEIGHT],
    };
    let bed = Bed::new(ORIGIN, 1.0, [WIDTH, HEIGHT], heights).expect("test cliff bed");
    let mut map = Map::prepare(
        bed,
        vec![top, pool],
        thrum_water_model::Profile {
            margin_height: 0.08,
            swash_ratio: 0.7,
        },
    )
    .expect("test sheets");
    map.add_river(River {
        name: "Falls test".into(),
        gauges: vec![0.0, 0.5, 1.0],
        gauge: Gauge::Stage(closed(0.0, 1.0, gauge)),
        charts: vec![ChartSamples {
            body: 0,
            samples: vec![sample.clone(), sample.clone(), sample],
            wetting: vec![vec![None; WIDTH * HEIGHT]; 2],
        }],
    })
    .expect("fall gauge");
    map
}

pub(crate) fn records() -> thrum_water_model::FallSamples {
    let fall = |discharge| Fall {
        name: "Free fall".into(),
        above: "Lip".into(),
        segments: (-1..=1)
            .map(|x| {
                let metres = f64::from(x);
                let mut distance = 0.0;
                let mut previous = [3.0, 0.0];
                let path = (0..=13)
                    .map(|index| {
                        let time = 0.065 * f64::from(index);
                        let place = [metres, time * 2.0];
                        let height = 3.0 - 0.5 * 9.81 * time * time;
                        distance += (height - previous[0]).hypot(place[1] - previous[1]);
                        previous = [height, place[1]];
                        PathPoint {
                            place,
                            height,
                            time,
                            distance,
                        }
                    })
                    .collect();
                FallSegment {
                    id: u64::try_from(x + 1).expect("segment"),
                    lip: [[metres - 0.5, 0.0], [metres + 0.5, 0.0]],
                    discharge,
                    launch_speed: 2.0,
                    launch_depth: discharge * 0.5,
                    below: Below::Reach("Plunge pool".into()),
                    path,
                }
            })
            .collect(),
    };
    RiverFile {
        key: [0; 32],
        gauge_level: 0.0,
        reaches: Vec::new(),
        falls: Vec::new(),
        upper: vec![
            UpperSample {
                gauge_level: 0.5,
                falls: vec![fall(0.017)],
                lip_wetting: (0..3).map(|i| (i, 0.08)).collect(),
            },
            UpperSample {
                gauge_level: 1.0,
                falls: vec![fall(0.34)],
                lip_wetting: Vec::new(),
            },
        ],
    }
    .fall_samples()
}

pub(crate) struct Scene {
    pub game: SkyGame,
    pub report: water_scene::SharedReport,
    pub map: Arc<Mutex<Map>>,
    pub foam: Option<foam::Tap>,
    pub wake: Option<interaction::WitnessTap>,
}

pub(crate) fn scene(gauge: f64, enabled: bool, interaction: bool, camera: Camera) -> Scene {
    let map = Arc::new(Mutex::new(map(gauge)));
    let forcing = channel::forcing(Wind::Calm);
    let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("calm");
    let profile = thrum_water::Profile {
        interaction: interaction.then_some(Limits::default()),
        optics: Some(thrum_water::Optics::default()),
        ..water_scene::profile(&spectrum)
    };
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("water");
    if enabled {
        water
            .enable_falls(vec![(0, records())], FallsProfile::default())
            .expect("fall");
    }
    let foam = interaction.then(|| {
        water.enable_foam([0.0; 2]).expect("plunge foam");
        water.tap_foam()
    });
    let wake = interaction.then(|| water.tap_interaction());
    let mut game = SkyGame::new(Controls::new(), camera, |_, settings| {
        settings.temporal_inputs = true;
        settings.sun_elevation_degrees = 65.0;
    })
    .with_ground(None)
    .with_tick_duration(STEP)
    .expect("test clock")
    .with_temporal_sampling(true);
    if let Some(mailbox) = water.interaction_mailbox() {
        game = game.with_fixed_update(move |tick, _| {
            mailbox.post_bodies(
                tick,
                Some(1),
                vec![BodyPressure {
                    id: 1,
                    place: POOL,
                    force: 0.0,
                    footprint: [0.25; 2],
                    heading: 0.0,
                }],
            );
        });
    }
    let report = water.report();
    game = game.with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    Scene {
        game,
        report,
        map,
        foam,
        wake,
    }
}

pub(crate) fn receiver_bank(width: u32, basin: bool) -> Map {
    use thrum_fixtures::brink_receiver::{
        CENTRE, LEVEL, LIP, MARGIN, RISE, SIZE, TOP, bed_heights,
    };
    let heights = bed_heights(width, 4.0);
    let count = heights.len();
    let chart =
        |inside: fn(usize, usize) -> bool, level: f64, ends: Vec<thrum_water_model::End>| {
            let footprint: Vec<_> = (0..count)
                .map(|i| inside(i % SIZE[0], i / SIZE[0]))
                .collect();
            let levels = heights
                .iter()
                .zip(&footprint)
                .map(|(&bed, &inside)| {
                    if inside || ends.is_empty() {
                        level
                    } else {
                        bed - 0.2
                    }
                })
                .collect();
            BodyKind::Chart(Chart {
                levels,
                footprint,
                ends,
                currents: vec![[0.0; 2]; count],
            })
        };
    let body = |name: &str, kind, level, max| Body {
        name: name.into(),
        kind,
        level,
        stage: Stage {
            min: 0.0,
            max,
            value: 0.0,
        },
        fetch: 100.0,
        gains: [0.0; 4],
    };
    let upper = chart(
        |x, z| (1..=2).contains(&x) && (2..=4).contains(&z),
        TOP,
        vec![thrum_water_model::End {
            body: thrum_water_model::BRINK,
            segment: [[f64::from(LIP), 2.0], [f64::from(LIP), 4.0]],
            length: 1.0,
            mouth: false,
        }],
    );
    let receiving = if basin {
        BodyKind::Basin {
            seed: [f64::from(LIP + width + 2), CENTRE],
        }
    } else {
        chart(
            |x, z| (3..=10).contains(&x) && (2..=4).contains(&z),
            LEVEL,
            vec![],
        )
    };
    let basin_level = if basin { LEVEL } else { 0.0 };
    Map::prepare(
        Bed::new([0.0, 0.0], 1.0, SIZE, heights).expect("receiver bed"),
        vec![
            body("Lip", upper, 0.0, 0.0),
            body("Plunge pool", receiving, basin_level, RISE),
        ],
        thrum_water_model::Profile {
            margin_height: MARGIN,
            swash_ratio: 0.7,
        },
    )
    .expect("receiver bank")
}
