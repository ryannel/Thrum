//! macOS V1 scene on a real surface.

#[cfg(target_os = "macos")]
mod metal_preparation;

#[cfg(target_os = "macos")]
fn main() {
    let names = match std::env::var("THRUM_HARDWARE") {
        Ok(names) => names,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(error) => report_capture_failure(&format!("THRUM_HARDWARE: {error}")),
    };
    let mut hardware = thrum_render::capabilities::HardwareRequest::parse(&names)
        .unwrap_or_else(|error| report_capture_failure(&error.to_string()));
    #[cfg(feature = "proving")]
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_RUNTIME") {
        let mut hardware = hardware;
        for request in &mut hardware {
            request.required = true;
        }
        let scenario = std::env::var("THRUM_CAPTURE_CASE").unwrap_or_else(|_| "pan".to_owned());
        let (wait, hold, recovery) =
            capture_admission().unwrap_or_else(|error| report_capture_failure(&error));
        let capture_output = match std::env::var(v1_game::CAPTURE_OBSERVER_VARIABLE).as_deref() {
            Ok("1") => v1_game::RuntimeCaptureOutput::ObserverOnly,
            Err(std::env::VarError::NotPresent) => v1_game::RuntimeCaptureOutput::Images,
            _ => report_capture_failure("capture observer arm must be 1 when set"),
        };
        #[cfg(feature = "draw-probe")]
        let capture_output = match std::env::var("THRUM_VALLEY_MESH_STAGE_PROBE").as_deref() {
            Err(std::env::VarError::NotPresent) => capture_output,
            Ok("culled") if capture_output == v1_game::RuntimeCaptureOutput::Images => {
                v1_game::RuntimeCaptureOutput::MeshStageImages
            }
            _ => report_capture_failure(
                "mesh-stage capture requires THRUM_VALLEY_MESH_STAGE_PROBE=culled with image capture",
            ),
        };
        let capture_output =
            match render_scale_trace().unwrap_or_else(|error| report_capture_failure(&error)) {
                Some(v1_game::RenderScaleTrace::Continuous)
                    if capture_output == v1_game::RuntimeCaptureOutput::Images =>
                {
                    v1_game::RuntimeCaptureOutput::ScaleTraceImages
                }
                Some(_) => {
                    report_capture_failure("realtime render scale needs continuous image capture")
                }
                None => capture_output,
            };
        let capture_output = if governor_enabled()
            .unwrap_or_else(|error| report_capture_failure(&error))
        {
            let images = match capture_output {
                v1_game::RuntimeCaptureOutput::Images => true,
                v1_game::RuntimeCaptureOutput::ObserverOnly => false,
                _ => report_capture_failure("governor capture needs the ordinary real-time route"),
            };
            v1_game::RuntimeCaptureOutput::Governor { images }
        } else {
            capture_output
        };
        if let Err(error) = v1_game::run_runtime_capture(
            std::path::Path::new(&output),
            &scenario,
            wait,
            hold,
            recovery,
            capture_output,
            status_capture().unwrap_or_else(|error| report_capture_failure(&error)),
            std::env::var("THRUM_CAPTURE_MENU_STATE").ok(),
            hardware,
        ) {
            report_capture_failure(&error);
        }
        return;
    }
    let metal_preparation = metal_preparation::options(&mut hardware)
        .unwrap_or_else(|error| report_capture_failure(&error));
    #[cfg(feature = "proving")]
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_SCENE") {
        let mut hardware = hardware;
        for request in &mut hardware {
            request.required = true;
        }
        let request = scene_capture_request(metal_preparation)
            .unwrap_or_else(|error| report_capture_failure(&error));
        if let Err(error) =
            v1_game::run_scene_capture(std::path::Path::new(&output), &request, &hardware)
        {
            report_capture_failure(&error);
        }
        return;
    }
    #[cfg(feature = "proving")]
    let window_capture =
        window_extent_capture().unwrap_or_else(|error| report_capture_failure(&error));
    #[cfg(feature = "proving")]
    let transition =
        window_transition_proof().unwrap_or_else(|error| report_capture_failure(&error));
    #[cfg(feature = "proving")]
    let qualification =
        window_qualification().unwrap_or_else(|error| report_capture_failure(&error));
    #[cfg(feature = "proving")]
    let display = window_display_options(transition.as_ref())
        .unwrap_or_else(|error| report_capture_failure(&error.to_string()));
    let governor = v1_game::WindowGovernor {
        enabled: governor_enabled().unwrap_or_else(|error| report_capture_failure(&error)),
        frame_rate: window_frame_rate().unwrap_or_else(|error| report_capture_failure(&error)),
        output_scale: output_scale_setting("THRUM_MAC_OUTPUT_SCALE")
            .unwrap_or_else(|error| report_capture_failure(&error)),
        #[cfg(feature = "proving")]
        solar_sweep: window_solar_sweep().unwrap_or_else(|error| report_capture_failure(&error)),
        #[cfg(feature = "proving")]
        exit_route_starts_paused: std::env::var_os("THRUM_SHOWCASE_ROUTE_START_PAUSED").is_some(),
        #[cfg(feature = "proving")]
        exit_content_report: std::env::var_os("THRUM_SHOWCASE_CONTENT_REPORT").map(Into::into),
        #[cfg(feature = "proving")]
        route: std::env::var_os("THRUM_GOVERNOR_ROUTE")
            .map(std::fs::read_to_string)
            .transpose()
            .unwrap_or_else(|error| report_capture_failure(&error.to_string())),
    };
    v1_game::run_window(
        hardware,
        #[cfg(feature = "proving")]
        window_capture,
        #[cfg(feature = "proving")]
        transition,
        #[cfg(feature = "proving")]
        qualification,
        #[cfg(feature = "proving")]
        display,
        #[cfg(feature = "proving")]
        window_menu().unwrap_or_else(|error| report_capture_failure(&error)),
        governor,
        metal_preparation,
    );
}

