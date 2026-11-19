//! Native encoder tests require Metal command buffers and pipelines.

#[cfg(target_os = "macos")]
mod metal;

#[test]
fn whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit() {
    let Some(_gpu) = thrum_test_support::gpu(
        "whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    #[cfg(target_os = "macos")]
    metal::whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit(&_gpu);
}

#[test]
fn locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback() {
    let Some(_gpu) = thrum_test_support::gpu(
        "locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    #[cfg(target_os = "macos")]
    metal::locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback(&_gpu);
}

#[test]
fn locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback() {
    let Some(_gpu) = thrum_test_support::gpu(
        "locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    #[cfg(target_os = "macos")]
    metal::locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback(&_gpu);
}
