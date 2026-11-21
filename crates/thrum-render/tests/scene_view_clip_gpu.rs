//! Reverse-Z reconstruction and clipping through the public slot record.
mod support;
use support::*;
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRecord, SecondaryViewRequest, ViewAir, ViewClipPlane},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};

fn instance(slot: u32, center: [f32; 3], half: [f32; 3]) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        instance.current[axis][axis] = half[axis];
        instance.normal[axis][axis] = half[axis].recip();
        instance.current[3][axis] = center[axis];
    }
    instance.previous = instance.current;
    instance
}
fn step(value: f32) -> f32 {
    let value = value.abs();
    f32::from_bits(value.to_bits() + 1) - value
}

#[test]
fn clip_and_inverse_record_keep_the_world_point_at_twenty_metres_and_three_kilometres() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary clip and depth",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for distance in [20.0, 3000.0] {
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.view = camera(0.0, 0.0);
            scene.instances = vec![instance(
                0,
                [0.0, 0.0, distance],
                [distance * 0.1, distance * 0.1, 0.01],
            )];
            scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
            scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
            let mut host = host_with_lighting(
                &capabilities,
                &queue,
                scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
            )
            .await;
            let (observer, control) = Observer::new(1);
            host.set_scene_contribution(Box::new(observer))
                .expect("attach");
            control.lock().expect("control").requests[0] = Some(
                SecondaryViewRequest::new(
                    &camera(0.0, 0.0),
                    [64, 64],
                    Some(ViewClipPlane::new([1.0, 0.0, 0.0, 0.0]).expect("plane")),
                    ViewAir::Camera,
                )
                .expect("request"),
            );
            host.capture_pixels().expect("capture");
            let bytes = output(
                &device,
                &queue,
                &control.lock().expect("control").outputs[0],
            );
            let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&bytes[..RECORD_BYTES]);
            let inverse = glam::Mat4::from_cols_array_2d(&record.inverse_view_proj);
            let (colour, depth) = pixel(&bytes, 32 * 64 + 31);
            assert_eq!(colour[3], 1.0, "kept panel");
            let ndc = glam::Vec2::new((31.5 / 64.0) * 2.0 - 1.0, 1.0 - (32.5 / 64.0) * 2.0);
            let unproject = |depth| {
                let homogeneous = inverse * glam::Vec4::new(ndc.x, ndc.y, depth, 1.0);
                homogeneous.truncate() / homogeneous.w
            };
            let actual = unproject(depth);
            let front = f64::from(distance) - f64::from(0.01_f32);
            let focal = (0.5_f64).tan().recip();
            let expected = [
                -f64::from(ndc.x) * front / focal,
                f64::from(ndc.y) * front / focal,
                front,
            ];
            let depth_allowance = (unproject(depth + 4.0 * step(depth)) - actual)
                .length()
                .max((unproject(depth - 4.0 * step(depth)) - actual).length());
            let float_allowance = 4.0 * step(actual.abs().max_element());
            let allowance = f64::from(depth_allowance + float_allowance);
            let difference = actual
                .to_array()
                .into_iter()
                .map(f64::from)
                .zip(expected)
                .map(|(a, e)| (a - e).powi(2))
                .sum::<f64>()
                .sqrt();
            eprintln!(
                "distance={distance}: reconstructed={actual:?}, true={expected:?}, difference={difference}, four depth steps={depth_allowance}, four float spacings={float_allowance}, allowance={allowance}"
            );
            assert!(difference <= allowance);
            let edge = (0..64)
                .filter(|&x| pixel(&bytes, 32 * 64 + x).0[3] > 0.0)
                .max()
                .expect("cut edge");
            assert!(
                (30..=32).contains(&edge),
                "the cut must reach its projected plane to within one pixel: {edge}"
            );
            for y in 0..64 {
                for x in 0..64 {
                    let (colour, _) = pixel(&bytes, y * 64 + x);
                    if colour[3] > 0.0 {
                        assert!(x <= 32, "nothing wholly on the rejected side draws");
                    }
                }
            }
        }
    });
}

#[test]
fn clipping_discards_the_ground_over_the_camera_and_clamps_a_near_plane() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary camera below ground",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let mut scene = SyntheticScene::grid(2).expect("scene");
        scene.view = camera(0.0, 0.0);
        scene.instances = vec![
            instance(0, [0.0, 1.0, 20.0], [5.0, 5.0, 0.01]),
            instance(1, [0.0, -1.0, 0.0], [1000.0, 0.2, 1000.0]),
        ];
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
        let mut host = host_with_lighting(
            &capabilities,
            &queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
        )
        .await;
        let (observer, control) = Observer::new(1);
        host.set_scene_contribution(Box::new(observer))
            .expect("attach");
        let camera = SyntheticView::perspective(
            [0.0, -2.0, 0.0],
            [0.0, 1.0, 20.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            5000.0,
        )
        .expect("underground camera");
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(
                &camera,
                [64, 64],
                Some(ViewClipPlane::new([0.0, 1.0, 0.0, 0.0]).expect("ground plane")),
                ViewAir::Camera,
            )
            .expect("request"),
        );
        host.capture_pixels().expect("capture");
        let bytes = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        assert_eq!(
            pixel(&bytes, 32 * 64 + 32).0,
            [1.0, 0.5, 0.25, 1.0],
            "the below-ground camera sees the panel above the plane"
        );
        let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&bytes[..RECORD_BYTES]);
        let inverse = glam::Mat4::from_cols_array_2d(&record.inverse_view_proj);
        let depth = pixel(&bytes, 32 * 64 + 32).1;
        let homogeneous = inverse * glam::Vec4::new(1.0 / 64.0, -1.0 / 64.0, depth, 1.0);
        let point = homogeneous.truncate() / homogeneous.w;
        assert!(
            point.y >= 0.0 && point.z > 19.0,
            "the depth belongs to the panel, never the ground: {point:?}"
        );
        let request = SecondaryViewRequest::new(
            &camera,
            [64, 64],
            Some(ViewClipPlane::new([0.0, 1.0, 0.0, 1.999]).expect("near plane")),
            ViewAir::Camera,
        )
        .expect("near request");
        control.lock().expect("control").requests[0] = Some(request);
        host.capture_pixels().expect("near capture");
        let bytes = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&bytes[..RECORD_BYTES]);
        let camera_side = record.clip_plane[1] * (-2.0) + record.clip_plane[3];
        assert!(
            (camera_side + 0.05).abs() < 1e-6,
            "effective plane is held at five centimetres: {camera_side}"
        );
    });
}
