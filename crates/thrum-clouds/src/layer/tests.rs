use super::*;
mod sun;

#[test]
fn shader_record_matches_rust_offsets() {
    thrum_test_support::records::named(
        "cloud layer parameters",
        include_str!("params.wgsl"),
        &[thrum_test_support::record!(
            Uniform as "CloudParams",
            weather,
            physical,
            erosion,
            scattering,
            drift,
            heights,
            settings,
            temporal,
            noise,
            inverse_raster,
            previous_raster
        )],
    );
}
