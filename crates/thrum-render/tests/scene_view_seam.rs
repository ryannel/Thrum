//! Slot declaration, admission and actual GPU reads through the public seam.
mod support;
use support::*;
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{
        SecondaryViewError, SecondaryViewRequest, SecondaryViewSlot, ViewAir, ViewClipPlane,
    },
    synthetic::{SyntheticScene, SyntheticView},
};

#[test]
fn improper_requests_and_declarations_are_refused_by_name() {
    assert_eq!(
        SecondaryViewRequest::new(&SyntheticView::default(), [64, 64], None, ViewAir::Camera),
        Err(SecondaryViewError::InvalidCamera)
    );
    let tilted = ViewClipPlane::new([0.1, 1.0, 0.0, 0.0]).expect("plane");
    assert_eq!(
        SecondaryViewRequest::new(&camera(0.0, -2.0), [64, 64], Some(tilted), ViewAir::Plane),
        Err(SecondaryViewError::AirNeedsLevelPlane)
    );
    let slot = SecondaryViewSlot::new(1.0).expect("slot");
    let mut limits = wgpu::Limits::default();
    assert_eq!(
        SecondaryViewSlot::validate(&[slot; 5], [64, 64], &limits),
        Err(SecondaryViewError::SlotLimit)
    );
    limits.max_uniform_buffer_binding_size = 256;
    assert_eq!(
        SecondaryViewSlot::validate(&[slot], [64, 64], &limits),
        Err(SecondaryViewError::RecordLimit)
    );
}

#[test]
fn a_contribution_reads_its_own_colour_depth_and_record_through_the_public_seam() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary public seam",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = camera(0.0, -2.0);
        let mut host = host(&capabilities, &queue, scene).await;
        let (observer, control) = Observer::new(1);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&camera(0.0, -2.0), [64, 64], None, ViewAir::Camera)
                .expect("request"),
        );
        host.set_scene_contribution(Box::new(observer))
            .expect("attach");
        host.capture_pixels().expect("capture");
        let bytes = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let (colour, depth) = pixel(&bytes, 32 * 64 + 32);
        assert_eq!(colour, [1.0, 0.5, 0.25, 1.0]);
        assert!(depth > 0.0 && depth < 1.0);
        let camera_z = f32::from_le_bytes(bytes[264..268].try_into().expect("record camera z"));
        assert_eq!(camera_z, -2.0);
        assert_eq!(control.lock().expect("control").handles[0].generation(), 1);
        let (invalid, _) = Observer::new(5);
        let error = host
            .set_scene_contribution(Box::new(invalid))
            .expect_err("over profile count");
        assert!(error.contains("secondary view slot limit"), "{error}");
        host.capture_pixels().expect("old contribution retained");
        let (first, first_control) = Observer::new(1);
        let (second, second_control) = Observer::new(2);
        host.set_scene_contribution(Box::new(
            thrum_render::contribution::SceneContributions::new(vec![
                Box::new(first),
                Box::new(second),
            ]),
        ))
        .expect("three owner-scoped slots");
        let first = first_control.lock().expect("first owner");
        let second = second_control.lock().expect("second owner");
        assert_eq!(first.handles.len(), 1);
        assert_eq!(second.handles.len(), 2);
        assert_ne!(
            first.handles[0].record(),
            second.handles[0].record(),
            "owners receive different physical records"
        );
    });
}
