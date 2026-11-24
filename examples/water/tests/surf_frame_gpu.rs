//! Compare submitted depth and motion against the forward f64 sheet.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn submitted_breaker_depth_motion_and_stage_history() {
    use glam::{Mat4, Vec2, Vec3};
    use support::surf_frame::{CENTER, EXTENT, cut_ray, projected_nodes};
    use std::sync::{Arc, Mutex};
    use support::surf::{CHOPPINESS, STEP, beach};
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_render::synthetic::SyntheticView;
    use thrum_water::{Profile, Water};
    use thrum_water_model::SurfProfile;

    const DEPTH_TOLERANCE: f32 = 0.0001;
    const MOTION_TOLERANCE: f32 = 0.16;

    let Some(gpu) = thrum_test_support::gpu("submitted breaker", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (mut map, spectrum, field) = beach();
    map.enable_surf(SurfProfile::default())
        .expect("prepared shore");
    let live = Arc::new(Mutex::new(map.clone()));
    let mut profile = Profile::default();
    profile.waves.choppiness = CHOPPINESS;
    let water = Water::mapped(Arc::clone(&live), spectrum, profile).expect("mapped water");
    let report = water.report();
    let eye = [CENTER[0], 3.0, CENTER[1]];
    let target = [CENTER[0] + 0.12, 0.0, CENTER[1]];
    let camera = Camera::look_at(eye, target, 45.0);
    let game = SkyGame::new(Controls::new(), camera, |_, sky| sky.temporal_inputs = true)
        .with_ground(None)
        .with_tick_duration(STEP)
        .expect("fixed ticks")
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    let mut fixture = support::Fixture::from_game(&gpu, EXTENT, game, report);
    let matrix = Mat4::from_cols_array_2d(
        &SyntheticView::perspective(
            eye,
            target,
            Vec3::Y.to_array(),
            45_f32.to_radians(),
            1.5,
            0.1,
            30_000.0,
        )
        .expect("camera")
        .view_proj_columns(),
    );
    let mut previous = map.clone();
    let mut previous_time = 0.0;
    let mut previous_stage = 0.0;
    let mut folded_pixels = 0;
    let mut max_depth_error = 0.0_f32;
    let mut max_motion_error = 0.0_f32;
    for (index, (tick, fraction, stage)) in [
        (3_u32, 0.25, 0.0),
        (3, 0.75, 0.0),
        (4, 0.25, 0.12),
        (4, 0.75, 0.12),
        (5, 0.25, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        live.lock()
            .expect("live beach")
            .set_stage(0, stage)
            .expect("tide edit");
        let frame =
            fixture.frame_without_timings((f64::from(tick) + fraction) * STEP.as_secs_f64());
        assert_eq!(frame.tick, u64::from(tick));
        let alpha = f64::from(frame.presentation_alpha);
        let time = (f64::from(tick) - 1.0 + alpha) * STEP.as_secs_f64();
        let displayed_stage = match tick {
            4 => 0.12 * alpha,
            5 => 0.12 * (1.0 - alpha),
            _ => 0.0,
        };
        map.set_stage(0, displayed_stage).expect("sampled tide");
        let now = map.surf_sheet(&field, 0, time).expect("current sheet");
        let old = previous
            .surf_sheet(&field, 0, previous_time)
            .expect("previous sheet");
        let nodes = projected_nodes(&now, &old, matrix, [displayed_stage, previous_stage]);
        let witness = fixture.witness();
        let extent = Vec2::new(EXTENT[0] as f32, EXTENT[1] as f32);
        let mut samples = 0;
        for y in 22..40 {
            for x in 36..60 {
                let pixel = (Vec2::new(x as f32 + 0.5, y as f32 + 0.5) / extent * 2.0 - Vec2::ONE)
                    * Vec2::new(1.0, -1.0);
                let hits = cut_ray(&nodes, pixel);
                let Some(&(depth, motion)) = hits.first() else {
                    continue;
                };
                let actual = witness[y * EXTENT[0] as usize + x];
                assert!(actual[0] > 0.1, "missing water at {x}, {y}");
                let error = (actual[1] - depth).abs();
                assert!(
                    error < DEPTH_TOLERANCE,
                    "depth at {x}, {y}: {} vs {depth}",
                    actual[1]
                );
                max_depth_error = max_depth_error.max(error);
                if hits.len() >= 3 && depth - hits[1].0 > 4.0 * DEPTH_TOLERANCE {
                    folded_pixels += 1;
                }
                if index == 0 {
                    assert!(
                        actual[2].abs() > 65_000.0,
                        "first submission resets history"
                    );
                } else {
                    let error = (Vec2::new(actual[2], actual[3]) - motion).length();
                    assert!(
                        error < MOTION_TOLERANCE,
                        "motion at {x}, {y}: {actual:?} vs {motion:?}"
                    );
                    max_motion_error = max_motion_error.max(error);
                }
                samples += 1;
            }
        }
        assert!(
            samples >= 64,
            "only {samples} water pixels for frame {index}"
        );
        previous.clone_from(&map);
        previous_time = time;
        previous_stage = displayed_stage;
    }
    assert!(
        folded_pixels > 0,
        "the witness must select the nearest branch of an overhang"
    );
    eprintln!(
        "{folded_pixels} folded pixels, depth error {max_depth_error:.7}, motion error {max_motion_error:.5} px"
    );
}
}
