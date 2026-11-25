//! Weather-map values and the visible ray through the same air-frame column.

thrum_test_support::macos_gpu_test! {
    fn weather_maps_follow_presets_ramps_and_integrated_wind() {
        weather_range::check_map();
    }
}

thrum_test_support::macos_gpu_test! {
    fn the_visible_trace_reads_cells() {
        weather_range::check_procedural_trace("cell", [0.5, 1.0]);
    }
}

thrum_test_support::macos_gpu_test! {
    fn the_visible_trace_reads_the_deck() {
        // Keep both a thick column and a clear neighbourhood in this small,
        // deck-only fixture. The map test crosses the authored preset coordinates.
        weather_range::check_procedural_trace("deck", [0.7, 0.0]);
    }
}

thrum_test_support::macos_gpu_test! {
    fn broken_banks_shape_both_visible_cloud_and_ground_shade() {
        weather_range::check_broken_bank();
    }
}

thrum_test_support::macos_gpu_test! {
    fn the_visible_trace_reads_placed_columns() {
        weather_range::check_placed_trace();
    }
}

#[cfg(target_os = "macos")]
mod placed_fixture;
#[cfg(target_os = "macos")]
mod scene_fixture;
#[cfg(target_os = "macos")]
mod weather_range;
