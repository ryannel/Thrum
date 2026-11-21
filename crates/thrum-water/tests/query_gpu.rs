//! Mac Metal proof of the public query mailbox through the renderer.
mod swash_gpu;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use swash_gpu::{Clock, Source};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost, SurfaceOutput},
    synthetic::{FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
};
use thrum_sea_model::{Cascade, Forcing, LongWaves, Spectrum};
use thrum_water::{
    Profile, Water,
    query::{BodyQuery, QueryLimits, QueryPoint},
};

const EXTENT: [u32; 2] = [96, 64];

fn metal(test: &str) -> Option<(wgpu::Device, wgpu::Queue, DeviceCapabilities)> {
    let thrum_test_support::Opened {
        gpu, device, queue, ..
    } = thrum_test_support::open(
        test,
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )?;
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    Some((device, queue, caps))
}

fn query_host(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    clock: Clock,
) -> (FrameHost<Clock, Source>, wgpu::TextureView) {
    let target = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("query scene output"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let scene = SyntheticScene::grid(1).expect("fixture");
    let view = SyntheticView::perspective(
        [-8.0, 4.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        55_f32.to_radians(),
        1.5,
        0.1,
        500.0,
    )
    .expect("camera");
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.3, 0.93, 0.2], 100_000.0, [1.0; 3], 55.0, 1).expect("noon"),
    );
    let host = pollster::block_on(FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(SurfaceProfile {
                format: OUTPUT_FORMAT,
                extent: EXTENT,
            }),
        },
        clock,
        Source { view, lighting },
        scene,
        1.0,
    ))
    .expect("frame host");
    (host, target)
}

