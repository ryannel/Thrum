//! A held view sees its receivers, lamp and hazed far geometry while the main
//! camera turns about its position; colour, depth and record keep every bit.
mod support;
use support::*;
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRecord, SecondaryViewRequest, ViewAir},
    synthetic::SyntheticView,
};

#[test]
fn turning_the_main_camera_does_not_change_the_held_view() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary camera independence",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let (mut scene, lighting, view) = lit_view_scene();
        scene.view = camera(100.0, 997.5);
        let receiver = glam::Vec3::new(100.0, 50.5, 1000.0);
        let lamp = glam::Vec3::from_array(scene.local_lights[0].position);
        let hill = glam::Vec3::from_array(
            scene.instances[4].current[3][..3]
                .try_into()
                .expect("hill position"),
        );
        let mut host = host_with_lighting(&capabilities, &queue, scene, lighting).await;
        let (observer, control) = Observer::new(1);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&view, [64, 64], None, ViewAir::Camera)
                .expect("request")
                .with_shadows(2, 128.0)
                .expect("Sun"),
        );
        host.set_scene_contribution(Box::new(observer))
            .expect("attach");
        host.capture_pixels().expect("first orientation");
        let first = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let reverse = SyntheticView::perspective(
            [100.0, 0.0, 997.5],
            [100.0, 0.0, 996.5],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            5000.0,
        )
        .expect("reverse main");
        let main = glam::Mat4::from_cols_array_2d(&reverse.view_proj_columns());
        for point in [receiver, lamp, hill] {
            assert!(
                (main * point.extend(1.0)).w < 0.0,
                "fixture is behind the reversed main camera"
            );
        }
        host.continuous_source_mut().view = reverse;
        host.capture_pixels().expect("opposite orientation");
        let second = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let different = first.iter().zip(&second).position(|(a, b)| a != b);
        assert!(
            different.is_none(),
            "colour, depth and record must retain every bit; first difference at byte {different:?}"
        );
        let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&first[..RECORD_BYTES]);
        let inverse = glam::Mat4::from_cols_array_2d(&record.inverse_view_proj);
        let camera = glam::Vec3::from_array(view.camera_position());
        let mut near = 0;
        let mut far = 0;
        for index in 0..4096 {
            let (colour, depth) = pixel(&first, index);
            if colour[3] == 0.0 {
                continue;
            }
            let x = (index % 64) as f32 + 0.5;
            let y = (index / 64) as f32 + 0.5;
            let homogeneous = inverse * glam::Vec4::new(x / 32.0 - 1.0, 1.0 - y / 32.0, depth, 1.0);
            let point = homogeneous.truncate() / homogeneous.w;
            let distance = point.distance(camera);
            near += usize::from(distance < 10.0 && colour[0] > 0.0);
            far += usize::from(distance > 1000.0 && colour[0] > 0.0);
        }
        eprintln!(
            "held view: {near} lit near pixels, {far} covered far pixels; all {} observed bytes identical",
            first.len()
        );
        assert!(
            near > 10 && far > 10,
            "the fixture must contain its near receivers and hazed far geometry"
        );
    });
}
