//! V1's application entry and game-specific setup for the shared UIKit shell.

pub mod device;

#[cfg(target_os = "ios")]
mod ios;

/// UIKit process entry point called by the shared Swift bootstrap.
#[cfg(target_os = "ios")]
#[expect(
    unsafe_code,
    reason = "the Swift bootstrap requires an exported C symbol"
)]
#[unsafe(no_mangle)]
pub extern "C" fn thrum_mobile_start() {
    use thrum_app::ios::{MobileOptions, run};

    let result = run::<ios::V1>(MobileOptions {
        title: "Thrum V1 scene",
        render_thread: "thrum-v1-ios-render",
        witness: ios::route_enabled().then_some("thrum-v1-ios-state.txt"),
    });
    if let Err(error) = result {
        #[expect(clippy::print_stderr, reason = "report a fatal mobile launch error")]
        {
            eprintln!("{error}");
        }
    }
}
