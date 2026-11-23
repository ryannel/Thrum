//! Capture the game to disk with a frozen or fixed-step world, on the engine's offscreen
//! driver: lighting stills, parameter routes, consecutive ticks, a motion sequence or the
//! moving proof route. None of these claims live presentation.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use thrum_app::offscreen::Offscreen;
use thrum_render::{
    capabilities::{FIRST_DEVICE_GENERATION, HardwareRequest},
    present::SurfaceProfile,
    synthetic::{DisplayOverlay, OUTPUT_FORMAT},
};

use crate::{
    GameHost,
    controls::GameControls,
    host::Runtime,
    hosted::HostedGame,
    window::{SceneCapture, WindowGame},
};

mod display;
mod motion;
mod route;

/// Scripted scale changes for a fixed-step capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderScaleTrace {
    /// Run the route retaining the ordinary submitted history.
    Continuous,
    /// Invalidate history at every scale change for the image oracle.
    ResetAtStep,
}

/// What a scene capture records. The binary reads it from the launch settings named on each
/// field; content selection still comes from `selected_setup`'s launch settings.
#[derive(Clone, Debug, Default)]
pub struct SceneCaptureRequest {
    /// `THRUM_SCENE`: the selected content's name, for the record and the exit route.
    pub scene: Option<String>,
    /// `THRUM_CAPTURE_MOTION_FRAMES`: record this many consecutive ticks as a motion sequence.
    pub motion_frames: Option<String>,
    /// `THRUM_CAPTURE_PAN_TO_DEGREES`: pan the motion sequence's camera to this yaw.
    pub pan_to_degrees: Option<String>,
    /// `THRUM_PARAMS`: a parameter file applied before the motion sequence.
    pub initial_params: Option<PathBuf>,
    /// `THRUM_CAPTURE_ROUTE`: the moving proof route's file.
    pub route: Option<PathBuf>,
    /// `THRUM_CAPTURE_DISPLAY_OVERLAY`: the route's display diagnostic.
    pub display_overlay: Option<String>,
    /// `THRUM_CAPTURE_EDR_HEADROOM`: a forced headroom for submitted still captures.
    pub edr_headroom: Option<f32>,
    /// `THRUM_CAPTURE_SOURCE`: the route's submitted-image stage.
    pub source: Option<String>,
    /// `THRUM_CAPTURE_PHASE`: a diagnostic in-tick phase in place of the route's.
    pub phase: Option<String>,
    /// `THRUM_CAPTURE_SURFACE_FORMAT`: the format of the texture standing in for the surface.
    pub surface_format: Option<String>,
    /// `THRUM_CAPTURE_SURFACE_EXTENT`: its extent, `WIDTHxHEIGHT`.
    pub surface_extent: Option<String>,
    /// `THRUM_CAPTURE_WINDOWS`: routed tick ranges whose every frame is kept, as JSON.
    pub windows: Option<String>,
    /// Scripted scale changes while driving the moving route.
    pub scale_trace: Option<RenderScaleTrace>,
    /// `THRUM_CAPTURE_PRESENT_CONTROL`: `on` keeps each checkpoint's surface texture too.
    pub present_control: Option<String>,
    /// `THRUM_CAPTURE_PARAMS`: parameter files, one still each.
    pub params: Option<Vec<PathBuf>>,
    /// `THRUM_CAPTURE_ROUTE_ONLY`: take only the parameter stills.
    pub route_only: bool,
    /// `THRUM_CAPTURE_LIVE_TICKS`: a consecutive-tick sunset of this many frames.
    pub live_ticks: Option<String>,
    /// `THRUM_G3T_OMIT_PLACEMENT`: the draw probe's omitted placement.
    pub omit_placement: Option<String>,
    /// `THRUM_G3T_OVERLAY_CONTROL` is set.
    pub overlay_control: bool,
}

/// The offscreen driver holding the game prepared for a scene capture.
type Driver = Offscreen<HostedGame<WindowGame>>;

const EXTENT: [u32; 2] = [1280, 720];

