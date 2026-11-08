//! Capture the real game host and its selected content, without claiming live presentation.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use thrum_render::{capabilities::DeviceCapabilities, synthetic::DisplayOverlay};
use v1_game::{FrameConfig, FrameHost, controls::GameControls};

#[path = "motion.rs"]
mod motion;

const EXTENT: [u32; 2] = [1280, 720];

pub(super) async fn run(output: PathBuf) -> Result<(), String> {
    if output.exists() {
        return Err("capture output must be a new directory".to_owned());
    }
    let motion_frames = motion::requested_frames()?;
    let route_only = std::env::var_os("THRUM_CAPTURE_ROUTE_ONLY").is_some();
    let params = std::env::var_os("THRUM_CAPTURE_PARAMS");
    if route_only && (params.is_none() || motion_frames.is_some()) {
        return Err(
            "route-only capture needs THRUM_CAPTURE_PARAMS and no motion capture".to_owned(),
        );
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("capture adapter: {error}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter)
                | DeviceCapabilities::requested_content_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|error| format!("capture device: {error}"))?;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
    let mut host = FrameHost::new_with_setup(
        &device,
        &queue,
        &capabilities,
        FrameConfig {
            render_extent: motion_frames.map_or(EXTENT, |_| motion::PROFILE.extent),
            surface: motion_frames.map(|_| motion::PROFILE),
        },
        v1_game::selected_setup()?,
    )
    .await?;
    let content = v1_game::attach_cooked_scene(&mut host)?;
    host.pause()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let cooked_generation = if let Some(stats) = &content {
        loop {
            {
                let stats = stats
                    .lock()
                    .map_err(|_| "capture content observations poisoned")?;
                if !stats.errors.is_empty() {
                    return Err(format!("capture content errors: {:?}", stats.errors));
                }
                if let Some(generation) = stats.drawn {
                    break Some(generation);
                }
            }
            if Instant::now() >= deadline {
                return Err("capture content did not become resident within 30 seconds".to_owned());
            }
            // This explicit readback waits for the encoder; no timing or cadence claim.
            drop(host.capture_pixels()?);
            std::thread::sleep(Duration::from_millis(2));
        }
    } else {
        None
    };
    std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    if let Some(frames) = motion_frames {
        return motion::record(
            &device,
            &queue,
            &mut host,
            &output,
            frames,
            cooked_generation,
        );
    }
    let mut manifest = serde_json::json!({
        "scope":"Same V1 FrameHost, frozen authority, real GPU offscreen readback. No live presentation, timing or physical-device qualification.",
        "adapter": capabilities.adapter().name,
        "extent":EXTENT,
        "scene":std::env::var("THRUM_SCENE").unwrap_or_else(|_| "diagnostic".to_owned()),
    });
    if !route_only {
        capture_diagnostics(&mut host, &output, &mut manifest)?;
    }
    if let Some(params) = params {
        manifest["route"] = serde_json::Value::Array(route(&mut host, &output, &params)?);
    }
    manifest["world_tick"] = serde_json::json!(host.tick());
    std::fs::write(
        output.join("captures.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn capture_diagnostics(
    host: &mut FrameHost,
    output: &Path,
    manifest: &mut serde_json::Value,
) -> Result<(), String> {
    let mut records = Vec::new();
    for (name, hours, yaw, pitch) in [
        ("dawn", 5.52, 0.0_f32, 0.0_f32),
        ("sunrise", 6.1, 0.0, 0.0),
        ("noon", 12.0, 0.0, 0.0),
        ("sunset", 17.9, 0.0, 0.0),
        ("twilight", 18.48, 0.0, 0.0),
        ("night", 0.0, 0.0, 0.0),
        ("night-upslope", 0.0, 0.0, 7.0),
        ("sunrise-sun-clear", 6.6, 90.0, 0.0),
        ("sunset-sun-clear", 17.4, -90.0, 0.0),
        ("sunrise-sun", 6.1, 90.0, 0.0),
        ("sunset-sun", 17.9, -90.0, 0.0),
    ] {
        host.set_camera_yaw(yaw.to_radians())?;
        host.set_camera_pitch(pitch.to_radians())?;
        host.set_time_of_day(hours / 24.0)?;
        let file = format!("{name}.ppm");
        save(host, &output.join(&file))?;
        records.push(
            serde_json::json!({"file":file,"hours":hours,"yaw_degrees":yaw,"pitch_degrees":pitch}),
        );
    }
    let sequence = output.join("sequence");
    std::fs::create_dir(&sequence).map_err(|error| error.to_string())?;
    for index in 0_u16..24 {
        let hours = 5.5 + 13.0 * f32::from(index) / 23.0;
        let yaw = (f32::from(index) / 23.0 - 0.5) * 0.2;
        host.set_time_of_day(hours / 24.0)?;
        host.set_camera_yaw(yaw)?;
        save(host, &sequence.join(format!("{index:02}.ppm")))?;
        records.push(serde_json::json!({"sequence_index":index,"hours":hours,"yaw_radians":yaw}));
    }
    let solar_sequence = output.join("solar-sequence");
    std::fs::create_dir(&solar_sequence).map_err(|error| error.to_string())?;
    host.set_camera_pitch(0.0)?;
    host.set_camera_yaw(-std::f32::consts::FRAC_PI_2)?;
    let mut solar_records = Vec::new();
    for index in 0_u16..97 {
        let hours = 17.4 + 0.8 * f32::from(index) / 96.0;
        host.set_time_of_day(hours / 24.0)?;
        let file = format!("{index:02}.ppm");
        save(host, &solar_sequence.join(&file))?;
        solar_records.push(serde_json::json!({
            "file":file, "hours":hours, "yaw_degrees":-90.0, "pitch_degrees":0.0,
        }));
        // Keep a completed-frame record even if a later capture fails.
        std::fs::write(
            solar_sequence.join("captures.json"),
            serde_json::to_vec_pretty(&solar_records).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    manifest["captures"] = serde_json::Value::Array(records);
    manifest["solar_sequence"] = serde_json::Value::Array(solar_records);
    manifest["live_sequence"] = serde_json::Value::Array(live_sequence(host, output)?);
    Ok(())
}

/// Optional consecutive-tick sunset, `THRUM_CAPTURE_LIVE_TICKS` frames from 17.7 h.
/// The time is set once and the world advances one fixed tick per readback.
/// These endpoint captures do not advance display history. Readback paces
/// the frames independently of presentation.
fn live_sequence(host: &mut FrameHost, output: &Path) -> Result<Vec<serde_json::Value>, String> {
    let Some(ticks) = std::env::var("THRUM_CAPTURE_LIVE_TICKS")
        .ok()
        .map(|value| value.parse::<u16>().map_err(|error| error.to_string()))
        .transpose()?
    else {
        return Ok(Vec::new());
    };
    let live = output.join("live-sequence");
    std::fs::create_dir(&live).map_err(|error| error.to_string())?;
    host.set_camera_pitch(0.0)?;
    host.set_camera_yaw(-std::f32::consts::FRAC_PI_2)?;
    host.set_time_of_day(17.7 / 24.0)?;
    let mut now = Duration::from_secs(1);
    host.resume(now)?;
    let mut records = Vec::new();
    for index in 0..ticks {
        let start = host.tick();
        while host.tick() == start {
            now += Duration::from_micros(16_667);
            host.advance_without_rendering(now)?;
        }
        let file = format!("{index:04}.ppm");
        save(host, &live.join(&file))?;
        records.push(serde_json::json!({
            "file":file, "tick":host.tick(), "hours":host.time_of_day() * 24.0,
        }));
    }
    host.pause()?;
    Ok(records)
}

/// Optional review route: `THRUM_CAPTURE_PARAMS`, a path list of parameter files.
/// Each file applies through the game's typed controls, the path the panel and the
/// browser page use, on top of the controls' defaults, so every stop is independent of
/// the one before. Each must pause the world, because capture never runs it. An
/// unreadable file, a refused entry or a running-world request fails the capture.
fn route(
    host: &mut FrameHost,
    output: &Path,
    params: &std::ffi::OsStr,
) -> Result<Vec<serde_json::Value>, String> {
    let files: Vec<PathBuf> = std::env::split_paths(params).collect();
    if files.iter().any(|file| file.as_os_str().is_empty()) {
        return Err("THRUM_CAPTURE_PARAMS needs one or more parameter file paths".to_owned());
    }
    let route = output.join("route");
    std::fs::create_dir(&route).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    for path in files {
        let refused = |reason: String| format!("capture parameters {}: {reason}", path.display());
        let text = std::fs::read_to_string(&path).map_err(|error| refused(error.to_string()))?;
        let mut controls = GameControls::new().map_err(|error| refused(error.to_string()))?;
        let report = controls
            .model_mut()
            .apply_file(&text)
            .map_err(|error| refused(error.to_string()))?;
        if !report.problems.is_empty() {
            let problems: Vec<String> = report.problems.iter().map(ToString::to_string).collect();
            return Err(refused(format!("{report}: {}", problems.join("; "))));
        }
        if controls.take_edits().paused != Some(true) {
            return Err(refused(
                "set world.paused to true; capture never runs the world".to_owned(),
            ));
        }
        let state = controls.replay();
        let (Some(time), Some(pitch), Some(yaw)) =
            (state.time_of_day, state.camera_pitch, state.camera_yaw)
        else {
            return Err(refused("controls replay omitted a value".to_owned()));
        };
        host.set_time_of_day(time)?;
        host.set_camera_pitch(pitch)?;
        host.set_camera_yaw(yaw)?;
        let overlay = state.display_overlay().unwrap_or(DisplayOverlay::Off);
        host.set_display_overlay(overlay);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| refused("needs a UTF-8 file name".to_owned()))?;
        let stem = name.strip_suffix(".params.json").unwrap_or(name);
        let file = format!("{stem}.ppm");
        if route.join(&file).exists() {
            return Err(refused(format!("a second file would also write {file}")));
        }
        save(host, &route.join(&file))?;
        let [pitch, yaw] = host.camera_orientation();
        records.push(serde_json::json!({
            "file":file,
            "parameters":path.display().to_string(),
            "changed":report.changed,
            "unchanged":report.unchanged,
            "hours":host.time_of_day() * 24.0,
            "yaw_degrees":yaw.to_degrees(),
            "pitch_degrees":pitch.to_degrees(),
            "exposure_zones":overlay == DisplayOverlay::ExposureZones,
        }));
    }
    host.set_display_overlay(DisplayOverlay::Off);
    Ok(records)
}

fn save(host: &mut FrameHost, path: &Path) -> Result<(), String> {
    let pixels = host.capture_pixels()?;
    let mut ppm = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
    for pixel in pixels.as_chunks::<4>().0 {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).map_err(|error| error.to_string())
}
