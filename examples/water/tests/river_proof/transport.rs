use std::time::Duration;
use thrum_water::query::{BodyQuery, QueryPoint};
use water_scene::{island::Wind, river};

use crate::support::Fixture;

const STEP: Duration = Duration::from_nanos(16_666_667);
const EXTENT: [u32; 2] = [96, 48];
const SUBMISSIONS_PER_SECOND: u32 = 30;

pub(crate) fn check(gpu: &thrum_test_support::Gpu) {
    query(gpu);
    // The ordinary check advances the same world ticks without rendering the
    // entire journey. The device evidence also runs every query submission.
    let submit = std::env::var_os("THRUM_RIVER_SUBMITTED_TRANSPORT").is_some();
    for wind in [Wind::Calm, Wind::Trade] {
        transport(gpu, wind, submit);
    }
}

#[expect(
    clippy::print_stdout,
    reason = "record the float's receiving body and passage speed"
)]
fn transport(gpu: &thrum_test_support::Gpu, wind: Wind, submit: bool) {
    let route =
        river::build(wind, submit, Some(&crate::support::river_store())).expect("river route");
    let mut scene = Fixture::from_game(gpu, EXTENT, route.game, route.report);
    scene.frame_without_timings(0.0);
    let mut entered = [false; 2];
    let mut pool_speed = None;
    let mut narrows_speed = None;
    let mut frame = 0;
    for seconds in [
        6, 12, 18, 24, 30, 36, 42, 50, 60, 70, 85, 100, 125, 160, 200, 250,
    ] {
        if submit {
            while frame < seconds * SUBMISSIONS_PER_SECOND {
                frame += 1;
                scene.frame_without_timings(f64::from(frame) / f64::from(SUBMISSIONS_PER_SECOND));
            }
        } else {
            scene
                .host
                .advance_without_rendering(Duration::from_secs(u64::from(seconds)))
                .expect("river tick page");
        }
        let floats = (route.floats)();
        let map = route.map.lock().expect("river map");
        for (i, float) in floats.into_iter().enumerate() {
            let place = [float.position.x, float.position.z];
            let water = map.sample(place).filter(|row| row.depth > 0.0);
            let owner = water.map(|row| row.body);
            let speed = float.velocity.x.hypot(float.velocity.z);
            println!(
                "Metal {wind:?} float {i} at {seconds}s: position {:?}, speed {speed:.3}, owner {owner:?}",
                float.position
            );
            assert!(float.position.is_finite());
            if let Some(sample) = water.filter(|row| i == 1 || row.body == 1) {
                let heave = float.position.y + float.draft() - sample.row.level;
                assert!(
                    heave.abs() < 1.5,
                    "{wind:?} float {i} left the waterline by {heave:.2} m at {seconds}s"
                );
            }
            if i == 0 {
                if (-70.0..-57.0).contains(&place[1]) {
                    pool_speed = Some(speed);
                } else if (-46.0..-34.0).contains(&place[1]) {
                    narrows_speed = Some(speed);
                }
                entered[0] |= owner == Some(1) && place[1] > -20.0;
            } else {
                entered[1] |= owner == Some(0) && place[0] > 348.0;
            }
        }
        if entered == [true; 2] {
            break;
        }
    }
    assert_eq!(
        entered, [true; 2],
        "{wind:?}: stream-to-lake and river-to-sea float routes at the default stages"
    );
    assert!(narrows_speed.expect("narrows traverse") > pool_speed.expect("pool traverse"));
    if submit {
        let stats = route.query.stats();
        assert!(stats.dispatches > 0 && stats.age_one + stats.age_two + stats.age_older > 0);
        println!("Metal {wind:?} submitted float course: {frame} frames, {stats:?}");
    }
}

#[expect(
    clippy::print_stdout,
    reason = "keep the actual submitted query readings"
)]
fn query(gpu: &thrum_test_support::Gpu) {
    let route = river::build_with_flow(
        Wind::Calm,
        true,
        Some(&crate::support::river_store()),
        false,
    )
    .expect("river query route");
    let map = route.map;
    let query = route.query;
    // Use the actual scene's query pipe and a static set of extra float stations.
    let game = route.game.with_fixed_update_endpoint(|_, _| {});
    let mut scene = Fixture::from_game(gpu, EXTENT, game, route.report);
    let points = [
        [180.0, -92.5],
        [180.0, -65.0],
        [180.0, -41.0],
        [180.0, -26.25],
        [180.0, -25.0],
        [180.0, -20.001],
        [180.0, -19.999],
        [207.499, 5.0],
        [207.501, 5.0],
        [347.499, 5.0],
        [347.501, 5.0],
        [179.668, -23.665],
        [325.515, 5.289],
    ];
    query.post(
        scene.endpoint(0),
        vec![BodyQuery {
            id: 40,
            generation: 1,
            points: points
                .into_iter()
                .map(|position| QueryPoint {
                    position,
                    radius: 0.18,
                    use_tick: 2,
                })
                .collect(),
        }],
    );
    scene.frame_without_timings(0.0);
    scene.frame_without_timings(STEP.as_secs_f64());
    let answers = query.drain(scene.endpoint(2), STEP);
    let answers = &answers
        .iter()
        .find(|answer| answer.id == 40)
        .expect("submitted Metal queries")
        .points;
    let map = map.lock().expect("river map");
    for (point, actual) in points.into_iter().zip(answers) {
        let expected = map
            .sample(point.map(f64::from))
            .filter(|row| row.depth > 0.0);
        assert_eq!(actual.surface.wet, expected.is_some(), "wet at {point:?}");
        if let Some(expected) = expected {
            assert_eq!(
                actual.surface.body,
                Some(expected.body),
                "owner at {point:?}"
            );
            assert!(
                (actual.surface.sample.height - expected.row.level).abs() < 0.000_04,
                "level at {point:?}"
            );
            for (found, expected) in actual.surface.current.into_iter().zip(expected.row.current) {
                assert!((found - expected).abs() < 0.000_004, "current at {point:?}");
            }
        }
        println!(
            "Metal query at {point:?}: wet={} body={:?}, height={:.7}, current={:?}",
            actual.surface.wet,
            actual.surface.body,
            actual.surface.sample.height,
            actual.surface.current
        );
    }
    let (cliff, pool) = (&answers[3].surface, &answers[4].surface);
    assert!(
        cliff.wet && pool.wet,
        "the plunge pool stands against the cliff"
    );
    assert_eq!(
        cliff.body, pool.body,
        "the reach owns the water beneath the fall"
    );
    assert!((cliff.sample.height - pool.sample.height).abs() < 0.000_04);
    for pair in answers[5..11].as_chunks::<2>().0 {
        assert!(pair.iter().all(|row| row.surface.wet));
        assert_ne!(pair[0].surface.body, pair[1].surface.body);
        assert!((pair[0].surface.sample.height - pair[1].surface.sample.height).abs() < 0.000_04);
        for (a, b) in pair[0]
            .surface
            .current
            .into_iter()
            .zip(pair[1].surface.current)
        {
            assert!((a - b).abs() < 0.000_004);
        }
    }
    assert!(
        answers[11].surface.current[1] > 0.01,
        "current into the lake"
    );
    assert!(
        answers[12].surface.current[0] > 0.01,
        "current into the sea"
    );
}
