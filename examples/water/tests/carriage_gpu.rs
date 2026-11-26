//! Track a known phase in the actual draw over three adjacent chart reaches.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn submitted_detail_moves_with_chart_current() {
    let Some(gpu) = thrum_test_support::gpu(
        "submitted_detail_moves_with_chart_current",
        Some(wgpu::Backend::Metal),
    ) else { return };
    let speeds = [0.0, 0.5, 1.5];
    let centres = [-6.0, 0.0, 6.0];
    let (mut scene, tap, camera, extent, _) = fixture(&gpu, map(speeds.map(|speed| {
        move |_, _| [speed, 0.0]
    })), false);
    let matrix = support::depth::matrix(camera, extent);
    let mode = marker().cascades[0].components()[0];
    let k = mode.wave_vector[0];
    let intrinsic_frequency = thrum_sea_model::cascade_frequency(mode);
    let duration = 12.0 / 60.0;
    let wrapped = |angle: f64| {
        (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
    };
    let mut frame_phases = Vec::new();
    // Observe both old blends, both wraps and successive periods with the
    // same fixture and time step, including the windows that used to reverse.
    for time in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.2,
        2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0, 4.2, 4.4, 5.0, 5.2, 7.9, 8.1] {
        scene.frame_without_timings(time + 0.000_000_01);
        frame_phases.push((time, phase(&tap.read(&scene), extent, matrix, centres, k)));
    }
    for pair in frame_phases.windows(2) {
        let (start, before) = pair[0];
        let (end, after) = pair[1];
        if end - start > duration + 1e-6 { continue; }
        let control = wrapped(after[0].0 - before[0].0);
        assert!((control + intrinsic_frequency * duration).abs() < 0.02,
            "the still-water control must have only the marker wave's intrinsic phase change");
        for (i, speed) in speeds.into_iter().enumerate() {
            let measured = -wrapped(after[i].0 - before[i].0 - control) / (k * duration);
            println!("Metal t={start:.1}–{end:.1}s, chart {speed:.2} m/s: {measured:.3} m/s after subtracting intrinsic phase");
            assert!((measured - speed).abs() < 0.22, "carriage: {measured:.4} vs {speed}");
        }
    }
}
}

thrum_test_support::macos_gpu_test! {
fn varying_current_keeps_the_submitted_marker_moving() {
    let Some(gpu) = thrum_test_support::gpu(
        "varying_current_keeps_the_submitted_marker_moving", Some(wgpu::Backend::Metal),
    ) else { return };
    let speeds = [0.0, 0.5, 1.5];
    let centres = [-6.0, 0.0, 6.0];
    let map = map(speeds.map(|speed| move |x, z| {
        if speed == 0.0 { return [0.0, 0.0]; }
        // The outer bank's transverse flow selects the general chart path.
        [(speed + 0.04 * f64::from(x)).max(0.1), if z % 6 == 2 { 0.05 } else { 0.0 }]
    }));
    let (mut scene, tap, camera, extent, _) = fixture(&gpu, map, false);
    let matrix = support::depth::matrix(camera, extent);
    let k = marker().cascades[0].components()[0].wave_vector[0];
    // x - u(x)t used to become flat at 25 s and reverse thereafter.
    for time in [0.0, 23.9, 24.2, 24.9, 25.2, 25.9, 26.2, 59.9, 60.2] {
        let mut samples = Vec::new();
        for t in [time, time + 0.2] {
            scene.frame_without_timings(t + 1e-8);
            let depth = tap.read(&scene);
            samples.push(phase(&depth, extent, matrix, centres, k));
        }
        let still = samples[1][0].0 - samples[0][0].0;
        for (i, speed) in speeds.into_iter().enumerate().skip(1) {
            let height = samples[0][i].1;
            let phase_delta = samples[1][i].0 - samples[0][i].0 - still;
            let measured = -phase_delta.sin().atan2(phase_delta.cos()) / (k * 0.2);
            println!("Metal t={time:.1}s, {speed:.2} m/s at centre: {height:.5} m peak-to-trough, {measured:.3} m/s after subtracting intrinsic phase");
            assert!(height > 0.08, "the marker collapsed at {time}s in reach {i}");
            assert!((measured - speed).abs() < 0.22, "carriage at {time}s: {measured:.4} vs {speed}");
        }
    }
}
}

