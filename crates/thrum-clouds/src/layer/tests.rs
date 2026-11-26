use super::*;
mod sun;

#[test]
fn sparse_view_admission_uses_layer_pixels() {
    let projection =
        glam::camera::rh::proj::directx::perspective_infinite_reverse(1.0, 16.0 / 9.0, 0.1);
    let extent = [960, 540];
    let moved = |view: Mat4, camera: Vec3| {
        view_crosses_refresh_block((projection * view).inverse(), projection, camera, extent, 2)
    };
    assert!(!moved(Mat4::IDENTITY, Vec3::ZERO));
    assert!(!moved(Mat4::from_rotation_y(0.0001), Vec3::ZERO));
    assert!(moved(Mat4::from_rotation_y(0.03), Vec3::ZERO));
    assert!(!moved(Mat4::from_translation(-Vec3::X), Vec3::X));
}

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
            light_samples,
            sun_angular_radius,
            render_extent,
            temporal,
            noise,
            bank_noise,
            density_profile,
            active_extent,
            sample_cap,
            field_overlay,
            inverse_raster,
            previous_raster
        )],
    );
}
