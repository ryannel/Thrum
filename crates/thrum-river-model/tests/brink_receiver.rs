//! The receiving body's bank, including stage ranges and interior backwater maxima.
mod setup;

use thrum_water_model::{
    Below, BodyKind, ChartSamples, Fall, FallSegment, FlowSample, Gauge, Map, PathPoint, River,
    Stage,
};

#[test]
fn basin_and_reach_wet_and_support_the_rock_below_narrow_and_wide_brinks() {
    for width in [1, 4] {
        for basin in [false, true] {
            let (mut map, receiver) = setup::fixture(width, basin);
            let fall = fall(4.0, width, basin);
            map.validate_fall(&fall).expect("the brink clears the pool");
            for stage in [0.0, setup::RISE] {
                map.set_stage(receiver, stage).expect("receiver stage");
                let level = setup::LEVEL + stage;
                let mut seen = [false; 3];
                for (station, place) in setup::points(width).enumerate() {
                    let bed = setup::rock(width, place[0]);
                    let actual = map.sample(place);
                    if station == 0 {
                        let lip = actual.expect("upper chart at the brink");
                        assert_eq!(lip.body, 0);
                        close(lip.row.level, setup::TOP);
                    } else if bed > level + setup::MARGIN {
                        assert!(
                            actual.is_none(),
                            "{width}, basin={basin}, {stage}, {place:?}: {actual:?}"
                        );
                        seen[0] = true;
                    } else {
                        let answer = actual.unwrap_or_else(|| {
                            panic!("receiver at {width}, basin={basin}, {stage}, {place:?}")
                        });
                        assert_eq!(answer.body, receiver);
                        close(answer.row.level, level);
                        close(answer.depth, level - bed);
                        seen[if bed < level { 2 } else { 1 }] = true;
                    }
                }
                if width == 4 {
                    assert_eq!(seen, [true; 3], "{stage}, basin={basin}");
                } else {
                    for place in setup::flanks() {
                        let wet = map
                            .sample(place)
                            .expect("pool wraps around the brink's end");
                        assert_eq!(wet.body, receiver);
                        let bed = map.bed().height(place).expect("cliff flank");
                        assert!(bed < level);
                        close(wet.depth, level - bed);
                    }
                }
            }
        }
    }
}

#[test]
fn a_fall_is_refused_by_name_at_the_top_of_a_basin_or_reach_stage() {
    for basin in [false, true] {
        for (rise, drowned) in [(3.3, false), (3.8, true)] {
            let (map, receiver) = unjoined(basin, rise);
            let fall = fall(3.5, 1, basin);
            check_twin(map, receiver, &fall, drowned);
        }
    }
}

#[test]
fn a_fall_is_refused_at_the_high_lake_stage_that_backs_up_a_reach() {
    for (rise, drowned) in [(0.2, false), (0.7, true)] {
        let (map, receiver) = joined(3.55, 3.0, 3.0, rise, 1.0);
        check_twin(map, receiver, &fall(3.55, 1, false), drowned);
    }
}

#[test]
fn a_fall_is_refused_at_an_interior_maximum_of_the_receiving_rows_range() {
    for (lip, drowned) in [(2.86, true), (2.95, false)] {
        let (mut map, receiver) = joined(lip, 3.0, 2.45, 0.40, 3.6);
        let foot = [3.0, setup::CENTRE];
        let levels = [0.0, 0.22, 0.4].map(|stage| {
            map.set_stage(0, stage).expect("lake stage");
            let sample = map.sample(foot).expect("water at the foot");
            assert_eq!(sample.body, receiver);
            sample.row.level
        });
        println!("lip {lip:.3}, receiving levels at lake stages 0, 0.22 and 0.4 m: {levels:?}");
        assert!(levels[0] < lip && levels[2] < lip, "{levels:?}");
        assert_eq!(levels[1] >= lip, drowned, "{levels:?}");
        verdict(&map, &fall(lip, 1, false), drowned);
    }
}

