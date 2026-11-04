//! UIKit lifecycle and surface adapter for the shared V1 game diagnostic.

#[cfg(target_os = "ios")]
mod ios;

/// Enter the iOS shell from Swift's process main. UIKit owns the process and
/// Winit owns the window event loop; the game host is shared with desktop/Web.
#[cfg(target_os = "ios")]
#[expect(
    unsafe_code,
    reason = "the unique Swift entry point requires an exported C symbol"
)]
#[unsafe(no_mangle)]
pub extern "C" fn thrum_v1_mobile_start() {
    ios::run();
}
