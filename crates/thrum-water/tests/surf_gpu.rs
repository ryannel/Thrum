//! Surf through the real Metal contribution and tick query.
mod frame_fixture;

use std::f64::consts::TAU;

use crate::frame_fixture::{Clock, QUERY_EXTENT, query_host};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_render::runtime::SurfaceOutput;
use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
use thrum_water::{
    Profile, Water,
    query::{BodyQuery, QueryLimits, QueryPoint},
};
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Sheet, Stage, SurfProfile};

const HEIGHT_TOLERANCE: f64 = 0.015;

fn fixture() -> Map {
    let size = [81, 39];
    let heights = (0..size[1])
        .flat_map(|z| {
            (0..size[0]).map(move |x| {
                let p = x as f64 * 2.0 - 80.0;
                let q = z as f64 * 2.0 - 38.0;
                (0.1_f64 * (p + 14.0))
                    .clamp(-6.0, 3.0)
                    .min(1.0 + 0.12 * (p - 53.0).hypot(q))
            })
        })
        .collect();
    let sea = Body {
        name: "Sea".into(),
        kind: BodyKind::Sea,
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.5,
            value: 0.18,
        },
        fetch: 100_000.0,
        gains: [1.0, 0.0, 0.0, 0.0],
    };
    let lake = Body {
        name: "Lake".into(),
        kind: BodyKind::Basin { seed: [53.0, 0.0] },
        level: 2.5,
        stage: Stage {
            min: 0.0,
            max: 0.32,
            value: 0.16,
        },
        fetch: 26.0,
        gains: [0.7, 0.0, 0.0, 0.0],
    };
    let bed = Bed::new([-80.0, -38.0], 2.0, size, heights).expect("bed");
    Map::prepare(
        bed,
        vec![sea, lake],
        thrum_water_model::Profile {
            margin_height: 0.45,
            swash_ratio: 0.7,
        },
    )
    .expect("map")
}

fn spectrum() -> Spectrum {
    let wave = |length, angle: f64, amplitude, phase| Component {
        wave_vector: [angle.cos() * TAU / length, angle.sin() * TAU / length],
        amplitude,
        phase,
    };
    Spectrum {
        long: vec![wave(38.0, 0.78, 0.21, 0.4), wave(58.0, -0.3, 0.11, 1.7)],
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm")),
        tail_slope_variance: 0.0,
    }
}

#[test]
fn surf_tick_and_repeat_queries_match_the_model() {
    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "surf tick and repeat queries",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let (mut host, target) = query_host(
        &device,
        &queue,
        &capabilities,
        Clock::new(true),
        QUERY_EXTENT,
    );
    let output = SurfaceOutput {
        view: &target,
        extent: QUERY_EXTENT,
    };
    let plain = fixture();
    let mut map = plain.clone();
    map.enable_surf(SurfProfile::default())
        .expect("shore transform");
    let map = Arc::new(Mutex::new(map));
    let spectrum = spectrum();
    let field = Field {
        layers: std::array::from_fn(|i| {
            LongWaves::new(
                if i == 0 {
                    spectrum.long.clone()
                } else {
                    vec![]
                },
                0.2,
            )
            .expect("waves")
        }),
    };
    let mut profile = Profile {
        query: Some(QueryLimits {
            points: 2048,
            max_age: 3,
        }),
        ..Profile::default()
    };
    profile.waves.choppiness = 0.2;
    let mut water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("water");
    let tap = water.tap_surf();
    let mailbox = water.query_mailbox().expect("query");
    host.set_scene_contribution(Box::new(water))
        .expect("contribution");
    let step = Duration::from_secs_f64(1.0 / 60.0);
    let mut largest_error = 0.0_f64;
    let mut largest_plain = 0.0_f64;
    let mut counted = [0; 2];
    let mut cache_points = 0;
    for (tick, elapsed) in [(1, step + step / 4), (2, step * 2 + step / 2)] {
        if tick == 2 {
            assert!(
                host.drive(step * 2, Some(&output))
                    .expect("next tick")
                    .submitted
            );
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("complete the tick frame before the repeat");
            while host.poll_completed().expect("tick completion").is_some() {}
        }
        let use_tick = tick + 1;
        let row_offset = if tick == 2 { 0.17 } else { 0.0 };
        let points: Vec<_> = (0_u16..2048)
            .map(|i| {
                let z = f32::from(i / 64) * 0.60 - 9.0 + row_offset;
                let u = f32::from(i % 64) / 63.0;
                let x = if i % 2 == 0 {
                    -76.0 + 65.0 * u
                } else {
                    42.0 + 21.0 * u
                };
                QueryPoint {
                    position: [x, z],
                    radius: 0.0,
                    use_tick,
                }
            })
            .collect();
        mailbox.post(
            tick,
            vec![BodyQuery {
                id: 8,
                generation: 1,
                points,
            }],
        );
        let frame = host
            .drive(elapsed, Some(&output))
            .expect("Metal submission");
        assert!(
            frame.submitted,
            "{frame:?}: {:?}",
            host.contribution_error()
        );
        assert_eq!(frame.completed_ticks, u32::from(tick == 1));
        let witness = tap.witness().expect("submitted surf cache");
        let clock = host.last_submitted_clock().expect("submitted clock");
        let frame_seconds =
            (clock.tick as f64 - 1.0 + f64::from(clock.presentation_alpha)) * step.as_secs_f64();
        cache_points += measure_cache(
            &device,
            &queue,
            &witness,
            &map.lock().expect("map"),
            &field,
            frame_seconds,
        );
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("completion");
        let mut replies = mailbox.drain(use_tick, step);
        let replies = replies.pop().expect("query reply");
        assert_eq!(replies.points.len(), 2048);
        for got in replies.points {
            let point = got.point.position.map(f64::from);
            let time = use_tick as f64 * step.as_secs_f64();
            let expected = map
                .lock()
                .expect("map")
                .surface(&field, point, time)
                .expect("model");
            let reference = plain.surface(&field, point, time).expect("without surf");
            assert_eq!(got.surface.body, expected.body, "{point:?}");
            let error = (got.surface.sample.height - expected.sample.height).abs();
            assert!(
                error < HEIGHT_TOLERANCE,
                "{point:?}: {error} m, got {:?}, expected {expected:?}",
                got.surface
            );
            assert_eq!(got.surface.wet, expected.wet, "{point:?}");
            if let Some(body) = expected.body {
                counted[body] += 1;
                largest_error = largest_error.max(error);
                largest_plain =
                    largest_plain.max((expected.sample.height - reference.sample.height).abs());
            }
        }
    }
    assert_eq!(cache_points, 4096);
    assert!(counted.iter().all(|&n| n >= 512), "{counted:?}");
    assert!(
        largest_plain > HEIGHT_TOLERANCE * 2.0,
        "untransformed control differs by only {largest_plain}"
    );
    eprintln!(
        "4096 query points on Metal; maximum height error {largest_error:.6} m. Untransformed control {largest_plain:.6} m; ownership {counted:?}."
    );
}