#[test]
fn mailbox_samples_the_gale_at_its_use_tick() {
    let Some((device, queue, caps)) = metal("mailbox_samples_the_gale_at_its_use_tick") else {
        return;
    };
    let (mut host, target) = query_host(&device, &queue, &caps, Clock::new(false));
    let surface = SurfaceOutput {
        view: &target,
        extent: EXTENT,
    };
    let map = Arc::new(Mutex::new(swash_gpu::beach(0.7, 3.0)));
    let mut spectrum = Spectrum::build(&Forcing {
        wind_speed: 25.0,
        wind_direction: 0.0,
        fetch: 100_000.0,
        swells: Vec::new(),
    })
    .expect("gale");
    // Sparse bands allow an independent direct sum of filtered texels.
    spectrum.cascades = [160.0, 37.0, 9.0].map(|length| {
        Cascade::new(length, &[(11, 7, 0.0001, 0.8), (-21, 13, 0.000_065, 1.9)]).expect("band")
    });
    let oracle = LongWaves::new(spectrum.long.clone(), 0.2).expect("long model");
    let mut profile = Profile {
        query: Some(QueryLimits {
            points: 2050,
            max_age: 3,
        }),
        ..Profile::default()
    };
    profile.waves.choppiness = 0.2;
    let water = Water::mapped(Arc::clone(&map), spectrum.clone(), profile).expect("water");
    let mailbox = water.query_mailbox().expect("query enabled");
    host.set_scene_contribution(Box::new(water))
        .expect("query contribution");
    let step = Duration::from_secs_f64(1.0 / 60.0);
    let n = 2048_u16;
    let points: Vec<_> = (0..n)
        .map(|i| QueryPoint {
            position: [-52.0 - f32::from(i % 64) * 0.37, f32::from(i / 64) * 0.53],
            radius: [0.0, 0.5, 1.0, 2.0][usize::from(i % 4)],
            use_tick: 2,
        })
        .collect();
    let dry = QueryPoint {
        position: [45.0, 0.0],
        radius: 0.0,
        use_tick: 1,
    };
    let swash = QueryPoint {
        position: [15.0, 0.0],
        radius: 0.0,
        use_tick: 2,
    };
    mailbox.post(
        1,
        vec![
            BodyQuery {
                id: 9,
                generation: 1,
                points: vec![dry; 2],
            },
            BodyQuery {
                id: 7,
                generation: 1,
                points: vec![dry],
            },
            BodyQuery {
                id: 5,
                generation: 1,
                points: vec![swash],
            },
            BodyQuery {
                id: 1,
                generation: 7,
                points,
            },
        ],
    );
    assert_eq!(mailbox.stats().refused, 1);
    drop(
        host.advance(step)
            .expect("discarded frame")
            .encode_scene()
            .expect("tentative query"),
    );
    assert_eq!(mailbox.stats().dispatches, 0);
    assert!(
        host.drive(step + step / 2, Some(&surface))
            .expect("submitted")
            .submitted
    );
    assert!(mailbox.drain(1, step).is_empty());
    // This wait is in the test driver only; the mailbox is callable beforehand.
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("test completion");
    mailbox.post(
        2,
        vec![
            BodyQuery {
                id: 1,
                generation: 7,
                points: vec![],
            },
            BodyQuery {
                id: 5,
                generation: 1,
                points: vec![],
            },
            BodyQuery {
                id: 7,
                generation: 2,
                points: vec![],
            },
        ],
    );
    let mut answer = mailbox.drain(2, step);
    assert_eq!(
        answer.iter().map(|body| body.id).collect::<Vec<_>>(),
        [1, 5]
    );
    let swash = answer.pop().expect("swash");
    let domain = map.lock().expect("model");
    let row = domain.sample([15.0, 0.0]).expect("swash margin");
    assert!(row.depth < 0.0);
    assert!(!swash.points[0].surface.wet);
    assert!((swash.points[0].surface.depth - row.depth).abs() < 0.00004);
    check_waves(&answer[0].points, &spectrum, &oracle, step);
    assert_eq!(mailbox.stats().dispatches, 1);
    assert_eq!(mailbox.stats().blocking_reads, 0);
    drop(domain);
    check_delay(&mut host, &device, &surface, &mailbox, step);

    let point = QueryPoint {
        position: [22.0, 0.0],
        radius: 0.5,
        use_tick: 12,
    };
    let position = point.position.map(f64::from);
    assert!(map.lock().expect("beach").sample(position).is_none());
    mailbox.post(
        10,
        vec![BodyQuery {
            id: 1,
            generation: 7,
            points: vec![point],
        }],
    );
    assert!(
        host.drive(10 * step, Some(&surface))
            .expect("submit dry point")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("dry point readback");
    map.lock()
        .expect("beach")
        .set_stage(0, 3.0)
        .expect("rise after dispatch");
    let flooded = mailbox.drain(12, step);
    assert_eq!(flooded.len(), 1);
    let domain = map.lock().expect("beach");
    check_flooded_waves(&flooded[0].points[0], &domain, &spectrum, &oracle, step);
    assert_eq!(
        mailbox.stats().late,
        2,
        "on-time stage correction is not extrapolation"
    );
}

#[test]
fn chart_queries_keep_both_tick_pages_and_sample_their_own_stage() {
    use thrum_water_model::{Bed, BodyKind, Chart, Map, Stage};
    let Some((device, queue, caps)) =
        metal("chart_queries_keep_both_tick_pages_and_sample_their_own_stage")
    else {
        return;
    };
    let (mut host, target) = query_host(&device, &queue, &caps, Clock::new(true));
    let output = SurfaceOutput {
        view: &target,
        extent: EXTENT,
    };
    let step = Duration::from_nanos(16_666_667);
    let footprint: Vec<_> = (0..25)
        .map(|i| (1..=3).contains(&(i % 5)) && (1..=3).contains(&(i / 5)))
        .collect();
    let heights = footprint
        .iter()
        .map(|&wet| if wet { 0.0 } else { 1.6 })
        .collect();
    let map = Arc::new(Mutex::new(
        Map::prepare(
            Bed::new([0.0; 2], 0.5, [5, 5], heights).expect("chart bed"),
            vec![thrum_water_model::Body {
                name: "Sloped chart".into(),
                kind: BodyKind::Chart(Chart {
                    levels: (0..25).map(|i| 0.80 + 0.005 * f64::from(i % 5)).collect(),
                    currents: vec![[2.0, 0.4]; 25],
                    footprint,
                    ends: Vec::new(),
                }),
                level: 0.0,
                stage: Stage {
                    min: 0.0,
                    max: 0.5,
                    value: 0.0,
                },
                fetch: 100.0,
                gains: [0.0; 4],
            }],
            thrum_water_model::Profile {
                margin_height: 0.3,
                swash_ratio: 0.7,
            },
        )
        .expect("chart map"),
    ));
    let spectrum = Spectrum {
        long: Vec::new(),
        cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm band")),
        tail_slope_variance: 0.0,
    };
    let water = Water::mapped(
        Arc::clone(&map),
        spectrum,
        Profile {
            query: Some(QueryLimits {
                points: 1,
                max_age: 3,
            }),
            ..Profile::default()
        },
    )
    .expect("chart contribution");
    let mailbox = water.query_mailbox().expect("chart mailbox");
    host.set_scene_contribution(Box::new(water))
        .expect("chart scene");
    let request = |use_tick| {
        vec![BodyQuery {
            id: 1,
            generation: 7,
            points: vec![QueryPoint {
                position: [0.738, 0.656],
                radius: 0.0,
                use_tick,
            }],
        }]
    };
    mailbox.post(7, request(7));
    map.lock()
        .expect("chart")
        .set_stage(0, 0.10)
        .expect("stage at tick 8");
    mailbox.post(8, request(10));
    drop(
        host.advance(8 * step)
            .expect("tentative frame")
            .encode_scene()
            .expect("encode two pages"),
    );
    assert_eq!(mailbox.stats().dispatches, 0);
    assert!(
        host.drive(8 * step + step / 4, Some(&output))
            .expect("submit")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("both test readbacks complete");
    let results = mailbox.drain(9, step);
    assert_eq!(mailbox.stats().dispatches, 2);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].posted, 7);
    assert_eq!(mailbox.stats().age_two, 1);

    let check = |answer: &thrum_water::query::QueryAnswer, stage: f64| {
        let model = map
            .lock()
            .expect("chart")
            .sample_stages(&[stage])
            .expect("oracle stage");
        let point = answer.point.position.map(f64::from);
        let row = model.sample(point).expect("chart support");
        assert_eq!(answer.surface.body, Some(row.body));
        assert!(answer.surface.wet);
        assert!((answer.surface.sample.height - row.row.level).abs() < 0.000_002);
        assert!((answer.surface.depth - row.depth).abs() < 0.000_002);
        assert!((answer.surface.current[0] - 2.0).abs() < 0.000_002);
        assert!((answer.surface.current[1] - 0.4).abs() < 0.000_002);
        for (gpu, cpu) in answer
            .surface
            .sample
            .slope
            .into_iter()
            .zip(model.level_slope(row.body, point))
        {
            assert!((gpu - cpu).abs() < 0.000_002, "{gpu} vs {cpu}");
        }
        assert!((answer.surface.sample.slope[0] - 0.01).abs() < 0.000_002);
    };
    check(&results[0].points[0], 0.0);

    map.lock()
        .expect("chart")
        .set_stage(0, 0.20)
        .expect("stage at tick 9");
    mailbox.post(9, request(10));
    assert!(
        host.drive(9 * step + step / 4, Some(&output))
            .expect("quarter-alpha stage frame")
            .submitted
    );
    let submitted = host.last_submitted_clock().expect("stage submission");
    assert!((submitted.presentation_alpha - 0.25).abs() < 0.001);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("stage result");
    let second = mailbox.drain(10, step);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].posted, 8);
    assert_eq!(second[0].points[0].point.use_tick, 10);
    check(&second[0].points[0], 0.20);
    assert_eq!(mailbox.stats().late, 1, "only the historical page is late");
    map.lock()
        .expect("chart")
        .set_stage(0, 0.30)
        .expect("stage at tick 11");
    let third = mailbox.drain(11, step);
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].posted, 9);
    check(&third[0].points[0], 0.20);
    assert_eq!(mailbox.stats().age_two, 3);

    // A hitch can make three pages ready together. Keep the two timely
    // replies rather than getting stuck one tick late as new pages arrive.
    for tick in 12..=14 {
        mailbox.post(tick, request(tick + 2));
    }
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("retire prior chart");
    assert!(
        host.drive(14 * step, Some(&output))
            .expect("catch-up submission")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("catch-up replies");
    assert_eq!(mailbox.stats().dispatches, 6);
    assert_eq!(mailbox.drain(15, step)[0].posted, 13);
    assert_eq!(mailbox.drain(16, step)[0].posted, 14);
    assert_eq!(mailbox.stats().age_older, 0);
}

