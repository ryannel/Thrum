//! MetalFX history and motion qualification.
#[cfg(target_os = "macos")]
mod metalfx_motion_fixture;
#[cfg(target_os = "macos")]
mod metalfx_reset_fixture;
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
    fn acquisition_submission_and_device_replacement_reset_native_history() {
        metalfx_reset_fixture::acquisition_submission_and_device_replacement_reset_native_history();
    }
}

thrum_test_support::macos_gpu_test! {
    fn metalfx_motion_and_jitter_signs_match_translating_edges() {
        metalfx_motion_fixture::metalfx_motion_and_jitter_signs_match_translating_edges();
    }
}