fn measure_cache(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    witness: &thrum_water::SurfWitness,
    map: &Map,
    field: &Field,
    time: f64,
) -> usize {
    use thrum_test_support::readback;
    let entries = readback::buffer(device, queue, &witness.nodes, ..);
    let displacement = readback::buffer(device, queue, &witness.samples, ..);
    let nodes: &[[f32; 4]] = bytemuck::cast_slice(&entries);
    let samples: &[[f32; 8]] = bytemuck::cast_slice(&displacement);
    let sheets = [0, 1].map(|body| map.surf_sheet(field, body, time).expect("model"));
    let [ox, oz] = witness.anchor.map(f32::floor);
    let mut counts = [0; 2];
    let mut control_differences = [0.0_f64; 2];
    for (&[x, z, spacing, slot], actual) in nodes.iter().zip(samples) {
        let body = usize::from(slot != 0.0);
        if counts[body] == 1024 || actual[3] == 0.0 {
            continue;
        }
        let point = if body == 0 {
            let half_width = if spacing <= 0.5 { 64.0 } else { 32.0 };
            if (x.abs().max(z.abs()) / spacing) >= half_width {
                continue;
            }
            [f64::from(x + ox), f64::from(z + oz)]
        } else {
            [f64::from(x), f64::from(z)]
        };
        let Some(row) = map.sample(point).filter(|row| row.body == body) else {
            continue;
        };
        let expected = sheets[body].forward(point);
        for (got, target) in actual[..3].iter().zip([
            expected.world[0] - point[0],
            expected.height,
            expected.world[1] - point[1],
        ]) {
            assert!(
                (f64::from(*got) - target).abs() < HEIGHT_TOLERANCE,
                "cache node {point:?} in {body}: {got:.5} m vs {target:.5} m"
            );
        }
        if body == 0
            && control_differences
                .iter()
                .any(|difference| *difference <= HEIGHT_TOLERANCE * 2.0)
            && let Some(controls) = control_heights(map, field, point, time, row)
        {
            for (difference, omitted) in control_differences.iter_mut().zip(controls) {
                *difference = difference.max((f64::from(actual[1]) - omitted).abs());
            }
        }
        counts[body] += 1;
    }
    assert_eq!(counts, [1024; 2]);
    for (name, difference) in ["without lag", "without section"]
        .into_iter()
        .zip(control_differences)
    {
        assert!(
            difference > HEIGHT_TOLERANCE * 2.0,
            "{name} differs by only {difference} m"
        );
    }
    counts.iter().sum()
}

// Both controls keep shoaling, exposure and the depth cap. The first removes
// phase lag alone. The second keeps the transformed cosine and omits the
// section. Compare on wet rest nodes below the swash fade.
fn control_heights(
    map: &Map,
    field: &Field,
    point: [f64; 2],
    time: f64,
    row: thrum_water_model::MapSample,
) -> Option<[f64; 2]> {
    let surf = map.surf_profile().expect("surf");
    let transform = map.shore_transform(0).expect("sea transform");
    let stage = map.body(0).expect("sea").stage.value;
    let depth = row.depth.max(surf.lag_floor);
    let limit = thrum_water_model::limit(row.row, row.depth, field.variances(), map.profile());
    if row.depth <= 0.5 * limit.swash_depth {
        return None;
    }
    let [mut re, mut im, mut transformed, mut variance] = [0.0_f64; 4];
    for wave in field.layers[0].components() {
        let [kx, kz] = wave.wave_vector;
        let k = kx.hypot(kz);
        let sector = thrum_water_model::shore_sector(wave.wave_vector) as usize;
        let (lag, _) = transform.sample(sector, TAU / k, stage, point);
        if !lag.is_finite() {
            return None;
        }
        let gain =
            thrum_water_model::shoaling(k, depth) * transform.exposure(sector, stage, point).sqrt();
        let a = wave.amplitude * row.row.gains[0] * gain;
        let phase = wave.anchored_phase(point, time);
        let (sin, cos) = phase.sin_cos();
        transformed += a * (phase + k * lag).cos();
        re += a * cos;
        im += a * sin;
        variance += 0.5 * a * a;
    }
    let envelope = re.hypot(im);
    let index = 2.0 * envelope / (surf.breaker_ratio * depth);
    let section = surf.section.sample(im.atan2(re), index);
    let scale = (0.6 * (row.depth + limit.swash_depth) / (4.0 * variance.sqrt())).min(1.0);
    Some([section.height * envelope * scale, transformed * scale])
}
