//! Bounded macOS V1 scene on a real surface.

#[cfg(target_os = "macos")]
#[path = "bin/window/macos.rs"]
mod macos;

#[cfg(target_os = "macos")]
#[path = "bin/window/capture.rs"]
mod capture;

#[cfg(target_os = "macos")]
fn main() {
    if let Some(output) = std::env::var_os("THRUM_CAPTURE_SCENE") {
        if let Err(error) = pollster::block_on(capture::run(output.into())) {
            report_capture_failure(&error);
        }
        return;
    }
    macos::run();
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
