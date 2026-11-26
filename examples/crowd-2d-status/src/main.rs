//! Native window and web packaging entry point for the Crowd status display.

#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() -> Result<(), thrum_app::GameError> {
    use std::{env, path::PathBuf};

    use crowd_2d_status::CrowdStatus;
    use thrum_app::{Config, ExtentWindow, GameError, InterfaceScript, WindowExtentCapture};

    let store = env::var_os("THRUM_ASSET_STORE").ok_or_else(|| {
        GameError::Rejected(
            "run ./dev scene examples/crowd-2d-status to cook the font store".into(),
        )
    })?;
    let script = env::var_os("THRUM_CROWD_INPUT")
        .zip(env::var_os("THRUM_CROWD_WITNESS"))
        .map(|(input, output)| InterfaceScript {
            input: input.into(),
            output: output.into(),
        });
    let game =
        || Ok::<_, GameError>(CrowdStatus::new(PathBuf::from(store).into()).with_script(script));
    let mut config = Config::new("crowd-2d-status", "Crowd");
    config.window_size = [960, 640];
    if let Some(output) = env::var_os("THRUM_CROWD_CAPTURE") {
        thrum_app::run_with_extent_capture(
            config,
            WindowExtentCapture {
                window: ExtentWindow::Points([560, 370]),
                output: output.into(),
                status: true,
                resize: None,
            },
            game,
        );
    } else {
        thrum_app::run_until_closed(config, game);
    }
    Ok(())
}
