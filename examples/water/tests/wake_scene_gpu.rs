//! The game applies public water query answers to its rigid bodies on Metal.
#[cfg(target_os = "macos")]
use crate::support;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use thrum_sea_model::{Cascade, Spectrum};
use thrum_water::{
    Profile, Water,
    interaction::{BodyPressure, Limits},
    query::{BodyQuery, QueryLimits, QueryPoint},
};
use thrum_water_model::{Bed, BodyKind, Map, Stage};
use water_scene::floating::{Body, Hull, WakeDemo, build_wake};
use water_scene::island::Wind;

thrum_test_support::macos_gpu_test! {
fn moored_craft_keeps_its_draft_and_refuses_reactions_by_capacity() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "moored_craft_keeps_its_draft_and_refuses_reactions_by_capacity",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let demo = WakeDemo {
        body_capacity: Some(1),
        ..WakeDemo::default()
    };
    let mut drafts = [0.0_f64; 2];
    for (disabled, draft) in [true, false].into_iter().zip(&mut drafts) {
        let scene = build_wake(Wind::Calm, false, 0.0, 0.0, WakeDemo { disabled, ..demo })
            .expect("calm scene");
        let bodies = scene.observer();
        let report = Arc::clone(&scene.report);
        let trace = Arc::new(Mutex::new([[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 8]));
        if !disabled {
            let write = Arc::clone(&trace);
            scene.observe_steps(move |body| {
                let interval = match body.tick {
                    1..=300 => 0,
                    1501..=1800 => 1,
                    _ => return,
                };
                if body.id != 0 && body.gpu_points == 6 {
                    let mut ranges = write.lock().expect("moored waterlines");
                    let [low, high] = &mut ranges[body.id as usize - 1][interval];
                    *low = low.min(body.waterline);
                    *high = high.max(body.waterline);
                }
            });
        }
        let mut host = support::Fixture::from_game(&test_gpu, [96, 64], scene.game, scene.report);
        for tick in 0..=if disabled { 720 } else { 1800 } {
            host.frame_without_timings(f64::from(tick) * 0.016_666_667);
        }
        let craft = bodies()[0];
        *draft = craft.position.y + craft.draft();
        eprintln!(
            "Metal rest, wake disabled {disabled}: draft offset {draft:.4} m, vertical speed {:.4} m/s",
            craft.velocity.y
        );
        assert!(craft.velocity.y.abs() < 0.015);
        if !disabled {
            let wake = report
                .lock()
                .expect("water report")
                .interaction
                .expect("interaction");
            assert!(wake.refused_bodies > 5_000);
            assert_eq!(wake.endpoints.expect("submitted endpoints")[1], 1800);
            // The other eight bodies exert no pressure at this capacity.
            // Their GPU waterlines observe the craft's surroundings, outside
            // its hull. Measure variation around the local settled height.
            for (index, ranges) in trace.lock().expect("moored waterlines").iter().enumerate() {
                assert!(ranges.iter().flatten().all(|value| value.is_finite()));
                let [early, late] = ranges.map(|[low, high]| high - low);
                eprintln!(
                    "Metal moored waterline at float {}: early {:.3} mm, settled {:.3} mm",
                    index + 1,
                    early * 1000.0,
                    late * 1000.0
                );
                assert!(
                    early > 0.005,
                    "start-up wave must reach float {}",
                    index + 1
                );
                assert!(
                    late < 0.002 && late < early * 0.1,
                    "persistent wave at float {}",
                    index + 1
                );
            }
        }
    }
    assert!(
        (drafts[0] - drafts[1]).abs() < 0.03,
        "draft offsets {drafts:?}"
    );
}
}

