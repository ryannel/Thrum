//! Opt-in submitted-output capture and paired GPU measurement driver.
#[cfg(target_os = "macos")]
mod realtime;
#[cfg(target_os = "macos")]
mod support;
use serde_json::json;
use std::{path::PathBuf, time::Duration};
use thrum_render::runtime::{CaptureLimits, FrameSample, FrameWorld, SceneSource, SubmittedImage};
use water_scene::floating::WhitewaterDemo;
use water_scene::island::Wind;

fn island_wind() -> Option<Wind> {
    if std::env::var("THRUM_WATER_LOCATION").is_ok_and(|value| value == "beach") {
        return foam_wind();
    }
    std::env::var("THRUM_WATER_ISLAND")
        .ok()
        .as_deref()
        .and_then(Wind::named)
}

fn proof_game(water: bool) -> (thrum_app::sky::SkyGame, water_scene::SharedReport) {
    if let Some(wind) = floating_wind() {
        let scene = floating_game(wind, water, water, foam_wind().is_some());
        return (scene.game, scene.report);
    }
    if let Some(wind) = island_wind() {
        let (game, report, _, _) =
            water_scene::island::build_foam(wind, water, foam_wind().is_some()).expect("island");
        (game, report)
    } else {
        water_scene::build_paired(
            std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
            water,
        )
        .expect("open sea")
    }
}

fn floating_game(
    wind: Wind,
    water: bool,
    wake: bool,
    foam: bool,
) -> water_scene::floating::FloatingScene {
    let speed = std::env::var("THRUM_WATER_SPEED")
        .expect("craft speed")
        .parse()
        .expect("valid speed");
    let view = std::env::var("THRUM_WATER_VIEW").unwrap_or_default();
    let chase = view == "chase";
    if wake_wind().is_some() || foam_wind().is_some() {
        let lake = std::env::var("THRUM_WATER_LOCATION").is_ok_and(|v| v == "lake");
        return water_scene::floating::build_wake_paired(
            wind,
            lake,
            speed,
            std::f32::consts::PI,
            water_scene::floating::WakeDemo {
                view: water_scene::floating::WakeView::named(&view),
                stone: std::env::var_os("THRUM_WATER_STONE").is_some(),
                disabled: !wake,
                body_capacity: None,
                whitewater: match (
                    foam,
                    std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
                ) {
                    (false, _) => WhitewaterDemo::Off,
                    (true, false) => WhitewaterDemo::Procedural,
                    (true, true) => WhitewaterDemo::Cooked,
                },
                whitewater_lighting: foam_wind().is_some(),
            },
            water,
        )
        .expect("wake scene");
    }
    let heading = if chase {
        std::f32::consts::FRAC_PI_2
    } else {
        std::f32::consts::PI
    };
    water_scene::floating::build_paired(wind, false, speed, heading, chase, water)
        .expect("floating scene")
}

fn floating_wind() -> Option<Wind> {
    let foam =
        foam_wind().filter(|_| !std::env::var("THRUM_WATER_LOCATION").is_ok_and(|v| v == "beach"));
    foam.or_else(wake_wind).or_else(|| {
        std::env::var("THRUM_WATER_BODIES")
            .ok()
            .as_deref()
            .and_then(Wind::named)
    })
}

fn foam_wind() -> Option<Wind> {
    std::env::var("THRUM_WATER_FOAM")
        .ok()
        .as_deref()
        .and_then(Wind::named)
}

