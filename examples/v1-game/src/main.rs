//! Bounded macOS V1 scene on a real surface.

#[cfg(target_os = "macos")]
#[path = "bin/window/capture.rs"]
mod capture;

#[cfg(target_os = "macos")]
fn main() {
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_RUNTIME") {
        let scenario = std::env::var("THRUM_CAPTURE_CASE").unwrap_or_else(|_| "pan".to_owned());
        let (wait, hold, recovery) =
            capture_admission().unwrap_or_else(|error| report_capture_failure(&error));
        let capture_output = match std::env::var(v1_game::CAPTURE_OBSERVER_VARIABLE).as_deref() {
            Ok("1") => v1_game::RuntimeCaptureOutput::ObserverOnly,
            Err(std::env::VarError::NotPresent) => v1_game::RuntimeCaptureOutput::Images,
            _ => report_capture_failure("capture observer arm must be 1 when set"),
        };
        if let Err(error) = v1_game::run_runtime_capture(
            std::path::Path::new(&output),
            &scenario,
            wait,
            hold,
            recovery,
            capture_output,
        ) {
            report_capture_failure(&error);
        }
        return;
    }
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_SCENE") {
        if let Err(error) = pollster::block_on(capture::run(output.into())) {
            report_capture_failure(&error);
        }
        return;
    }
    v1_game::run_window();
}

#[cfg(target_os = "macos")]
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
