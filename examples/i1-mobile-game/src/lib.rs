//! Preloaded local I1 game and native mobile entry proof.
//!
//! This is a bounded game fixture using Thrum's authority and replica rules.
//! It does not add a second simulation or networking implementation.

mod game;

/// Exercise the bundled rules through the ordinary headless authority and replica.
/// This is a bounded native entry proof, not a build-readiness check.
///
/// # Errors
///
/// Returns a message if input admission, callback delivery, or the observed result differs.
pub fn headless_probe() -> Result<(), &'static str> {
    use std::time::{Duration, Instant};

    use thrum_core::{Admission, HostPublication};

    let origin = Instant::now();
    let (mut host, mut receipts) = game::create_host(origin);
    if host.submit(game::tap_input(1, 0)) != Ok(Admission::Queued) {
        return Err("tap input was not queued");
    }
    let callback = host
        .on_callback(origin + Duration::from_millis(20))
        .map_err(|_| "headless callback failed")?;
    let mut view = game::LocalView::default();
    receipts
        .deliver(&mut host, &callback, |publication| {
            if let HostPublication::Replicated { admitted, .. } = publication {
                view.observe(admitted);
            }
        })
        .map_err(|_| "headless receipt delivery failed")?;
    let taps = host
        .replica()
        .and_then(|replica| replica.get(game::COUNTER))
        .map(|counter| counter.taps);
    if taps != Some(1)
        || view.handled_results != 1
        || view.rejected_results != 0
        || view.last_event_id != Some(1)
    {
        return Err("headless authority/replica outcome differed");
    }
    Ok(())
}

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod render;

#[cfg(target_os = "ios")]
/// Enter the single winit event loop from the iOS app's C/Swift main function.
#[allow(
    unsafe_code,
    reason = "mobile C ABI entry points require exported symbols"
)]
#[unsafe(no_mangle)]
pub extern "C" fn thrum_i1_mobile_start() {
    mobile::start_ios();
}

#[cfg(target_os = "android")]
/// Enter the single winit event loop from android-activity's GameActivity glue.
#[allow(
    unsafe_code,
    reason = "mobile C ABI entry points require exported symbols"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    mobile::start_android(app);
}

#[cfg(target_os = "android")]
/// Receive the Java GameActivity foreground signal before native work resumes.
#[allow(
    unsafe_code,
    reason = "mobile C ABI entry points require exported symbols"
)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_thrum_i1_MainActivity_nativeSetForeground(
    _environment: *mut core::ffi::c_void,
    _activity_class: *mut core::ffi::c_void,
    active: u8,
) {
    mobile::android_set_foreground(active != 0);
}
