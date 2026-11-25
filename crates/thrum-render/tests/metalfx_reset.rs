//! MetalFX reset qualification.
#[cfg(target_os = "macos")]
mod metalfx_reset_fixture;
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
    fn acquisition_submission_and_device_replacement_reset_native_history() {
        metalfx_reset_fixture::acquisition_submission_and_device_replacement_reset_native_history();
    }
}
