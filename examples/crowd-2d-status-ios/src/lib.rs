//! Application entry point for the Crowd status display.

/// UIKit process entry from the shared Swift bootstrap.
#[cfg(target_os = "ios")]
#[expect(
    unsafe_code,
    reason = "the Swift bootstrap requires an exported C symbol"
)]
#[unsafe(no_mangle)]
pub extern "C" fn thrum_mobile_start() {
    use thrum_app::ios::{MobileOptions, run};

    let qualification = std::env::var("THRUM_CROWD_ROUTE").as_deref() == Ok("1");
    if let Err(error) = run::<crowd_2d_status::MobileCrowd>(MobileOptions {
        title: "Crowd",
        render_thread: "thrum-crowd-ios-render",
        witness: qualification.then_some("thrum-crowd-ios-state.txt"),
    }) {
        #[expect(clippy::print_stderr, reason = "report a fatal mobile launch error")]
        {
            eprintln!("{error}");
        }
    }
}
