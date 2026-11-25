//! macOS V1 scene on a real surface.

#[cfg(target_os = "macos")]
fn main() {
    let names = match std::env::var("THRUM_HARDWARE") {
        Ok(names) => names,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(error) => report_capture_failure(&format!("THRUM_HARDWARE: {error}")),
    };
    let hardware = thrum_render::capabilities::HardwareRequest::parse(&names)
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
            hardware,
        ) {
            report_capture_failure(&error);
        }
        return;
    }
    #[cfg(feature = "proving")]
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_SCENE") {
        let mut hardware = hardware;
        for request in &mut hardware {
            request.required = true;
        }
        let request =
            scene_capture_request().unwrap_or_else(|error| report_capture_failure(&error));
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
    let display =
        window_display_options().unwrap_or_else(|error| report_capture_failure(&error.to_string()));
    let governor = v1_game::WindowGovernor {
        enabled: governor_enabled().unwrap_or_else(|error| report_capture_failure(&error)),
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
        governor,
    );
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_display_options()
-> Result<Option<thrum_app::WindowDisplayOptions>, thrum_app::DisplayOverrideError> {
    let Ok(forced) = std::env::var("THRUM_MAC_EDR") else {
        return Ok(None);
    };
    Ok(Some(thrum_app::WindowDisplayOptions {
        forced: forced.parse()?,
        output: std::env::var_os("THRUM_MAC_EDR_PROOF").map(Into::into),
        capture: std::env::var_os("THRUM_MAC_EDR_TAP").is_some(),
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
fn window_extent_capture() -> Result<Option<thrum_app::WindowExtentCapture>, String> {
    use thrum_app::{ExtentWindow, WindowExtentCapture};

    let Some(output) = std::env::var_os("THRUM_MAC_EXTENT_PROOF") else {
        return Ok(None);
    };
    let requested = std::env::var("THRUM_MAC_EXTENT_WINDOW").map_err(|error| error.to_string())?;
    let window = if requested == "fullscreen" {
        ExtentWindow::Fullscreen
    } else {
        let (width, height) = requested.split_once('x').ok_or("expected widthxheight")?;
        let parse = |n: &str| n.parse::<u32>().map_err(|error| error.to_string());
        let extent = [parse(width)?, parse(height)?];
        if extent.contains(&0) {
            return Err("window point extent must be nonzero".to_owned());
        }
        ExtentWindow::Points(extent)
    };
    Ok(Some(WindowExtentCapture {
        output: output.into(),
        window,
    }))
}

#[cfg(all(target_os = "macos", feature = "proving"))]
fn window_transition_proof() -> Result<Option<thrum_app::WindowTransitionProof>, String> {
    use thrum_app::{TransitionArm, WindowTransitionProof};
    let Some(output) = std::env::var_os("THRUM_MAC_TRANSITION") else {
        return Ok(None);
    };
    let arm = match std::env::var("THRUM_MAC_TRANSITION_ARM").as_deref() {
        Ok("baseline") => TransitionArm::Baseline,
        Ok("sweep") => TransitionArm::Sweep,
        Ok("synchronous") => TransitionArm::Synchronous,
        Ok("restore") => TransitionArm::Restore,
        Ok("skip-restore") => TransitionArm::SkipRestore,
        _ => return Err("unknown Mac transition proof arm".to_owned()),
    };
    Ok(Some(WindowTransitionProof {
        output: output.into(),
        arm,
        tap: std::env::var_os("THRUM_MAC_TRANSITION_TAP").is_some(),
        governor: match std::env::var("THRUM_GOVERNOR_RECORD").as_deref() {
            Ok("thermal") => Some(thrum_app::GovernorProof::Thermal {
                from: 90,
                until: 210,
            }),
            Ok("observed") => Some(thrum_app::GovernorProof::Observed),
            Err(std::env::VarError::NotPresent) => None,
            _ => return Err("THRUM_GOVERNOR_RECORD must be observed or thermal".to_owned()),
        },
    }))
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
fn scene_capture_request() -> Result<v1_game::SceneCaptureRequest, String> {
    fn text(name: &str) -> Result<Option<String>, String> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(format!("{name}: {error}")),
        }
    }
    let path = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
    Ok(v1_game::SceneCaptureRequest {
        scene: text("THRUM_SCENE")?,
        motion_frames: text("THRUM_CAPTURE_MOTION_FRAMES")?,
        pan_to_degrees: text("THRUM_CAPTURE_PAN_TO_DEGREES")?,
        initial_params: path("THRUM_PARAMS"),
        route: path("THRUM_CAPTURE_ROUTE"),
        display_overlay: text("THRUM_CAPTURE_DISPLAY_OVERLAY")?,
        source: text("THRUM_CAPTURE_SOURCE")?,
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