#[test]
fn the_receiving_reachs_middle_gauge_sample_can_drown_a_fall() {
    let (mut map, receiver) = unjoined(false, 0.0);
    let BodyKind::Chart(chart) = &map.body(receiver).expect("receiver").kind else {
        panic!("receiving chart");
    };
    let count = chart.levels.len();
    let sample = |level: f64| FlowSample {
        levels: vec![level; count],
        currents: vec![[0.0; 2]; count],
        loss: vec![0.0; count],
        gains: vec![[0.0; 2]; count],
    };
    map.add_river(River {
        name: "Variable receiving reach".into(),
        gauges: vec![0.0, 1.0, 2.0],
        gauge: Gauge::Stage(Stage {
            min: 0.0,
            max: 2.0,
            value: 0.0,
        }),
        charts: vec![ChartSamples {
            body: receiver,
            samples: vec![sample(0.0), sample(3.8), sample(1.0)],
            wetting: vec![vec![None; count]; 2],
        }],
    })
    .expect("receiving flow samples");
    let fall = fall(3.5, 1, false);
    let foot = fall.segments[0].path[1].place;
    for (gauge, level) in [(0.0, 0.0), (1.0, 3.8), (2.0, 1.0)] {
        map.set_gauge(0, gauge).expect("test gauge");
        close(map.sample(foot).expect("receiving row").row.level, level);
    }
    verdict(&map, &fall, true);
}

fn unjoined(basin: bool, rise: f64) -> (Map, usize) {
    let heights = setup::bed_heights(1, 3.5);
    let upper = setup::upper_chart(&heights, 3.57);
    let receiver = if basin {
        setup::body(
            "Plunge pool",
            BodyKind::Basin {
                seed: [6.0, setup::CENTRE],
            },
            0.0,
            rise,
        )
    } else {
        setup::body(
            "Plunge pool",
            BodyKind::Chart(setup::chart(
                |x, z| (3..=10).contains(&x) && (2..=4).contains(&z),
                0.0,
                vec![],
            )),
            0.0,
            rise,
        )
    };
    (setup::prepare(heights, vec![upper, receiver]), 1)
}

fn joined(lip: f64, own: f64, lake: f64, rise: f64, length: f64) -> (Map, usize) {
    let heights = setup::bed_heights(1, lip);
    let upper = setup::upper_chart(&heights, lip + 0.07);
    let lake = setup::body(
        "Lake",
        BodyKind::Basin {
            seed: [8.0, setup::CENTRE],
        },
        lake,
        rise,
    );
    let chart = setup::chart(
        |x, z| (3..=5).contains(&x) && (2..=4).contains(&z),
        own,
        vec![setup::end(0, 5, length)],
    );
    let receiver = setup::body("Plunge pool", BodyKind::Chart(chart), 0.0, 0.0);
    (setup::prepare(heights, vec![lake, upper, receiver]), 2)
}

fn fall(lip: f64, width: u32, basin: bool) -> Fall {
    let name = "Plunge pool".into();
    let below = if basin {
        Below::Body(name)
    } else {
        Below::Reach(name)
    };
    let x = f64::from(setup::LIP);
    let [a, b] = [x, x + f64::from(width)];
    Fall {
        name: "Test overfall".into(),
        above: "Lip".into(),
        segments: vec![FallSegment {
            id: 0,
            lip: [[a, 2.5], [a, 3.5]],
            discharge: 0.3,
            launch_depth: 0.1,
            launch_speed: 3.0,
            below,
            path: vec![
                PathPoint {
                    place: [a, setup::CENTRE],
                    height: lip,
                    time: 0.0,
                    distance: 0.0,
                },
                PathPoint {
                    place: [b, setup::CENTRE],
                    height: -0.7,
                    time: 1.0,
                    distance: (b - a).hypot(lip + 0.7),
                },
            ],
        }],
    }
}

fn check_twin(mut map: Map, receiver: usize, fall: &Fall, drowned: bool) {
    let stage_body = if receiver == 2 { 0 } else { receiver };
    let range = map.body(stage_body).expect("stage body").stage;
    let place = fall.segments[0].path[1].place;
    let lip = fall.segments[0].path[0].height;
    map.set_stage(stage_body, range.min).expect("low stage");
    assert!(map.sample(place).expect("foot").row.level < lip);
    map.set_stage(stage_body, range.max).expect("high stage");
    assert_eq!(map.sample(place).expect("foot").row.level >= lip, drowned);
    // Start preparation's check at the safe stage even in the drowned twin.
    map.set_stage(stage_body, range.min).expect("low stage");
    verdict(&map, fall, drowned);
}

fn verdict(map: &Map, fall: &Fall, drowned: bool) {
    let result = map.validate_fall(fall);
    if drowned {
        let error = result.expect_err("the receiver drowns this fall");
        assert_eq!(error.name, fall.name);
        assert_eq!(error.reason, "receiving water can drown the fall");
    } else {
        result.expect("the higher lip or smaller range clears the receiver");
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} vs {expected}");
}
