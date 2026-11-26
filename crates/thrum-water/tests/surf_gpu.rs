//! Surf through the real Metal contribution and tick query.
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
    query::{BodyAnswers, BodyQuery, QueryLimits, QueryPoint},
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
    // The guard samples both bodies, including the sea behind the ordinary
    // query camera. Frame the whole test bed in the cache's actual draw view.
    host.source_mut().view = thrum_render::synthetic::SyntheticView::perspective(
        [0.0, 125.0, 0.0],
        [0.0; 3],
        [0.0, 0.0, -1.0],
        60_f32.to_radians(),
        1.5,
        0.1,
        500.0,
    )
    .expect("whole bed camera");
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
    assert!(
        host.drive(Duration::ZERO, Some(&output))
            .expect("seed water")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("seed completion");
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
            host.world().endpoint(tick),
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
        let mut replies = mailbox.drain(host.world().endpoint(use_tick), step);
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
    let samples: &[[f32; 12]] = bytemuck::cast_slice(&displacement);
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
            let half_width = match spacing {
                ..=0.5 => 64.0,
                1.0 => 80.0,
                2.0 => 48.0,
                8.0 => 40.0,
                _ => 32.0,
            };
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

#[test]
fn many_component_metal_cut_matches_the_model_with_grouped_or_single_stations() {
    use thrum_fixtures::surf as fixture;

    const FOLD_STATIONS: usize = 6;
    const STATION_COUNT: usize = 12;

    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "many-component highest surface cut",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let bed = Bed::new(
        fixture::ORIGIN,
        fixture::SPACING,
        fixture::SIZE.map(|n| n as usize),
        (0..fixture::SIZE[1])
            .flat_map(|_| (0..fixture::SIZE[0]).map(fixture::bed))
            .collect(),
    )
    .expect("beach");
    let mut map = Map::prepare(
        bed,
        vec![Body {
            name: "Sea".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1e6,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        thrum_water_model::Profile {
            margin_height: 1.5,
            swash_ratio: 0.7,
        },
    )
    .expect("sea on beach");
    map.enable_surf(SurfProfile::default()).expect("surf");
    // One sea and the shared 64-component spectrum select the wide query
    // pipelines, which sum each node's waves across eight lanes.
    let spectrum = Spectrum {
        long: (0..fixture::COMPONENTS)
            .map(|i| {
                let (wave_vector, amplitude, phase) = fixture::component(i);
                Component {
                    wave_vector,
                    amplitude,
                    phase,
                }
            })
            .collect(),
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm")),
        tail_slope_variance: 0.0,
    };
    let field = Field {
        layers: std::array::from_fn(|layer| {
            LongWaves::new(
                if layer == 0 {
                    spectrum.long.clone()
                } else {
                    vec![]
                },
                fixture::CHOP,
            )
            .expect("valid waves")
        }),
    };
    let step = Duration::from_secs_f64(1.0 / 60.0);
    let use_tick = u64::from(fixture::USE_TICK);
    let time = (step * fixture::USE_TICK).as_secs_f64();
    let sheet = map.surf_sheet(&field, 0, time).expect("model sheet");
    let mut columns = fixture::fold_columns(|p| sheet.forward(p).world, FOLD_STATIONS);
    assert_eq!(columns.len(), FOLD_STATIONS, "overturned triangles");
    columns.extend([-75.0, -62.0, -49.0, -36.0, -23.0, -10.0].map(|x| [x, 0.125]));
    assert_eq!(columns.len(), STATION_COUNT);
    let expected: Vec<_> = columns
        .iter()
        .map(|&p| {
            map.surface(&field, p, time)
                .expect("model query")
                .sample
                .height
        })
        .collect();
    let mut profile = Profile {
        query: Some(QueryLimits {
            points: u32::try_from(STATION_COUNT).expect("small fixture"),
            max_age: 3,
        }),
        ..Profile::default()
    };
    profile.waves.choppiness = fixture::CHOP;
    let water = Water::mapped(Arc::new(Mutex::new(map)), spectrum, profile).expect("water");
    let mailbox = water.query_mailbox().expect("query");
    let (mut host, target) = query_host(
        &device,
        &queue,
        &capabilities,
        Clock::new(true),
        QUERY_EXTENT,
    );
    host.set_scene_contribution(Box::new(water))
        .expect("contribution");
    let output = SurfaceOutput {
        view: &target,
        extent: QUERY_EXTENT,
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "shared coordinates are exactly representable"
    )]
    let points: Vec<_> = columns
        .iter()
        .map(|p| QueryPoint {
            position: p.map(|v| v as f32),
            radius: 0.0,
            use_tick,
        })
        .collect();
    assert!(
        host.drive(Duration::ZERO, Some(&output))
            .expect("seed water")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("seed completion");
    let mut grouped = Vec::new();
    let mut maximum_error = 0.0_f64;
    for (tick, stride) in [(1_u32, 6), (2, 1)] {
        let dispatches_before = mailbox.stats().dispatches;
        let bodies = points
            .chunks(stride)
            .enumerate()
            .map(|(id, points)| BodyQuery {
                id: u32::try_from(id).expect("small fixture"),
                generation: 1,
                points: points.to_vec(),
            })
            .collect();
        mailbox.post(host.world().endpoint(u64::from(tick)), bodies);
        let frame = host
            .drive(step * tick + step / 4, Some(&output))
            .expect("Metal submission");
        assert!(
            frame.submitted,
            "{frame:?}: {:?}",
            host.contribution_error()
        );
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("fixture completion");
        while host.poll_completed().expect("frame completion").is_some() {}
        let drain = mailbox.drain(host.world().endpoint(use_tick + u64::from(tick == 2)), step);
        assert_eq!(
            mailbox.stats().dispatches - dispatches_before,
            1,
            "tick {tick}"
        );
        assert_eq!(drain.len(), STATION_COUNT / stride);
        let [coarse, fine] = count_cut_nodes(&drain, u64::from(tick), stride);
        assert!(
            coarse > 0 && fine > 0,
            "tick {tick}: {coarse} coarse, {fine} fine"
        );
        eprintln!("Metal: tick {tick} evaluated {coarse} coarse and {fine} fine nodes");
        for (i, answer) in drain.into_iter().flat_map(|body| body.points).enumerate() {
            // The second batch was drained one tick after its use tick. Its long-layer
            // height is the raw dispatch value at the requested tick in both batches.
            assert_eq!(answer.point.position.map(f64::from), columns[i]);
            assert_eq!(answer.point.use_tick, use_tick);
            assert!(answer.supported[0], "tick {tick}, station {i}");
            let height = f64::from(answer.wave_heights[0]);
            let error = (height - expected[i]).abs();
            maximum_error = maximum_error.max(error);
            assert!(
                error < HEIGHT_TOLERANCE,
                "{i} {:?}: {height} vs {}",
                columns[i],
                expected[i]
            );
            if stride == 6 {
                grouped.push(height);
            } else {
                assert!(
                    (height - grouped[i]).abs() < HEIGHT_TOLERANCE,
                    "station {i}: single {height}, grouped {}",
                    grouped[i]
                );
            }
        }
    }
    eprintln!(
        "Metal: {STATION_COUNT} stations, {FOLD_STATIONS} overturned triangles, maximum error {maximum_error:.6} m"
    );
}

fn count_cut_nodes(replies: &[BodyAnswers], tick: u64, stations_per_body: usize) -> [u64; 2] {
    replies.iter().fold([0, 0], |[coarse, fine], reply| {
        assert_eq!(reply.posted, tick);
        assert!(!reply.held);
        assert_eq!(reply.points.len(), stations_per_body);
        [coarse + reply.cut_nodes[0], fine + reply.cut_nodes[1]]
    })
}

mod cache_first_frame {
    use super::{
        Arc, Bed, Body, BodyKind, Clock, Component, Field, HEIGHT_TOLERANCE, LongWaves, Map, Mutex,
        Profile, QUERY_EXTENT, Sheet, Spectrum, Stage, SurfProfile, SurfaceOutput, TAU, Water,
        query_host, spectrum,
    };
    use glam::{Mat4, Vec4};
    use thrum_render::{runtime::FrameWorld, synthetic::SyntheticView};
    use thrum_test_support::readback;
    use thrum_water::{SurfTap, SurfWitness};

    fn map() -> Map {
        let size = [20, 46];
        let heights = (0..size[1])
            .flat_map(|_| {
                (0..size[0])
                    .map(|i| (f64::from(u32::try_from(i).expect("small bed")) * 2.0 - 30.0) * 0.12)
            })
            .collect();
        let mut map = Map::prepare(
            Bed::new([-30.0, -44.0], 2.0, size, heights).expect("plane beach"),
            vec![Body {
                name: "Sea".into(),
                kind: BodyKind::Sea,
                level: 0.0,
                stage: Stage {
                    min: 0.0,
                    max: 0.5,
                    value: 0.0,
                },
                fetch: 100_000.0,
                gains: [1.0, 0.0, 0.0, 0.0],
            }],
            thrum_water_model::Profile {
                margin_height: 0.85,
                swash_ratio: 0.7,
            },
        )
        .expect("map");
        map.enable_surf(SurfProfile::default()).expect("transform");
        map
    }

    fn forcing(gain: f64) -> (Spectrum, Field) {
        let mut spectrum = spectrum();
        spectrum.long = (0..16)
            .map(|i| {
                let i = f64::from(i);
                let (z, x) = (0.17 * i - 1.12).sin_cos();
                let k = TAU / (26.0 + 2.5 * i);
                Component {
                    wave_vector: [x * k, z * k],
                    amplitude: gain * (0.055 + 0.001 * i),
                    phase: 0.37 * i,
                }
            })
            .collect();
        let field = Field {
            layers: std::array::from_fn(|i| {
                LongWaves::new(
                    if i == 0 {
                        spectrum.long.clone()
                    } else {
                        Vec::new()
                    },
                    0.2,
                )
                .expect("model waves")
            }),
        };
        (spectrum, field)
    }

    fn contribution(map: &Arc<Mutex<Map>>, spectrum: Spectrum) -> (Water, SurfTap) {
        let mut profile = Profile::default();
        profile.waves.choppiness = 0.2;
        let mut water = Water::mapped(Arc::clone(map), spectrum, profile).expect("water");
        let tap = water.tap_surf();
        (water, tap)
    }

    fn view(z: f32, z_direction: f32) -> SyntheticView {
        SyntheticView::perspective(
            [-9.0, 16.0, z],
            [-9.0, 0.0, z + 18.0 * z_direction],
            [0.0, 1.0, 0.0],
            72_f32.to_radians(),
            1.5,
            0.1,
            500.0,
        )
        .expect("camera")
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "project fixture metres with the draw's f32 view"
    )]
    fn visible_rest(view: Mat4, point: [f64; 2], stage: f64) -> bool {
        let clipped = view * Vec4::new(point[0] as f32, stage as f32, point[1] as f32, 1.0);
        // Well inside the view, with room for the wave displacement and one
        // triangle on either side. The cache is not the visibility oracle.
        clipped.w > 0.0 && clipped.x.abs().max(clipped.y.abs()) < clipped.w * 0.8
    }

    fn model_displacement(sheet: &impl Sheet, rest: [f64; 2]) -> [f64; 3] {
        let sample = sheet.forward(rest);
        [
            sample.world[0] - rest[0],
            sample.height,
            sample.world[1] - rest[1],
        ]
    }

    fn compare(actual: &[f32], sheet: &impl Sheet, a: [f64; 2], b: [f64; 2], fraction: f64) -> f64 {
        let first = model_displacement(sheet, a);
        let second = if fraction == 0.0 {
            first
        } else {
            model_displacement(sheet, b)
        };
        actual
            .iter()
            .zip(first.into_iter().zip(second))
            .map(|(actual, (first, second))| {
                let expected = (second - first).mul_add(fraction, first);
                let error = (f64::from(*actual) - expected).abs();
                assert!(
                    error < HEIGHT_TOLERANCE,
                    "{a:?} to {b:?} ({fraction}): {actual} vs {expected}"
                );
                error
            })
            .fold(0.0, f64::max)
    }

    #[derive(Clone, Copy)]
    struct CheckFrame<'a> {
        name: &'a str,
        map: &'a Map,
        old_map: &'a Map,
        field: &'a Field,
        original_field: &'a Field,
        now: f64,
        before: Option<f64>,
        camera: &'a SyntheticView,
        prior_samples: Option<&'a [[f32; 12]]>,
    }

    #[expect(
        clippy::float_cmp,
        clippy::print_stderr,
        reason = "exact dyadic mesh spacing and ring edge; record the model error on the target"
    )]
    fn compare_frame(nodes: &[[f32; 4]], samples: &[[f32; 12]], frame: CheckFrame<'_>) {
        let CheckFrame {
            name,
            map,
            old_map,
            field,
            original_field,
            now,
            before,
            camera,
            prior_samples,
        } = frame;
        let view = Mat4::from_cols_array_2d(&camera.view_proj_columns());
        let [x, _, z] = camera.camera_position().map(f32::floor);
        let anchor = [f64::from(x), f64::from(z)];
        let stage = map.body(0).expect("sea").stage.value;
        let current = map.surf_sheet(field, 0, now).expect("current sheet");
        let previous =
            before.map(|time| old_map.surf_sheet(field, 0, time).expect("previous sheet"));
        let stale = match name {
            "stage" => Some(
                map.surf_sheet(field, 0, before.expect("stage predecessor"))
                    .expect("wrong stage control"),
            ),
            "wind" => Some(
                map.surf_sheet(original_field, 0, now)
                    .expect("stale wind control"),
            ),
            _ => None,
        };
        let mut interior = 0;
        let mut edge = 0;
        let mut newly_visible = 0;
        let mut strata = [0; 3];
        let mut maximum = 0.0_f64;
        let mut stale_error = 0.0_f64;
        for (i, (&[x, z, spacing, slot], sample)) in nodes.iter().zip(samples).enumerate() {
            if slot != 0.0 || spacing != 0.25 {
                continue;
            }
            let rest = [anchor[0] + f64::from(x), anchor[1] + f64::from(z)];
            if !visible_rest(view, rest, stage) {
                continue;
            }
            let depth = stage - map.bed().height(rest).expect("camera over the bed");
            let stratum = if depth > 2.0 {
                0
            } else if depth > 0.4 {
                1
            } else {
                2
            };
            if depth < -0.25 {
                continue;
            }
            let boundary = x.abs().max(z.abs()) == 16.0;
            let coarse = [f64::from(x) * 2.0, f64::from(z) * 2.0];
            let fraction = coarse.map(|p| p - p.floor());
            let fraction = fraction[0].max(fraction[1]);
            let (a, b, fraction) = if boundary {
                if fraction == 0.0 {
                    continue;
                }
                (
                    std::array::from_fn(|axis| anchor[axis] + coarse[axis].floor() * 0.5),
                    std::array::from_fn(|axis| anchor[axis] + coarse[axis].ceil() * 0.5),
                    fraction,
                )
            } else {
                if strata[stratum] >= 512 {
                    continue;
                }
                (rest, rest, 0.0)
            };
            assert!(sample[3] >= 1.0, "{name} first submission omitted {rest:?}");
            maximum = maximum.max(compare(&sample[..3], &current, a, b, fraction));
            if let Some(previous) = &previous {
                maximum = maximum.max(compare(&sample[4..7], previous, a, b, fraction));
            }
            if stale_error <= HEIGHT_TOLERANCE * 2.0
                && !boundary
                && let Some(stale) = &stale
            {
                let height = sample[if name == "stage" { 5 } else { 1 }];
                stale_error =
                    stale_error.max((f64::from(height) - model_displacement(stale, rest)[1]).abs());
            }
            if boundary {
                edge += 1;
            } else {
                strata[stratum] += 1;
                interior += 1;
            }
            if prior_samples.is_some_and(|earlier| earlier[i][3] == 0.0) {
                newly_visible += 1;
            }
        }
        eprintln!(
            "{name}: {interior} interior {strata:?}, {edge} blended ring edge, {newly_visible} previously uncached, {maximum:.7} m maximum error"
        );
        assert!(interior >= 1024 && edge >= 32 && strata.iter().all(|count| *count > 0));
        if prior_samples.is_some() {
            assert!(newly_visible >= 32);
        }
        if stale.is_some() {
            assert!(
                stale_error > HEIGHT_TOLERANCE * 2.0,
                "{name} stale control differs by only {stale_error} m"
            );
        }
    }

    fn read_samples(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        witness: &SurfWitness,
        count: u64,
    ) -> Vec<[f32; 12]> {
        let bytes = readback::buffer(device, queue, &witness.samples, ..count * 48);
        bytemuck::cast_slice(&bytes).to_vec()
    }

    #[test]
    fn cache_first_frame_after_turn_move_stage_and_wind() {
        let Some(thrum_test_support::Opened {
            device,
            queue,
            capabilities,
            ..
        }) = thrum_test_support::open(
            "surf first submitted cache",
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
            Clock::new(false),
            QUERY_EXTENT,
        );
        host.set_timing_requested(false);
        let output = SurfaceOutput {
            view: &target,
            extent: QUERY_EXTENT,
        };
        let map = Arc::new(Mutex::new(map()));
        let (spectrum, field) = forcing(1.0);
        let (water, mut tap) = contribution(&map, spectrum);
        let (_, changed_field) = forcing(1.67);
        host.set_scene_contribution(Box::new(water)).expect("water");
        let step = host.world().tick_duration();
        let mut previous_map = map.lock().expect("test map").clone();
        let mut prior_samples = Vec::new();
        let mut nodes = Vec::new();
        for (tick, (name, camera)) in [
            ("seed", view(-25.0, -1.0)),
            ("turn", view(-25.0, 1.0)),
            ("move", view(25.0, 1.0)),
            ("stage", view(25.0, 1.0)),
            ("wind", view(25.0, 1.0)),
        ]
        .into_iter()
        .enumerate()
        {
            if name == "stage" {
                map.lock()
                    .expect("test map")
                    .set_stage(0, 0.25)
                    .expect("stage");
            }
            if name == "wind" {
                let (water, new_tap) = contribution(&map, forcing(1.67).0);
                host.set_scene_contribution(Box::new(water))
                    .expect("changed wind");
                tap = new_tap;
            }
            host.continuous_source_mut().view = camera;
            let elapsed = step * u32::try_from(tick).expect("short test");
            assert!(host.drive(elapsed, Some(&output)).expect("frame").submitted);
            assert!(
                host.contribution_error().is_none(),
                "{:?}",
                host.contribution_error()
            );
            let witness = tap.witness().expect("first submitted field cache");
            if nodes.is_empty() {
                nodes =
                    bytemuck::cast_slice(&readback::buffer(&device, &queue, &witness.nodes, ..))
                        .to_vec();
            }
            let samples = read_samples(&device, &queue, &witness, nodes.len() as u64);
            let current_map = map.lock().expect("test map");
            if tick > 0 {
                compare_frame(
                    &nodes,
                    &samples,
                    CheckFrame {
                        name,
                        camera: &camera,
                        map: &current_map,
                        old_map: &previous_map,
                        field: if name == "wind" {
                            &changed_field
                        } else {
                            &field
                        },
                        original_field: &field,
                        now: elapsed.as_secs_f64(),
                        before: (name != "wind")
                            .then_some(elapsed.saturating_sub(step).as_secs_f64()),
                        prior_samples: (name == "turn").then_some(&prior_samples[..]),
                    },
                );
            }
            previous_map = current_map.clone();
            prior_samples = samples;
            thrum_test_support::wait::until(
                "submitted test frame",
                std::time::Duration::from_secs(4),
                || host.poll_completed().expect("completion"),
            );
        }
    }
}
