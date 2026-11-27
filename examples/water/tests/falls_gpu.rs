//! Submitted waterfall geometry, frame attachments and runtime spacing.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
#[expect(clippy::print_stderr, reason = "record submitted sheet geometry and temporal values")]
fn sheet_draws_before_the_gather_and_writes_temporal_inputs() {
    use glam::{Mat4, Vec3};
    use support::{Fixture, fall};
    use thrum_app::sky::Camera;
    use thrum_render::{runtime::FrameWorld, synthetic::{DisplayOverlay, SyntheticView}};

    let Some(gpu) = thrum_test_support::gpu("fall sheet", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let extent = [160, 112];
    let camera = Camera::look_at([0.0, 2.5, 5.6], [0.0, 1.6, 0.9], 45.0);
    let projection = SyntheticView::perspective(camera.position, camera.target, [0.0, 1.0, 0.0],
        45_f32.to_radians(), 160.0 / 112.0, 0.1, 30_000.0).expect("sheet lens");
    let matrix = Mat4::from_cols_array_2d(&projection.view_proj_columns());
    let start = |enabled| {
        let fall::Scene { game, report, map, .. } = fall::scene(1.0, enabled, false, camera);
        let game = game.with_display_overlay(DisplayOverlay::TemporalInputs);
        (Fixture::from_game(&gpu, extent, game, report), map)
    };

    let (mut fixture, map) = start(true);
    let knob = fixture.host.quality_knobs().into_iter()
        .find(|entry| entry.name.ends_with("river.fall_path_spacing")).expect("sheet spacing knob");
    assert_eq!(knob.range, 100..=1200);
    assert_eq!(knob.invalidation, thrum_render::contribution::QualityInvalidation::None);
    for tick in 0..5 {
        fixture.frame_without_timings((fall::STEP * tick).as_secs_f64());
    }
    fixture.frame_without_timings((fall::STEP * 5).as_secs_f64());
    let graph = fixture.host.scene_graph().expect("prepared Metal scene");
    let names = graph.pass_names().map(str::to_owned).collect::<Vec<_>>();
    let index = |name: &str| names.iter().position(|candidate| candidate.ends_with(name)).unwrap_or_else(|| panic!("{name} absent: {names:?}"));
    assert!(index("waterfall sheet") < index("water gather"));

    let pixels = fixture.witness();
    let moving = pixels.iter().filter(|[reactive, depth, _, y]| {
        *reactive > 0.15 && *depth > 0.0 && *y < -0.05 && *y > -24.0
    }).count();
    assert!(moving > 100, "fall needs moving reactive pixels ({moving})");
    let sample = |target: Vec3, values: &[[f32; 4]]| {
        let p = matrix.project_point3(target);
        let [x, y] = pixel(p, extent);
        values[y * extent[0] as usize + x]
    };
    let path = |time: f32| Vec3::new(0.0, 3.0 - 4.905 * time * time, time * 2.0);
    let low = sample(path(0.36), &pixels);
    let foot = sample(path(0.67), &pixels);
    eprintln!("Metal waterfall vertical motion at lip={low:?}, foot={foot:?}, pixels={moving}");
    assert!(foot[3] < low[3], "the foot moves faster across the submitted frame");
    for time in [0.28, 0.40, 0.50, 0.60, 0.68] {
        let world = path(time);
        let actual = sample(world, &pixels)[1];
        let expected = matrix.project_point3(world).z;
        assert!((actual - expected).abs() < 0.002, "world {world} depth {actual} vs {expected}");
    }

    let creations = fixture.host.pipeline_creation_count();
    let authority = map.lock().expect("fall map").sample(fall::POOL);
    fixture.host.set_quality_knob(&knob.name, 600).expect("set sheet spacing");
    fixture.frame_without_timings((fall::STEP * 6).as_secs_f64());
    assert_eq!(fixture.host.pipeline_creation_count(), creations);
    assert_eq!(map.lock().expect("fall map").sample(fall::POOL), authority);
    let lower = fixture.witness();
    map.lock().expect("fall map").set_stage(1, 0.33).expect("upper pool stage");
    for tick in 7..=9 {
        fixture.frame_without_timings((fall::STEP * tick).as_secs_f64());
    }
    let raised = fixture.witness();
    let covered = path(0.76);
    let lower_hit = sample(covered, &lower);
    let upper_hit = sample(covered, &raised);
    eprintln!("Metal sheet foot at the two pool stages {lower_hit:?} / {upper_hit:?}");
    assert!(upper_hit[1] > lower_hit[1] + 0.0004, "raised pool covers the foot");
    map.lock().expect("fall map").set_gauge(0, 0.5).expect("trickle sample");
    for tick in 10..=12 {
        fixture.frame_without_timings((fall::STEP * tick).as_secs_f64());
    }
    let thin = fixture.witness().iter().zip(&raised)
        .filter(|(trickle, flood)| flood[0] > 0.28 && trickle[1] < flood[1] - 0.001)
        .count();
    eprintln!("Metal flood pixels that open into gaps at the lowest running sample {thin}");
    assert!(thin > 40, "the trickle leaves gaps in the flood's sheet");
    fixture.host.world_mut().pause();
    let time = fixture.host.world().source().tick;
    fixture.frame_without_timings(2.0);
    let held = fixture.host.capture_pixels().expect("capture held sheet");
    fixture.frame_without_timings(4.0);
    assert_eq!(fixture.host.world().source().tick, time);
    assert_eq!(held, fixture.host.capture_pixels().expect("capture at held time"));
    drop(fixture);

    let (mut off, _) = start(false);
    off.frame_without_timings(0.0);
    let graph = off.host.scene_graph().expect("disabled graph");
    assert!(graph.pass_names().all(|name| !name.ends_with("waterfall sheet")));
    assert!(off.host.quality_knobs().iter().all(|entry| entry.name != knob.name));
    assert!(off.report.lock().expect("resources").resources.iter().all(|(name, _)| !name.contains("waterfall")));
}
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "fixture points lie inside the frame"
)]
fn pixel(point: glam::Vec3, [width, height]: [u32; 2]) -> [usize; 2] {
    [
        ((point.x + 1.0) * 0.5 * width as f32).floor() as usize,
        ((1.0 - point.y) * 0.5 * height as f32).floor() as usize,
    ]
}
