//! macOS window and device-backed records for the Crowd replay.

#[cfg(target_os = "macos")]
mod native;

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args_os().len() > 1 {
        return native::proof();
    }
    thrum_app::run_until_closed(thrum_app::Config::new("crowd-2d-view", "Crowd"), || {
        crowd_2d_view::CrowdView::new(10_000, false)
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {}
