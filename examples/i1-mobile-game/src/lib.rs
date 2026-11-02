//! Preloaded local I1 game and native mobile entry proof.
//!
//! This is a bounded game fixture using Thrum's authority and replica rules.
//! It does not add a second simulation or networking implementation.

#[cfg(any(target_os = "ios", target_os = "android", test))]
mod game;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod render;

#[cfg(target_os = "ios")]
/// Enter the single winit event loop from the iOS app's C/Swift main function.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn thrum_i1_mobile_start() {
    mobile::start_ios();
}

#[cfg(target_os = "android")]
/// Enter the single winit event loop from android-activity's GameActivity glue.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    mobile::start_android(app);
}

#[cfg(target_os = "android")]
/// Receive the Java GameActivity foreground signal before native work resumes.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_thrum_i1_MainActivity_nativeSetForeground(
    _environment: *mut core::ffi::c_void,
    _activity_class: *mut core::ffi::c_void,
    active: u8,
) {
    mobile::android_set_foreground(active != 0);
}
