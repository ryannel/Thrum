//! Opt-in submitted-output capture and paired GPU measurement driver.
#[cfg(target_os = "macos")]
mod ground_inspection;
#[cfg(target_os = "macos")]
mod optics_cost;
#[cfg(target_os = "macos")]
mod rain_cost;
#[cfg(target_os = "macos")]
mod rain_inspection;
#[cfg(target_os = "macos")]
mod realtime;
#[cfg(target_os = "macos")]
mod river_inspection;
#[cfg(target_os = "macos")]
mod support;
#[cfg(target_os = "macos")]
mod surf_cost;
#[cfg(target_os = "macos")]
mod surf_running;
#[cfg(target_os = "macos")]
use serde_json::json;
#[cfg(target_os = "macos")]
use std::{path::PathBuf, time::Duration};
#[cfg(target_os = "macos")]
use thrum_render::runtime::{CaptureLimits, FrameSample, FrameWorld, SceneSource, SubmittedImage};
#[cfg(target_os = "macos")]
use water_scene::island::{DiveSite, Wind};
#[cfg(target_os = "macos")]
use water_scene::settings::{Floating, Settings};

/// The scene settings, parsed from this process's environment as the window parses its own.
#[cfg(target_os = "macos")]
fn settings() -> Settings {
    Settings::from_variables(|name| std::env::var(name).ok())
}

