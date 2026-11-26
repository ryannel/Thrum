//! Track submitted ripple heights and compare the public mailbox's slope with its heights.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn separate_uniform_reaches_carry_submitted_detail() {
    let Some(gpu) = thrum_test_support::gpu(
        "separate_uniform_reaches_carry_submitted_detail", Some(wgpu::Backend::Metal),
    ) else { return };
    let speeds = [0.0, 0.5, 1.5];
    let map = chart_map(speeds.map(|speed| move |_, _| [speed, 0.0]), false);
    check_carriage(&gpu, "separate", map, [-6.0, 0.0, 6.0], speeds, &steady_windows());
}
}

thrum_test_support::macos_gpu_test! {
fn connected_lanes_carry_submitted_detail() {
    let Some(gpu) = thrum_test_support::gpu(
        "connected_lanes_carry_submitted_detail", Some(wgpu::Backend::Metal),
    ) else { return };
    let speeds = [0.0, 0.5, 1.5];
    let map = chart_map([|_, z| {
        let lane = if z < -3 { 0 } else if z < 3 { 1 } else { 2 };
        [speeds[lane], 0.0]
    }], true);
    check_carriage(&gpu, "connected", map, [-6.0, 0.0, 6.0], speeds, &steady_windows());
}
}

thrum_test_support::macos_gpu_test! {
fn still_chart_has_only_the_intrinsic_wave_motion() {
    let Some(gpu) = thrum_test_support::gpu(
        "still_chart_has_only_the_intrinsic_wave_motion", Some(wgpu::Backend::Metal),
    ) else { return };
    let map = chart_map([|_, _| [0.0, 0.0]], true);
    check_carriage(&gpu, "still", map, [-6.0, 0.0, 6.0], [0.0; 3], &steady_windows());
}
}

thrum_test_support::macos_gpu_test! {
fn varying_current_keeps_the_submitted_marker_moving() {
    let Some(gpu) = thrum_test_support::gpu(
        "varying_current_keeps_the_submitted_marker_moving", Some(wgpu::Backend::Metal),
    ) else { return };
    let speeds = [0.5, 1.5];
    let map = chart_map(speeds.map(|speed| move |x, z| {
        // The outer bank's transverse flow selects the general chart path.
        [(speed + 0.04 * f64::from(x)).max(0.1), if z % 6 == 2 { 0.05 } else { 0.0 }]
    }), false);
    check_carriage(&gpu, "varying", map, [-6.0, 0.0], speeds,
        &[0, 240, 242, 244, 246, 248, 250, 252, 254, 256, 599, 602]);
}
}

#[cfg(target_os = "macos")]
fn steady_windows() -> Vec<u32> {
    // Tenths of a second. The four-second profile period is sampled end to end.
    (0..40).step_by(2).chain([79, 249, 599]).collect()
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stdout,
    reason = "record submitted Metal phase and height for review"
)]
fn check_carriage<const N: usize>(
    gpu: &thrum_test_support::Gpu,
    case: &str,
    map: thrum_water_model::Map,
    centres: [f64; N],
    speeds: [f64; N],
    starts: &[u32],
) {
    let (mut scene, tap, camera, extent, _) = fixture(gpu, map, false);
    let matrix = support::depth::matrix(camera, extent);
    let mode = marker().cascades[0].components()[0];
    let k = mode.wave_vector[0];
    let intrinsic = thrum_sea_model::cascade_frequency(mode) * 0.2;
    let mut frames = std::collections::BTreeMap::new();
    for t in starts
        .iter()
        .flat_map(|&t| [t, t + 2])
        .collect::<std::collections::BTreeSet<_>>()
    {
        scene.frame_without_timings(f64::from(t) / 10.0 + 1e-8);
        frames.insert(t, phase(&tap.read(&scene), extent, matrix, centres, k));
    }
    let initial = frames[&0];
    for &t in starts {
        let before = frames[&t];
        let after = frames[&(t + 2)];
        let measured: [f64; N] = std::array::from_fn(|i| {
            let delta = after[i].0 - before[i].0 + intrinsic;
            -delta.sin().atan2(delta.cos()) / (k * 0.2)
        });
        for (i, speed) in speeds.into_iter().enumerate() {
            let drift = measured[i];
            let height = before[i].1.min(after[i].1);
            let initial_height = initial[i].1;
            println!(
                "Metal {case} t={:.1}–{:.1}s, {speed:.2} m/s: drift {drift:.4} m/s, height {height:.5} m of {initial_height:.5} m",
                f64::from(t) / 10.0,
                f64::from(t + 2) / 10.0
            );
            assert!(
                height >= 0.5 * initial_height,
                "{case} t={t}, marker flattened in reach {i}"
            );
            if speed == 0.0 {
                assert!(
                    drift.abs() < 0.1,
                    "{case} t={t}, still reach drifted by {drift:.4} m/s"
                );
            } else {
                assert!(
                    drift > 0.0,
                    "{case} t={t}, reach {i} did not travel downstream"
                );
            }
        }
        if let (Some(slow), Some(fast)) = (
            speeds
                .iter()
                .position(|speed| speed.to_bits() == 0.5_f64.to_bits()),
            speeds
                .iter()
                .position(|speed| speed.to_bits() == 1.5_f64.to_bits()),
        ) {
            assert!(
                measured[fast] > measured[slow],
                "{case} t={t}, speed order: {measured:?}"
            );
        }
    }
}

