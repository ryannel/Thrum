//! Native encoder tests require Metal command buffers and pipelines.

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod metal;

#[test]
fn whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit() {
    let Some(gpu) = thrum_test_support::gpu(
        "whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal::whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit(&gpu);
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    drop(gpu);
}

#[test]
fn locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback() {
    let Some(opened) = thrum_test_support::open(
        "locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
    ) else {
        return;
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal::locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback(&opened.device);
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    drop(opened);
}

#[test]
fn locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback() {
    let Some(opened) = thrum_test_support::open(
        "locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
    ) else {
        return;
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal::locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback(&opened.device);
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    drop(opened);
}
