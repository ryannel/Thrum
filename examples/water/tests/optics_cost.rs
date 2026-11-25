//! Capture-off paired optics measurements at the scene's ordinary views.
use serde_json::json;
use std::path::Path;
use water_scene::island::Wind;

pub(super) fn run(gpu: &thrum_test_support::Gpu, output: &Path) {
    let mut cases = Vec::new();
    let mut views = [Wind::Calm, Wind::Gale]
        .into_iter()
        .flat_map(|wind| [0, 1, 9].map(|body_count| (wind, Some(body_count))))
        .collect::<Vec<_>>();
    views.push((Wind::Calm, None));
    let arms = match std::env::var("THRUM_WATER_OPTICS_COST_ARM").as_deref() {
        Ok("prior") => &[false][..],
        Ok("optics") => &[true][..],
        _ => &[false, true][..],
    };
    for (wind, body_count) in views {
        let mut runs = Vec::new();
        for &enabled in arms {
            let (game, report) = if let Some(count) = body_count {
                let scene = water_scene::floating::build_optics_arm(wind, count, enabled)
                    .expect("optics at sea");
                (scene.game, scene.report)
            } else {
                let (game, report, _, _) = water_scene::island::build_optics_arm(wind, enabled)
                    .expect("optics at the lake");
                (game, report)
            };
            let mut fixture = super::support::Fixture::from_game(gpu, [2560, 1440], game, report);
            if body_count.is_none() {
                fixture.parameters(r#"{"camera.island":1}"#);
            }
            fixture.warm_cooked();
            for i in 0..7 {
                fixture.frame(1.0 + f64::from(i) / 120.0);
            }
            let frames = (7..23)
                .map(|i| {
                    let frame = fixture.frame(1.0 + f64::from(i) / 120.0);
                    let mut row = super::sample_json(&frame);
                    row["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                    row
                })
                .collect::<Vec<_>>();
            runs.push(json!({
                "enabled": enabled, "frames": frames, "storage": fixture.host.storage_report(),
                "timing": format!("{:?}", fixture.host.timing_stats()),
                "resources": fixture.report.lock().expect("water report").resources
            }));
        }
        cases.push(json!({
            "view": if body_count.is_some() {"sea"} else {"lake"},
            "wind": format!("{wind:?}"), "body_count": body_count, "arms": runs,
        }));
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
