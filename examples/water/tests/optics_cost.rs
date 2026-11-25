//! Capture-off paired optics measurements at the scene's ordinary views.
use serde_json::json;
use std::path::Path;
use thrum_render::runtime::{FrameWorld, SceneSource};
use water_scene::{floating::OPTICS_COST_CASTER, island::Wind};

pub(super) fn run(gpu: &thrum_test_support::Gpu, output: &Path, capture_views: bool) {
    let mut cases = Vec::new();
    let mut views = [Wind::Calm, Wind::Gale]
        .into_iter()
        .flat_map(|wind| [0, 1, 9].map(|body_count| (wind, Some(body_count))))
        .collect::<Vec<_>>();
    views.push((Wind::Calm, None));
    let arms = if capture_views {
        &[true][..]
    } else {
        match std::env::var("THRUM_WATER_OPTICS_COST_ARM").as_deref() {
            Ok("prior") => &[false][..],
            Ok("optics") => &[true][..],
            _ => &[false, true][..],
        }
    };
    for (wind, body_count) in views {
        let mut runs = Vec::new();
        for &enabled in arms {
            let (game, report, observe) = if let Some(count) = body_count {
                let scene = water_scene::floating::build_optics_arm(wind, count, enabled)
                    .expect("optics at sea");
                let observe = scene.observer();
                (scene.game, scene.report, Some(observe))
            } else {
                let (game, report, _, _) = water_scene::island::build_optics_arm(wind, enabled)
                    .expect("optics at the lake");
                (game, report, None)
            };
            let mut fixture = super::support::Fixture::from_game(gpu, [2560, 1440], game, report);
            if body_count.is_none() {
                fixture.parameters(r#"{"camera.island":1}"#);
            }
            fixture.warm_cooked();
            for i in 0..7 {
                fixture.frame(1.0 + f64::from(i) / 120.0);
            }
            if capture_views {
                let view =
                    body_count.map_or("lake".into(), |count| format!("sea-{wind:?}-{count}"));
                fixture
                    .host
                    .enable_submitted_capture_with_limits(1, super::limits())
                    .expect("cost-view tap");
                let frame = fixture.frame_without_timings(1.0 + 7.0 / 120.0);
                let image = super::take_image(&mut fixture);
                assert_eq!(image.sample.submission_order, frame.submission_order);
                std::fs::write(output.join(format!("{view}.rgba")), &image.rgba)
                    .expect("cost-view image");
                continue;
            }
            let mut visibility = None;
            let frames = (7..23)
                .map(|i| {
                    let frame = fixture.frame(1.0 + f64::from(i) / 120.0);
                    let mut row = super::sample_json(&frame);
                    row["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                    if i == 7
                        && let (Some(observer), Some(count)) = (&observe, body_count)
                    {
                        let view = fixture
                            .host
                            .source()
                            .frame(
                                fixture.host.world().source(),
                                frame.presentation_alpha,
                                fixture.extent,
                                None,
                            )
                            .expect("cost view")
                            .view;
                        let matrix = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
                        visibility = Some(check_view(matrix, &observer(), count));
                    }
                    row
                })
                .collect::<Vec<_>>();
            runs.push(json!({
                "enabled": enabled, "frames": frames, "storage": fixture.host.storage_report(),
                "timing": format!("{:?}", fixture.host.timing_stats()),
                "resources": fixture.report.lock().expect("water report").resources,
                "visibility": visibility
            }));
        }
        cases.push(json!({
            "view": if body_count.is_some() {"sea"} else {"lake"},
            "wind": format!("{wind:?}"), "body_count": body_count, "arms": runs,
        }));
    }
    if capture_views {
        return;
    }
    let data = json!({
        "schema": 1, "target": "mac-metal", "extent": [2560, 1440], "capture": false,
        "commit": std::env::var("THRUM_WATER_COMMIT").expect("benchmark revision"), "views": cases
    });
    std::fs::write(
        output.join("optics-cost.json"),
        serde_json::to_vec_pretty(&data).expect("json"),
    )
    .expect("retain optics comparison");
}

fn check_view(
    matrix: glam::Mat4,
    bodies: &[water_scene::floating::Body],
    count: usize,
) -> serde_json::Value {
    use glam::{Vec3, Vec4};

    let clip = |point: Vec3| matrix * point.extend(1.0);
    let caster = Vec3::from_array(OPTICS_COST_CASTER);
    // The unit cooked float fits within this box. A single separating plane
    // must exclude every corner; a test of its centre alone can miss a rim.
    let corners: [Vec4; 8] = core::array::from_fn(|index| {
        let offset = Vec3::from_array(core::array::from_fn(|axis| {
            if index & (1_usize << axis) == 0 {
                -0.51
            } else {
                0.51
            }
        }));
        clip(caster + offset)
    });
    let margins: [fn(Vec4) -> f32; 6] = [
        |p: Vec4| p.w + p.x,
        |p: Vec4| p.w - p.x,
        |p: Vec4| p.w + p.y,
        |p: Vec4| p.w - p.y,
        |p: Vec4| p.z,
        |p: Vec4| p.w - p.z,
    ];
    let separating_plane = margins
        .into_iter()
        .position(|side| corners.iter().all(|&point| side(point) < 0.0))
        .expect("shadow caster must be outside the submitted cost view");
    assert!(count <= bodies.len());
    let centres: Vec<_> = bodies
        .iter()
        .take(count)
        .map(|body| {
            let point = clip(body.position.as_vec3());
            assert!(
                margins.into_iter().all(|side| side(point) > 0.0),
                "a requested floating body is outside the submitted cost view: {:?}",
                body.position
            );
            (point.truncate() / point.w).to_array()
        })
        .collect();
    json!({"caster": OPTICS_COST_CASTER, "caster_clip_corners": corners.map(|p| p.to_array()),
        "separating_plane": separating_plane, "fleet_centres_ndc": centres,
        "view_projection": matrix.to_cols_array_2d()})
}
