//! Two public slots keep independent targets, records and camera products.
mod support;
use support::*;
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
};

#[test]
fn each_view_matches_its_alone_bytes_in_either_request_order() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary view pair",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let (scene, lighting, second_camera) = lit_view_scene();
        let mut host = host_with_lighting(&capabilities, &queue, scene, lighting).await;
        let (observer, control) = Observer::new(2);
        host.set_scene_contribution(Box::new(observer))
            .expect("attach");
        let requests = [
            SecondaryViewRequest::new(
                &aimed([100.0, 47.0, 997.5], [0.0, 3.5, 2.5]),
                [64, 64],
                Some(ViewClipPlane::new([1.0, 0.0, 0.0, 0.2]).expect("clip")),
                ViewAir::Camera,
            )
            .expect("request")
            .with_shadows(2, 64.0)
            .expect("Sun"),
            SecondaryViewRequest::new(
                &second_camera,
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
            &second_camera,
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
