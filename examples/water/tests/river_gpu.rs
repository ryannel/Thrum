//! Metal river map proofs using the same WGSL as the submitted water.
mod river_proof;

#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn submitted_queries_and_both_floats_follow_the_rivers() {
    let Some(gpu) = thrum_test_support::gpu(
        "submitted_queries_and_both_floats_follow_the_rivers", Some(wgpu::Backend::Metal),
    ) else { return };
    support::island::with_cooked_island("submitted_queries_and_both_floats_follow_the_rivers", || {
        river_proof::transport::check(&gpu);
    });
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_depth_follows_all_three_joins_in_swell() {
    let Some(gpu) = thrum_test_support::gpu(
        "river joins", Some(wgpu::Backend::Metal),
    ) else { return };
    river_proof::surface::check(&gpu);
}
}
