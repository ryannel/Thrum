//! The model and the float query see the same folded sheet and swash.
#![cfg(target_os = "macos")]

mod support;

#[path = "support/surf.rs"]
mod surf_fixture;

use std::sync::{Arc, Mutex};
use surf_fixture::{CHOPPINESS, STEP, beach};
use thrum_water::{Profile, Water, query::QueryLimits};
use thrum_water_model::{CutConfig, Sheet, SurfProfile, cut};
use water_scene::floating::{Body, Hull, sample_answer};

const TOLERANCE: f64 = 0.015;
const LIP: [f64; 2] = [-28.705, 3.2795];

#[expect(clippy::print_stderr, reason = "record both failing controls in the target receipt")]
fn check_lip(
    map: &thrum_water_model::Map,
    plain: &thrum_water_model::Map,
    field: &thrum_water_model::Field,
    column: [f64; 2],
    time: f64,
    height: f64,
) {
    let sheet = map.surf_sheet(field, 0, time).expect("breaker sheet");
    let cut = cut(&sheet, column, CutConfig::default()).expect("breaker cut");
    assert!(
        cut.hits >= 3 && cut.converged,
        "three-surface witness: {cut:?}"
    );
    assert!((cut.height - height).abs() < 0.001);
    let mut rest = column;
    for _ in 0..128 {
        let p = sheet.forward(rest);
        rest = std::array::from_fn(|a| column[a] - (p.world[a] - rest[a]));
    }
    let trough = sheet.forward(rest);
    let residual = (trough.world[0] - column[0]).hypot(trough.world[1] - column[1]);
    assert!(
        residual < 0.001 && height - trough.height > 0.10,
        "plain iteration must converge on the trough: {trough:?}"
    );
    let no_transform = plain
        .surface(field, column, time)
        .expect("untransformed wave");
    assert!((height - no_transform.sample.height).abs() > 0.10);
    eprintln!(
        "fold: {} crossings, lip {:.6} m, trough {:.6} m, without transform {:.6} m",
        cut.hits, height, trough.height, no_transform.sample.height
    );
}

#[test]
fn float_queries_match_the_breaker_and_swash() {
    let (plain, spectrum, field) = beach();
    let mut surf = plain.clone();
    surf.enable_surf(SurfProfile::default())
        .expect("shore transform");
    let Some(gpu) = thrum_test_support::gpu("float queries in surf", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let mut fixture = support::Fixture::paired(&gpu, [96, 64], false);
    let mut profile = Profile {
        query: Some(QueryLimits {
            points: 12,
            max_age: 3,
        }),
        ..Profile::default()
    };
    profile.waves.choppiness = CHOPPINESS;
    let water =
        Water::mapped(Arc::new(Mutex::new(surf.clone())), spectrum, profile).expect("mapped surf");
    let mailbox = water.query_mailbox().expect("query mailbox");
    fixture
        .host
        .set_scene_contribution(Box::new(water))
        .expect("surf contribution");
    let mut float = Body::new(1, Hull::Float, [0.0; 2], 0.0, 0.0);
    let mut largest_error = 0.0_f64;
    let station = float.station(0);
    for (tick, position, wet) in [
        (1_u32, LIP, true),
        (20, [2.5, 0.0], true),
        (98, [2.5, 0.0], false),
        (248, [2.5, 0.0], true),
        (348, [2.5, 0.0], false),
    ] {
        float.position.x = position[0] - station.x;
        float.position.z = position[1] - station.z;
        let request = float.request(u64::from(tick), STEP);
        mailbox.post(u64::from(tick), vec![request]);
        let frame = fixture.frame_without_timings((STEP * tick + STEP / 4).as_secs_f64());
        assert_eq!(frame.tick, u64::from(tick));
        let use_tick = tick + 2;
        let replies = mailbox.drain(u64::from(use_tick), STEP);
        assert_eq!(
            replies.len(),
            1,
            "float needs a GPU reply at use tick {use_tick}"
        );
        assert_eq!(replies[0].points.len(), 6);
        for (index, point) in replies[0].points.iter().enumerate() {
            assert_eq!(point.point.use_tick, u64::from(use_tick));
            let column = point.point.position.map(f64::from);
            let time = f64::from(use_tick) * STEP.as_secs_f64();
            let expected = surf.surface(&field, column, time).expect("model column");
            let (actual, _) = sample_answer(Some(&replies[0]), index, float.station(index))
                .expect("float station answer");
            let error = (actual.sample.height - expected.sample.height).abs();
            assert_eq!(actual.wet, expected.wet, "{column:?} at tick {use_tick}");
            assert!(
                error < TOLERANCE,
                "{column:?} at tick {use_tick}: {error} m"
            );
            largest_error = largest_error.max(error);
            if index == 0 {
                assert_eq!(actual.wet, wet, "lip and swash station at tick {use_tick}");
                if tick == 1 {
                    check_lip(&surf, &plain, &field, column, time, expected.sample.height);
                }
                eprintln!(
                    "use tick {use_tick}: station {column:?}, height {:.6} m, wet {}",
                    actual.sample.height, actual.wet
                );
            }
        }
    }
    assert_eq!(mailbox.stats().blocking_reads, 0);
    eprintln!("largest error at a float station: {largest_error:.6} m");
}
