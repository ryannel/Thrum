use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
use thrum_app::{
    offscreen::{
        Offscreen,
        writer::{ImageWriter, SpoolEncoding},
    },
    sky::SkyGame,
};
use thrum_render::runtime::{CaptureLimits, FrameSample, FrameWorld, SceneSource};

// Even a stalled writer can retain this complete interval in its finite queue.
// Keep ordinary elapsed-time stepping and pan speed; bound the observer, not time.
const MAX_SUBMISSIONS: usize = 120;
const CHANNEL_SUBMISSIONS: usize = 180;
const DIVE_SUBMISSIONS: usize = 190;
const DIVE_CROSSING_SECONDS: f64 = 4.0;
const DIVE_CROSSING_HZ: f64 = 17.0;
const CAUSTIC_WINDOW_CAMERA_TRAVEL: f64 = 10.0;
const RIVER_SUBMISSIONS: usize = 240;
const RIVER_CAPTURE_LEASES: usize = 256;
const WRITER_QUEUE: usize = 192;
const CPU_LEASES: usize = 200;
const PIXEL_ALLOWANCE: u64 = 3 * 1024 * 1024 * 1024;
const _: () = assert!(MAX_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(MAX_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(CHANNEL_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(CHANNEL_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(RIVER_SUBMISSIONS < RIVER_CAPTURE_LEASES);
const _: () = assert!(RIVER_SUBMISSIONS as u64 * 960 * 540 * 4 < PIXEL_ALLOWANCE);
const _: () = assert!(CHANNEL_SUBMISSIONS as u64 * 1280 * 720 * 4 < PIXEL_ALLOWANCE);
const _: () = assert!(DIVE_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(DIVE_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(DIVE_SUBMISSIONS as u64 * 2560 * 1440 * 4 < PIXEL_ALLOWANCE);

pub(super) fn dive_crossing_depth(way_down: f64, crossing: f64) -> f64 {
    let phase = (2.0 * way_down / DIVE_CROSSING_SECONDS - 1.0) * std::f64::consts::FRAC_PI_2;
    crossing * phase.sin().powi(3)
}

fn dive_view(route: &str) -> Option<(f64, f64)> {
    match route {
        "dive-up-1m" => Some((1.0, 89.99)),
        "dive-inclined-1m" => Some((1.0, 45.0)),
        "dive-up-5m" => Some((5.0, 89.99)),
        "dive-inclined-5m" => Some((5.0, 45.0)),
        "dive-hold" => Some((0.0, 0.0)),
        _ => None,
    }
}

fn fill_footfall_yard(
    runtime: &mut Offscreen<SkyGame>,
    device: &wgpu::Device,
    map: &water_scene::SharedMap,
    route: &str,
) -> f64 {
    let submit = |runtime: &mut Offscreen<SkyGame>, time| {
        runtime
            .step_at(Duration::from_secs_f64(time))
            .expect("advance yard preroll");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("yard preroll completion");
        while runtime
            .completed()
            .expect("yard preroll completion")
            .is_some()
        {}
    };
    runtime
        .parameters(r#"{"rain.rate":100,"rain.time_scale":600}"#)
        .expect("fill the yard");
    let mut final_levels = [None; 2];
    let joined = route == "yard-joined";
    for step in 1..=360 {
        submit(runtime, f64::from(step) / 60.0);
        final_levels = super::rain_cost::levels(&map.lock().expect("yard map"));
        let filled = match final_levels {
            [Some(left), Some(right)] if joined => {
                left.to_bits() == right.to_bits() && right > 18.08
            }
            [Some(left), Some(right)] => left > right && right > 18.015,
            _ => false,
        };
        if filled {
            runtime
                .parameters(r#"{"rain.rate":0,"rain.time_scale":1}"#)
                .expect("hold the yard at this fill");
            // Roll the full rate history out before collecting submitted images.
            for dry in step + 1..=step + 65 {
                submit(runtime, f64::from(dry) / 60.0);
            }
            return f64::from(step + 65) / 60.0;
        }
    }
    panic!("could not reach the requested footfall yard fill; last levels {final_levels:?}");
}

#[expect(
    clippy::too_many_lines,
    reason = "one timeline coordinates submitted frames, stage edits and the capture spool"
)]
pub(super) fn capture(gpu: &thrum_test_support::Gpu, directory: &Path) -> Value {
    let (device, queue, caps) = super::support::device(gpu);
    let island = super::island_wind();
    let dive_site = super::settings().dive();
    let dive = dive_site.is_some();
    let yard = matches!(super::settings(), super::Settings::RainYard(_));
    let channel = super::channel_route();
    let channel_view = std::env::var("THRUM_WATER_CHANNEL_CAMERA").unwrap_or_else(|_| "2".into());
    let river = super::river_route();
    let river_inspection = river.then(super::river_inspection::shot).flatten();
    let river_view = std::env::var("THRUM_WATER_RIVER_CAMERA").unwrap_or_else(|_| "0".into());
    let floating = super::floating();
    let wake = super::wake_route();
    let whitewater = super::whitewater();
    let surf = super::surf_route();
    let route = std::env::var("THRUM_WATER_ROUTE").unwrap_or_default();
    let wet_yard = yard && matches!(route.as_str(), "wet-shower" | "wet-dry");
    let shore_ebb = matches!(super::settings(), super::Settings::WetShore(_)) && route == "ebb";
    let yard_camera_cut = yard && route == "yard-walk-low-sun-cut";
    let walking = yard_camera_cut
        || matches!(
            route.as_str(),
            "yard-walk" | "yard-walk-low-sun" | "yard-walk-high-sun"
        );
    let footfall = yard
        && (walking
            || matches!(
                route.as_str(),
                "yard-stone-left" | "yard-stone-right" | "yard-sill-dry" | "yard-joined"
            ));
    let footfall_edit = match route.as_str() {
        _ if walking => r#"{"rain.walk":1}"#,
        "yard-stone-right" => r#"{"rain.stone_right":1}"#,
        "yard-sill-dry" | "yard-joined" => r#"{"rain.stone_sill":1}"#,
        _ => r#"{"rain.stone_left":1}"#,
    };
    let caustic_window = dive && route == "caustic-window";
    let held_dive = std::env::var("THRUM_WATER_DIVE_HOLD")
        .ok()
        .and_then(|text| {
            let (depth, pitch) = text.split_once(',')?;
            Some((depth.parse::<f64>().ok()?, pitch.parse::<f64>().ok()?))
        })
        .or_else(|| dive_view(&route))
        .or_else(|| caustic_window.then_some((1.8, -12.0)));
    let dive_travel = std::env::var("THRUM_WATER_DIVE_TRAVEL")
        .ok()
        .and_then(|travel| travel.parse::<f64>().ok());
    let dive_light = std::env::var("THRUM_WATER_DIVE_SUN")
        .ok()
        .and_then(|sun| sun.parse::<f64>().ok())
        .unwrap_or(60.0);
    let dive_bearing = std::env::var("THRUM_WATER_DIVE_BEARING")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(90.0);
    let dive_setback = std::env::var("THRUM_WATER_DIVE_SETBACK")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let dive_lateral = std::env::var("THRUM_WATER_DIVE_LATERAL")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let dive_fov = std::env::var("THRUM_WATER_DIVE_FOV")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(105.0);
    let optics_height = std::env::var("THRUM_WATER_OPTICS_HEIGHT").unwrap_or_else(|_| "2".into());
    let river_sequence = river && route == "flow-inspection";
    let river_start = river_inspection.as_ref().map(|shot| shot.start);
    let river_warmup = river_start.filter(|_| route == "continuous");
    let river_gauges = river && route == "foam-gauge-inspection";
    let river_shots: &[u8] = if river_sequence {
        &[17, 18, 0, 1, 4, 5, 8, 3]
    } else if river_gauges {
        &[4, 4]
    } else {
        &[]
    };
    let shot_seconds = 4.0;
    let cpu_leases = if river_sequence {
        RIVER_CAPTURE_LEASES
    } else {
        CPU_LEASES
    };
    let writer_queue = if river_sequence {
        RIVER_CAPTURE_LEASES
    } else {
        WRITER_QUEUE
    };
    let river_stage = river && matches!(route.as_str(), "stage" | "lake-stage" | "spring-stage");
    let late_channel = channel && route == "late";
    let handoff_cut = surf && route == "handoff-cut";
    let island_view = std::env::var("THRUM_WATER_ISLAND_CAMERA").unwrap_or_else(|_| {
        match (yard, route.as_str()) {
            (true, "yard-stone-left") => "9",
            (true, "yard-stone-right") => "10",
            (true, _) if walking => "11",
            (true, _) if footfall => "8",
            _ => "0",
        }
        .into()
    });
    if handoff_cut {
        assert_eq!(island_view, "7", "start at the foam handoff camera");
    }
    let mut fleet = None;
    let mut yard_map = None;
    let (game, report) = if let Some(floating) = floating {
        let scene = super::floating_game(floating, true, true, whitewater);
        fleet = Some((scene.observer(), scene.mailbox.clone()));
        (scene.game, scene.report)
    } else if let super::Settings::RainYard(wind) = super::settings() {
        let (game, report, map, _) = if wet_yard {
            water_scene::island::build_rain_ground(wind, true, true)
        } else {
            water_scene::island::build_rain_yard(wind, true)
        }
        .expect("rain yard");
        yard_map = Some(map);
        (game, report)
    } else {
        super::proof_game(true)
    };
    let (extent, interval, opportunities, max_submissions) = if footfall {
        // The moving take samples the same game at the browser proof's
        // extent to keep the capture observer from hiding intermediate fronts.
        let extent = if walking { [960, 540] } else { [1280, 720] };
        (extent, if walking { 5.5 } else { 5.0 }, 120.0, 750)
    } else if wet_yard && route == "wet-dry" {
        ([1280, 720], 26.0, 16.0, 432)
    } else if wet_yard || shore_ebb {
        ([1280, 720], 9.0, 16.0, CHANNEL_SUBMISSIONS)
    } else if yard {
        ([1280, 720], 20.0, 9.0, CHANNEL_SUBMISSIONS)
    } else if dive && dive_travel.is_some() {
        ([2560, 1440], 20.0, 4.0, DIVE_SUBMISSIONS)
    } else if dive && (route == "under-light" || caustic_window) {
        ([2560, 1440], 5.0, 16.0, MAX_SUBMISSIONS)
    } else if dive {
        ([1280, 720], 5.0, 20.0, MAX_SUBMISSIONS)
    } else if surf && island_view == "2" {
        // A complete set of unequal breaks takes longer than a nearshore take.
        ([2560, 1440], 22.0, 5.0, MAX_SUBMISSIONS)
    } else if surf && island_view == "9" {
        ([2560, 1440], 18.0, 6.5, MAX_SUBMISSIONS)
    } else if river_sequence {
        ([960, 540], 8.0 * shot_seconds, 7.0, RIVER_SUBMISSIONS)
    } else if river_gauges {
        ([960, 540], 2.0 * shot_seconds, 16.0, CHANNEL_SUBMISSIONS)
    } else if river_inspection
        .as_ref()
        .is_some_and(|shot| shot.name.starts_with("ledge-") || shot.name.starts_with("rock-"))
    {
        ([960, 540], 4.0, 20.0, MAX_SUBMISSIONS)
    } else if river_stage {
        ([1280, 720], 20.5, 8.0, CHANNEL_SUBMISSIONS)
    } else if river {
        ([1280, 720], 4.0, 12.0, MAX_SUBMISSIONS)
    } else if channel {
        ([1280, 720], 9.0, 20.0, CHANNEL_SUBMISSIONS)
    } else if surf {
        ([2560, 1440], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if whitewater && route == "stop" {
        ([1280, 720], 72.0, 10.0, 740)
    } else if whitewater || wake {
        ([1280, 720], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if floating.is_some() {
        ([2560, 1440], 3.7, 30.0, MAX_SUBMISSIONS)
    } else if island.is_some() {
        // Inspect the lake reflection and its moving silhouettes at the stills' native size.
        let extent = if matches!(route.as_str(), "optics-pan" | "optics-breeze") {
            [2560, 1440]
        } else {
            [1280, 720]
        };
        (extent, 7.0, 16.0, MAX_SUBMISSIONS)
    } else {
        ([2560, 1440], 1.0, 120.0, MAX_SUBMISSIONS)
    };
    let mut runtime =
        Offscreen::new(game, &device, &caps, &queue, extent).expect("ordinary game runtime");
    if let Some((_, site)) = dive_site {
        let (depth, pitch) = held_dive.unwrap_or((-site.crossing_depth(), 0.0));
        let follow = u8::from(
            dive_travel.is_some()
                || (route == "under-light" && held_dive.is_some_and(|(depth, _)| depth == 0.0)),
        );
        let setback = dive_setback + 1.7 * f64::from(follow);
        runtime
            .parameters(&format!(
                r#"{{"camera.depth":{depth},"camera.pitch":{pitch},"camera.bearing":{dive_bearing},"camera.fov":{dive_fov},"camera.surface_follow":{follow},"camera.setback":{setback},"camera.lateral":{dive_lateral},"sun.elevation":{dive_light}}}"#
            ))
            .expect("initial dive view");
    }
    let footfall_sun = match route.as_str() {
        "yard-walk-high-sun" => Some([60, 12]),
        "yard-walk-low-sun" | "yard-walk-low-sun-cut" => Some([8, 0]),
        _ => None,
    };
    if yard {
        let lighting = match footfall_sun {
            Some([elevation, azimuth]) => {
                format!(r#","sun.elevation":{elevation},"sun.azimuth":{azimuth}"#)
            }
            None if wet_yard => r#","sun.elevation":38"#.into(),
            None if !footfall => r#","sun.elevation":8"#.into(),
            None => String::new(),
        };
        runtime
            .parameters(&format!(
                r#"{{"camera.island":{island_view},"rain.rate":0{lighting}}}"#
            ))
            .expect("initial dry yard");
    }
    // The island's camera index is a meaning only this capture needs, so it has a variable of its own.
    if island.is_some() {
        runtime
            .parameters(&format!(r#"{{"camera.island":{island_view}}}"#))
            .expect("island view");
    }
    if channel {
        runtime
            .parameters(&format!(
                r#"{{"camera.channel":{channel_view},"sea.stage":{}}}"#,
                if matches!(route.as_str(), "tide" | "low-tide") {
                    -0.5
                } else {
                    0.0
                }
            ))
            .expect("channel view");
    }
    if river {
        let river_view = river_shots
            .first()
            .map_or_else(|| river_view.clone(), u8::to_string);
        let stages = match route.as_str() {
            "stage" => r#","lake.stage":0,"spring.stage":0"#,
            "lake-stage" => r#","lake.stage":0,"spring.stage":0.5"#,
            "spring-stage" | "foam-gauge-inspection" => r#","lake.stage":0.5,"spring.stage":0"#,
            _ => "",
        };
        let sun = std::env::var("THRUM_WATER_RIVER_SUN")
            .ok()
            .map_or_else(String::new, |v| format!(r#","sun.elevation":{v}"#));
        let gauge = std::env::var("THRUM_WATER_RIVER_GAUGE")
            .ok()
            .map_or_else(String::new, |v| format!(r#","spring.stage":{v}"#));
        runtime
            .parameters(&format!(
                r#"{{"camera.river":{river_view}{stages}{sun}{gauge}}}"#
            ))
            .expect("river view");
    }
    if shore_ebb {
        runtime
            .parameters(r#"{"camera.island":0,"sea.stage":0.5,"sun.elevation":37}"#)
            .expect("starting high tide");
    }
    for _ in 0..8 {
        if let Some(shot) = &river_inspection {
            runtime
                .step_at(Duration::from_secs_f64(if river_warmup.is_some() {
                    0.0
                } else {
                    shot.start
                }))
                .expect("river inspection preroll");
        } else if footfall || wet_yard || shore_ebb {
            runtime.step_at(Duration::ZERO).expect("footfall preroll");
        } else {
            runtime.step().expect("warm submit");
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .expect("warm poll");
        while runtime.completed().expect("warm completion").is_some() {}
    }
    let mut river_preroll_submissions = 0_u32;
    let river_start = if let Some(start) = river_warmup {
        let clock = Instant::now();
        while clock.elapsed().as_secs_f64() < start {
            let report = runtime
                .step_at(clock.elapsed())
                .expect("continuous river submit");
            river_preroll_submissions += u32::from(report.submitted);
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("river warmup completion");
            while runtime
                .completed()
                .expect("river warmup completion")
                .is_some()
            {}
            std::thread::sleep(Duration::from_millis(4));
        }
        Some(clock.elapsed().as_secs_f64())
    } else {
        river_start
    };
    let footfall_start = footfall.then(|| {
        fill_footfall_yard(
            &mut runtime,
            &device,
            yard_map.as_ref().expect("yard"),
            &route,
        )
    });
    let mut wet_start = 0.0;
    if wet_yard && route == "wet-dry" {
        runtime
            .parameters(r#"{"rain.rate":10,"rain.drying":0,"rain.time_scale":600}"#)
            .expect("soak yard");
        for step in 1..=38 {
            wet_start = f64::from(step) / 15.0;
            runtime
                .step_at(Duration::from_secs_f64(wet_start))
                .expect("wetness preroll");
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("warm wetness");
            while runtime.completed().expect("wetness preroll").is_some() {}
        }
        runtime
            .parameters(r#"{"rain.rate":0,"rain.drying":0,"rain.time_scale":1}"#)
            .expect("soaked hold");
    }
    runtime
        .enable_capture_with_limits(
            3,
            CaptureLimits {
                staging_slots: 6,
                cpu_buffers: cpu_leases,
                byte_allowance: PIXEL_ALLOWANCE,
            },
        )
        .expect("tap");
    let (file, encoding) = if floating.is_some() || whitewater || channel || river || yard || dive {
        ("motion.spool.zst", SpoolEncoding::ZstdDelta)
    } else {
        ("motion.spool.raw", SpoolEncoding::Raw)
    };
    let output = std::fs::File::create(directory.join(file)).expect("spool");
    let writer =
        ImageWriter::start(output, writer_queue, 6 * 1024 * 1024 * 1024, encoding).expect("writer");
    let begin = Instant::now();
    let mut submitted = 0;
    let mut frames = Vec::new();
    let mut completed = BTreeMap::new();
    let mut poses = Vec::new();
    let mut dive_positions = BTreeMap::new();
    let mut applied = "";
    let mut cut = None;
    let mut weather_edits = Vec::new();
    let mut yard_levels = Vec::new();
    let mut footfall_steps = Vec::new();
    let yard_tick = Duration::from_secs_f64(1.0 / 60.0);
    let mut deadline = interval;
    let mut footfall_edit_time = None;
    let mut yard_view = 11;
    let mut river_shot = 0;
    while begin.elapsed() < Duration::from_secs_f64(deadline) && submitted < max_submissions {
        let elapsed = begin.elapsed().as_secs_f64();
        if let Some(start) = footfall_start {
            let target_tick =
                Duration::from_secs_f64(start + elapsed).as_nanos() / yard_tick.as_nanos();
            let previous_tick = report
                .lock()
                .expect("water report")
                .interaction
                .and_then(|wave| wave.endpoints)
                .map_or(0, |ends| u128::from(ends[1]));
            // Trigger on a frame that can actually step the source's tick.
            // Preserve real elapsed time even if a loaded device skips a frame.
            let gap = target_tick.saturating_sub(previous_tick);
            if elapsed >= 0.30
                && applied.is_empty()
                && (1..=2).contains(&gap)
                && runtime.pending_frames() < 2
            {
                runtime
                    .parameters(footfall_edit)
                    .expect("yard interaction control");
                weather_edits.push(json!({"elapsed":elapsed,"parameters":serde_json::from_str::<Value>(footfall_edit).expect("footfall edit")}));
                applied = footfall_edit;
                footfall_edit_time = Some(elapsed);
                deadline = interval + elapsed;
            }
        } else if wet_yard && route == "wet-dry" && elapsed >= 18.0 && applied != "finish dry" {
            let change = r#"{"rain.rate":0,"rain.drying":100,"rain.time_scale":240}"#;
            runtime
                .parameters(change)
                .expect("finish draining the pothole");
            weather_edits.push(json!({"elapsed":elapsed, "parameters":serde_json::from_str::<Value>(change).expect("weather")}));
            applied = "finish dry";
        } else if wet_yard
            && route == "wet-dry"
            && (7.0..18.0).contains(&elapsed)
            && applied != "margin"
        {
            let change = r#"{"rain.rate":0,"rain.drying":100,"rain.time_scale":45}"#;
            runtime
                .parameters(change)
                .expect("watch the shallow margin recede");
            weather_edits.push(json!({"elapsed":elapsed, "parameters":serde_json::from_str::<Value>(change).expect("weather")}));
            applied = "margin";
        } else if wet_yard && elapsed >= 0.8 && applied.is_empty() {
            let change = if route == "wet-dry" {
                r#"{"rain.rate":0,"rain.drying":100,"rain.time_scale":90}"#
            } else {
                r#"{"rain.rate":10,"rain.drying":0,"rain.time_scale":240}"#
            };
            runtime.parameters(change).expect("wet ground timeline");
            weather_edits.push(json!({"elapsed":elapsed, "parameters":serde_json::from_str::<Value>(change).expect("weather")}));
            applied = change;
        } else if yard && !wet_yard {
            let change = match elapsed {
                t if t >= 7.8 => r#"{"rain.rate":0,"rain.drying":100,"rain.time_scale":600}"#,
                t if t >= 4.45 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":600}"#,
                t if t >= 2.85 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":45}"#,
                t if t >= 1.10 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":150}"#,
                t if t >= 0.25 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":5}"#,
                _ => "",
            };
            if !change.is_empty() && change != applied {
                runtime.parameters(change).expect("yard weather controls");
                weather_edits.push(json!({"elapsed":elapsed, "parameters":serde_json::from_str::<Value>(change).expect("weather")}));
                applied = change;
            }
        }
        if yard_camera_cut && let Some(start) = footfall_edit_time {
            let walk_elapsed = elapsed - start;
            let next = if (1.4..2.0).contains(&walk_elapsed) {
                3
            } else {
                11
            };
            if next != yard_view {
                runtime
                    .parameters(&format!(r#"{{"camera.island":{next}}}"#))
                    .expect("yard camera change");
                weather_edits.push(json!({"elapsed":elapsed,"parameters":{"camera.island":next}}));
                yard_view = next;
            }
        }
        if let Some((_, site)) = dive_site
            && held_dive.is_none()
        {
            let crossing = site.crossing_depth();
            let depth = if let Some(maximum) = dive_travel {
                let way_down = elapsed.min(interval - elapsed).max(0.0);
                let crossing_time = DIVE_CROSSING_SECONDS;
                if way_down < crossing_time {
                    dive_crossing_depth(way_down, crossing)
                } else {
                    let progress = (way_down - crossing_time) / (0.5 * interval - crossing_time);
                    let eased = 0.5 * (1.0 - (progress * std::f64::consts::PI).cos());
                    (maximum - crossing).mul_add(eased, crossing)
                }
            } else {
                -crossing * (elapsed * std::f64::consts::TAU / interval).cos()
            };
            if let Some(maximum) = dive_travel {
                let below = (depth - crossing).max(0.0);
                let retreat = match site {
                    water_scene::island::DiveSite::Lake | water_scene::island::DiveSite::HighLake => -0.4,
                    _ => 0.9,
                };
                let hull_pitch = 32.0 * (below / 4.0).clamp(0.0, 1.0);
                let bed_view = ((depth / maximum - 0.8) / 0.2).clamp(0.0, 1.0);
                let bed_view = bed_view * bed_view * (3.0 - 2.0 * bed_view);
                let pitch = (-35.0 - hull_pitch).mul_add(bed_view, hull_pitch);
                let release = (below / 1.5).clamp(0.0, 1.0);
                let follow = 1.0 - release * release * (3.0 - 2.0 * release);
                // Leave the crossing setback near the surface and finish the
                // tour over the surveyed dive station, clear of the bank.
                let distance = (dive_setback * follow + 1.7 + retreat * below) * (1.0 - bed_view);
                runtime.parameters(&format!(
                    r#"{{"camera.depth":{depth:.4},"camera.setback":{distance:.4},"camera.pitch":{pitch:.4},"camera.surface_follow":{follow:.4}}}"#,
                ))
            } else {
                runtime.parameters(&format!(r#"{{"camera.depth":{depth:.4}}}"#))
            }
            .expect("lower and raise the lens");
        }
        if caustic_window {
            let distance = 0.5 * (1.0 - (elapsed * std::f64::consts::TAU / interval).cos());
            let setback = CAUSTIC_WINDOW_CAMERA_TRAVEL.mul_add(distance, dive_setback);
            runtime
                .parameters(&format!(r#"{{"camera.setback":{setback:.4}}}"#))
                .expect("translate the caustic window");
        }
        if !dive && !yard && !channel && !river && island.is_none() && floating.is_none() {
            runtime
                .parameters(&format!(
                    r#"{{"camera.bearing":{}}}"#,
                    70.0 + elapsed * 12.0
                ))
                .expect("pan control");
        }
        if let Some(&view) = river_shots.get(river_shot + 1)
            && elapsed
                >= shot_seconds * f64::from(u8::try_from(river_shot + 1).expect("river shot count"))
        {
            river_shot += 1;
            let edit = if river_gauges {
                json!({"camera.river":view, "spring.stage":1})
            } else {
                json!({"camera.river":view})
            };
            runtime
                .parameters(&edit.to_string())
                .expect("river inspection controls");
            weather_edits.push(json!({"elapsed":elapsed,"parameters":edit}));
        }
        if river_stage {
            let stage = (1.0 - (elapsed / 10.0 - 1.0).abs()).clamp(0.0, 1.0);
            let lake = if route == "spring-stage" { 0.5 } else { stage };
            let spring = if route == "lake-stage" { 0.5 } else { stage };
            runtime
                .parameters(&format!(
                    r#"{{"lake.stage":{lake:.4},"spring.stage":{spring:.4}}}"#
                ))
                .expect("river stage sweep");
        }
        if channel && route == "tide" {
            let height = ((elapsed - 1.0) / 6.0).clamp(0.0, 1.0);
            runtime
                .parameters(&format!(
                    r#"{{"lake.stage":{height:.3},"sea.stage":{:.3}}}"#,
                    height - 0.5
                ))
                .expect("channel stages");
        }
        if route == "optics-pan" {
            if floating.is_some() {
                let yaw = -56.0 + elapsed * 30.0;
                runtime.parameters(&format!(r#"{{"camera.optics_height":{optics_height},"camera.optics_pan":{yaw},"sun.elevation":6}}"#))
                    .expect("fleet pan");
            } else if island.is_some() {
                let yaw = elapsed * 14.0;
                let tilt = elapsed * 4.0;
                runtime
                    .parameters(&format!(
                        r#"{{"camera.pan":{yaw},"camera.tilt":{tilt},"sun.elevation":12}}"#
                    ))
                    .expect("lake pan");
            }
        } else if route == "optics-breeze" {
            let gain = ((elapsed - 1.0) / 4.0).clamp(0.0, 1.0);
            runtime
                .parameters(&format!(r#"{{"lake.waves":{gain}}}"#))
                .expect("breeze");
        }
        if handoff_cut && elapsed >= 5.0 && cut.is_none() {
            runtime
                .parameters(r#"{"camera.island":8}"#)
                .expect("approach view");
            cut = Some(
                json!({"from":7,"to":8,"driver_elapsed":elapsed,"next_capture_index":submitted}),
            );
        } else if shore_ebb {
            let stage = (0.5 - (elapsed - 0.8).max(0.0) / 4.8).max(-0.5);
            runtime
                .parameters(&format!(r#"{{"sea.stage":{stage:.3}}}"#))
                .expect("ebbing tide");
        } else if surf && route == "tide" {
            let stage = if elapsed <= 6.0 {
                elapsed / 6.0 - 0.5
            } else {
                1.5 - elapsed / 6.0
            };
            runtime
                .parameters(&format!(r#"{{"sea.stage":{:.3}}}"#, stage.clamp(-0.5, 0.5)))
                .expect("tide control");
        } else if wake || whitewater {
            let change = match (route.as_str(), elapsed) {
                ("turn", t) if t >= 4.0 => r#"{"craft.turn":65}"#,
                ("restart", t) if t >= 8.0 => r#"{"craft.speed":6}"#,
                ("restart", t) if t >= 4.0 => r#"{"craft.speed":0}"#,
                ("stop", t) if t >= 7.0 => r#"{"craft.speed":0}"#,
                _ => "",
            };
            if !change.is_empty() && change != applied {
                runtime.parameters(change).expect("wake route controls");
                applied = change;
            }
        }
        let submitted_frame = if let Some(start) = river_start {
            runtime.step_at(Duration::from_secs_f64(start + elapsed))
        } else if wet_yard || shore_ebb {
            runtime.step_at(Duration::from_secs_f64(wet_start + elapsed))
        } else if let Some(start) = footfall_start {
            runtime.step_at(Duration::from_secs_f64(start + elapsed))
        } else if late_channel {
            runtime.step_at(Duration::from_secs_f64(60.0 + elapsed))
        } else {
            runtime.step()
        };
        if submitted_frame.expect("ordinary submit").submitted {
            if dive {
                let host = runtime.host();
                let source = host.world().source();
                let clock = host.last_submitted_clock().expect("submitted dive clock");
                let camera = host
                    .source()
                    .frame(source, clock.presentation_alpha, extent, None)
                    .expect("submitted dive view")
                    .view
                    .camera_position();
                dive_positions.insert(clock.submission_order, camera);
            }
            if footfall {
                let interaction = report
                    .lock()
                    .expect("water report")
                    .interaction
                    .expect("yard interaction");
                let fine = interaction.fine.expect("yard fine level");
                footfall_steps.push(json!({
                    "tick": interaction.endpoints.expect("submitted endpoints")[1],
                    "regions": fine.regions, "solves": fine.solves, "steps": fine.steps,
                    "dropped_ticks": fine.dropped_ticks,
                }));
            }
            if let Some(map) = &yard_map {
                let map = map.lock().expect("yard map");
                yard_levels.push(super::rain_cost::levels(&map));
            }
            if let Some((observe, _)) = &fleet {
                poses.push(observe().into_iter().map(|body| json!({
                    "id": body.id, "position": body.position.to_array(),
                    "rotation": body.rotation.to_array(), "velocity": body.velocity.to_array(),
                })).collect::<Vec<_>>());
            }
            submitted += 1;
        }
        assert!(
            drain(&mut runtime, &writer, &mut frames, &mut completed).is_ok(),
            "writer rejected a frame; underlying outcome: {:?}",
            writer.finish(Duration::from_secs(60))
        );
        let rate =
            if dive_travel.is_some() && elapsed.min(interval - elapsed) < DIVE_CROSSING_SECONDS {
                DIVE_CROSSING_HZ
            } else {
                opportunities
            };
        if let Some(wait) = Duration::from_secs_f64((elapsed * rate).floor() / rate + 1.0 / rate)
            .checked_sub(begin.elapsed())
        {
            std::thread::sleep(wait);
        }
    }
    let drained = thrum_test_support::wait::until("capture drain", Duration::from_secs(30), || {
        if runtime.pending_frames() == 0 && frames.len() >= submitted {
            return Some(true);
        }
        device.poll(wgpu::PollType::Poll).expect("final poll");
        // A rejected frame ends the wait; the writer's outcome is reported below.
        drain(&mut runtime, &writer, &mut frames, &mut completed)
            .is_err()
            .then_some(false)
    });
    assert!(
        drained,
        "writer rejected a frame; underlying outcome: {:?}",
        writer.finish(Duration::from_secs(60))
    );
    for frame in &mut frames {
        let serial = frame["sample"]["submission"].as_u64().expect("serial");
        frame["sample"] =
            super::sample_json(completed.get(&serial).expect("matched GPU completion"));
        if let Some(camera) = dive_positions.remove(&serial) {
            frame["camera"] = json!(camera);
        }
    }
    let written = writer
        .finish(Duration::from_secs(60))
        .expect("lossless writer");
    assert_eq!(written.frames, frames.len() as u64);
    assert_eq!(runtime.omissions(), 0, "complete submitted interval");
    if footfall {
        let edit_time = footfall_edit_time.expect("a frame that could step the impulse");
        if walking {
            let end = frames.last().expect("walking frames")["elapsed"]
                .as_f64()
                .expect("submitted elapsed time");
            assert!(end - edit_time >= 4.4, "capture the whole walk");
        }
        assert!(
            footfall_steps
                .iter()
                .any(|row| row["solves"].as_u64().is_some_and(|n| n > 0)),
            "the submitted sequence must contain fine solves from its source"
        );
        let joined = route == "yard-joined";
        assert_eq!(yard_levels.len(), frames.len());
        assert!(yard_levels.iter().all(|pair| matches!(pair, [Some(left), Some(right)] if (left.to_bits() == right.to_bits()) == joined)), "the held fill changed during the dry interaction take: {yard_levels:?}");
    } else if wet_yard && route == "wet-dry" {
        assert!(
            yard_levels
                .first()
                .is_some_and(|row| row.iter().all(Option::is_some)),
            "soaked start"
        );
        assert_eq!(
            yard_levels.last(),
            Some(&[None; 2]),
            "both puddles must dry during the take"
        );
    } else if yard && !wet_yard {
        assert_eq!(
            yard_levels.len(),
            frames.len(),
            "one level pair per submitted frame"
        );
        let joined = |levels: &[Option<f64>; 2]| matches!(levels, [Some(a), Some(b)] if a.to_bits() == b.to_bits());
        assert_eq!(yard_levels.first(), Some(&[None; 2]), "the yard starts dry");
        assert_eq!(yard_levels.last(), Some(&[None; 2]), "the yard dries out");
        assert!(
            yard_levels.iter().copied().any(super::rain_cost::at_brim),
            "the paving reaches its outlet before drying"
        );
        let merge = yard_levels
            .iter()
            .position(joined)
            .expect("observed a joined puddle");
        assert!(
            yard_levels[..merge]
                .iter()
                .any(|row| matches!(row, [Some(lower), Some(upper)] if lower > upper)),
            "the lower hollow spills into the upper hollow before joining"
        );
        assert!(
            yard_levels[merge + 1..]
                .iter()
                .any(|row| row.iter().all(Option::is_some) && !joined(row)),
            "observed a parting"
        );
    }
    let prepared = report.lock().expect("report").clone();
    assert!(
        !handoff_cut || cut.is_some(),
        "capture reached the camera change"
    );
    let dive_camera = dive_site.map(|(_, site)| {
        let mut record = held_dive.map_or_else(
            || {
                json!({"sweep_depth":site.crossing_depth(),"pitch":0,
                      "maximum_depth":dive_travel.unwrap_or_else(|| site.crossing_depth())})
            },
            |(depth, pitch)| json!({"depth":depth,"pitch":pitch}),
        );
        record["sun_elevation"] = json!(dive_light);
        record["surface_follow"] = json!(
            dive_travel.is_some()
                || (route == "under-light" && held_dive.is_some_and(|(depth, _)| depth == 0.0))
        );
        record["bearing"] = json!(dive_bearing);
        record["setback"] = json!(dive_setback);
        record["lateral"] = json!(dive_lateral);
        if caustic_window {
            record["maximum_setback"] = json!(dive_setback + CAUSTIC_WINDOW_CAMERA_TRAVEL);
        }
        if dive_travel.is_some() {
            record["crossing_opportunities_hz"] = json!(DIVE_CROSSING_HZ);
        }
        record["fov"] = json!(dive_fov);
        record
    });
    json!({"schema":1,"target":"mac-metal","mode":"realtime","commit":std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_|"working tree diagnostic".into()),"extent":extent,"vertices":prepared.vertices,"resources":prepared.resources,"wind":island.or(floating.map(|f| f.wind)).or(dive_site.map(|(wind,_)| wind)).map(|w| format!("{w:?}")),"dive_site":dive_site.map(|(_,site)|site.name()),"dive_camera":dive_camera,"island_view":(yard || island.is_some()).then_some(&island_view),"weather_edits":weather_edits,"ground_route":(wet_yard || shore_ebb).then_some(&route),"footfall_route":footfall.then_some(&route),"footfall_sun":footfall_sun,"footfall_steps":footfall_steps,"yard_levels":yard_levels,"camera_cut":cut,"channel_view":channel.then_some(channel_view),"channel_route":channel.then_some(&route),"river_view":river.then_some(river_view),"river_inspection":river_inspection.map(|shot| json!({"name":shot.name,"start":river_start.unwrap_or(shot.start),"continuous":river_warmup.is_some(),"preroll_submissions":river_preroll_submissions,"position":shot.camera.position,"target":shot.camera.target})),"river_route":river.then_some(&route),"floating_view":floating.map(|f| f.wake.map_or_else(|| if f.chase {"chase".into()} else {String::new()}, |wake| format!("{:?}", wake.view).to_lowercase())),"craft_speed":floating.map(|f| f.speed),"clock":if late_channel {"ordinary submits at 60 seconds plus actual elapsed time"} else if footfall {"ordinary submits on monotonic elapsed time after the dry yard preroll"} else {"ordinary Offscreen::step on actual monotonic world time; asynchronous in-flight frames"},"opportunities_hz":opportunities,"frames":frames,"hashes":written.frame_hashes,"encoding":format!("{encoding:?}"),"omissions":runtime.omissions(),"capture_limits":{"interval_seconds":interval,"max_submissions":max_submissions,"gpu_staging_slots":6,"cpu_image_leases":cpu_leases,"pixel_byte_allowance":PIXEL_ALLOWANCE,"writer_queue":writer_queue,"raw_spool_allowance":6_u64*1024*1024*1024},"writer_seconds":written.write_time.as_secs_f64(),"body_tick_poses":poses,"query":fleet.map(|(_,mailbox)|format!("{:?}",mailbox.stats())), "wake_route": (wake || whitewater).then_some(route), "interaction":prepared.interaction.map(|wake| format!("{wake:?}")), "foam":prepared.foam.map(|foam| format!("{foam:?}")), "foam_pattern":prepared.foam_pattern})
}
fn drain(
    runtime: &mut Offscreen<SkyGame>,
    writer: &ImageWriter,
    frames: &mut Vec<Value>,
    completed: &mut BTreeMap<u64, FrameSample>,
) -> Result<(), ()> {
    while let Some(sample) = runtime.completed().expect("completed") {
        completed.insert(sample.submission_order, sample);
    }
    while let Some(image) = runtime.image().expect("submitted image") {
        if let Some(previous) = frames.last() {
            assert!(
                previous["sample"]["submission"].as_u64().expect("serial")
                    < image.sample.submission_order,
                "ordered submitted sources"
            );
        }
        frames.push(json!({"elapsed":image.submitted_elapsed.as_secs_f64(),"sample":super::sample_json(&image.sample),"submitted_elapsed":image.submitted_elapsed.as_secs_f64(),"exposure_words":image.exposure_gpu_words}));
        if let Some(image) = writer.submit(image) {
            runtime.recycle(image);
            return Err(());
        }
    }
    while let Some(image) = writer.recycled() {
        runtime.recycle(image);
    }
    Ok(())
}