fn check_flooded_waves(
    answer: &thrum_water::query::QueryAnswer,
    map: &thrum_water_model::Map,
    spectrum: &Spectrum,
    oracle: &LongWaves,
    step: Duration,
) {
    let position = answer.point.position.map(f64::from);
    let row = map.sample(position).expect("new stage owns this point");
    let variances = [
        spectrum
            .long
            .iter()
            .map(|c| 0.5 * c.amplitude.powi(2))
            .sum(),
        spectrum.cascades[0].variance(),
        spectrum.cascades[1].variance(),
        spectrum.cascades[2].variance(),
    ];
    let gains = thrum_water_model::limit(row.row, row.depth, variances, map.profile()).gains;
    let time = 12.0 * step.as_secs_f64();
    let frame = 10.0 * step.as_secs_f64();
    assert_eq!(answer.surface.body, Some(0));
    assert!((answer.surface.depth - row.depth).abs() < 0.000_002);
    assert_eq!(
        answer.sources,
        [Some(time), Some(frame), Some(frame), Some(frame), None]
    );
    let long = oracle
        .scaled(gains[0])
        .expect("depth-limited gain")
        .evaluate(position, time)
        .expect("long wave");
    assert!((f64::from(answer.wave_heights[0]) - long.height).abs() < 0.002);
    let mut height = row.row.level + long.height;
    for (band, cascade) in spectrum.cascades.iter().enumerate() {
        let expected =
            gains[band + 1] * filtered_height(cascade, position, 0.5, frame, time - frame);
        let error = (f64::from(answer.wave_heights[band + 1]) - expected).abs();
        assert!(
            error / cascade.variance().sqrt() < 0.0004,
            "recovered band {band}: {error}"
        );
        height += expected;
    }
    assert!((answer.surface.sample.height - height).abs() < 0.002);
    assert_eq!(answer.surface.wet, row.bed.is_none_or(|bed| height > bed));
}