#[cfg(target_os = "macos")]
fn output_scale_setting(
    name: &str,
) -> Result<Option<thrum_render::synthetic::RenderScale>, String> {
    use thrum_render::synthetic::RenderScale;
    let value = match std::env::var(name) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("{name}: {error}")),
    };
    RenderScale::STEPS
        .into_iter()
        .find(|step| value == step.percent().to_string())
        .map(Some)
        .ok_or_else(|| format!("{name} must be 100, 85, 75, 67 or 50"))
}

#[cfg(target_os = "macos")]
const MAC_FRAME_RATE_VARIABLE: &str = "THRUM_MAC_FRAME_RATE";

#[cfg(target_os = "macos")]
fn window_frame_rate() -> Result<Option<thrum_app::MacFrameRate>, String> {
    match std::env::var(MAC_FRAME_RATE_VARIABLE) {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Ok(value) if value == "60" => Ok(Some(thrum_app::MacFrameRate::Sixty)),
        Ok(value) if value == "120" => Ok(Some(thrum_app::MacFrameRate::Full)),
        _ => Err(format!("{MAC_FRAME_RATE_VARIABLE} must be 60 or 120")),
    }
}

#[cfg(all(target_os = "macos", feature = "proving"))]
const SOLAR_SWEEP_VARIABLE: &str = "THRUM_MAC_SOLAR_SWEEP";

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_solar_sweep() -> Result<Option<v1_game::WindowSolarSweep>, String> {
    let Ok(value) = std::env::var(SOLAR_SWEEP_VARIABLE) else {
        return Ok(None);
    };
    let (ticks, hours): v1_game::WindowSolarSweep =
        serde_json::from_str(&value).map_err(|error| error.to_string())?;
    if ticks[0] >= ticks[1]
        || ticks[1] - ticks[0] > 1800
        || !((0.0..24.0).contains(&hours[0]) && (0.0..=24.0).contains(&hours[1]))
    {
        return Err("invalid route solar sweep".to_owned());
    }
    Ok(Some((ticks, hours)))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_menu() -> Result<v1_game::WindowMenu, String> {
    let ticket = std::env::args()
        .skip_while(|arg| arg != "--join-ticket")
        .nth(1);
    let script = std::env::var_os("THRUM_UI_EVENT_SCRIPT")
        .map(|input| {
            Ok::<_, String>(thrum_app::InterfaceScript {
                input: input.into(),
                output: std::env::var_os("THRUM_UI_EVENT_REPORT")
                    .ok_or("interface script needs THRUM_UI_EVENT_REPORT")?
                    .into(),
            })
        })
        .transpose()?;
    Ok(v1_game::WindowMenu { ticket, script })
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_display_options(
    transition: Option<&thrum_app::WindowTransitionProof>,
) -> Result<Option<thrum_app::WindowDisplayOptions>, thrum_app::DisplayOverrideError> {
    let Ok(forced) = std::env::var("THRUM_MAC_EDR") else {
        return Ok(None);
    };
    Ok(Some(thrum_app::WindowDisplayOptions {
        forced: forced.parse()?,
        output: std::env::var_os("THRUM_MAC_EDR_PROOF").map(Into::into),
        capture: std::env::var_os("THRUM_MAC_EDR_TAP").is_some(),
        capture_first_pixels: std::env::var_os("THRUM_MAC_EDR_PIXELS").is_some(),
        capture_limits: if transition
            .is_some_and(|proof| proof.arm == thrum_app::TransitionArm::OutputScaleFullscreen)
        {
            thrum_render::runtime::CaptureLimits {
                staging_slots: 2,
                cpu_buffers: 2,
                ..Default::default()
            }
        } else {
            Default::default()
        },
    }))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_qualification() -> Result<Option<thrum_app::WindowQualification>, String> {
    type QualificationRoute = (
        [f64; 4],
        Option<fn(f64) -> f64>,
        Option<std::time::Duration>,
    );
    let Some(output) = std::env::var_os("THRUM_MAC_QUALIFY") else {
        return Ok(None);
    };
    let capture = match std::env::var("THRUM_MAC_TAP").as_deref() {
        Ok("on") => true,
        Ok("off") => false,
        _ => return Err("qualification requires THRUM_MAC_TAP=on or off".to_owned()),
    };
    let (boundaries, sample, refresh_measurement): QualificationRoute =
        match std::env::var("THRUM_MAC_QUALIFY_MODE").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("pan") => (
                [3.0, 4.0, 6.0, 7.1],
                Some(|elapsed| ((elapsed - 4.0) / 2.0).clamp(0.0, 1.0) * 36.0),
                None,
            ),
            Ok("wind") if !capture => ([3.0, 10.0, 16.0, 23.1], None, None),
            Ok("cadence") if !capture => (
                [0.25, 1.0, 21.0, 22.0],
                None,
                Some(std::time::Duration::from_secs(20)),
            ),
            Ok("wind" | "cadence") => {
                return Err("wind and cadence qualification require THRUM_MAC_TAP=off".to_owned());
            }
            _ => return Err("THRUM_MAC_QUALIFY_MODE must be pan, wind or cadence".to_owned()),
        };
    let gpu_timestamps = match std::env::var("THRUM_MAC_CADENCE_TIMING").as_deref() {
        Err(std::env::VarError::NotPresent) => sample.is_some(),
        Ok("1") if refresh_measurement.is_some() => true,
        _ => return Err("THRUM_MAC_CADENCE_TIMING=1 requires cadence qualification".to_owned()),
    };
    Ok(Some(thrum_app::WindowQualification {
        output: output.into(),
        control: "camera.yaw",
        control_column: "yaw_degrees",
        boundaries,
        refresh_measurement,
        sample,
        accepted: |radians| f64::from(radians.to_degrees()),
        capture,
        gpu_timestamps,
        replace_surface: std::env::var("THRUM_MAC_REPLACE").as_deref() == Ok("1"),
    }))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn status_capture() -> Result<v1_game::StatusCapture, String> {
    let tick = std::env::var("THRUM_CAPTURE_STATUS_TICK")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let insets = std::env::var("THRUM_CAPTURE_STATUS_INSETS")
        .ok()
        .map(|value| serde_json::from_str::<[f64; 4]>(&value))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or([0.0; 4]);
    let density = std::env::var("THRUM_CAPTURE_STATUS_DENSITY")
        .ok()
        .map(|value| value.parse::<f64>())
        .transpose()
        .map_err(|error| error.to_string())?;
    if density.is_some_and(|density| !(1.0..=3.0).contains(&density)) {
        return Err("status capture display density must be in 1..=3".into());
    }
    Ok(v1_game::StatusCapture {
        tick,
        insets,
        density,
    })
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_extent_capture() -> Result<Option<thrum_app::WindowExtentCapture>, String> {
    use thrum_app::{ExtentWindow, WindowExtentCapture};

    let Some(output) = std::env::var_os("THRUM_MAC_EXTENT_PROOF") else {
        return Ok(None);
    };
    let read_points = |value: &str| {
        let (width, height) = value.split_once('x').ok_or("expected widthxheight")?;
        let parse = |n: &str| n.parse::<u32>().map_err(|error| error.to_string());
        let extent = [parse(width)?, parse(height)?];
        if extent.contains(&0) {
            return Err("window point extent must be nonzero".to_owned());
        }
        Ok(extent)
    };
    let requested = std::env::var("THRUM_MAC_EXTENT_WINDOW").map_err(|error| error.to_string())?;
    let window = if requested == "fullscreen" {
        ExtentWindow::Fullscreen
    } else {
        ExtentWindow::Points(read_points(&requested)?)
    };
    let status = std::env::var("THRUM_INTERFACE_TEXT").as_deref() == Ok("status");
    let resize = std::env::var("THRUM_MAC_EXTENT_RESIZE")
        .ok()
        .map(|value| read_points(&value))
        .transpose()?;
    Ok(Some(WindowExtentCapture {
        output: output.into(),
        window,
        status,
        resize,
    }))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_transition_proof() -> Result<Option<thrum_app::WindowTransitionProof>, String> {
    use thrum_app::{ScaleReplay, TransitionArm, WindowTransitionProof};
    let Some(output) = std::env::var_os("THRUM_MAC_TRANSITION") else {
        return Ok(None);
    };
    let arm = match std::env::var("THRUM_MAC_TRANSITION_ARM").as_deref() {
        Ok("baseline") => TransitionArm::Baseline,
        Ok("fullscreen") => TransitionArm::Fullscreen,
        Ok("output-scale-fullscreen") => TransitionArm::OutputScaleFullscreen,
        Ok("output-scale-mode") => TransitionArm::OutputScaleMode,
        Ok("route") => TransitionArm::Route,
        Ok("sweep") => TransitionArm::Sweep,
        Ok("status") => TransitionArm::Status,
        Ok("synchronous") => TransitionArm::Synchronous,
        Ok("restore") => TransitionArm::Restore,
        Ok("skip-restore") => TransitionArm::SkipRestore,
        _ => return Err("unknown Mac transition proof arm".to_owned()),
    };
    let record = std::env::var("THRUM_GOVERNOR_RECORD");
    let scale_replay = if record.as_deref() == Ok("replay") {
        let origin = std::env::var("THRUM_ROUTE_SCALE_ORIGIN")
            .map_err(|error| format!("scale replay origin: {error}"))?
            .parse::<u64>()
            .map_err(|error| format!("scale replay origin: {error}"))?;
        Some(ScaleReplay {
            origin,
            scale_at: v1_game::route_scale_replay,
        })
    } else {
        None
    };
    Ok(Some(WindowTransitionProof {
        output: output.into(),
        arm,
        tap: std::env::var_os("THRUM_MAC_TRANSITION_TAP").is_some(),
        capture_cue: fullscreen_capture_cue(arm)?,
        scale_replay,
        fullscreen_output_scale: output_scale_setting("THRUM_MAC_OUTPUT_SCALE_PROOF_REGISTRY")?,
        governor: match record.as_deref() {
            Ok("thermal") => Some(thrum_app::GovernorProof::Thermal {
                from: 90,
                until: 210,
            }),
            Ok("observed" | "replay") => Some(thrum_app::GovernorProof::Observed),
            Err(std::env::VarError::NotPresent) => None,
            _ => return Err("THRUM_GOVERNOR_RECORD must be observed, replay or thermal".to_owned()),
        },
    }))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn fullscreen_capture_cue(
    arm: thrum_app::TransitionArm,
) -> Result<Option<([u64; 2], thrum_render::runtime::CaptureLimits)>, String> {
    let Ok(value) = std::env::var("THRUM_MAC_CAPTURE_CUE") else {
        return Ok(None);
    };
    let (ticks, staging_slots, cpu_buffers, mib): ([u64; 2], usize, usize, u64) =
        serde_json::from_str(&value).map_err(|error| error.to_string())?;
    if arm != thrum_app::TransitionArm::Fullscreen
        || std::env::var_os("THRUM_MAC_TRANSITION_TAP").is_none()
        || ticks[0] > ticks[1]
        || ticks[1] - ticks[0] > 360
        || !(1..=32).contains(&staging_slots)
        || !(1..=96).contains(&cpu_buffers)
        || !(1..=4096).contains(&mib)
    {
        return Err("invalid fullscreen capture cue or working-set limits".to_owned());
    }
    Ok(Some((
        ticks,
        thrum_render::runtime::CaptureLimits {
            staging_slots,
            cpu_buffers,
            byte_allowance: mib * 1024 * 1024,
        },
    )))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn capture_admission() -> Result<(bool, std::time::Duration, v1_game::NativeRecovery), String> {
    let wait = match std::env::var("THRUM_CAPTURE_ADMISSION").as_deref() {
        Ok("omit") | Err(_) => false,
        Ok("wait") => true,
        _ => return Err("capture admission must be omit or wait".to_owned()),
    };
    let hold = std::env::var("THRUM_CAPTURE_HOLD_FULL_MS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    let recovery = match std::env::var(v1_game::NATIVE_RECOVERY_VARIABLE).as_deref() {
        Err(_) => v1_game::NativeRecovery::Disabled,
        Ok("control") => v1_game::NativeRecovery::Control,
        Ok("faults") => v1_game::NativeRecovery::Faults,
        _ => return Err("native recovery must be control or faults".into()),
    };
    Ok((wait, std::time::Duration::from_millis(hold), recovery))
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stderr,
    reason = "the game binary reports capture failures to its caller"
)]
fn report_capture_failure(error: &str) -> ! {
    eprintln!("capture failed: {error}");
    std::process::exit(1);
}

#[cfg(not(target_os = "macos"))]
#[expect(
    clippy::print_stderr,
    reason = "the platform-specific diagnostic reports unsupported execution"
)]
fn main() {
    eprintln!("v1-game is available on macOS; use v1-game for the offscreen diagnostic");
    std::process::exit(1);
}

/// The scene capture's launch settings, read here so the library reads no environment.
#[cfg(all(target_os = "macos", feature = "proving"))]
fn scene_capture_request(
    metal_preparation: Option<thrum_render::capabilities::metallib::Preparation>,
) -> Result<v1_game::SceneCaptureRequest, String> {
    fn text(name: &str) -> Result<Option<String>, String> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(format!("{name}: {error}")),
        }
    }
    let path = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
    let scene = text("THRUM_SCENE")?;
    let source = text("THRUM_CAPTURE_SOURCE")?;
    let still_governor = (scene.as_deref() == Some("exit")
        && source.as_deref() == Some("submitted-sdr")
        && governor_enabled()?)
    .then_some(thrum_render::governor::DisplayRecord {
        interval: std::time::Duration::ZERO,
        thermal: thrum_render::governor::ThermalState::Nominal,
    });
    Ok(v1_game::SceneCaptureRequest {
        scene,
        motion_frames: text("THRUM_CAPTURE_MOTION_FRAMES")?,
        pan_to_degrees: text("THRUM_CAPTURE_PAN_TO_DEGREES")?,
        initial_params: path("THRUM_PARAMS"),
        route: path("THRUM_CAPTURE_ROUTE"),
        metal_preparation,
        #[cfg(feature = "creation-counters")]
        creation_proof: match text("THRUM_CREATION_PROOF")?.as_deref() {
            None => None,
            Some("launch") => Some(v1_game::CreationProof::Launch),
            Some("metallib") => Some(v1_game::CreationProof::Metallib),
            Some("route") => Some(v1_game::CreationProof::Route),
            Some("late-pipeline") => Some(v1_game::CreationProof::LatePipeline),
            Some(_) => return Err("unknown THRUM_CREATION_PROOF arm".to_owned()),
        },
        display_overlay: text("THRUM_CAPTURE_DISPLAY_OVERLAY")?,
        source,
        still_governor,
        fixed_eye: text("THRUM_CAPTURE_FIXED_EYE")?
            .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
            .transpose()?,
        edr_headroom: text("THRUM_CAPTURE_EDR_HEADROOM")?
            .map(|value| value.parse::<f32>().map_err(|error| error.to_string()))
            .transpose()?
            .map(|value| {
                (1.0..=16.0)
                    .contains(&value)
                    .then_some(value)
                    .ok_or("capture headroom must be in [1, 16]")
            })
            .transpose()?,
        phase: text("THRUM_CAPTURE_PHASE")?,
        surface_format: text("THRUM_CAPTURE_SURFACE_FORMAT")?,
        surface_extent: text("THRUM_CAPTURE_SURFACE_EXTENT")?,
        windows: text("THRUM_CAPTURE_WINDOWS")?,
        scale_trace: render_scale_trace()?,
        governor_thermal: std::env::var("THRUM_GOVERNOR_RECORD").as_deref() == Ok("thermal"),
        governor_identity: std::env::var_os("THRUM_GOVERNOR_IDENTITY")
            .map(|_| governor_enabled())
            .transpose()?,
        present_control: text("THRUM_CAPTURE_PRESENT_CONTROL")?,
        params: std::env::var_os("THRUM_CAPTURE_PARAMS")
            .map(|value| std::env::split_paths(&value).collect()),
        route_only: std::env::var_os("THRUM_CAPTURE_ROUTE_ONLY").is_some(),
        live_ticks: text("THRUM_CAPTURE_LIVE_TICKS")?,
        omit_placement: text("THRUM_G3T_OMIT_PLACEMENT")?,
        overlay_control: std::env::var_os("THRUM_G3T_OVERLAY_CONTROL").is_some(),
    })
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn render_scale_trace() -> Result<Option<v1_game::RenderScaleTrace>, String> {
    use v1_game::RenderScaleTrace;
    match std::env::var("THRUM_ROUTE_RENDER_SCALE").as_deref() {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Ok("trace") => Ok(Some(RenderScaleTrace::Continuous)),
        Ok("reset") => Ok(Some(RenderScaleTrace::ResetAtStep)),
        _ => Err("THRUM_ROUTE_RENDER_SCALE must be trace or reset".to_owned()),
    }
}

#[cfg(target_os = "macos")]
fn governor_enabled() -> Result<bool, String> {
    match std::env::var("THRUM_GOVERNOR").as_deref() {
        Ok("on") => Ok(true),
        Ok("off") | Err(std::env::VarError::NotPresent) => Ok(false),
        _ => Err("THRUM_GOVERNOR must be on or off".to_owned()),
    }
}
