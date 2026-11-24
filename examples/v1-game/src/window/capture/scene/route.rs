//! Drive the moving proof route offscreen: one tick per step at the route's fixed
//! in-tick phase, with the camera installed one tick before the origin.
//! Checkpoints retain exact stored bytes from the selected submitted-image stage.

use std::{fs::File, path::Path, time::Duration};

use thrum_core::ClockConfig;
use thrum_render::{
    governor::DisplayRecord, present::SurfaceProfile, runtime::CaptureLimits,
    synthetic::CaptureSource,
};

use super::{
    Driver, RenderScaleTrace, SceneCaptureRequest,
    motion::{CaptureTarget, completed_sample, write_json_line},
};
use crate::{CameraRoute, GameHost};

/// Phase tolerance on the logged presentation alpha.
const ALPHA_TOLERANCE: f32 = 1e-4;

/// The route file named by `THRUM_CAPTURE_ROUTE`, with its declared extent and overlay.
pub(super) fn requested(
    request: &SceneCaptureRequest,
) -> Result<Option<(String, [u32; 2], crate::RouteOverlay)>, String> {
    let Some(path) = request.route.as_deref() else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("route file {}: {error}", path.display()))?;
    let (extent, overlay) = CameraRoute::declared(&text)?;
    if extent[0] % 64 != 0 || extent[1] == 0 {
        return Err("route render_extent width must be a nonzero multiple of 64".to_owned());
    }
    Ok(Some((text, extent, overlay)))
}