thrum_test_support::macos_gpu_test! {
fn moored_craft_keeps_its_draft_when_only_one_level_admits_pressure() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "moored_craft_keeps_its_draft_when_only_one_level_admits_pressure",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let baseline = moored_strip_draft(&test_gpu, [1, 0], false);
    eprintln!("Metal moored strip, pressure off: draft offset {baseline:.5} m");
    for solves in [[1, 0], [0, 1]] {
        let draft = moored_strip_draft(&test_gpu, solves, true);
        eprintln!("Metal moored strip, fine/coarse solves {solves:?}: draft offset {draft:.5} m");
        assert!(
            (draft - baseline).abs() < 0.03,
            "{solves:?} solves changed the craft's draft by {} m",
            draft - baseline
        );
    }
}
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the public GPU query takes station coordinates in its float format"
)]
#[cfg(target_os = "macos")]
fn moored_strip_draft(
    gpu: &thrum_test_support::Gpu,
    expected_solves: [u32; 2],
    active: bool,
) -> f64 {
    // At 10 m the fine level needs one depth operator and the coarse level
    // needs two. In the other strip, only the fine grid sees the shallower
    // column at x=9 m and needs a second operator.
    let bed_at = |x| match expected_solves {
        [1, 0] => -10.0,
        _ if x == 9 => -6.0,
        _ => -8.0,
    };
    let (min_x, max_x) = if expected_solves == [1, 0] {
        (-16, 16)
    } else {
        // The coarse case must cover the window: beyond the bed the map
        // would select the deep-water operator as well as the 8 m operator.
        (-256, 256)
    };
    let row: Vec<_> = (min_x..=max_x).map(bed_at).collect();
    let bed = Bed::new(
        [f64::from(min_x), -268.0],
        1.0,
        [row.len(), 537],
        row.repeat(537),
    )
    .expect("strip bed");
    let map = Map::prepare(
        bed,
        vec![thrum_water_model::Body {
            name: "Calm sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100_000.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 0.8,
            swash_ratio: 0.7,
        },
    )
    .expect("calm strip map");
    let water = Water::mapped(
        Arc::new(Mutex::new(map)),
        calm_spectrum(),
        Profile {
            interaction: Some(Limits {
                bodies: 1,
                impulses: 1,
                solves: 1,
            }),
            query: Some(QueryLimits {
                points: 6,
                max_age: 4,
            }),
            ..Profile::default()
        },
    )
    .expect("mapped wake contribution");
    let sources = water.interaction_mailbox().expect("wake sources");
    let queries = water.query_mailbox().expect("water query");
    let report = water.report();
    let mut fixture = support::Fixture::paired(gpu, [96, 64], false);
    fixture
        .host
        .set_scene_contribution(Box::new(water))
        .expect("prepare mapped water");
    fixture.frame_without_timings(0.0);
    let step = Duration::from_nanos(16_666_667);
    let mut craft = Body::new(1, Hull::Craft, [0.0; 2], 0.0, 0.0);
    for tick in 1..=1200 {
        let axis = craft.rotation.mul_vec3([1.0, 0.0, 0.0].into());
        sources.post_bodies(
            u64::from(tick),
            None,
            vec![BodyPressure {
                id: 1,
                place: [craft.position.x, craft.position.z],
                heading: axis.z.atan2(axis.x),
                footprint: [1.45, 0.58],
                force: if active {
                    3600.0 * craft.water_reaction
                } else {
                    0.0
                },
            }],
        );
        queries.post(
            u64::from(tick - 1),
            vec![BodyQuery {
                id: 1,
                generation: 0,
                points: (0..6)
                    .map(|index| {
                        let p = craft.station(index);
                        QueryPoint {
                            position: [p.x as f32, p.z as f32],
                            radius: 0.2784,
                            use_tick: u64::from(tick),
                        }
                    })
                    .collect(),
            }],
        );
        fixture.frame_without_timings((step * tick).as_secs_f64());
        let answers = queries.drain(u64::from(tick), step);
        assert_eq!(answers.len(), 1, "query response at tick {tick}");
        craft.advance(step.as_secs_f64(), |i, _| {
            (answers[0].points[i].surface, Some(bed_at(0)))
        });
    }
    let wake = report
        .lock()
        .expect("water report")
        .interaction
        .expect("interaction");
    assert_eq!(wake.endpoints.expect("submitted endpoints")[1], 1200);
    assert_eq!(wake.solves, expected_solves);
    assert!(
        craft.velocity.y.abs() < 0.015,
        "craft still moving with solves {expected_solves:?}: {} m/s",
        craft.velocity.y
    );
    craft.position.y + craft.draft()
}

fn calm_spectrum() -> Spectrum {
    Spectrum {
        long: Vec::new(),
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm cascade")),
        tail_slope_variance: 0.0,
    }
}

thrum_test_support::macos_gpu_test! {
fn float_twenty_metres_off_track_responds_to_the_public_query() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "float_twenty_metres_off_track_responds_to_the_public_query",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let scene = build_wake(
        Wind::Calm,
        false,
        6.0,
        0.0,
        WakeDemo {
            body_capacity: Some(1),
            ..WakeDemo::default()
        },
    )
    .expect("calm wake");
    let trace = Arc::new(Mutex::new(Vec::new()));
    let write = Arc::clone(&trace);
    scene.observe_steps(move |body| {
        if body.id == 1 {
            write.lock().expect("float trace").push((
                body.tick,
                body.heave,
                body.waterline,
                body.gpu_points,
            ));
        }
    });
    let mut host = support::Fixture::from_game(&test_gpu, [96, 64], scene.game, scene.report);
    for tick in 0..=1_800 {
        if tick == 450 {
            host.parameters(r#"{"craft.speed":0}"#);
        }
        host.frame_without_timings(f64::from(tick) * 0.016_666_667);
    }
    let trace = trace.lock().expect("float trace");
    assert!(trace.iter().filter(|step| step.3 == 6).count() > 1_000);
    let range = |from, to| {
        trace
            .iter()
            .filter(|step| (from..to).contains(&step.0))
            .fold([f64::INFINITY, f64::NEG_INFINITY], |[low, high], step| {
                [low.min(step.1), high.max(step.1)]
            })
    };
    let before = range(1, 40);
    let during = range(120, 1_150);
    let after = range(1_690, 1_800);
    eprintln!("Metal float heave, early {before:?}, in wake {during:?}, after {after:?}");
    assert!(before[1] - before[0] < 0.001);
    assert!(during[0] < -0.001 && during[1] > 0.001);
    assert!(after[1] - after[0] < (during[1] - during[0]) * 0.3);
}
}

thrum_test_support::macos_gpu_test! {
fn the_island_lake_wake_leaves_the_pond_and_sea_at_rest() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "the_island_lake_wake_leaves_the_pond_and_sea_at_rest",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let map = water_scene::island::map(Wind::Calm).expect("same island as the crossing");
    let columns = [[192.0, 2.7], [104.0, 93.0], [364.0, 10.0]];
    for (point, body) in columns.into_iter().zip([1, 2, 0]) {
        let sample = map.sample(point).expect("authored water column");
        assert_eq!(sample.body, body);
        assert!(sample.depth > 0.0);
    }
    let water = Water::mapped(
        Arc::new(Mutex::new(map)),
        calm_spectrum(),
        Profile {
            interaction: Some(Limits::default()),
            query: Some(QueryLimits {
                points: 3,
                max_age: 4,
            }),
            ..Profile::default()
        },
    )
    .expect("mapped wake contribution");
    let sources = water.interaction_mailbox().expect("wake sources");
    let queries = water.query_mailbox().expect("water query");
    let mut fixture = support::Fixture::new(&test_gpu, [96, 64]);
    fixture
        .host
        .set_scene_contribution(Box::new(water))
        .expect("prepare mapped water");
    fixture.frame_without_timings(0.0);
    let step = Duration::from_nanos(16_666_667);
    let mut lake_peak = 0.0_f64;
    for tick in (8..=256).step_by(8) {
        for tick in tick - 7..=tick {
            sources.post_bodies(
                u64::from(tick),
                Some(0),
                vec![BodyPressure {
                    id: 0,
                    place: [204.0 - f64::from(tick) * 0.05, 0.0],
                    heading: std::f64::consts::PI,
                    footprint: [1.45, 0.58],
                    force: 36_000.0,
                }],
            );
        }
        let use_tick = u64::from(tick) + 1;
        queries.post(
            u64::from(tick),
            vec![BodyQuery {
                id: 99,
                generation: 1,
                points: [[192.0, 2.7], [104.0, 93.0], [364.0, 10.0]]
                    .map(|position| QueryPoint {
                        position,
                        radius: 0.0,
                        use_tick,
                    })
                    .to_vec(),
            }],
        );
        fixture.frame_without_timings((step * tick).as_secs_f64());
        let reply = queries.drain(use_tick, step);
        let heights: Vec<_> = reply[0]
            .points
            .iter()
            .map(|p| p.surface.sample.height)
            .collect();
        lake_peak = lake_peak.max((heights[0] - 12.0).abs());
        assert!(
            (heights[1] - 10.6).abs() < 0.000_001,
            "pond reply at tick {tick}: {}",
            heights[1]
        );
        assert_eq!(heights[2].to_bits(), 0, "sea reply at tick {tick}");
    }
    assert!(lake_peak > 0.005, "the wake must reach the lake probe");
    eprintln!("Metal island crossing: lake peak {lake_peak:.6} m, pond and sea unchanged");
}
}
