//! Compare submitted depth and motion against the forward f64 sheet.
#![cfg(target_os = "macos")]

mod support;
#[path = "support/surf.rs"]
mod surf_fixture;

use glam::{Mat4, Vec2, Vec3, Vec4};
use std::sync::{Arc, Mutex};
use surf_fixture::{CHOPPINESS, STEP, beach};
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use thrum_render::synthetic::SyntheticView;
use thrum_water::{Profile, Water};
use thrum_water_model::{Sheet, SheetPoint, SurfProfile};

const EXTENT: [u32; 2] = [96, 64];
const CENTER: [f32; 2] = [-28.705, 3.2795];
const SIDE: usize = 41;
const DEPTH_TOLERANCE: f32 = 0.0001;
const MOTION_TOLERANCE: f32 = 0.16;

#[derive(Clone, Copy)]
struct ProjectedNode {
    now: Vec4,
    before: Vec4,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "analytic world positions projected into the submitted f32 camera"
)]
fn projected_nodes(
    now: &impl Sheet,
    old: &impl Sheet,
    matrix: Mat4,
    level: [f64; 2],
) -> Vec<ProjectedNode> {
    let project = |p: SheetPoint, stage: f64| {
        matrix
            * Vec4::new(
                p.world[0] as f32,
                (p.height + stage) as f32,
                p.world[1] as f32,
                1.0,
            )
    };
    let origin = CENTER.map(|v| f64::from(v.floor()) - 5.0);
    (0..SIDE)
        .flat_map(|z| {
            (0..SIDE).map(move |x| [origin[0] + x as f64 * 0.25, origin[1] + z as f64 * 0.25])
        })
        .map(|rest| ProjectedNode {
            now: project(now.forward(rest), level[0]),
            before: project(old.forward(rest), level[1]),
        })
        .collect()
}

fn cut_ray(nodes: &[ProjectedNode], pixel: Vec2) -> Vec<(f32, Vec2)> {
    let mut hits = Vec::new();
    for row in 0..SIDE - 1 {
        for column in 0..SIDE - 1 {
            let a = row * SIDE + column;
            let d = a + SIDE + 1;
            for ids in [[a, a + 1, d], [a, d, a + SIDE]] {
                let triangle = ids.map(|i| nodes[i]);
                let ndc = triangle.map(|n| n.now.truncate() / n.now.w);
                let edges = [
                    ndc[1].truncate() - ndc[0].truncate(),
                    ndc[2].truncate() - ndc[0].truncate(),
                ];
                let from_a = pixel - ndc[0].truncate();
                let determinant = edges[0].perp_dot(edges[1]);
                if determinant.abs() < 1e-10 {
                    continue;
                }
                let u = from_a.perp_dot(edges[1]) / determinant;
                let v = edges[0].perp_dot(from_a) / determinant;
                let weights = [1.0 - u - v, u, v];
                if weights.iter().any(|w| *w < 0.0001) {
                    continue;
                }
                let mut previous = Vec4::ZERO;
                let mut depth = 0.0;
                for i in 0..3 {
                    depth += weights[i] * ndc[i].z;
                    previous += (weights[i] / triangle[i].now.w) * triangle[i].before;
                }
                let motion = (previous.truncate().truncate() / previous.w - pixel)
                    * Vec2::new(EXTENT[0] as f32 * 0.5, -(EXTENT[1] as f32) * 0.5);
                hits.push((depth, motion));
            }
        }
    }
    hits.sort_by(|a, b| b.0.total_cmp(&a.0));
    hits
}

#[test]
fn submitted_breaker_depth_motion_and_stage_history() {
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
        let frame = fixture.frame((f64::from(tick) + fraction) * STEP.as_secs_f64());
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
        for y in (24..40).step_by(2) {
            for x in (36..60).step_by(2) {
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
