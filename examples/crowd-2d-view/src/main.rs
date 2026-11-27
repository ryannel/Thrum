//! macOS window and device-backed records for the Crowd replay.

#[cfg(target_os = "macos")]
mod native;

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() > 1 {
        return native::proof();
    }
    let config = thrum_app::Config::new("crowd-2d-view", "Crowd");
    let game = || crowd_2d_view::CrowdView::new(10_000, false);
    if let Ok(forced) = std::env::var("THRUM_MAC_EDR") {
        thrum_app::run_with_display(
            config,
            thrum_app::WindowDisplayOptions {
                forced: forced.parse()?,
                output: std::env::var_os("THRUM_MAC_EDR_PROOF").map(Into::into),
                capture: false,
                capture_limits: thrum_render::runtime::CaptureLimits::default(),
                capture_first_pixels: false,
            },
            game,
        );
    } else {
        thrum_app::run_until_closed(config, game);
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {}
