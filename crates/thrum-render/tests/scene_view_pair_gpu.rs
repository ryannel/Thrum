//! Two public slots keep independent targets, records and camera products.
mod support;
use support::*;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
    synthetic::{FrameLighting, SyntheticPointLight, SyntheticScene},
};

#[test]
fn each_view_matches_its_alone_bytes_in_either_request_order() {
    let Some(gpu) = thrum_test_support::gpu("secondary view pair", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let mut scene = SyntheticScene::grid(5).expect("scene");
        scene.view = camera(0.0, 2.0);
        scene.sun_shadows = true;
        scene.camera_culling = thrum_render::synthetic::CullMode::Frustum;
        let toward = glam::Vec3::new(0.0, 1.0, 1.0).normalize();
        let eye = glam::Vec3::new(100.0, 48.0, 997.5);
        let far = eye + toward * 3000.0;
        for (instance, center) in scene.instances.iter_mut().zip([
            [0.0, 0.0, 0.5],
            [100.0, 50.5, 1000.0],
            [0.35, 0.35, 0.45],
            [100.35, 50.85, 999.95],
            far.to_array(),
        ]) {
            instance.current[3] = [center[0], center[1], center[2], 1.0];
            instance.previous = instance.current;
        }
        for (index, instance) in scene.instances.iter_mut().enumerate() {
            let half = match index {
                0 | 1 => [0.9, 0.9, 0.02],
                2 | 3 => [0.2; 3],
                _ => [500.0; 3],
            };
            for (axis, extent) in half.into_iter().enumerate() {
                for row in 0..3 {
                    instance.current[axis][row] = if axis == row { extent } else { 0.0 };
                    instance.normal[axis][row] = if axis == row { extent.recip() } else { 0.0 };
                }
            }
            instance.previous = instance.current;
        }
        scene.local_lights = vec![SyntheticPointLight {
            position: [100.0, 50.5, 999.5],
            radius: 1.0,
            candela: [10_000.0, 1000.0, 100.0],
        }];
        let lighting = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.7, 0.7, -0.1], 100_000.0, [1.0; 3], 0.0, 1).expect("air"),
        );
        let mut host = host_with_lighting(&capabilities, &queue, scene, lighting).await;
        let (observer, control) = Observer::new(2);
        host.set_scene_contribution(Box::new(observer))
            .expect("attach");
        let requests = [
            SecondaryViewRequest::new(
                &camera(0.0, -2.0),
                [64, 64],
                Some(ViewClipPlane::new([1.0, 0.0, 0.0, 0.2]).expect("clip")),
                ViewAir::Camera,
            )
            .expect("request")
            .with_shadows(2, 64.0)
            .expect("Sun"),
            SecondaryViewRequest::new(
                &thrum_render::synthetic::SyntheticView::perspective(
                    eye.to_array(),
                    (eye + toward).to_array(),
                    [0.0, 1.0, 0.0],
                    1.0,
                    1.0,
                    0.1,
                    5000.0,
                )
                .expect("second camera"),
                [16, 16],
                Some(ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane")),
                ViewAir::Plane,
            )
            .expect("request")
            .with_shadows(1, 128.0)
            .expect("Sun")
            .with_least_pixels(1.0)
            .expect("size"),
        ];
        let idle_main = host.capture_pixels().expect("idle capture");
        let mut alone = Vec::new();
        for (index, request) in requests.iter().enumerate() {
            control.lock().expect("control").requests = vec![None; 2];
            control.lock().expect("control").requests[index] = Some(*request);
            let main = host.capture_pixels().expect("single capture");
            assert_eq!(idle_main, main, "one view leaves main alone");
            let observed = output(
                &device,
                &queue,
                &control.lock().expect("control").outputs[index],
            );
            assert!(
                (0..4096)
                    .filter(|&pixel_index| pixel(&observed, pixel_index).0[3] > 0.0)
                    .count()
                    > 4,
                "view must draw"
            );
            alone.push(observed);
        }
        // The fixture must actually exercise the view-owned products. A
        // cascade-less substitute or a whole-camera air path is distinguishable.
        for (index, request) in requests.iter().enumerate() {
            {
                let mut state = control.lock().expect("control");
                state.requests = vec![None; 2];
                state.requests[index] = Some(request.with_shadows(0, 128.0).expect("height only"));
            }
            host.capture_pixels().expect("height-only comparison");
            let observed = output(
                &device,
                &queue,
                &control.lock().expect("control").outputs[index],
            );
            assert_ne!(
                alone[index][RECORD_BYTES..],
                observed[RECORD_BYTES..],
                "slot {index} includes a cascade shadow"
            );
        }
        let camera_air = SecondaryViewRequest::new(
            &thrum_render::synthetic::SyntheticView::perspective(
                eye.to_array(),
                (eye + toward).to_array(),
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                5000.0,
            )
            .expect("second camera"),
            [16, 16],
            Some(ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane")),
            ViewAir::Camera,
        )
        .expect("whole-camera comparison")
        .with_shadows(1, 128.0)
        .expect("Sun")
        .with_least_pixels(1.0)
        .expect("size");
        {
            let mut state = control.lock().expect("control");
            state.requests = vec![None, Some(camera_air)];
        }
        host.capture_pixels().expect("whole-camera air comparison");
        let observed = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[1],
        );
        assert_ne!(
            alone[1][RECORD_BYTES..],
            observed[RECORD_BYTES..],
            "the second view's stretch must be distinguishable"
        );
        control.lock().expect("control").requests = vec![None, Some(requests[1])];
        let lamp = host.source().lights[0];
        host.continuous_source_mut().lights[0].candela = [0.0; 3];
        host.capture_pixels().expect("lamp off comparison");
        let observed = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[1],
        );
        assert_ne!(
            alone[1][RECORD_BYTES..],
            observed[RECORD_BYTES..],
            "the second view must contain the local lamp"
        );
        host.continuous_source_mut().lights[0] = lamp;
        for order in [vec![0, 1], vec![1, 0]] {
            {
                let mut state = control.lock().expect("control");
                state.requests = requests.map(Some).to_vec();
                state.order = order;
            }
            assert_eq!(idle_main, host.capture_pixels().expect("paired capture"));
            let state = control.lock().expect("control");
            for (index, expected) in alone.iter().enumerate() {
                assert_eq!(
                    *expected,
                    output(&device, &queue, &state.outputs[index]),
                    "slot {index} must ignore the other request"
                );
            }
        }
    });
}