thrum_test_support::macos_gpu_test! {
fn query_slope_matches_the_carried_height_in_both_sections() {
    use thrum_water::query::{BodyQuery, QueryPoint};
    let Some(gpu) = thrum_test_support::gpu(
        "query_slope_matches_the_carried_height_in_both_sections", Some(wgpu::Backend::Metal),
    ) else { return };
    let map = map([0, 1, 2].map(|reach| move |x, _| {
        let speed = match reach {
            0 => 0.0,
            1 => 1.0,
            _ if x < 0 => 0.5,
            _ => 1.5,
        };
        [speed, 0.0]
    }));
    let (mut scene, _, _, _, mailbox) = fixture(&gpu, map, true);
    let mailbox = mailbox.expect("query mailbox");
    let stations = [-9.07_f32, -7.6, -5.71, -3.72, 3.72, 5.71, 7.6, 9.07];
    let points = stations.into_iter().flat_map(|x| {
        [x - 0.15, x, x + 0.15].map(|x| QueryPoint {
            position: [x, 6.0], radius: 0.0, use_tick: 0,
        })
    }).collect();
    mailbox.post(scene.endpoint(0), vec![BodyQuery { id: 1, generation: 1, points }]);
    scene.frame_without_timings(0.0);
    let step = std::time::Duration::from_nanos(16_666_667);
    scene.frame_without_timings(step.as_secs_f64());
    let answers = mailbox.drain(scene.endpoint(2), step);
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].points.len(), 3 * stations.len());
    for (x, points) in stations.into_iter().zip(answers[0].points.as_chunks::<3>().0) {
        let [left, centre, right] = [0, 1, 2].map(|i| points[i].surface);
        assert!(left.wet && centre.wet && right.wet);
        let reported = centre.sample.slope[0];
        // The mailbox carries height to the drain tick using the rate. Compare
        // the slope and neighbouring heights at the same submitted field time.
        let frame_height = |answer: &thrum_water::query::QueryAnswer| {
            let carried = 2.0 * step.as_secs_f64() - answer.sources[1].expect("cascade time");
            answer.surface.sample.height - carried * answer.surface.sample.height_dt
        };
        let numerical = (frame_height(&points[2]) - frame_height(&points[0])) / 0.3;
        println!("Metal query x={x:.2}: reported slope {reported:.6}, height derivative {numerical:.6}");
        assert!((reported - numerical).abs() < 0.012, "query did not return the world-space slope at {x}");
    }
}
}

#[cfg(target_os = "macos")]
fn marker() -> thrum_sea_model::Spectrum {
    thrum_sea_model::Spectrum {
        long: Vec::new(),
        cascades: [
            thrum_sea_model::Cascade::new(64.0, &[(8, 0, 0.0018, 0.0)]).expect("marker wave"),
            thrum_sea_model::Cascade::new(16.0, &[]).expect("empty cascade"),
            thrum_sea_model::Cascade::new(4.0, &[]).expect("empty cascade"),
        ],
        tail_slope_variance: 0.0,
    }
}

