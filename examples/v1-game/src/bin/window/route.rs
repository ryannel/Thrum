//! Drive the moving proof route offscreen: one tick per step at the route's fixed
//! in-tick phase, with the camera installed one tick before the origin.
//! Checkpoints retain exact stored bytes from the selected submitted-image stage.

use std::{fs::File, path::Path, time::Duration};

use thrum_core::ClockConfig;
use thrum_render::{present::SurfaceProfile, synthetic::CaptureSource};
use v1_game::{CameraRoute, FrameHost, SurfaceOutput};

use super::motion::{CaptureTarget, completed_sample, write_json_line};

/// Phase tolerance on the logged presentation alpha.
const ALPHA_TOLERANCE: f32 = 1e-4;

/// The route file named by `THRUM_CAPTURE_ROUTE`, with its declared extent and overlay.
pub(super) fn requested() -> Result<Option<(String, [u32; 2], v1_game::RouteOverlay)>, String> {
    let Some(path) = std::env::var_os("THRUM_CAPTURE_ROUTE") else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("route file {}: {error}", Path::new(&path).display()))?;
    let (extent, overlay) = CameraRoute::declared(&text)?;
    if extent[0] % 64 != 0 || extent[1] == 0 {
        return Err("route render_extent width must be a nonzero multiple of 64".to_owned());
    }
    Ok(Some((text, extent, overlay)))
}

pub(super) fn profile(mut extent: [u32; 2]) -> Result<SurfaceProfile, String> {
    let format = match std::env::var("THRUM_CAPTURE_SURFACE_FORMAT").as_deref() {
        Err(_) | Ok("bgra8") => wgpu::TextureFormat::Bgra8Unorm,
        Ok("bgra8-srgb") => wgpu::TextureFormat::Bgra8UnormSrgb,
        _ => return Err("unknown capture surface format".to_owned()),
    };
    if let Ok(value) = std::env::var("THRUM_CAPTURE_SURFACE_EXTENT") {
        let (width, height) = value
            .split_once('x')
            .ok_or("surface extent must be WIDTHxHEIGHT")?;
        extent = [
            width.parse::<u32>().map_err(|e| e.to_string())?,
            height.parse::<u32>().map_err(|e| e.to_string())?,
        ];
        if extent.contains(&0) || extent.iter().any(|&value| value > 16_384) {
            return Err("invalid capture surface extent".to_owned());
        }
    }
    Ok(SurfaceProfile { format, extent })
}