pub(in crate::window) async fn run(
    output: &Path,
    request: &SceneCaptureRequest,
    hardware: &[HardwareRequest],
) -> Result<(), String> {
    if output.exists() {
        return Err("capture output must be a new directory".to_owned());
    }
    let output = output.to_owned();
    let motion_frames = motion::requested_frames(request)?;
    let moving_route = route::requested(request)?;
    let mut setup = crate::selected_setup()?;
    if let Some((_, _, overlay)) = &moving_route {
        if motion_frames.is_some() || request.params.is_some() {
            return Err("a moving route capture takes no motion or parameter capture".to_owned());
        }
        if setup.route_overlay != *overlay {
            return Err("THRUM_ROUTE_OVERLAY must match the route file's overlay".to_owned());
        }
        setup.route_overlay = *overlay;
    }
    let route_only = request.route_only;
    let params = request.params.as_deref();
    if route_only && (params.is_none() || motion_frames.is_some()) {
        return Err(
            "route-only capture needs THRUM_CAPTURE_PARAMS and no motion capture".to_owned(),
        );
    }
    #[cfg(feature = "draw-probe")]
    let omit = request
        .omit_placement
        .as_deref()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|error| error.to_string())?;
    #[cfg(not(feature = "draw-probe"))]
    if request.omit_placement.is_some() || request.overlay_control {
        return Err("G3t omission controls require the draw-probe build feature".to_owned());
    }
    let (render_extent, surface) = match &moving_route {
        Some((_, extent, _)) => (*extent, route::profile(request, *extent)?),
        None => match motion_frames {
            Some(_) => (motion::PROFILE.extent, motion::PROFILE),
            // A still is an endpoint capture; the engine texture standing in for the surface
            // takes no part in it.
            None => (
                EXTENT,
                SurfaceProfile {
                    format: if request.edr_headroom.is_some() {
                        wgpu::TextureFormat::Rgba16Float
                    } else {
                        OUTPUT_FORMAT
                    },
                    extent: EXTENT,
                },
            ),
        },
    };
    let instance = super::metal_instance();
    let (_, device, queue, capabilities) =
        super::metal_device(&instance, FIRST_DEVICE_GENERATION, hardware).await?;
    let game = HostedGame(WindowGame {
        demo: GameControls::new().map_err(|error| error.to_string())?,
        world: None,
        held_keys: 0,
        capture_route: None,
        capture_eye: None,
        capture_cooked: None,
        cloud_capture: false,
        scene: Some(SceneCapture {
            setup,
            render_extent,
            #[cfg(feature = "draw-probe")]
            omit,
            content: None,
        }),
        hardware: Vec::new(),
    });
    let mut driver = Driver::with_surface(game, &device, &capabilities, &queue, surface)?;
    let content = driver
        .game()
        .0
        .scene
        .as_ref()
        .and_then(|scene| scene.content.clone());
    driver.host_mut().world_mut().pause()?;
    let cooked_generation = content
        .map(|stats| {
            thrum_asset_gpu::await_drawn(driver.host_mut(), &stats, thrum_asset_gpu::Drawn::Any)
        })
        .transpose()?;
    std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    if let Some((text, _, _)) = &moving_route {
        let overlay = match request.display_overlay.as_deref() {
            None | Some("off") => DisplayOverlay::Off,
            Some("motion") => DisplayOverlay::Motion,
            Some("exposure-zones") => DisplayOverlay::ExposureZones,
            _ => return Err("unknown route display overlay".to_owned()),
        };
        driver.host_mut().set_display_overlay(overlay);
        return route::record(
            &capabilities,
            &queue,
            &mut driver,
            request,
            &output,
            text,
            cooked_generation,
        );
    }
    if let Some(frames) = motion_frames {
        return motion::record(&mut driver, request, &output, frames, cooked_generation);
    }
    if request.edr_headroom.is_some()
        || matches!(
            request.source.as_deref(),
            Some("submitted-sdr" | "submitted-linear")
        )
    {
        if !route_only {
            return Err("submitted stills need the still route's parameter files".into());
        }
        let records = display::record(
            &mut driver,
            &output,
            params.ok_or("submitted stills need parameter files")?,
            request.edr_headroom,
            request.source.as_deref() == Some("submitted-linear"),
        )?;
        return std::fs::write(output.join("stills.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "adapter": capabilities.adapter().name, "backend": format!("{:?}", capabilities.adapter().backend),
            "recipe": format!("{:?}", capabilities.recipe()), "extent": EXTENT,
            "mode": "still", "headroom": request.edr_headroom, "stills": records
        })).map_err(|error| error.to_string())?).map_err(|error| error.to_string());
    }
    let host = driver.host_mut();
    let mut manifest = serde_json::json!({
        "scope":"The V1 game on the engine's offscreen driver, frozen authority, real GPU offscreen readback. No live presentation, timing or physical-device qualification.",
        "adapter": capabilities.adapter().name,
        "extent":EXTENT,
        "scene":request.scene.as_deref().unwrap_or("diagnostic"),
        "cloud_shade": if setup.diagnostic_clouds { "diagnostic stand-in, plane 2000 m, XZ drift [160,40] m/s" } else { "off" },
    });
    if !route_only {
        capture_diagnostics(host, request, &output, &mut manifest)?;
    }
    if let Some(params) = params {
        manifest["route"] = serde_json::Value::Array(route(host, &output, params)?);
    }
    manifest["world_tick"] = serde_json::json!(host.world().tick());
    std::fs::write(
        output.join("captures.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn capture_diagnostics(
    host: &mut Runtime,
    request: &SceneCaptureRequest,
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
    manifest["live_sequence"] =
        serde_json::Value::Array(live_sequence(host, request.live_ticks.as_deref(), output)?);
    Ok(())
}

/// Optional consecutive-tick sunset, `THRUM_CAPTURE_LIVE_TICKS` frames from 17.7 h.
/// The time is set once and the world advances one fixed tick per readback.
/// These endpoint captures do not advance display history. Readback paces
/// the frames independently of presentation.
fn live_sequence(
    host: &mut Runtime,
    ticks: Option<&str>,
    output: &Path,
) -> Result<Vec<serde_json::Value>, String> {
    let Some(ticks) = ticks
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
    host.world_mut().resume(now)?;
    let mut records = Vec::new();
    for index in 0..ticks {
        let start = host.world().tick();
        while host.world().tick() == start {
            now += Duration::from_micros(16_667);
            host.advance_without_rendering(now)?;
        }
        let file = format!("{index:04}.ppm");
        save(host, &live.join(&file))?;
        records.push(serde_json::json!({
            "file":file, "tick":host.world().tick(), "hours":host.time_of_day() * 24.0,
        }));
    }
    host.world_mut().pause()?;
    Ok(records)
}

/// Optional review route: `THRUM_CAPTURE_PARAMS`, a path list of parameter files.
/// Each file applies through the game's typed controls, the path the panel and the
/// browser page use, on top of the controls' defaults, so every stop is independent of
/// the one before. Each must pause the world, because capture never runs it. An
/// unreadable file, a refused entry or a running-world request fails the capture.
fn route(
    host: &mut Runtime,
    output: &Path,
    files: &[PathBuf],
) -> Result<Vec<serde_json::Value>, String> {
    if files.is_empty() || files.iter().any(|file| file.as_os_str().is_empty()) {
        return Err("THRUM_CAPTURE_PARAMS needs one or more parameter file paths".to_owned());
    }
    let route = output.join("route");
    std::fs::create_dir(&route).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    for path in files {
        let (overlay, report) = apply_still(host, path)?;
        let refused = |reason: String| format!("capture parameters {}: {reason}", path.display());
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

fn apply_still(
    host: &mut Runtime,
    path: &Path,
) -> Result<(DisplayOverlay, thrum_controls::FileReport), String> {
    let refused = |reason: String| format!("capture parameters {}: {reason}", path.display());
    let text = std::fs::read_to_string(path).map_err(|error| refused(error.to_string()))?;
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
    Ok((overlay, report))
}

fn save(host: &mut Runtime, path: &Path) -> Result<(), String> {
    let pixels = host.capture_pixels()?;
    let mut ppm = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
    for pixel in pixels.as_chunks::<4>().0 {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).map_err(|error| error.to_string())
}