fn check_delay(
    host: &mut FrameHost<Clock, Source>,
    device: &wgpu::Device,
    surface: &SurfaceOutput<'_>,
    mailbox: &thrum_water::query::QueryMailbox,
    step: Duration,
) {
    let request = |use_tick| {
        vec![BodyQuery {
            id: 1,
            generation: 7,
            points: vec![
                QueryPoint {
                    position: [-72.0, 1.0],
                    radius: 0.2,
                    use_tick,
                },
                QueryPoint {
                    position: [45.0, 0.0],
                    radius: 0.0,
                    use_tick,
                },
            ],
        }]
    };
    mailbox.post(3, request(4));
    assert!(
        host.drive(3 * step, Some(surface))
            .expect("new pass")
            .submitted
    );
    // Two discarded presentation opportunities at the same 60 Hz endpoint.
    // No maintenance call in these paths can release the readback.
    host.advance_without_rendering(4 * step).expect("skip view");
    host.advance_without_rendering(4 * step + step / 2)
        .expect("skip repeat");
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("test releases stalled observation");
    let answer = mailbox.drain(5, step);
    assert_eq!(answer.len(), 1);
    assert_eq!(answer[0].posted, 3);
    assert_eq!(answer[0].points[0].point.use_tick, 4);
    assert_eq!(answer[0].points[1].surface.body, None);
    assert!(!answer[0].points[1].surface.wet);
    assert_eq!(mailbox.stats().age_two, 1);
    assert_eq!(mailbox.stats().late, 2);
    assert!(
        mailbox.drain(5, step).is_empty(),
        "a result can be carried once"
    );
    mailbox.post(6, request(7));
    assert!(
        host.drive(6 * step, Some(surface))
            .expect("third pass")
            .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("test completes expired result");
    assert!(mailbox.drain(10, step).is_empty());
    assert_eq!(mailbox.stats().unavailable, 1);
    assert_eq!(mailbox.stats().blocking_reads, 0);
}

fn check_waves(
    points: &[thrum_water::query::QueryAnswer],
    spectrum: &Spectrum,
    oracle: &LongWaves,
    step: Duration,
) {
    for point in points {
        let world = point.point.position.map(f64::from);
        let use_time = 2.0 * step.as_secs_f64();
        let expected = oracle.evaluate(world, use_time).expect("contractive model");
        assert_eq!(point.sources[0], Some(use_time));
        assert_eq!(point.sources[4], None);
        assert!((f64::from(point.wave_heights[0]) - expected.height).abs() < 0.002);
        assert_eq!(point.surface.body, Some(0));
        assert!(point.surface.wet && point.surface.depth.is_infinite());
        assert_eq!(point.surface.current, [0.0; 2]);
        for (band, cascade) in spectrum.cascades.iter().enumerate() {
            let frame = point.sources[band + 1].expect("supported cascade");
            let expected = filtered_height(
                cascade,
                world,
                f64::from(point.point.radius),
                frame,
                use_time - frame,
            );
            let error = (f64::from(point.wave_heights[band + 1]) - expected).abs();
            assert!(
                error / cascade.variance().sqrt() < 0.0004,
                "band {band} at {world:?}: {error}"
            );
        }
    }
}

fn filtered_height(cascade: &Cascade, p: [f64; 2], radius: f64, time: f64, dt: f64) -> f64 {
    let width = cascade.length() / 256.0;
    let lod = (radius * 2.0 / width).max(1.0).log2();
    let base = (2.0_f64).powf(lod.floor());
    let at_mip = |span: f64| {
        let coords = p.map(|value| value / (width * span) - 0.5);
        let corner = coords.map(f64::floor);
        let frac = std::array::from_fn::<_, 2, _>(|i| coords[i] - corner[i]);
        let mut sum = [0.0; 4];
        let n = (1..=256_u16).find(|n| f64::from(*n) >= span).expect("mip");
        for (i, value) in sum.iter_mut().enumerate() {
            let x = corner[0] + f64::from(u16::try_from(i % 2).expect("column"));
            let z = corner[1] + f64::from(u16::try_from(i / 2).expect("row"));
            for a in 0..n {
                for b in 0..n {
                    let sample = cascade.direct(
                        [
                            width * (x * span + f64::from(a) + 0.5),
                            width * (z * span + f64::from(b) + 0.5),
                        ],
                        time,
                    );
                    *value += sample.displacement[1] + dt * sample.velocity[1];
                }
            }
            *value /= span * span;
        }
        let a = (sum[1] - sum[0]).mul_add(frac[0], sum[0]);
        let b = (sum[3] - sum[2]).mul_add(frac[0], sum[2]);
        (b - a).mul_add(frac[1], a)
    };
    let low = at_mip(base);
    (at_mip(2.0 * base) - low).mul_add(lod.fract(), low)
}
