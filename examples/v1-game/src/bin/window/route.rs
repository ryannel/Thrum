//! Drive the moving proof route offscreen: one tick per step at the route's fixed
//! in-tick phase, with the camera installed one tick before the origin.
//! Images are the native submitted SDR output, not a pre-display capture.

use std::{fs::File, path::Path, time::Duration};

use thrum_core::ClockConfig;
use thrum_render::present::SurfaceProfile;
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

pub(super) fn profile(extent: [u32; 2]) -> SurfaceProfile {
    SurfaceProfile {
        format: wgpu::TextureFormat::Bgra8Unorm,
        extent,
    }
}

/// Run the route from world time zero and write `route.jsonl` and checkpoint images.
pub(super) fn record(
    device: &wgpu::Device,
    host: &mut FrameHost,
    directory: &Path,
    text: &str,
    cooked_generation: Option<u64>,
) -> Result<(), String> {
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
    let phi = f64::from(numerator) / f64::from(denominator);
    let target = CaptureTarget::new(device, route.render_extent);
    let surface = SurfaceOutput {
        view: &target.view,
        extent: route.render_extent,
    };
    let checkpoints_dir = directory.join("checkpoints");
    std::fs::create_dir(&checkpoints_dir).map_err(|error| error.to_string())?;
    let mut log = File::create(directory.join("route.jsonl")).map_err(|error| error.to_string())?;
    write_json_line(
        &mut log,
        &serde_json::json!({
            "target": "native", "image_label": "native submitted SDR output",
            "render_extent": route.render_extent, "window": "n/a, offscreen",
            "tick_hz": tick_hz, "phase": route.phase, "origin_tick": route.origin_tick,
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
            host.enable_submitted_capture(tick + 1)?;
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
                "checkpoint": checkpoint,
            }),
        )?;
        if let Some(name) = checkpoint {
            let mut bgra = Vec::new();
            target.write_pixels(host, sample.submission_order, &mut bgra)?;
            host.disable_submitted_capture();
            write_ppm(
                &checkpoints_dir.join(format!("{name}.ppm")),
                route.render_extent,
                &bgra,
            )?;
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