thrum_test_support::macos_gpu_test! {
fn query_reports_world_slopes_for_uniform_parallel_and_sheared_charts() {
    let Some(gpu) = thrum_test_support::gpu(
        "query_reports_world_slopes_for_uniform_parallel_and_sheared_charts", Some(wgpu::Backend::Metal),
    ) else { return };
    let x = [-9.07_f32, -7.6, -5.71, -3.72, 3.72, 5.71, 7.6, 9.07];
    let cases: [(&str, thrum_water_model::Map, [[f32; 2]; 8]); 3] = [
        ("uniform", chart_map([|_, _| [1.0, 0.0]], true), x.map(|x| [x, 0.0])),
        ("parallel", chart_map([|x, _| [if x < 0 { 0.5 } else { 1.5 }, 0.0]], true),
            x.map(|x| [x, 0.0])),
        ("sheared", chart_map([|_, z| [1.0 + 0.25 * f64::from(z).clamp(-2.0, 2.0), 0.0]], true),
            std::array::from_fn(|i| [x[i], [-1.75, -1.25, -0.75, -0.25, 0.25, 0.75, 1.25, 1.75][i]])),
    ];
    for (case, map, stations) in cases {
        check_query_slopes(&gpu, case, map, stations);
    }
}
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stdout,
    reason = "record mailbox slopes and neighbouring heights for review"
)]
fn check_query_slopes(
    gpu: &thrum_test_support::Gpu,
    case: &str,
    map: thrum_water_model::Map,
    stations: [[f32; 2]; 8],
) {
    use thrum_water::query::{BodyQuery, QueryAnswer, QueryPoint};
    const STEP: std::time::Duration = std::time::Duration::from_nanos(16_666_667);
    const POST: u32 = 252;
    const DRAIN: u32 = POST + 2;
    let (mut scene, _, _, _, mailbox) = fixture(gpu, map, true);
    let mailbox = mailbox.expect("query mailbox");
    scene.frame_without_timings(0.0);
    let before = scene.frame_without_timings((STEP * (POST - 1)).as_secs_f64() + 1e-8);
    assert_eq!(before.tick, u64::from(POST - 1));
    let points = stations
        .into_iter()
        .flat_map(|[x, z]| {
            [
                [x, z],
                [x - 0.15, z],
                [x + 0.15, z],
                [x, z - 0.15],
                [x, z + 0.15],
            ]
            .map(|position| QueryPoint {
                position,
                radius: 0.0,
                use_tick: u64::from(POST),
            })
        })
        .collect();
    mailbox.post(
        scene.endpoint(u64::from(POST)),
        vec![BodyQuery {
            id: 1,
            generation: 1,
            points,
        }],
    );
    for tick in [POST, POST + 1] {
        scene.frame_without_timings((STEP * tick).as_secs_f64() + 1e-8);
    }
    let answers = mailbox.drain(scene.endpoint(u64::from(DRAIN)), STEP);
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].points.len(), 5 * stations.len());
    let field_time = answers[0].points[0].sources[1].expect("cascade time");
    assert!(field_time >= 4.0);
    let frame_height = |answer: &QueryAnswer| {
        assert!(answer.surface.wet);
        assert_eq!(answer.sources[1], Some(field_time));
        let carried = (STEP * DRAIN).as_secs_f64() - field_time;
        answer.surface.sample.height - carried * answer.surface.sample.height_dt
    };
    let mut greatest_cross_tilt = 0.0_f64;
    for ([x, z], [centre, x0, x1, z0, z1]) in stations
        .into_iter()
        .zip(answers[0].points.as_chunks::<5>().0)
    {
        assert!(centre.surface.wet);
        let reported = centre.surface.sample.slope;
        let numerical = [
            (frame_height(x1) - frame_height(x0)) / 0.3,
            (frame_height(z1) - frame_height(z0)) / 0.3,
        ];
        greatest_cross_tilt = greatest_cross_tilt.max(numerical[1].abs());
        println!(
            "Metal {case} t={field_time:.3}s x={x:.2} z={z:.2}, slope {reported:.6?}, neighbouring derivative {numerical:.6?}"
        );
        for axis in 0..2 {
            assert!(
                (reported[axis] - numerical[axis]).abs() < 0.012,
                "{case} at [{x}, {z}], slope component {axis}: {reported:?} vs {numerical:?}"
            );
        }
    }
    if case == "sheared" {
        assert!(
            greatest_cross_tilt > 0.012,
            "sheared query did not exercise a distinguishable cross-channel tilt"
        );
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
        points: 40,
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
fn chart_map<const N: usize>(
    currents: [impl Fn(i32, i32) -> [f64; 2]; N],
    connected: bool,
) -> thrum_water_model::Map {
    use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Profile, Stage};
    let size = [37_usize, 21];
    let points: Vec<_> = (-10..=10)
        .flat_map(|z| (-18..=18).map(move |x| (x, z)))
        .collect();
    let footprints: [Vec<_>; N] = std::array::from_fn(|i| {
        let centre = [-6, 0, 6][i];
        points
            .iter()
            .map(|&(x, z): &(i32, i32)| {
                x.abs() <= 16
                    && if connected {
                        z.abs() <= 9
                    } else {
                        (z - centre).abs() <= 2
                    }
            })
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
    .expect("prepared charts")
}

#[cfg(target_os = "macos")]
fn phase<const N: usize>(
    depth: &[f32],
    extent: [u32; 2],
    matrix: glam::Mat4,
    centres: [f64; N],
    k: f64,
) -> [(f64, f64); N] {
    let mut fits = [[0.0; 2]; N];
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; N];
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