fn wake_wind() -> Option<Wind> {
    std::env::var("THRUM_WATER_WAKE")
        .ok()
        .as_deref()
        .and_then(Wind::named)
}
thrum_test_support::macos_gpu_test! {
fn proof_driver() {
    let Some(gpu) = thrum_test_support::gpu("proof_driver", Some(wgpu::Backend::Metal)) else {
        return;
    };
    if std::env::var_os("THRUM_WATER_PROOF").is_some() {
        driver(&gpu);
    }
}
}
#[expect(
    clippy::too_many_lines,
    reason = "the three submitted proof modes share target setup and the saved report"
)]
#[cfg(target_os = "macos")]
fn driver(gpu: &thrum_test_support::Gpu) {
    let mode = std::env::var("THRUM_WATER_PROOF").expect("mode");
    let directory = PathBuf::from(std::env::var("THRUM_WATER_OUTPUT").expect("output"));
    std::fs::create_dir_all(&directory).expect("output folder");
    if mode == "realtime" {
        let result = realtime::capture(gpu, &directory);
        std::fs::write(
            directory.join("realtime.json"),
            serde_json::to_vec_pretty(&result).expect("json"),
        )
        .expect("record");
        return;
    }
    let mut fixture = if let Some(wind) = floating_wind() {
        let scene = floating_game(wind, true, true, foam_wind().is_some());
        support::Fixture::from_game(gpu, [2560, 1440], scene.game, scene.report)
    } else if island_wind().is_some() {
        let (game, report) = proof_game(true);
        let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
        fixture.warm_cooked();
        fixture
    } else if std::env::var("THRUM_WATER_RECIPE").is_ok_and(|v| v == "portable") {
        support::Fixture::antialiased(gpu, [2560, 1440])
    } else {
        support::Fixture::new(gpu, [2560, 1440])
    };
    for index in 0..8 {
        fixture.frame(f64::from(index) / 120.0);
    }
    let report = fixture.report.lock().expect("report").clone();
    let mut result = json!({"schema":1,"target":"mac-metal","mode":mode,"extent":fixture.extent,"vertices":report.vertices,"resources":report.resources});
    if let Some(wind) = island_wind() {
        result["wind"] = json!(format!("{wind:?}"));
        result["body_vertices"] = json!(report.bodies);
        result["cameras"] = json!(water_scene::island::CAMERAS.map(|c| json!({"position":c.position,"target":c.target,"fov_degrees":c.vertical_fov_degrees})));
    }
    result["recipe"] =
        json!(std::env::var("THRUM_WATER_RECIPE").unwrap_or_else(|_| "native".into()));
    result["camera"] = json!({"focus_xz":water_scene::CAMERA_FOCUS,"eye_at_2m_along_wind":water_scene::camera_position(2.0,90.0),"vertical_fov_degrees":water_scene::CAMERA_FOV_DEGREES,"look_drop_over_100_m":water_scene::CAMERA_LOOK_DROP,"orbit_radius":"24m at 2m height, 40m at 40m height; bearing orbits the focus"});
    result["commit"] = json!(
        std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_| "working tree diagnostic".into())
    );
    match mode.as_str() {
        "cost" => {
            let mut arms = Vec::new();
            let wake = wake_wind().is_some();
            let whitewater = foam_wind().is_some();
            let beach = island_wind().is_some();
            let variants: &[(bool, bool, bool)] = if whitewater {
                &[
                    (true, !beach, true),
                    (true, !beach, false),
                    (false, false, false),
                ]
            } else if wake {
                &[
                    (true, true, false),
                    (true, false, false),
                    (false, false, false),
                ]
            } else {
                &[(true, false, false), (false, false, false)]
            };
            for &(present, interaction, foam) in variants {
                let mut fixture = if let Some(wind) = floating_wind() {
                    let scene = floating_game(wind, present, interaction, foam);
                    let mut fixture =
                        support::Fixture::from_game(gpu, [2560, 1440], scene.game, scene.report);
                    if foam && std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some() {
                        fixture.warm_cooked();
                    }
                    fixture
                } else if let Some(wind) = island_wind() {
                    let (game, report, _, _) = water_scene::island::build_foam(wind, present, foam)
                        .expect("island cost arm");
                    let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
                    fixture.warm_cooked();
                    if whitewater {
                        fixture.parameters(r#"{"camera.island":0}"#);
                    }
                    fixture
                } else {
                    support::Fixture::paired(gpu, [2560, 1440], present)
                };
                for index in 0..8 {
                    fixture.frame(f64::from(index) / 120.0);
                }
                let mut frames = Vec::new();
                for index in 0..32 {
                    let start = 1.0;
                    let sample = fixture.frame(start + f64::from(index) / 120.0);
                    if index >= 8 {
                        let mut record = sample_json(&sample);
                        record["completion_upper_ms"] =
                            json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                        let report = fixture.report.lock().expect("report");
                        if let Some(wake) = report.interaction {
                            record["interaction"] = json!({
                                "endpoints": wake.endpoints,
                                "regions": wake.regions,
                                "solves": wake.solves,
                                "labelling_cpu_ms": wake.labelling_ms,
                                "refused_regions": wake.refused_regions,
                            });
                        }
                        if let Some(foam) = report.foam {
                            record["foam"] = json!({"endpoints":foam.ticks, "dropped":foam.dropped_ticks, "pattern":report.foam_pattern});
                        }
                        frames.push(record);
                    }
                }
                let pixels = if present {
                    fixture
                        .witness()
                        .iter()
                        .filter(|p| p[0] > 0.1 && p[1] > 0.0)
                        .count()
                } else {
                    0
                };
                let mut arm = json!({"water":present,"calibration_float":island_wind().is_none() && floating_wind().is_none(),"frames":frames,"water_pixels_shaded":pixels,"pixel_count_method":"visible coverage from submitted reactive/depth witness; helper and hidden invocations excluded","storage":fixture.host.storage_report()});
                if wake || whitewater {
                    arm["wake"] = json!(interaction);
                    let report = fixture.report.lock().expect("water report");
                    arm["water_vertices"] = json!(report.vertices);
                    arm["water_resources"] = json!(report.resources);
                }
                if whitewater {
                    arm["foam"] = json!(foam);
                }
                if present && beach && !whitewater {
                    let counts = island_pixels(&fixture);
                    assert_eq!(counts.iter().sum::<usize>(), pixels);
                    arm["body_pixels"] = json!(counts);
                }
                arms.push(arm);
            }
            result["capture"] = json!(false);
            result["arms"] = json!(arms);
        }
        "fixed" => {
            let mut images = Vec::new();
            let views = [
                ("noon-2-along", 2, 90, 60),
                ("noon-2-across", 2, 0, 60),
                ("noon-40-along", 40, 90, 60),
                ("noon-40-across", 40, 0, 60),
                ("low-sun", 2, 90, 6),
                ("twilight", 2, 90, -4),
            ];
            let checkpoints: Vec<_> = if foam_wind().is_some() {
                let beach = island_wind().is_some();
                let views: &[(&str, u32)] = if beach {
                    &[("beach", 0), ("overhead", 6), ("lake", 1)]
                } else {
                    &[("sea", 0)]
                };
                let mut checkpoints = Vec::new();
                for &(name, view) in views {
                    let params = if beach {
                        format!(r#"{{"camera.island":{view}}}"#)
                    } else {
                        "{}".into()
                    };
                    checkpoints.extend((0..3).map(|phase| {
                        (
                            format!("{name}-{phase}"),
                            params.clone(),
                            16.0 + f64::from(phase) * 0.7 + 24.0 * f64::from(view),
                        )
                    }));
                }
                checkpoints.sort_by(|a, b| a.2.total_cmp(&b.2));
                checkpoints
            } else if island_wind().is_some() {
                ["beach", "lake", "pond", "hill"]
                    .into_iter()
                    .zip(0..)
                    .flat_map(|(name, view)| {
                        (0..3).map(move |phase| {
                            (
                                format!("{name}-{phase}"),
                                format!(r#"{{"camera.island":{view}}}"#),
                                10.0 + f64::from(view * 6 + phase) * 1.25,
                            )
                        })
                    })
                    .collect()
            } else {
                views.into_iter().map(|(name,height,bearing,sun)| (name.to_owned(),
                    format!(r#"{{"camera.height":{height},"camera.bearing":{bearing},"sun.elevation":{sun}}}"#),
                    10.0,
                )).collect()
            };
            for (session, (name, params, time)) in (2_u64..).zip(checkpoints) {
                fixture.parameters(&params);
                if foam_wind().is_some() {
                    let steps = if name.ends_with("-0") { 72 } else { 7 };
                    for index in (1..=steps).rev() {
                        fixture.frame_without_timings(time - f64::from(index) * 0.10);
                    }
                }
                fixture
                    .host
                    .enable_submitted_capture_with_limits(session, limits())
                    .expect("fixed checkpoint tap");
                let sample = fixture.frame(time);
                let eye = fixture
                    .host
                    .source()
                    .frame(
                        fixture.host.world().source(),
                        sample.presentation_alpha,
                        fixture.extent,
                        None,
                    )
                    .expect("submitted view")
                    .view
                    .camera_position();
                let image = take_image(&mut fixture);
                assert_eq!(image.sample.submission_order, sample.submission_order);
                assert_eq!(image.sample.tick, sample.tick);
                let file = format!("{name}.rgba");
                std::fs::write(directory.join(&file), &image.rgba).expect("fixed image");
                images.push(json!({"name":name,"file":file,"camera":eye,"sample":sample_json(&sample),"exposure_words":image.exposure_gpu_words,"submitted_elapsed":image.submitted_elapsed.as_secs_f64()}));
                fixture.host.recycle_submitted_image(image);
                fixture.host.disable_submitted_capture();
            }
            result["images"] = json!(images);
        }
        _ => panic!("unknown proof mode"),
    }
    std::fs::write(
        directory.join(format!("{mode}.json")),
        serde_json::to_vec_pretty(&result).expect("json"),
    )
    .expect("record");
}

#[cfg(target_os = "macos")]
fn island_pixels(fixture: &support::Fixture) -> [usize; 3] {
    let c = water_scene::island::CAMERAS[3];
    let view = thrum_render::synthetic::SyntheticView::perspective(
        c.position,
        c.target,
        [0.0, 1.0, 0.0],
        c.vertical_fov_degrees.to_radians(),
        fixture.extent[0] as f32 / fixture.extent[1] as f32,
        0.1,
        30_000.0,
    )
    .expect("island camera");
    let inverse = glam::Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    let mut counts = [0; 3];
    for (i, sample) in fixture.witness().into_iter().enumerate() {
        if sample[0] <= 0.1 || sample[1] <= 0.0 {
            continue;
        }
        let width = fixture.extent[0] as usize;
        let x = ((i % width) as f32 + 0.5) * 2.0 / fixture.extent[0] as f32 - 1.0;
        let y = 1.0 - ((i / width) as f32 + 0.5) * 2.0 / fixture.extent[1] as f32;
        let height = inverse.project_point3(glam::vec3(x, y, sample[1])).y;
        // Body elevations differ by much more than the sheltered wave heights
        // and the half-float witness's reconstruction error.
        let body = if height < 5.0 {
            0
        } else if height < 11.3 {
            2
        } else {
            1
        };
        counts[body] += 1;
    }
    counts
}
fn limits() -> CaptureLimits {
    CaptureLimits {
        staging_slots: 3,
        cpu_buffers: 16,
        byte_allowance: 300 * 1024 * 1024,
    }
}
#[cfg(target_os = "macos")]
fn take_image(fixture: &mut support::Fixture) -> SubmittedImage {
    thrum_test_support::wait::until("tap", Duration::from_secs(30), || {
        fixture
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("tap poll");
        fixture.host.try_submitted_image().expect("image")
    })
}
fn sample_json(sample: &FrameSample) -> serde_json::Value {
    let Some(gpu) = sample.gpu_pass_ns.as_ref() else {
        return json!({"tick":sample.tick,"alpha":sample.presentation_alpha,"tick_frame":sample.completed_ticks>0,"submission":sample.submission_order,"gpu_ms":null,"passes_ms":{},"timing_fault":sample.timing_fault,"timing_requested":sample.timing_requested,"cpu_wall_ms":sample.wall.as_secs_f64()*1000.0});
    };
    let mut passes: std::collections::BTreeMap<_, _> = gpu
        .passes()
        .iter()
        .map(|p| (p.name.to_string(), p.nanoseconds / 1e6))
        .collect();
    if let Some(present) = sample.gpu_present_ns {
        passes.insert(thrum_render::present::PASS_NAME.into(), present / 1e6);
    }
    json!({"tick":sample.tick,"alpha":sample.presentation_alpha,"tick_frame":sample.completed_ticks>0,"submission":sample.submission_order,"gpu_ms":(gpu.sum()+sample.gpu_present_ns.unwrap_or(0.0))/1e6,"scene_gpu_ms":gpu.sum()/1e6,"present_gpu_ms":sample.gpu_present_ns.map(|v|v/1e6),"gpu_scope":"timed scene and surface-present pass sum; untimed copies and query readback are not imputed as zero","cpu_wall_ms":sample.wall.as_secs_f64()*1000.0,"passes_ms":passes})
}
