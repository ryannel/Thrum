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
fn submitted_depth_follows_river_interiors_and_all_three_joins_in_swell() {
    let Some(gpu) = thrum_test_support::gpu(
        "river joins", Some(wgpu::Backend::Metal),
    ) else { return };
    river_proof::surface::check(&gpu, true);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_depth_follows_the_hollow_and_fall_reach_wetting() {
    let Some(gpu) = thrum_test_support::gpu("river wetting draw", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check_wetting(&gpu);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_depth_follows_the_whole_wet_length_at_lower_gauges() {
    let Some(gpu) = thrum_test_support::gpu("lower river gauges", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check(&gpu, false);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_matches_the_live_river_query() {
    let Some(gpu) = thrum_test_support::gpu("trade depth and query", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check_tidal_depth(&gpu, false, 1..=180);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_from_the_tidal_bank_matches_the_live_river_query() {
    let Some(gpu) = thrum_test_support::gpu("oblique tidal depth", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check_tidal_depth(&gpu, true, 380..=400);
}
}

#[cfg(target_os = "macos")]
fn check_trade_transect(stage: f64) {
    let Some(gpu) =
        thrum_test_support::gpu("full trade river transects", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    river_proof::surface::check_live_depth(&gpu, stage);
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_at_the_lowest_gauges() {
    check_trade_transect(0.0);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_between_gauge_samples() {
    check_trade_transect(0.37);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_at_the_middle_gauge_samples() {
    check_trade_transect(0.5);
}
}

thrum_test_support::macos_gpu_test! {
fn submitted_trade_depth_at_the_highest_gauges() {
    check_trade_transect(1.0);
}
}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: seven seconds of submitted swell at the tidal column from above"]
fn submitted_tidal_depth_from_above_through_the_first_seven_seconds() {
    let Some(gpu) = thrum_test_support::gpu("tidal depth series from above", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check_tidal_depth(&gpu, false, 1..=420);
}
}

thrum_test_support::macos_gpu_test! {
#[ignore = "long: seven seconds of oblique intersections against the live wind-wave field"]
fn submitted_tidal_depth_from_the_bank_through_the_first_seven_seconds() {
    let Some(gpu) = thrum_test_support::gpu("tidal depth series from the bank", Some(wgpu::Backend::Metal))
    else { return };
    river_proof::surface::check_tidal_depth(&gpu, true, 1..=420);
}
}
