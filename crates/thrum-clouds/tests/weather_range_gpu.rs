//! Weather-map values and the visible ray through the same air-frame column.

thrum_test_support::macos_gpu_test! {
    fn weather_maps_follow_presets_ramps_and_integrated_wind() {
        weather_range::check_map();
    }
}

thrum_test_support::macos_gpu_test! {
    fn the_visible_trace_reads_cells_and_decks() {
        weather_range::check_procedural_trace();
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