pub(super) fn profile(
    request: &SceneCaptureRequest,
    mut extent: [u32; 2],
) -> Result<SurfaceProfile, String> {
    let format = match request.surface_format.as_deref() {
        None | Some("bgra8") => wgpu::TextureFormat::Bgra8Unorm,
        Some("bgra8-srgb") => wgpu::TextureFormat::Bgra8UnormSrgb,
        _ => return Err("unknown capture surface format".to_owned()),
    };
    if let Some(value) = request.surface_extent.as_deref() {
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
#[expect(
    clippy::too_many_lines,
    reason = "the fixed route drives and observes one ordinary submission per step"
)]
pub(super) fn record(
    capabilities: &thrum_render::capabilities::DeviceCapabilities,
    queue: &wgpu::Queue,
    driver: &mut Driver,
    request: &SceneCaptureRequest,
    directory: &Path,
    text: &str,
    cooked_generation: Option<u64>,
) -> Result<(), String> {
    let device = capabilities.site();
    let source = match request.source.as_deref() {
        Some("sdr") | None => CaptureSource::Sdr,
        Some("scene-linear") => CaptureSource::SceneLinear,
        Some("native-output") => CaptureSource::NativeOutput,
        Some("native-history") => CaptureSource::NativeHistory,
        _ => return Err("unknown route capture source".to_owned()),
    };
    let camera = driver
        .host()
        .scene_camera()
        .ok_or("the moving route needs THRUM_SCENE=valley")?;
    let tick_hz = ClockConfig::default().tick_hz.get();
    let route = if request.scene.as_deref() == Some("exit") {
        CameraRoute::parse_long_capture(text, camera, tick_hz).map_err(|error| error.to_string())
    } else {
        CameraRoute::parse(text, camera, tick_hz)
    }?;
    if cooked_generation != Some(route.cooked_generation) {
        return Err(format!(
            "cooked generation {cooked_generation:?} is not the route's {}",
            route.cooked_generation
        ));
    }
    let [numerator, denominator] = route.phase;
    let diagnostic_phase = request
        .phase
        .as_deref()
        .map(str::parse::<f64>)
        .transpose()
        .map_err(|error| error.to_string())?;
    if diagnostic_phase.is_some_and(|value| !value.is_finite() || value <= 0.0 || value >= 1.0) {
        return Err("diagnostic capture phase must be between zero and one".to_owned());
    }
    let phi = diagnostic_phase.unwrap_or(f64::from(numerator) / f64::from(denominator));
    let profile = profile(request, route.render_extent)?;
    let target = CaptureTarget::new(route.render_extent);
    let checkpoints_dir = directory.join("checkpoints");
    std::fs::create_dir(&checkpoints_dir).map_err(|error| error.to_string())?;
    let last_route_tick = route.keys.last().map_or(0, |key| key.tick);
    let windows = capture_windows(request.windows.as_deref(), last_route_tick)?;
    let windows_dir = directory.join("windows");
    if !windows.is_empty() {
        std::fs::create_dir(&windows_dir).map_err(|error| error.to_string())?;
    }
    let mut log = File::create(directory.join("route.jsonl")).map_err(|error| error.to_string())?;
    write_json_line(
        &mut log,
        &serde_json::json!({
            "target": "native", "backend":format!("{:?}",capabilities.adapter().backend), "adapter":capabilities.adapter().name, "image_label": match source { CaptureSource::Sdr => "native submitted SDR output", CaptureSource::SceneLinear => "native submitted scene-linear RGBA16Float output", _ => "labelled native copy/select RGBA16Float diagnostic" },
            "recipe":format!("{:?}",capabilities.recipe()), "registry_digest":capabilities.registry_digest(), "resident_device_bytes":capabilities.resident_device_bytes(),
            "unified_memory":capabilities.unified_memory(), "storage_report":driver.host().storage_report(),
            "reconstruction":format!("{:?}",driver.host().reconstruction()),
            "display_overlay":request.display_overlay.as_deref().unwrap_or("off"),
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
    let last = route.origin_tick + last_route_tick;
    let mut checkpoints = route.checkpoints.clone().into_iter().peekable();
    driver.host_mut().world_mut().resume(Duration::ZERO)?;
    let mut pending = Some(route.clone());
    let scale_arm = request.scale_trace;
    let governor_identity = request.governor_identity.is_some();
    let thermal_proof = request
        .governor_thermal
        .then_some(thrum_app::GovernorProof::Thermal {
            from: 235,
            until: 310,
        });
    let display = DisplayRecord {
        interval: Duration::from_secs(1) / tick_hz,
        thermal: thrum_app::thermal_state(),
    };
    if (request.governor_identity == Some(true) || thermal_proof.is_some())
        && !driver.host_mut().enable_governor(display)
    {
        return Err("governor proof requires a prepared TAAU scene".to_owned());
    }
    #[cfg(feature = "creation-counters")]
    let mut thermal_pipelines = None;
    let mut thermal_rows = Vec::new();
    let mut scale = thrum_render::synthetic::RenderScale::Native;
    let mut loaded_frames = 0;
    let mut minimum_scale = 100;
    loop {
        let host = driver.host_mut();
        let tick = host.world().tick();
        if tick + 1 == route.origin_tick
            && let Some(route) = pending.take()
        {
            host.set_camera_route(route)?;
        }
        if tick >= last {
            break;
        }
        let time = Duration::from_secs_f64(host.route_step_ms(phi) / 1000.0);
        let next_tick = tick + 1;
        if let Some(proof) = thermal_proof {
            #[cfg(feature = "creation-counters")]
            if next_tick == 234 {
                thermal_pipelines = Some(pipeline_counts(host));
            }
            host.governor_display(proof.display_record(display, next_tick), false);
        }
        if scale_arm.is_some() {
            let next_scale =
                crate::camera::route_render_scale(next_tick.saturating_sub(route.origin_tick));
            if next_scale != scale {
                host.set_render_scale(next_scale)
                    .map_err(|error| error.to_string())?;
                if scale_arm == Some(RenderScaleTrace::ResetAtStep) {
                    host.invalidate_history();
                }
                scale = next_scale;
            }
        }
        let window = next_tick
            .checked_sub(route.origin_tick)
            .is_some_and(|routed| {
                windows
                    .iter()
                    .any(|[first, last]| (*first..=*last).contains(&routed))
            });
        let capture = thermal_proof.is_none()
            && (window
                || checkpoints
                    .peek()
                    .is_some_and(|checkpoint| route.origin_tick + checkpoint.tick == tick + 1));
        if capture {
            host.enable_submitted_capture_from(tick + 1, CaptureLimits::default(), source)?;
        }
        let frame = driver.step_at(time)?;
        let host = driver.host_mut();
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
        if governor_identity {
            loaded_frames += u32::from(
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .and_then(|gpu| {
                        gpu.passes()
                            .iter()
                            .find(|pass| pass.name.ends_with("debug load"))
                    })
                    .is_some(),
            );
            if let Some(governor) = &sample.governor {
                minimum_scale = minimum_scale.min(governor.state.scale.percent());
            }
        }
        if thermal_proof.is_some()
            && let Some(quality) = &sample.governor
        {
            thermal_rows.push(serde_json::json!({
                "tick": sample.tick, "submission": sample.submission_order,
                "observation_ns": sample.observation.as_nanos(),
                "quality": {
                    "scale": quality.state.scale.percent(), "tier": quality.state.tier,
                    "reason": format!("{:?}", quality.state.reason),
                    "thermal": format!("{:?}", quality.display.thermal),
                    "knobs": quality.knobs.iter().map(|knob| (&knob.name, knob.value))
                        .collect::<std::collections::BTreeMap<_, _>>(),
                    "aerial_age": quality.aerial_age,
                },
            }));
        }
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
                "render_extent": sample.render_extent, "jitter_phase": sample.jitter_phase,
                "history": sample.history_identity, "render_scale": scale.percent(),
                "dropped_light_references":sample.dropped_light_references,
                "gpu_pass_ns":sample.gpu_pass_ns.as_ref().map(thrum_render::timing::SceneGpuTimings::values),
                "checkpoint": checkpoint,
            }),
        )?;
        if capture {
            let mut image = target.submitted_image(host, sample.submission_order, source)?;
            let identity = serde_json::json!({"source_tick":image.source.tick,"epoch":image.source.epoch,
                "timeline":image.source.timeline,"alpha_bits":image.sample.presentation_alpha.to_bits(),
                "history_epoch":image.history_epoch,"submission_id":image.sample.submission_order,
                "exposure_gpu_words":image.exposure_gpu_words,"capture_source":format!("{source:?}"),
                "device_generation":image.device_generation,"presenter_generation":image.presenter_generation,"headroom":1});
            if window {
                retain_image(
                    &windows_dir,
                    &format!("tick-{:06}", frame.tick),
                    source,
                    &identity,
                    &image.rgba,
                )?;
            }
            if let Some(name) = &checkpoint {
                retain_image(&checkpoints_dir, name, source, &identity, &image.rgba)?;
            }
            if let (Some(name), CaptureSource::Sdr) = (&checkpoint, source) {
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
            if let Some(name) = checkpoint
                && request.present_control.as_deref() == Some("on")
            {
                let pixels = surface_pixels(driver.output(), device, queue)?;
                std::fs::write(
                    checkpoints_dir.join(format!("{name}.surface-bgra8")),
                    pixels,
                )
                .map_err(|error| error.to_string())?;
            }
        }
    }
    if thermal_proof.is_some() {
        #[cfg(feature = "creation-counters")]
        let pipelines = thermal_pipelines.map(|before| [before, pipeline_counts(driver.host())]);
        #[cfg(not(feature = "creation-counters"))]
        let pipelines: Option<[[u64; 2]; 2]> = None;
        std::fs::write(
            directory.join("thermal.json"),
            serde_json::json!({
                "samples": thermal_rows, "pipelines": pipelines,
                "platform_thermal": [format!("{:?}", display.thermal),
                                     format!("{:?}", thrum_app::thermal_state())],
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if governor_identity {
        let world = driver.host().world();
        let hash = world
            .inline_authority_hash()
            .ok_or("identity needs the inline game world")?;
        std::fs::write(
            directory.join("authority.json"),
            serde_json::json!({
                "tick": world.tick(), "hash": format!("{hash:016x}"),
                "loaded_frames": loaded_frames, "minimum_scale": minimum_scale,
            })
            .to_string(),
        )
        .map_err(|error| error.to_string())?;
    }
    if let Some(missed) = checkpoints.next() {
        return Err(format!("route ended before checkpoint {}", missed.name));
    }
    Ok(())
}

#[cfg(feature = "creation-counters")]
fn pipeline_counts(host: &crate::host::Runtime) -> [u64; 2] {
    let report = host.creation_report();
    [
        report
            .pipelines
            .iter()
            .map(|row| row.render + row.compute + row.mesh)
            .sum(),
        report.violations,
    ]
}

fn capture_windows(value: Option<&str>, last_tick: u64) -> Result<Vec<[u64; 2]>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let windows: Vec<[u64; 2]> =
        serde_json::from_str(value).map_err(|error| format!("capture windows: {error}"))?;
    if windows
        .iter()
        .any(|[first, last]| first > last || *last > last_tick)
    {
        return Err(format!(
            "capture windows must be ordered endpoints within 0..={last_tick}"
        ));
    }
    Ok(windows)
}

fn retain_image(
    directory: &Path,
    name: &str,
    source: CaptureSource,
    identity: &serde_json::Value,
    rgba: &[u8],
) -> Result<(), String> {
    let suffix = if source == CaptureSource::Sdr {
        "rgba8"
    } else {
        "rgba16f"
    };
    let identity = serde_json::to_vec_pretty(identity).map_err(|error| error.to_string())?;
    std::fs::write(directory.join(format!("{name}.identity.json")), identity)
        .and_then(|()| std::fs::write(directory.join(format!("{name}.{suffix}")), rgba))
        .map_err(|error| error.to_string())
}

fn write_ppm(path: &Path, extent: [u32; 2], bgra: &[u8]) -> Result<(), String> {
    let mut ppm = format!("P6\n{} {}\n255\n", extent[0], extent[1]).into_bytes();
    for pixel in bgra.as_chunks::<4>().0 {
        ppm.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }
    std::fs::write(path, ppm).map_err(|error| error.to_string())
}

const SURFACE_TIMEOUT: Duration = Duration::from_secs(10);

/// Diagnostic present-control readback, after the ordinary submission.
fn surface_pixels(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Vec<u8>, String> {
    let width = texture.width();
    let height = texture.height();
    let row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fixed-time present control"),
        size: u64::from(row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    let submission = queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        let _ = send.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(SURFACE_TIMEOUT),
        })
        .map_err(|error| error.to_string())?;
    receive
        .recv_timeout(SURFACE_TIMEOUT)
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let mapped = buffer
        .get_mapped_range(..)
        .map_err(|error| error.to_string())?;
    let pixels = mapped
        .chunks_exact(row as usize)
        .flat_map(|bytes| bytes[..width as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}