/// Run the route from world time zero and write `route.jsonl` and checkpoint images.
pub(super) fn record(
    capabilities: &thrum_render::capabilities::DeviceCapabilities,
    queue: &wgpu::Queue,
    host: &mut FrameHost,
    directory: &Path,
    text: &str,
    cooked_generation: Option<u64>,
) -> Result<(), String> {
    let device = capabilities.site();
    let source = match std::env::var("THRUM_CAPTURE_SOURCE").as_deref() {
        Ok("sdr") | Err(_) => CaptureSource::Sdr,
        Ok("scene-linear") => CaptureSource::SceneLinear,
        _ => return Err("unknown route capture source".to_owned()),
    };
    let camera = host
        .scene_camera()
        .ok_or("the moving route needs THRUM_SCENE=valley")?;
    let tick_hz = ClockConfig::default().tick_hz.get();
    let route = CameraRoute::parse(text, camera, tick_hz)?;
    if cooked_generation != Some(route.cooked_generation) {
        return Err(format!(
            "cooked generation {cooked_generation:?} is not the route's {}",
            route.cooked_generation
        ));
    }
    let [numerator, denominator] = route.phase;
    let diagnostic_phase = std::env::var("THRUM_CAPTURE_PHASE")
        .ok()
        .map(|value| value.parse::<f64>())
        .transpose()
        .map_err(|error| error.to_string())?;
    if diagnostic_phase.is_some_and(|value| !value.is_finite() || value <= 0.0 || value >= 1.0) {
        return Err("diagnostic capture phase must be between zero and one".to_owned());
    }
    let phi = diagnostic_phase.unwrap_or(f64::from(numerator) / f64::from(denominator));
    let profile = profile(route.render_extent)?;
    let target = CaptureTarget::for_surface(device, route.render_extent, profile);
    let surface = SurfaceOutput {
        view: &target.view,
        extent: profile.extent,
    };
    let checkpoints_dir = directory.join("checkpoints");
    std::fs::create_dir(&checkpoints_dir).map_err(|error| error.to_string())?;
    let mut log = File::create(directory.join("route.jsonl")).map_err(|error| error.to_string())?;
    write_json_line(
        &mut log,
        &serde_json::json!({
            "target": "native", "backend":format!("{:?}",capabilities.adapter().backend), "adapter":capabilities.adapter().name, "image_label": if source == CaptureSource::Sdr { "native submitted SDR output" } else { "native submitted scene-linear RGBA16Float output" },
            "capture_source": format!("{source:?}"), "draw_probe_enabled":cfg!(feature="draw-probe"), "bytes_per_pixel": source.bytes_per_pixel(), "mode":"fixed",
            "render_extent": route.render_extent, "window": "n/a, offscreen",
            "surface_format":format!("{:?}",profile.format), "surface_extent":profile.extent,
            "surface_control":"diagnostic copy of ordinary presenter output after its submission",
            "tick_hz": tick_hz, "phase": route.phase, "origin_tick": route.origin_tick,
            "diagnostic_phase_override":diagnostic_phase,
            "cooked_generation": cooked_generation, "overlay": format!("{:?}", route.overlay),
            "presentation_lead_ns": 0,
        }),
    )?;
    let last = route.origin_tick + route.keys.last().map_or(0, |key| key.tick);
    let mut checkpoints = route.checkpoints.clone().into_iter().peekable();
    host.resume(Duration::ZERO)?;
    let mut pending = Some(route.clone());
    loop {
        let tick = host.tick();
        if tick + 1 == route.origin_tick
            && let Some(route) = pending.take()
        {
            host.set_camera_route(route)?;
        }
        if tick >= last {
            break;
        }
        let time = Duration::from_secs_f64(host.route_step_ms(phi) / 1000.0);
        let capture = checkpoints
            .peek()
            .is_some_and(|checkpoint| route.origin_tick + checkpoint.tick == tick + 1);
        if capture {
            host.enable_submitted_capture_from(tick + 1, source)?;
        }
        let frame = host.drive(time, Some(&surface))?;
        if !frame.surface_presented
            || frame.dropped_view
            || frame.overload.is_some()
            || frame.completed_ticks != 1
            || frame.queued_ticks != 0
            || frame.tick != tick + 1
        {
            return Err(format!(
                "route step at tick {} did not submit one fresh update: {frame:?}",
                tick + 1
            ));
        }
        let index = u32::try_from(frame.tick).map_err(|error| error.to_string())?;
        let sample = completed_sample(host, index, frame.tick)?;
        let alpha = sample.presentation_alpha;
        #[expect(clippy::cast_possible_truncation, reason = "phase is a small fraction")]
        if (alpha - phi as f32).abs() > ALPHA_TOLERANCE {
            return Err(format!(
                "route step at tick {} drifted to alpha {alpha}",
                frame.tick
            ));
        }
        if frame.tick < route.origin_tick {
            continue;
        }
        let pose = host
            .route_pose(frame.tick, alpha)
            .ok_or("route not installed at the origin")?;
        let committed = host.time_of_day() * 24.0;
        let rendered = host.rendered_time_of_day(alpha) * 24.0;
        let checkpoint = checkpoints
            .next_if(|checkpoint| route.origin_tick + checkpoint.tick == frame.tick)
            .map(|checkpoint| checkpoint.name);
        write_json_line(
            &mut log,
            &serde_json::json!({
                "tick": frame.tick, "route_tick": frame.tick - route.origin_tick,
                "alpha": alpha, "alpha_bits": alpha.to_bits(),
                "committed_solar_hours": committed, "committed_solar_bits": committed.to_bits(),
                "rendered_solar_hours": rendered, "rendered_solar_bits": rendered.to_bits(),
                "eye": pose.eye, "yaw": pose.yaw, "pitch": pose.pitch, "epoch": pose.epoch,
                "submission": sample.submission_order, "visual_revision": sample.visual_revision,
                "dropped_light_references":sample.dropped_light_references,
                "checkpoint": checkpoint,
            }),
        )?;
        if let Some(name) = checkpoint {
            let mut image = target.submitted_image(host, sample.submission_order, source)?;
            std::fs::write(checkpoints_dir.join(format!("{name}.identity.json")),
                serde_json::to_vec_pretty(&serde_json::json!({"source_tick":image.source.tick,"epoch":image.source.epoch,
                    "timeline":image.source.timeline,"alpha_bits":image.sample.presentation_alpha.to_bits(),
                    "history_epoch":image.history_epoch,"submission_id":image.sample.submission_order,
                    "exposure_gpu_words":image.exposure_gpu_words,"capture_source":format!("{source:?}"),
                    "device_generation":image.device_generation,"presenter_generation":image.presenter_generation,"headroom":1}))
                    .map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
            let suffix = if source == CaptureSource::Sdr {
                "rgba8"
            } else {
                "rgba16f"
            };
            std::fs::write(
                checkpoints_dir.join(format!("{name}.{suffix}")),
                &image.rgba,
            )
            .map_err(|error| error.to_string())?;
            if source == CaptureSource::Sdr {
                for pixel in image.rgba.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
                write_ppm(
                    &checkpoints_dir.join(format!("{name}.ppm")),
                    route.render_extent,
                    &image.rgba,
                )?;
            }
            host.recycle_submitted_image(image);
            host.disable_submitted_capture();
            if std::env::var("THRUM_CAPTURE_PRESENT_CONTROL").as_deref() == Ok("on") {
                let pixels = target.surface_pixels(device, queue)?;
                std::fs::write(
                    checkpoints_dir.join(format!("{name}.surface-bgra8")),
                    pixels,
                )
                .map_err(|error| error.to_string())?;
            }
        }
    }
    if let Some(missed) = checkpoints.next() {
        return Err(format!("route ended before checkpoint {}", missed.name));
    }
    Ok(())
}

fn write_ppm(path: &Path, extent: [u32; 2], bgra: &[u8]) -> Result<(), String> {
    let mut ppm = format!("P6\n{} {}\n255\n", extent[0], extent[1]).into_bytes();
    for pixel in bgra.as_chunks::<4>().0 {
        ppm.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    std::fs::write(path, ppm).map_err(|error| error.to_string())
}