#[cfg(target_os = "macos")]
fn island_wind() -> Option<Wind> {
    match settings() {
        Settings::Island { wind, .. } | Settings::Surf(wind) | Settings::WetShore(wind) => {
            Some(wind)
        }
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn channel_route() -> bool {
    matches!(settings(), Settings::Channel(_))
}

#[cfg(target_os = "macos")]
fn river_route() -> bool {
    matches!(settings(), Settings::River(_))
}

#[cfg(target_os = "macos")]
fn river_cost_view() -> usize {
    match settings() {
        Settings::River(Wind::Calm) => 3,
        _ => 14,
    }
}

#[cfg(target_os = "macos")]
fn surf_route() -> bool {
    matches!(settings(), Settings::Surf(_) | Settings::WetShore(_))
}

#[cfg(target_os = "macos")]
fn floating() -> Option<Floating> {
    match settings() {
        Settings::Floating(floating) => Some(floating),
        _ => None,
    }
}

/// The wake route, chosen by a wake or by foam away from the beach.
#[cfg(target_os = "macos")]
fn wake_route() -> bool {
    floating().is_some_and(|floating| floating.wake.is_some())
}

/// Transported whitewater, on the beach or on the wake route.
#[cfg(target_os = "macos")]
fn whitewater() -> bool {
    match settings() {
        Settings::Island { foam, .. } => foam,
        Settings::Surf(_) | Settings::WetShore(_) => true,
        Settings::Floating(floating) => floating.wake.is_some_and(|wake| wake.whitewater),
        Settings::OpenSea
        | Settings::OpticsSea(_)
        | Settings::OpticsShade(_)
        | Settings::Dive(_, _)
        | Settings::DiveShade(_, _)
        | Settings::RainYard(_)
        | Settings::Channel(_)
        | Settings::River(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn proof_game(water: bool) -> (thrum_app::sky::SkyGame, water_scene::SharedReport) {
    match settings() {
        Settings::Floating(floating) => {
            let scene = floating_game(floating, water, water, whitewater());
            (scene.game, scene.report)
        }
        Settings::Island { wind, foam } => {
            let (game, report, _, _) =
                water_scene::island::build_foam(wind, water, foam).expect("island");
            (game, report)
        }
        Settings::Dive(wind, site) => {
            if std::env::var("THRUM_WATER_DIVE_LIGHT").as_deref() == Ok("off") {
                water_scene::island::build_dive_light_cost(wind, site, false)
                    .expect("dive comparison")
            } else {
                water_scene::island::build_dive(wind, site, water).expect("dive scene")
            }
        }
        Settings::DiveShade(wind, site) => {
            water_scene::island::build_dive_shade(wind, site).expect("shaded dive scene")
        }
        Settings::RainYard(wind) => {
            let (game, report, _, _) =
                water_scene::island::build_rain_yard(wind, water).expect("rain yard");
            (game, report)
        }
        Settings::WetShore(wind) => {
            let (game, report, _, _) =
                water_scene::island::build_wet_shore(wind, water).expect("wet shore");
            (game, report)
        }
        Settings::River(wind) => {
            let scene = water_scene::river::build(wind, water, Some(&support::river_store()))
                .expect("river scene");
            let game = if let Some(shot) = river_inspection::shot() {
                scene.game.with_presentation_camera(move |_| shot.camera)
            } else {
                scene.game
            };
            (game, scene.report)
        }
        Settings::Channel(wind) => {
            let scene = water_scene::channel::build(wind, water, true).expect("channel scene");
            (scene.game, scene.report)
        }
        Settings::Surf(wind) => {
            let (game, report, _, _) =
                water_scene::island::build_surf(wind, water).expect("surf island");
            (game, report)
        }
        Settings::OpticsSea(wind) => water_scene::build_optics_sea(
            wind,
            std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
            water,
        )
        .expect("optics sea"),
        Settings::OpticsShade(wind) => water_scene::build_optics_shade(
            wind,
            std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
            water,
        )
        .expect("optics shade"),
        Settings::OpenSea => water_scene::build_paired(
            std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
            water,
        )
        .expect("open sea"),
    }
}

/// Build one arm of the floating scene: `wake` and `foam` switch the producers off for a paired arm.
#[cfg(target_os = "macos")]
fn floating_game(
    floating: Floating,
    water: bool,
    wake: bool,
    foam: bool,
) -> water_scene::floating::FloatingScene {
    let Floating {
        wind,
        lake,
        speed,
        heading,
        chase,
        ..
    } = floating;
    if let Some(route) = floating.wake {
        let cooked = std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some();
        let demo = water_scene::settings::Wake {
            whitewater: foam,
            ..route
        }
        .demo(cooked);
        return water_scene::floating::build_wake_paired(
            wind,
            lake,
            speed,
            heading,
            water_scene::floating::WakeDemo {
                disabled: !wake,
                whitewater_lighting: route.whitewater,
                ..demo
            },
            water,
        )
        .expect("wake scene");
    }
    water_scene::floating::build_paired(wind, lake, speed, heading, chase, water)
        .expect("floating scene")
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
    if matches!(mode.as_str(), "lake-film" | "pond-film") {
        std::fs::write(
            directory.join("fixed.json"),
            serde_json::to_vec_pretty(&ground_inspection::basin_film(
                gpu,
                &directory,
                mode == "pond-film",
            ))
            .expect("basin checkpoints JSON"),
        )
        .expect("basin film record");
        return;
    }
    if mode == "rain-ground-cost" {
        std::fs::write(
            directory.join("rain-ground-cost.json"),
            serde_json::to_vec_pretty(&ground_inspection::cost(gpu)).expect("cost JSON"),
        )
        .expect("cost record");
        return;
    }
    let wet_shore = match settings() {
        Settings::WetShore(wind) => Some(wind),
        _ => None,
    };
    if mode == "fixed"
        && (wet_shore.is_some() || std::env::var_os("THRUM_WATER_WET_GROUND_FIXED").is_some())
    {
        std::fs::write(
            directory.join("fixed.json"),
            serde_json::to_vec_pretty(&ground_inspection::fixed(gpu, &directory, wet_shore))
                .expect("checkpoints JSON"),
        )
        .expect("fixed record");
        return;
    }
    if mode == "rain-rings-inspection" {
        rain_inspection::sequences(gpu, &directory);
        return;
    }
    if mode == "rain-footfall-cost" {
        rain_inspection::footfall_costs(gpu, &directory);
        return;
    }
    if mode == "rain-rings-cost" {
        rain_inspection::costs(gpu, &directory);
        return;
    }
    if mode == "rain-puddles-cost" {
        let record = rain_cost::measure(gpu);
        std::fs::write(
            directory.join("rain-puddles-cost.json"),
            serde_json::to_vec_pretty(&record).expect("cost record"),
        )
        .expect("save cost record");
        return;
    }
    if mode == "realtime" {
        let result = realtime::capture(gpu, &directory);
        std::fs::write(
            directory.join("realtime.json"),
            serde_json::to_vec_pretty(&result).expect("json"),
        )
        .expect("record");
        return;
    }
    if mode == "dive-cost" {
        let (wind, site) = settings()
            .dive()
            .expect("dive cost requires THRUM_WATER_DIVE");
        assert!(
            matches!(site, DiveSite::Lake | DiveSite::Sea),
            "use the recorded lake or sea cost site"
        );
        let location = site.name();
        let (game, report) =
            water_scene::island::build_dive_cost(wind, site).expect("dive cost scene");
        let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
        fixture.warm_cooked();
        let views: Vec<_> = [("above", -2.0), ("half", 0.0), ("under", 2.0)]
            .into_iter()
            .zip([1.0, 2.0, 3.0])
            .map(|((name, depth), start)| {
                fixture.parameters(&format!(
                    r#"{{"camera.depth":{depth},"camera.pitch":0,"sun.elevation":60}}"#
                ));
                let mut frames = Vec::new();
                for index in 0..16 {
                    let frame = fixture.frame(start + f64::from(index) / 120.0);
                    if index >= 4 {
                        frames.push(sample_json(&frame));
                    }
                }
                json!({ "view": name, "depth_m": depth, "frames": frames })
            })
            .collect();
        let record = json!({
            "target": "mac-metal", "extent": [2560, 1440], "capture": false,
            "commit": std::env::var("THRUM_WATER_COMMIT").expect("cost revision"),
            "wind": format!("{wind:?}"), "location": location, "views": views,
        });
        std::fs::write(
            directory.join(format!("{location}-dive-cost.json")),
            serde_json::to_vec_pretty(&record).expect("cost JSON"),
        )
        .expect("retain lake dive cost");
        return;
    }
    if mode == "under-light-cost" {
        under_light_cost(gpu, &directory);
        return;
    }
    if mode == "dive-wake-cost" {
        dive_wake_cost(gpu, &directory);
        return;
    }
    if matches!(mode.as_str(), "optics-cost" | "optics-cost-views") {
        optics_cost::run(gpu, &directory, mode == "optics-cost-views");
        return;
    }
    if mode == "surf-cost" || mode == "surf-running" {
        let (file, result) = if mode == "surf-cost" {
            ("surf-cost.json", surf_cost::run(gpu))
        } else {
            ("surf-running.json", surf_running::run(gpu))
        };
        std::fs::write(
            directory.join(file),
            serde_json::to_vec_pretty(&result).expect("json"),
        )
        .expect("record");
        return;
    }
    let mut yard_map = None;
    let mut fixture = if let Settings::RainYard(wind) = settings() {
        let (game, report, map, _) =
            water_scene::island::build_rain_yard(wind, true).expect("rain yard");
        yard_map = Some(map);
        let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
        fixture.warm_cooked();
        fixture
    } else if let Some(floating) = floating() {
        let scene = floating_game(floating, true, true, whitewater());
        support::Fixture::from_game(gpu, [2560, 1440], scene.game, scene.report)
    } else if channel_route()
        || river_route()
        || island_wind().is_some()
        || settings().dive().is_some()
        || matches!(
            settings(),
            Settings::OpticsSea(_) | Settings::OpticsShade(_)
        )
    {
        let (game, report) = proof_game(true);
        let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
        if std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some() {
            fixture.warm_cooked();
        }
        fixture
    } else if std::env::var("THRUM_WATER_RECIPE").is_ok_and(|v| v == "portable") {
        support::Fixture::antialiased(gpu, [2560, 1440])
    } else {
        support::Fixture::new(gpu, [2560, 1440])
    };
    if river_route() {
        fixture.parameters(&format!(r#"{{"camera.river":{}}}"#, river_cost_view()));
    }
    for index in 0..8 {
        fixture.frame(f64::from(index) / 120.0);
    }
    let report = fixture.report.lock().expect("report").clone();
    let mut result = json!({"schema":1,"target":"mac-metal","mode":mode,"extent":fixture.extent,"vertices":report.vertices,"resources":report.resources});
    if let Some((wind, site)) = settings().dive() {
        result["wind"] = json!(format!("{wind:?}"));
        result["dive_site"] = json!(site.name());
    }
    if let Some(wind) = island_wind() {
        result["wind"] = json!(format!("{wind:?}"));
        result["body_vertices"] = json!(report.bodies);
        let cameras: &[_] = if surf_route() {
            &water_scene::island::SURF_VIEWS
        } else {
            &water_scene::island::CAMERAS
        };
        result["cameras"] = json!(cameras.iter().map(|c| json!({
            "position": c.position, "target": c.target, "fov_degrees": c.vertical_fov_degrees
        })).collect::<Vec<_>>());
    }
    if channel_route() {
        result["body_vertices"] = json!(report.bodies);
        result["camera"] = json!({"position":water_scene::channel::VIEWS[2].position,
            "target": water_scene::channel::VIEWS[2].target});
    }
    result["recipe"] =
        json!(std::env::var("THRUM_WATER_RECIPE").unwrap_or_else(|_| "native".into()));
    if river_route() {
        let (_, camera) = water_scene::river::VIEWS[river_cost_view()];
        result["body_vertices"] = json!(report.bodies);
        result["camera"] = json!({"position": camera.position, "target": camera.target});
    }
    if !channel_route() && !river_route() {
        result["camera"] = json!({"focus_xz":water_scene::CAMERA_FOCUS,"eye_at_2m_along_wind":water_scene::camera_position(2.0,90.0),"vertical_fov_degrees":water_scene::CAMERA_FOV_DEGREES,"look_drop_over_100_m":water_scene::CAMERA_LOOK_DROP,"orbit_radius":"24m at 2m height, 40m at 40m height; bearing orbits the focus"});
    }
    result["commit"] = json!(
        std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_| "working tree diagnostic".into())
    );
    match mode.as_str() {
        "cost" => {
            let mut arms = Vec::new();
            let wake = wake_route();
            let whitewater = whitewater();
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
                let mut fixture = if let Some(floating) = floating() {
                    let scene = floating_game(floating, present, interaction, foam);
                    let mut fixture =
                        support::Fixture::from_game(gpu, [2560, 1440], scene.game, scene.report);
                    if foam && std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some() {
                        fixture.warm_cooked();
                    }
                    fixture
                } else if channel_route() || river_route() {
                    let (game, report) = proof_game(present);
                    let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
                    fixture.warm_cooked();
                    if river_route() {
                        fixture.parameters(&format!(r#"{{"camera.river":{}}}"#, river_cost_view()));
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
                let mut arm = json!({"water":present,"calibration_float":island_wind().is_none() && floating().is_none() && !channel_route() && !river_route(),"frames":frames,"water_pixels_shaded":pixels,"pixel_count_method":"visible coverage from submitted reactive/depth witness; helper and hidden invocations excluded","storage":fixture.host.storage_report()});
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
            let checkpoints: Vec<_> = if matches!(settings(), Settings::RainYard(_)) {
                yard_checkpoints()
            } else if river_route()
                && let Some(shot) = river_inspection::shot()
            {
                vec![(shot.name, "{}".into(), shot.start + 0.8)]
            } else if river_route() {
                water_scene::river::VIEWS
                    .iter()
                    .zip(0..)
                    .map(|((name, _), view)| {
                        (
                            name.to_lowercase().replace(' ', "-"),
                            format!(r#"{{"camera.river":{view}}}"#),
                            10.0 + f64::from(view),
                        )
                    })
                    .collect()
            } else if surf_route() {
                surf_checkpoints()
            } else if whitewater() {
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
            } else if std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("optics")
                && (floating().is_some() || island_wind().is_some())
            {
                optics_checkpoints()
            } else if let Some((_, site)) = settings().dive()
                && std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("under-light")
            {
                under_light_checkpoints(site)
            } else if let Some((_, site)) = settings().dive()
                && std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("dive-crossing-detail")
            {
                // Supplement the elapsed-time take with a denser look at its entry.
                let clock_offset = std::env::var("THRUM_WATER_DIVE_CLOCK_OFFSET")
                    .expect("crossing clock offset from the realtime receipt")
                    .parse::<f64>()
                    .expect("crossing clock offset in seconds");
                (42..=93).map(|index| {
                    let elapsed = f64::from(index) / 30.0;
                    let depth = realtime::dive_crossing_depth(elapsed, site.crossing_depth());
                    (
                        format!("entry-{index:02}"),
                        format!(r#"{{"camera.depth":{depth:.4},"camera.pitch":0,"camera.setback":4.2,"camera.surface_follow":1,"sun.elevation":1}}"#),
                        elapsed + clock_offset,
                    )
                }).collect()
            } else if settings().dive().is_some()
                && std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("caustic-window")
            {
                [
                    (0.0, -50), (0.6, -50), (1.2, -50), (1.8, -50),
                    (5.5, -12), (7.0, -12), (8.5, -12), (10.0, -12),
                ].into_iter().enumerate().map(|(index, (setback, pitch))| (
                    format!("window-{index}"),
                    format!(r#"{{"camera.depth":1.8,"camera.pitch":{pitch},"camera.setback":{setback},"sun.elevation":60}}"#),
                    10.0,
                )).collect()
            } else if let Some((_, site @ (DiveSite::Sea | DiveSite::DeepSea | DiveSite::Lake))) =
                settings().dive()
                && std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("dive-depths")
            {
                dive_depth_checkpoints(site)
            } else if settings().dive().is_some()
                && std::env::var("THRUM_WATER_ROUTE").as_deref() == Ok("dive-waterline")
            {
                (-18..=18)
                    .map(|step| {
                        let depth = f64::from(step) * 0.025;
                        (
                            format!("crossing-{step:+04}"),
                            format!(
                                r#"{{"camera.depth":{depth},"camera.pitch":0,"sun.elevation":1}}"#
                            ),
                            10.0,
                        )
                    })
                    .collect()
            } else if let Some((_, site)) = settings().dive() {
                let mut views = vec![
                    ("up-1m", 1.0, 89.99, 60.0),
                    ("inclined-1m", 1.0, 45.0, 60.0),
                    ("level-0.5m", 0.5, 0.0, 60.0),
                    ("down-3m", 3.0, -35.0, 60.0),
                    ("dawn-level-3m", 3.0, 0.0, 1.0),
                    ("twilight-level-3m", 3.0, 0.0, -5.0),
                ];
                if site == DiveSite::Swash {
                    views = vec![
                        ("above-swash", -0.35, 0.0, 60.0),
                        ("split-swash", 0.0, 0.0, 60.0),
                        ("under-swash", 0.25, 25.0, 60.0),
                    ];
                } else if matches!(site, DiveSite::Sea | DiveSite::HighLake) {
                    views.extend([
                        ("up-5m", 5.0, 89.99, 60.0),
                        ("inclined-5m", 5.0, 45.0, 60.0),
                    ]);
                    if site == DiveSite::Sea {
                        views.extend([
                            ("level-10m", 10.0, 0.0, 60.0),
                            ("down-10m", 10.0, -35.0, 60.0),
                        ]);
                    }
                }
                views.into_iter().map(|(name, depth, pitch, sun)| (
                    name.into(),
                    format!(r#"{{"camera.depth":{depth},"camera.pitch":{pitch},"sun.elevation":{sun}}}"#),
                    10.0,
                )).collect()
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
            } else if matches!(
                settings(),
                Settings::OpticsSea(_) | Settings::OpticsShade(_)
            ) {
                let mut views: Vec<_> = [("dawn", 1.0), ("noon", 60.0), ("low-sun", 6.0), ("twilight", -5.0), ("night", -18.0)]
                    .into_iter().flat_map(|(name,sun)| [2,40].map(|height| {
                        let pitch = if name == "noon" { -33.024 } else { -18.004 };
                        (format!("{name}-{height}"), format!(r#"{{"camera.height":{height},"camera.bearing":90,"camera.pitch":{pitch},"sun.elevation":{sun}}}"#), 10.0)
                    })).collect();
                if std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some() {
                    views.push((
                        "submerged-noon-6".into(),
                        r#"{"camera.height":6,"camera.bearing":90,"camera.pitch":-33.024,"sun.elevation":60}"#.into(),
                        10.0,
                    ));
                }
                views
            } else {
                views.into_iter().map(|(name,height,bearing,sun)| (name.to_owned(),
                    format!(r#"{{"camera.height":{height},"camera.bearing":{bearing},"sun.elevation":{sun}}}"#),
                    10.0,
                )).collect()
            };
            for (session, (name, params, time)) in (2_u64..).zip(checkpoints) {
                fixture.parameters(&params);
                if whitewater() && !surf_route() {
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
                let mut record = json!({"name":name,"file":file,"camera":eye,"sample":sample_json(&sample),"exposure_words":image.exposure_gpu_words,"submitted_elapsed":image.submitted_elapsed.as_secs_f64()});
                if let Some(map) = &yard_map {
                    let levels = rain_cost::levels(&map.lock().expect("yard map"));
                    if name.starts_with("yard-brim-") {
                        assert!(
                            rain_cost::at_brim(levels),
                            "fixed frame reached the paving outlet: {levels:?}"
                        );
                    }
                    record["yard_levels"] = json!(levels);
                }
                images.push(record);
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
fn dive_wake_cost(gpu: &thrum_test_support::Gpu, directory: &std::path::Path) {
    use water_scene::floating::{WakeDemo, WakeView, build_wake};
    let arms: Vec<_> = [false, true]
        .into_iter()
        .map(|live| {
            let scene = build_wake(
                Wind::Calm,
                false,
                6.0,
                std::f32::consts::PI,
                WakeDemo {
                    view: WakeView::LensCost,
                    disabled: !live,
                    ..WakeDemo::default()
                },
            )
            .expect("craft wake lens cost scene");
            let mut fixture =
                support::Fixture::from_game(gpu, [2560, 1440], scene.game, scene.report);
            let frames: Vec<_> = (0..36)
                .filter_map(|index| {
                    let sample = fixture.frame(f64::from(index) / 120.0);
                    (index >= 24).then(|| {
                        let mut record = sample_json(&sample);
                        let water = fixture.report.lock().expect("submitted water cost report");
                        record["underwater_admitted"] = json!(water.underwater_admitted);
                        record["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                        record
                    })
                })
                .collect();
            json!({"live": live, "frames": frames, "timing_stats": format!("{:?}", fixture.host.timing_stats())})
        })
        .collect();
    let record = json!({"target": "mac-metal", "extent": [2560, 1440], "capture": false,
        "height_m": 5.0, "speed_mps": 6.0, "wind": "Calm",
        "commit": std::env::var("THRUM_WATER_COMMIT").expect("cost revision"), "arms": arms});
    std::fs::write(
        directory.join("wake-dive-cost.json"),
        serde_json::to_vec_pretty(&record).expect("cost JSON"),
    )
    .expect("retain live wake dive cost");
}

#[cfg(target_os = "macos")]
fn dive_depth_checkpoints(site: DiveSite) -> Vec<(String, String, f64)> {
    let depths: &[f64] = match site {
        DiveSite::DeepSea => &[0.5, 3.0, 10.0, 20.0],
        DiveSite::Sea => &[0.5, 3.0, 10.0],
        _ => &[0.5, 3.0],
    };
    [("dawn", 1), ("noon", 60), ("twilight", -5)].into_iter()
        .flat_map(|(light, sun)| depths.iter().flat_map(move |depth| {
            let (hull_pitch, setback) = if *depth <= 0.5 {
                (0.0, 0.0)
            } else {
                (35.0, depth * 0.9)
            };
            let bed_setback = if site == DiveSite::Sea && *depth <= 3.0 {
                -65.0
            } else {
                0.0
            };
            let bed_pitch = if *depth >= 20.0 { -35.0 } else { -72.0 };
            [("hull", hull_pitch, setback), ("bed", bed_pitch, bed_setback), ("level", 0.0, 0.0)]
                .map(|(view, pitch, setback)| (
                    format!("{light}-{view}-{depth}m"),
                    format!(r#"{{"camera.depth":{depth},"camera.pitch":{pitch},"camera.setback":{setback},"sun.elevation":{sun}}}"#),
                    10.0,
                ))
        }))
        .collect()
}

#[cfg(target_os = "macos")]
fn yard_checkpoints() -> Vec<(String, String, f64)> {
    [("first", 0.2), ("separate", 0.5), ("brim", 5.8)]
        .into_iter()
        .flat_map(|(fill, time)| {
            [("noon", 60), ("low-sun", 8)].into_iter().flat_map(move |(light, elevation)| {
                ["west", "east", "roof", "overhead"].into_iter().enumerate().map(
                    move |(camera, view)| (
                        format!("yard-{fill}-{view}-{light}"),
                        format!(r#"{{"camera.island":{camera},"sun.elevation":{elevation},"rain.rate":100,"rain.time_scale":600}}"#),
                        time,
                    ),
                )
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn optics_checkpoints() -> Vec<(String, String, f64)> {
    if floating().is_some() {
        return [
            ("hull-2", 2, 60),
            ("hull-6", 6, 60),
            ("reflection-2", 2, 6),
            ("reflection-10", 10, 6),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (name, height, sun))| {
            (
                name.into(),
                format!(r#"{{"camera.optics_height":{height},"sun.elevation":{sun}}}"#),
                9.0 + f64::from(u32::try_from(i).expect("four checkpoints")),
            )
        })
        .collect();
    }
    let views = [
        ("beach", 0),
        ("lake", 1),
        ("pond", 2),
        ("lake-high", 7),
        ("beach-down", 8),
        ("lake-down", 9),
        ("pond-down", 10),
        ("lake-shore", 11),
    ];
    [("dawn", 1), ("noon", 60), ("low-sun", 6)]
        .into_iter()
        .flat_map(|(name, sun)| {
            views.map(|(view, camera)| {
                (
                    format!("{view}-{name}"),
                    format!(r#"{{"camera.island":{camera},"sun.elevation":{sun}}}"#),
                    10.0,
                )
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn surf_checkpoints() -> Vec<(String, String, f64)> {
    let swell_stills = std::env::var_os("THRUM_WATER_SWELL_STILLS").is_some();
    [
        "sand", "beach", "offshore", "lee", "headland", "island", "seaward", "handoff", "approach",
    ]
    .into_iter()
    .zip(0_u32..)
    .filter(|(_, view)| !swell_stills || matches!(view, 2 | 7))
    .flat_map(|(name, view)| {
        (0_u32..3).map(move |phase| {
            // Sample thirds of the 6.5 s trade swell at each review camera.
            let time = if swell_stills {
                16.0 + f64::from(view) * 2.7 + f64::from(phase) * (6.5 / 3.0)
            } else {
                16.0 + f64::from(view * 3 + phase) * 0.9
            };
            (
                format!("surf-{name}-{phase}"),
                format!(r#"{{"camera.island":{view}}}"#),
                time,
            )
        })
    })
    .collect()
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
#[cfg(target_os = "macos")]
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
#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn under_light_checkpoints(site: DiveSite) -> Vec<(String, String, f64)> {
    let views: &[(&str, f32, f32, f32)] = match site {
        DiveSite::Pond | DiveSite::Swash => &[("through-shallows", -2.0, -73.0, 90.0)],
        DiveSite::Lake | DiveSite::HighLake => &[
            ("bed-0.5m", 0.5, -45.0, 90.0),
            ("bed-3m", 3.0, -45.0, 90.0),
            ("level-0.5m", 0.5, 0.0, 90.0),
            ("level-1m", 1.0, 0.0, 90.0),
            ("level-3m", 3.0, 0.0, 90.0),
            ("sunk-float", 0.5, -20.0, 349.0),
            ("shaft-1m", 1.0, 67.0, 228.0),
            ("shaft-3m", 3.0, 67.0, 228.0),
            ("away-1m", 1.0, 67.0, 273.0),
            ("away-3m", 3.0, 67.0, 273.0),
        ],
        _ => &[
            ("bed-0.5m", 0.5, -45.0, 90.0),
            ("bed-3m", 3.0, -45.0, 90.0),
            ("level-0.5m", 0.5, 0.0, 90.0),
            ("level-1m", 1.0, 0.0, 90.0),
            ("level-3m", 3.0, 0.0, 90.0),
            ("sunk-float", 0.5, -20.0, 349.0),
            ("shaft-1m", 1.0, 67.0, 228.0),
            ("shaft-3m", 3.0, 67.0, 228.0),
            ("shaft-10m", 10.0, 67.0, 228.0),
            ("level-10m", 10.0, 0.0, 90.0),
            ("away-1m", 1.0, 67.0, 273.0),
            ("away-3m", 3.0, 67.0, 273.0),
            ("away-10m", 10.0, 67.0, 273.0),
        ],
    };
    [("noon", 60), ("low", 15), ("night", -17)].into_iter()
        .flat_map(|(light, sun)| views.iter().map(move |&(view, depth, pitch, bearing)| {
            let aimed = view.starts_with("shaft-");
            let off_sun = view.starts_with("away-");
            let beam_pitch = if sun == 15 { 43.0 } else { 67.0 };
            let (pitch, bearing, fov) = if aimed || off_sun {
                (if aimed { beam_pitch } else { beam_pitch - 45.0 }, 228.0, 55.0)
            } else if view == "sunk-float" {
                (pitch, bearing, 60.0)
            } else {
                (pitch, bearing, 105.0)
            };
            let setback = if view == "sunk-float" { 5.7 } else { 0.0 };
            (format!("{light}-{view}"), format!(
                r#"{{"camera.depth":{depth},"camera.pitch":{pitch},"camera.bearing":{bearing},"camera.fov":{fov},"camera.setback":{setback},"sun.elevation":{sun}}}"#,
            ), 10.0)
        })).collect()
}

#[cfg(target_os = "macos")]
fn under_light_cost(gpu: &thrum_test_support::Gpu, directory: &std::path::Path) {
    let (wind, site) = settings()
        .dive()
        .expect("under-light cost requires a dive site");
    assert_eq!(wind, Wind::Trade);
    let on = std::env::var("THRUM_WATER_UNDER_LIGHT_ARM").as_deref() == Ok("on");
    let (game, report) =
        water_scene::island::build_dive_light_cost(wind, site, on).expect("under-light cost scene");
    let mut fixture = support::Fixture::from_game(gpu, [2560, 1440], game, report);
    fixture.warm_cooked();
    let (depth, pitch) = if site == DiveSite::Swash {
        (-2.0, 55.0)
    } else {
        (3.0, 0.0)
    };
    fixture.parameters(&format!(
        r#"{{"camera.depth":{depth},"camera.pitch":{pitch},"sun.elevation":60}}"#
    ));
    let mut frames = Vec::new();
    for index in 0..16 {
        let frame = fixture.frame(1.0 + f64::from(index) / 120.0);
        if index >= 4 {
            frames.push(sample_json(&frame));
        }
    }
    let record = json!({
        "target": "mac-metal", "extent": [2560, 1440], "capture": false,
        "commit": std::env::var("THRUM_WATER_COMMIT").expect("cost revision"),
        "wind": format!("{wind:?}"), "location": site.name(), "depth_m": depth,
        "pitch_degrees": pitch, "under_light": on, "frames": frames,
        "resources": fixture.report.lock().expect("water report").resources.clone(),
    });
    std::fs::write(
        directory.join(format!("{}-{on}.json", site.name())),
        serde_json::to_vec_pretty(&record).expect("json"),
    )
    .expect("under-light cost record");
}
