//! The game applies public water query answers to its rigid bodies on Metal.
mod support;

use std::sync::{Arc, Mutex};
use water_scene::floating::{WakeDemo, build_wake};
use water_scene::island::Wind;

#[test]
fn moored_craft_keeps_its_draft_and_refuses_reactions_by_capacity() {
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
        let mut host = support::Fixture::from_game([96, 64], scene.game, scene.report);
        for tick in 0..=720 {
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
            assert_eq!(wake.endpoints.expect("submitted endpoints")[1], 720);
        }
    }
    assert!(
        (drafts[0] - drafts[1]).abs() < 0.03,
        "draft offsets {drafts:?}"
    );
}

#[test]
fn float_twenty_metres_off_track_responds_to_the_public_query() {
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
    let mut host = support::Fixture::from_game([96, 64], scene.game, scene.report);
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
