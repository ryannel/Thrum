//! Bounded macOS V1 scene on a real surface.

#[cfg(target_os = "macos")]
#[path = "window/macos.rs"]
mod macos;

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(not(target_os = "macos"))]
#[expect(
    clippy::print_stderr,
    reason = "the platform-specific diagnostic reports unsupported execution"
)]
fn main() {
    eprintln!("v1-window is available on macOS; use v1-scene for the offscreen diagnostic");
    std::process::exit(1);
}
