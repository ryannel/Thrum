//! The same game-owned forces on a compact lake and in a prepared chart.

#[cfg(target_os = "macos")]
use crate::support;
use glam::DVec3;
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(target_os = "macos")]
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use thrum_sea_model::{Cascade, LongWaves, Spectrum};
#[cfg(target_os = "macos")]
use thrum_water::{
    Water,
    query::{QueryLimits, QueryMailbox},
};
use thrum_water_model::{Bed, BodyKind, Chart, Field, Map, Stage, Surface};
#[cfg(target_os = "macos")]
use water_scene::floating::sample_answer;
use water_scene::{
    floating::{Body, Hull, fallback},
    island::Wind,
};

const STEP: Duration = Duration::from_nanos(16_666_667);
const LAKE_LEVEL: f64 = 12.0;

fn lake(wind: Wind) -> Map {
    let heights = (-24..=24)
        .flat_map(|z| {
            (-24..=24).map(move |x| LAKE_LEVEL - 4.0 + 0.17 * f64::from(x).hypot(f64::from(z)))
        })
        .collect();
    let bed = Bed::new([-48.0; 2], 2.0, [49, 49], heights).expect("lake bed");
    let mut map = Map::prepare(
        bed,
        vec![thrum_water_model::Body {
            name: "Lake".into(),
            kind: BodyKind::Basin { seed: [0.0; 2] },
            level: LAKE_LEVEL,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1_000.0,
            gains: [0.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 0.3,
            swash_ratio: 0.7,
        },
    )
    .expect("contained lake");
    map.set_forcing(&wind.forcing()).expect("sheltered gains");
    map
}

fn sample(map: &Map, field: &Field, position: DVec3, time: f64) -> (Surface, Option<f64>) {
    let point = [position.x, position.z];
    (
        map.surface(field, point, time).expect("water surface"),
        map.bed().height(point),
    )
}

#[cfg(target_os = "macos")]
fn query_scene(
    gpu: &thrum_test_support::Gpu,
    map: Map,
    spectrum: Spectrum,
) -> (support::Fixture, QueryMailbox) {
    let mut profile = water_scene::profile(&spectrum);
    profile.query = Some(QueryLimits {
        points: 12,
        max_age: 2,
    });
    let contribution =
        Water::mapped(Arc::new(Mutex::new(map)), spectrum, profile).expect("mapped contribution");
    let mailbox = contribution.query_mailbox().expect("query mailbox");
    let report = contribution.report();
    let game = SkyGame::new(
        Controls::new(),
        Camera::look_at([-4.0, 16.0, 6.0], [3.0, 10.0, 2.0], 60.0),
        |_, _| {},
    )
    .with_tick_duration(STEP)
    .expect("supported tick")
    .with_contribution(Box::new(contribution), || ContentStatus::Drawn(1));
    (
        support::Fixture::from_game(gpu, [48, 32], game, report),
        mailbox,
    )
}

fn spectrum(wind: Wind) -> Spectrum {
    let mut spectrum = Spectrum::build(&wind.forcing()).expect("forcing");
    // One travelling mode keeps each short band at its original power so both the analytic
    // force test and the submitted query see the fetch-limited waves that move the float.
    spectrum.cascades = std::array::from_fn(|i| {
        let band = &spectrum.cascades[i];
        let mode = [10, 10, 8][i];
        Cascade::new(band.length(), &[(mode, 0, band.variance(), 0.4)])
            .expect("representative cascade")
    });
    spectrum
}

fn lake_field(spectrum: &Spectrum) -> Field {
    let [outer, middle, inner] = spectrum
        .cascades
        .each_ref()
        .map(|band| LongWaves::new(band.components().to_vec(), 0.0).expect("fixed cascade column"));
    let long = LongWaves::new(
        spectrum.long.clone(),
        water_scene::profile(spectrum).waves.choppiness,
    )
    .expect("contractive long field");
    Field {
        layers: [long, outer, middle, inner],
    }
}

#[test]
fn compact_lake_float_settles_and_remains_sheltered() {
    for wind in [Wind::Calm, Wind::Gale] {
        let map = lake(wind);
        let spectrum = spectrum(wind);
        let field = lake_field(&spectrum);
        let mut float = Body::new(1, Hull::Float, [0.0; 2], LAKE_LEVEL, 0.0);
        let mut largest = 0.0_f64;
        for tick in 0..=112 {
            let time = f64::from(tick) * STEP.as_secs_f64();
            float.advance(STEP.as_secs_f64(), |_, p| sample(&map, &field, p, time));
            if tick >= 60 {
                largest = largest.max((float.position.y + float.draft() - LAKE_LEVEL).abs());
            }
        }
        if matches!(wind, Wind::Calm) {
            assert!(largest < 0.001, "calm lake draft deviated by {largest} m");
        } else {
            assert!((0.003..0.30).contains(&largest), "lake heave {largest} m");
        }
        eprintln!("{wind:?} maximum settled float heave: {largest:.6} m");
    }
}

thrum_test_support::macos_gpu_test! {
fn lake_remains_sheltered_in_the_gale_and_settles_in_calm() {
    let Some(gpu) = thrum_test_support::gpu("sheltered lake query", Some(wgpu::Backend::Metal)) else {
        return;
    };
    for wind in [Wind::Calm, Wind::Gale] {
        let map = lake(wind);
        let spectrum = spectrum(wind);
        let field = lake_field(&spectrum);
        let mut float = Body::new(1, Hull::Float, [0.0; 2], LAKE_LEVEL, 0.0);
        let (mut fixture, mailbox) = query_scene(&gpu, map.clone(), spectrum);
        mailbox.post(fixture.endpoint(0), vec![float.request(0, STEP)]);
        fixture.frame_without_timings(0.0);
        fixture.frame_without_timings(STEP.as_secs_f64());
        let answers = mailbox.drain(fixture.endpoint(2), STEP);
        assert_eq!(answers.len(), 1, "{wind:?}");
        assert_eq!(mailbox.stats().age_two, 1);
        for answer in &answers[0].points {
            assert_eq!(answer.surface.body, Some(0));
            assert!(answer.surface.wet);
            let point = answer.point.position.map(f64::from);
            let expected = map.surface(&field, point, 2.0 * STEP.as_secs_f64()).expect("lake oracle");
            assert!((answer.surface.sample.height - expected.sample.height).abs() < 0.004);
        }
        float.advance(STEP.as_secs_f64(), |index, point| {
            sample_answer(answers.first(), index, point).expect("query used by the float")
        });
        assert_eq!(mailbox.stats().blocking_reads, 0);
    }
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

#[test]
fn chart_current_carries_a_float_to_rest_on_its_dry_bank() {
    let map = channel();
    let field = fallback(&spectrum(Wind::Calm), 0.0).expect("calm field");
    let mut stream = Body::new(0, Hull::Float, [1.5, 2.0], 1.0, 0.0);
    let mut landing = Body::new(1, Hull::Float, [8.0, 2.0], 1.0, 0.0);
    let mut drift = 0.0_f64;
    let mut dry = false;
    for tick in 1..=300 {
        let time = f64::from(tick) * STEP.as_secs_f64();
        stream.advance(STEP.as_secs_f64(), |_, p| sample(&map, &field, p, time));
        let mut all_dry = true;
        landing.advance(STEP.as_secs_f64(), |_, p| {
            let answer = sample(&map, &field, p, time);
            all_dry &= !answer.0.wet;
            answer
        });
        dry |= all_dry;
        if stream.position.x < 8.0 {
            drift = drift.max(stream.velocity.x);
        }
    }
    assert!(drift > 2.85, "approached {drift} m/s of the 3 m/s current");
    assert!(dry, "landed float never reached dry ground");
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
}

thrum_test_support::macos_gpu_test! {
fn query_current_carries_a_float_and_the_dry_bank_stops_it() {
    let Some(gpu) = thrum_test_support::gpu("chart float queries", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (mut fixture, mailbox) = query_scene(&gpu, channel(), spectrum(Wind::Calm));
    let mut stream = Body::new(0, Hull::Float, [1.5, 2.0], 1.0, 0.0);
    let mut landing = Body::new(1, Hull::Float, [10.75, 2.0], 1.0, 0.0);
    landing.velocity.x = 1.0;
    mailbox.post(fixture.endpoint(0), vec![stream.request(0, STEP), landing.request(0, STEP)]);
    fixture.frame_without_timings(0.0);
    fixture.frame_without_timings(STEP.as_secs_f64());
    let answers = mailbox.drain(fixture.endpoint(2), STEP);
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].id, 0);
    assert_eq!(answers[1].id, 1);
    for point in &answers[0].points {
        assert!(point.surface.wet);
        assert!((point.surface.current[0] - 3.0).abs() < 0.000_004);
        assert_eq!(point.surface.current[1], 0.0);
        assert_eq!(point.surface.body, Some(0));
    }
    for point in &answers[1].points {
        assert!(!point.surface.wet);
        assert!((f64::from(point.bed.expect("dry bank bed")) - 1.1).abs() < 0.000_004);
    }
    for (body, reply) in [&mut stream, &mut landing].into_iter().zip(&answers) {
        body.advance(STEP.as_secs_f64(), |index, p| {
            sample_answer(Some(reply), index, p).expect("query at predicted point")
        });
    }
    assert!(stream.velocity.x > 0.0, "GPU current must drive the float");
    assert!(landing.velocity.x < 1.0, "GPU bed must slow the float");
    assert_eq!(mailbox.stats().age_two, 2);
    assert_eq!(mailbox.stats().blocking_reads, 0);
}
}