#[cfg(target_os = "macos")]
fn fixture(
    gpu: &thrum_test_support::Gpu,
    map: thrum_water_model::Map,
    query: bool,
) -> (
    support::Fixture,
    support::depth::DepthTap,
    thrum_app::sky::Camera,
    [u32; 2],
    Option<thrum_water::query::QueryMailbox>,
) {
    let extent = [320, 192];
    let camera = thrum_app::sky::Camera::look_at([0.0, 13.0, 0.4], [0.0, 0.0, 0.0], 65.0);
    let mut profile = thrum_water::Profile::default();
    profile.waves.choppiness = 0.0;
    profile.query = query.then_some(thrum_water::query::QueryLimits {
        points: 24,
        max_age: 8,
    });
    let water = thrum_water::Water::mapped(
        std::sync::Arc::new(std::sync::Mutex::new(map)),
        marker(),
        profile,
    )
    .expect("mapped water");
    let report = water.report();
    let mailbox = water.query_mailbox();
    let tap = support::depth::DepthTap::default();
    let game = thrum_app::sky::SkyGame::new(thrum_app::sky::Controls::new(), camera, |_, _| {})
        .with_ground(None)
        .with_tick_duration(std::time::Duration::from_nanos(16_666_667))
        .expect("clock")
        .with_contribution(Box::new(water), || thrum_app::sky::ContentStatus::Drawn(1))
        .with_contribution(Box::new(tap.contribution(extent)), || {
            thrum_app::sky::ContentStatus::Drawn(1)
        });
    (
        support::Fixture::from_game(gpu, extent, game, report),
        tap,
        camera,
        extent,
        mailbox,
    )
}

#[cfg(target_os = "macos")]
fn map(currents: [impl Fn(i32, i32) -> [f64; 2]; 3]) -> thrum_water_model::Map {
    use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Profile, Stage};
    let size = [37_usize, 21];
    let points: Vec<_> = (-10..=10)
        .flat_map(|z| (-18..=18).map(move |x| (x, z)))
        .collect();
    let footprints = [-6, 0, 6].map(|centre| {
        points
            .iter()
            .map(|&(x, z): &(i32, i32)| x.abs() <= 16 && (z - centre).abs() <= 2)
            .collect::<Vec<_>>()
    });
    let heights = (0..points.len())
        .map(|i| {
            if footprints.iter().any(|f| f[i]) {
                -3.0
            } else {
                4.0
            }
        })
        .collect();
    let bed = Bed::new([-18.0, -10.0], 1.0, size, heights).expect("bed");
    let bodies = footprints
        .into_iter()
        .zip(currents)
        .enumerate()
        .map(|(i, (footprint, current))| Body {
            name: format!("Reach {i}"),
            kind: BodyKind::Chart(Chart {
                levels: vec![0.0; points.len()],
                currents: points.iter().map(|&(x, z)| current(x, z)).collect(),
                footprint,
                ends: Vec::new(),
            }),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 20.0,
            gains: [0.0, 1.0, 0.0, 0.0],
        })
        .collect();
    Map::prepare(
        bed,
        bodies,
        Profile {
            margin_height: 0.1,
            swash_ratio: 0.5,
        },
    )
    .expect("three charts")
}

#[cfg(target_os = "macos")]
fn phase(
    depth: &[f32],
    extent: [u32; 2],
    matrix: glam::Mat4,
    centres: [f64; 3],
    k: f64,
) -> [(f64, f64); 3] {
    let mut fits = [[0.0; 2]; 3];
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for z in 0..extent[1] {
        for x in 0..extent[0] {
            let d = depth[(z * extent[0] + x) as usize];
            if d == 0.0 {
                continue;
            }
            let point = support::world_at(matrix, extent, [x, z], d);
            if point.x.abs() >= 12.0 {
                continue;
            }
            if let Some(i) = centres
                .iter()
                .position(|centre| (f64::from(point.z) - centre).abs() < 0.9)
            {
                let (sin, cos) = (k * f64::from(point.x)).sin_cos();
                fits[i][0] += f64::from(point.y) * cos;
                fits[i][1] -= f64::from(point.y) * sin;
                bounds[i][0] = bounds[i][0].min(f64::from(point.y));
                bounds[i][1] = bounds[i][1].max(f64::from(point.y));
            }
        }
    }
    std::array::from_fn(|i| {
        let [a, b] = fits[i];
        assert!(a.hypot(b) > 1.0, "no submitted wave");
        (b.atan2(a), bounds[i][1] - bounds[i][0])
    })
}
