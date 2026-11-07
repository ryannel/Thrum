//! Capture the real game host and its selected content, without claiming live presentation.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use thrum_render::capabilities::DeviceCapabilities;
use v1_game::{FrameConfig, FrameHost};

const EXTENT: [u32; 2] = [1280, 720];

pub(super) async fn run(output: PathBuf) -> Result<(), String> {
    if output.exists() {
        return Err("capture output must be a new directory".to_owned());
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
            render_extent: EXTENT,
            surface: None,
        },
        v1_game::selected_setup()?,
    )
    .await?;
    let content = v1_game::attach_cooked_scene(&mut host)?;
    host.pause()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    if let Some(stats) = &content {
        loop {
            {
                let stats = stats
                    .lock()
                    .map_err(|_| "capture content observations poisoned")?;
                if !stats.errors.is_empty() {
                    return Err(format!("capture content errors: {:?}", stats.errors));
                }
                if stats.drawn.is_some() {
                    break;
                }
            }
            if Instant::now() >= deadline {
                return Err("capture content did not become resident within 30 seconds".to_owned());
            }
            // This explicit readback waits for the encoder; no timing or cadence claim.
            drop(host.capture_pixels()?);
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    for (name, hours, yaw) in [
        ("dawn", 5.52, 0.0_f32),
        ("sunrise", 6.1, 0.0),
        ("noon", 12.0, 0.0),
        ("sunset", 17.9, 0.0),
        ("twilight", 18.48, 0.0),
        ("night", 0.0, 0.0),
        ("sunrise-sun-clear", 6.6, 90.0),
        ("sunset-sun-clear", 17.4, -90.0),
        ("sunrise-sun", 6.1, 90.0),
        ("sunset-sun", 17.9, -90.0),
    ] {
        host.set_camera_yaw(yaw.to_radians())?;
        host.set_time_of_day(hours / 24.0)?;
        let file = format!("{name}.ppm");
        save(&mut host, &output.join(&file))?;
        records.push(serde_json::json!({"file":file,"hours":hours,"yaw_degrees":yaw}));
    }
    let sequence = output.join("sequence");
    std::fs::create_dir(&sequence).map_err(|error| error.to_string())?;
    for index in 0_u16..24 {
        let hours = 5.5 + 13.0 * f32::from(index) / 23.0;
        let yaw = (f32::from(index) / 23.0 - 0.5) * 0.2;
        host.set_time_of_day(hours / 24.0)?;
        host.set_camera_yaw(yaw)?;
        save(&mut host, &sequence.join(format!("{index:02}.ppm")))?;
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
        save(&mut host, &solar_sequence.join(&file))?;
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
    let live_records = live_sequence(&mut host, &output)?;
    let manifest = serde_json::json!({
        "scope":"Same V1 FrameHost, frozen authority, real GPU offscreen readback. No live presentation, timing or physical-device qualification.",
        "adapter": capabilities.adapter().name,
        "extent":EXTENT,
        "world_tick":host.tick(),
        "scene":std::env::var("THRUM_SCENE").unwrap_or_else(|_| "diagnostic".to_owned()),
        "captures":records,
        "solar_sequence":solar_records,
        "live_sequence":live_records,
    });
    std::fs::write(
        output.join("captures.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Optional consecutive-tick sunset, `THRUM_CAPTURE_LIVE_TICKS` frames from 17.7 h.
/// The time is set once, then the world advances one fixed tick per frame, so
/// temporal history carries over as it would live. Readback paces the frames;
/// this makes no cadence or presentation claim.
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

fn save(host: &mut FrameHost, path: &Path) -> Result<(), String> {
    let pixels = host.capture_pixels()?;
    let mut ppm = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
    for pixel in pixels.as_chunks::<4>().0 {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).map_err(|error| error.to_string())
}
