//! Public camera admission: malformed requests never reach GPU recording.

use thrum_render::{
    contribution::{
        SecondaryViewError, SecondaryViewRequest, SecondaryViewSlot, ViewAir, ViewClipPlane,
    },
    synthetic::SyntheticView,
};

fn camera(height: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, height, 0.0],
        [0.0, height, -1.0],
        [0.0, 1.0, 0.0],
        60.0_f32.to_radians(),
        1.0,
        0.1,
        5000.0,
    )
    .expect("rigid camera")
}

#[test]
fn refuses_camera_extent_plane_and_profile_by_name() {
    assert_eq!(
        SecondaryViewRequest::new(&SyntheticView::default(), [64, 64], None, ViewAir::Camera),
        Err(SecondaryViewError::InvalidCamera)
    );
    assert_eq!(
        SecondaryViewRequest::new(&camera(0.0), [0, 64], None, ViewAir::Camera),
        Err(SecondaryViewError::InvalidExtent)
    );
    for equation in [
        [0.0; 4],
        [0.0, f32::NAN, 0.0, 0.0],
        [1.0, 0.0, 0.0, f32::INFINITY],
    ] {
        assert_eq!(
            ViewClipPlane::new(equation),
            Err(SecondaryViewError::InvalidClipPlane)
        );
    }
    for angle in [2.0_f32, 20.0] {
        let angle = angle.to_radians();
        let plane =
            ViewClipPlane::new([angle.sin(), angle.cos(), 0.0, -50.0]).expect("tilted plane");
        assert_eq!(
            SecondaryViewRequest::new(&camera(48.0), [64, 64], Some(plane), ViewAir::Plane),
            Err(SecondaryViewError::AirNeedsLevelPlane)
        );
        SecondaryViewRequest::new(&camera(48.0), [64, 64], Some(plane), ViewAir::Camera)
            .expect("tilted clip without plane air");
    }
    assert_eq!(
        SecondaryViewRequest::new(&camera(0.0), [64, 64], None, ViewAir::Plane),
        Err(SecondaryViewError::AirNeedsLevelPlane)
    );
    let request =
        SecondaryViewRequest::new(&camera(0.0), [64, 64], None, ViewAir::Camera).expect("request");
    for pixels in [-1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            request.with_least_pixels(pixels),
            Err(SecondaryViewError::InvalidLeastSize)
        );
    }
    for (count, reach) in [(3, 128.0), (0, 0.0), (1, f32::INFINITY), (2, f32::NAN)] {
        assert_eq!(
            request.with_shadows(count, reach),
            Err(SecondaryViewError::InvalidShadowProfile)
        );
    }
}

#[test]
fn plane_is_normalized_and_near_plane_is_held_at_five_cm() {
    let plane = ViewClipPlane::new([0.0, 2.0, 0.0, -100.0]).expect("plane");
    assert_eq!(plane.equation(), [0.0, 1.0, 0.0, -50.0]);
    for (height, expected_offset) in [
        (49.99, -50.04),
        (50.0, -50.05),
        (50.01, -49.96),
        (48.0, -50.0),
    ] {
        let request =
            SecondaryViewRequest::new(&camera(height), [64, 64], Some(plane), ViewAir::Plane)
                .expect("level plane");
        let actual = request.clip_plane().expect("effective plane").equation();
        assert!(
            (actual[3] - expected_offset).abs() < 1e-5,
            "height {height}: {actual:?}"
        );
    }
}

#[test]
fn declaration_obeys_profile_and_device_limits() {
    for scale in [0.0, 0.249, 1.001, f32::NAN, f32::INFINITY] {
        assert_eq!(
            SecondaryViewSlot::new(scale),
            Err(SecondaryViewError::InvalidExtent)
        );
    }
    let slot = SecondaryViewSlot::new(0.25).expect("slot");
    assert_eq!(slot.extent([255, 129]), [64, 33]);
    let limits = wgpu::Limits::default();
    SecondaryViewSlot::validate(&[slot; 4], [2560, 1440], &limits).expect("four admitted");
    assert_eq!(
        SecondaryViewSlot::validate(&[slot; 5], [64, 64], &limits),
        Err(SecondaryViewError::SlotLimit)
    );
    assert_eq!(
        SecondaryViewSlot::validate(&[slot], [limits.max_texture_dimension_2d * 8, 64], &limits),
        Err(SecondaryViewError::InvalidExtent)
    );
}
