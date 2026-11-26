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
    let map = map(speeds);
    let spectrum = thrum_sea_model::Spectrum {
        long: Vec::new(),
        cascades: [
            thrum_sea_model::Cascade::new(64.0, &[(8, 0, 0.0018, 0.0)]).expect("marker wave"),
            thrum_sea_model::Cascade::new(16.0, &[]).expect("empty cascade"),
            thrum_sea_model::Cascade::new(4.0, &[]).expect("empty cascade"),
        ],
        tail_slope_variance: 0.0,
    };
    let mode = spectrum.cascades[0].components()[0];
    let k = mode.wave_vector[0];
    let intrinsic_frequency = thrum_sea_model::cascade_frequency(mode);
    let step = 1.0 / 60.0;
    let duration = 12.0 * step;
    let extent = [320, 192];
    let camera = thrum_app::sky::Camera::look_at([0.0, 13.0, 0.4], [0.0, 0.0, 0.0], 65.0);
    let matrix = support::depth::matrix(camera, extent);
    let mut profile = thrum_water::Profile::default();
    profile.waves.choppiness = 0.0;
    let water = thrum_water::Water::mapped(
        std::sync::Arc::new(std::sync::Mutex::new(map)), spectrum,
        profile,
    ).expect("mapped water");
    let report = water.report();
    let tap = support::depth::DepthTap::default();
    let game = thrum_app::sky::SkyGame::new(thrum_app::sky::Controls::new(), camera, |_, _| {})
        .with_ground(None)
        .with_tick_duration(std::time::Duration::from_nanos(16_666_667)).expect("clock")
        .with_contribution(Box::new(water), || thrum_app::sky::ContentStatus::Drawn(1))
        .with_contribution(Box::new(tap.contribution(extent)), || thrum_app::sky::ContentStatus::Drawn(1));
    let mut scene = support::Fixture::from_game(&gpu, extent, game, report);
    scene.frame_without_timings(0.000_000_01);
    let before = phase(&tap.read(&scene), extent, matrix, centres, k);
    scene.frame_without_timings(duration + 0.000_000_01);
    let after = phase(&tap.read(&scene), extent, matrix, centres, k);
    let control = after[0] - before[0];
    assert!((control + intrinsic_frequency * duration).abs() < 0.02,
        "the still-water control must have only the marker wave's intrinsic phase change");
    for (i, speed) in speeds.into_iter().enumerate() {
        let measured = -(after[i] - before[i] - control) / (k * duration);
        let residual = (measured - speed).abs();
        println!("Metal chart {speed:.2} m/s: {measured:.3} m/s after subtracting still-water wave phase");
        assert!(residual < 0.22, "carriage: {measured:.4} vs {speed}");
    }
}
}

#[cfg(target_os = "macos")]
fn map(speeds: [f64; 3]) -> thrum_water_model::Map {
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
        .zip(speeds)
        .map(|(footprint, speed)| Body {
            name: format!("{speed} m/s"),
            kind: BodyKind::Chart(Chart {
                levels: vec![0.0; points.len()],
                currents: vec![[speed, 0.0]; points.len()],
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
) -> [f64; 3] {
    let mut fits = [[0.0; 2]; 3];
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
            }
        }
    }
    fits.map(|[a, b]| {
        assert!(a.hypot(b) > 1.0, "no submitted wave");
        b.atan2(a)
    })
}
