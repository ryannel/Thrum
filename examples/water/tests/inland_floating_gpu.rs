//! The same game-owned forces on the lake and in a prepared chart.
#[cfg(target_os = "macos")]
use crate::support;

use glam::DVec3;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_sea_model::Spectrum;
use thrum_water::{Water, query::QueryLimits};
use thrum_water_model::{Bed, BodyKind, Chart, Map, Stage};
use water_scene::{
    floating::{Body, Hull, sample_answer},
    island::Wind,
};

thrum_test_support::macos_gpu_test! {
fn lake_remains_sheltered_in_the_gale_and_settles_in_calm() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "lake_remains_sheltered_in_the_gale_and_settles_in_calm",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    // The test's name in the package's `it` binary, which the child process runs by.
    support::island::with_cooked_island(
        "inland_floating_gpu::lake_remains_sheltered_in_the_gale_and_settles_in_calm",
        || {
            for wind in [Wind::Calm, Wind::Gale] {
                let scene =
                    water_scene::floating::build(wind, true, 0.0, 0.0, false).expect("lake");
                let bodies = scene.observer();
                let mailbox = scene.mailbox.clone();
                let mut host =
                    support::Fixture::from_game(&test_gpu, [120, 80], scene.game, scene.report);
                host.warm_cooked();
                let mut largest = 0.0_f64;
                for frame in 121..=225 {
                    host.frame(f64::from(frame) / 120.0);
                    let float = bodies()[1];
                    largest = largest.max((float.position.y + float.draft() - 12.0).abs());
                }
                assert!(mailbox.stats().age_one + mailbox.stats().age_two > 200);
                if matches!(wind, Wind::Calm) {
                    assert!(largest < 0.001, "calm lake draft deviated by {largest} m");
                } else {
                    assert!((0.003..0.30).contains(&largest), "lake heave {largest} m");
                }
            }
        },
    );
}
}

fn channel() -> Map {
    let size = [27, 9];
    let footprint: Vec<bool> = (0..size[1])
        .flat_map(|j| (0..size[0]).map(move |i| (1..=19).contains(&i) && (1..=7).contains(&j)))
        .collect();
    let heights = footprint
        .iter()
        .map(|&inside| if inside { 0.0 } else { 1.1 })
        .collect();
    let bed = Bed::new([0.0; 2], 0.5, size, heights).expect("channel bed");
    let chart = Chart {
        levels: vec![1.0; footprint.len()],
        currents: vec![[3.0, 0.0]; footprint.len()],
        footprint,
        ends: Vec::new(),
    };
    Map::prepare(
        bed,
        vec![thrum_water_model::Body {
            name: "Uniform current".into(),
            kind: BodyKind::Chart(chart),
            level: 0.0,
            gains: [0.0; 4],
            fetch: 100.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
        }],
        thrum_water_model::Profile {
            margin_height: 0.3,
            swash_ratio: 0.0,
        },
    )
    .expect("contained chart")
}

thrum_test_support::macos_gpu_test! {
fn query_current_carries_a_float_and_the_dry_bank_stops_it() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "query_current_carries_a_float_and_the_dry_bank_stops_it",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::paired(&test_gpu, [96, 64], false);
    let map = Arc::new(Mutex::new(channel()));
    let spectrum = Spectrum::build(&Wind::Calm.forcing()).expect("calm");
    let profile = thrum_water::Profile {
        query: Some(QueryLimits {
            points: 12,
            max_age: 2,
        }),
        ..thrum_water::Profile::default()
    };
    let water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("flow surface");
    let mailbox = water.query_mailbox().expect("query");
    fixture
        .host
        .set_scene_contribution(Box::new(water))
        .expect("flow contribution");
    let mut stream = Body::new(0, Hull::Float, [1.5, 2.0], 1.0, 0.0);
    let mut landing = Body::new(1, Hull::Float, [8.0, 2.0], 1.0, 0.0);
    let step = Duration::from_nanos(16_666_667);
    let mut drift = 0.0_f64;
    let mut dry = false;
    for tick in 0..=300_u32 {
        if tick != 0 {
            let answers = mailbox.drain(u64::from(tick), step);
            assert_eq!(answers.len(), 2);
            for (body, reply) in [&mut stream, &mut landing].into_iter().zip(&answers) {
                for point in &reply.points {
                    if point.surface.wet {
                        assert!((point.surface.current[0] - 3.0).abs() < 0.000_004);
                        assert_eq!(point.surface.current[1], 0.0);
                        assert_eq!(point.surface.body, Some(0));
                    }
                }
                body.advance(step.as_secs_f64(), |index, p| {
                    sample_answer(Some(reply), index, p).expect("query at predicted point")
                });
            }
            if stream.position.x < 8.0 {
                drift = drift.max(stream.velocity.x);
            }
            dry |= answers[1].points.iter().all(|point| !point.surface.wet);
        }
        mailbox.post(
            u64::from(tick),
            [&stream, &landing]
                .into_iter()
                .map(|body| body.request(u64::from(tick), step))
                .collect(),
        );
        fixture.frame(f64::from(tick) * step.as_secs_f64());
    }
    assert!(drift > 2.85, "approached {drift} m/s of the 3 m/s current");
    assert!(dry, "landed float never returned dry");
    assert!(
        landing.position.x > 9.5,
        "bank crossing {}",
        landing.position.x
    );
    assert!(
        landing.velocity.distance(DVec3::ZERO) < 0.08,
        "float did not settle: {:?}",
        landing.velocity
    );
    assert_eq!(mailbox.stats().blocking_reads, 0);
}
}
