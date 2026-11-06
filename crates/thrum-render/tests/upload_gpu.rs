//! Opt-in real-device proof that changed instance rows reach rendered pixels.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::synthetic::{
    OffscreenTarget, PreparedSynthetic, RenderError, SyntheticLight, SyntheticScene, SyntheticView,
};

fn pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
) -> Vec<u8> {
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 1, target, None, &mut readback)
        .expect("valid prepared view and free readback slot");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("device completes submitted frame");
    readback
        .try_collect(1)
        .expect("readback succeeds")
        .expect("readback completes with frame")
}

#[test]
fn changed_slot_updates_pixels_and_invalid_row_keeps_prior_draw() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device request succeeds");
        let mut scene = SyntheticScene::grid(4).expect("four valid cubes");
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
            .await
            .expect("prepared four-cube view");
        let target = OffscreenTarget::new(&device, &prepared, 65, 33).expect("valid target");
        let before = pixels(&device, &queue, &prepared, &target);

        scene.instances[0].current[3][0] += 0.5;
        let report = prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .expect("one sorted changed slot");
        assert_eq!(
            (
                report.changed_rows,
                report.dropped_invalid,
                report.writes,
                report.bytes
            ),
            (1, 0, 1, 192)
        );
        let after = pixels(&device, &queue, &prepared, &target);
        assert_ne!(before, after, "the changed transform reached the GPU draw");

        scene.instances[0].current[3][0] = f32::NAN;
        let report = prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .expect("malformed presentation data degrades");
        assert_eq!(
            (report.changed_rows, report.dropped_invalid, report.writes),
            (0, 1, 0)
        );
        assert_eq!(after, pixels(&device, &queue, &prepared, &target));
        assert!(matches!(
            prepared.upload_instances(&queue, &scene.instances, &[0, 0]),
            Err(RenderError::InvalidInstanceUpdate)
        ));
        assert_eq!(after, pixels(&device, &queue, &prepared, &target));

        for instance in &mut scene.instances[1..] {
            instance.current[3][0] += 0.3;
        }
        let report = prepared
            .upload_instances(&queue, &scene.instances, &[0, 1, 2, 3])
            .expect("mixed valid and invalid presentation rows degrade");
        assert_eq!(
            (
                report.changed_rows,
                report.dropped_invalid,
                report.writes,
                report.bytes
            ),
            (3, 1, 1, 4 * 192)
        );
        let moved = pixels(&device, &queue, &prepared, &target);
        assert_ne!(after, moved);

        scene.instances[1].ids[0] = 0;
        let duplicate = prepared
            .upload_instances(&queue, &scene.instances, &[1])
            .expect("duplicate logical identity degrades");
        assert_eq!((duplicate.changed_rows, duplicate.dropped_invalid), (0, 1));
        assert_eq!(moved, pixels(&device, &queue, &prepared, &target));
        scene.instances[1].ids[0] = 1;

        scene.instances[1].ids[3] = 0;
        prepared
            .upload_instances(&queue, &scene.instances, &[1])
            .expect("retired instance upload");
        let hidden = pixels(&device, &queue, &prepared, &target);
        assert_ne!(moved, hidden, "retired GPU instance no longer draws");
        scene.instances[1].ids[3] = 1;
        prepared
            .upload_instances(&queue, &scene.instances, &[1])
            .expect("reused instance upload");
        assert_eq!(moved, pixels(&device, &queue, &prepared, &target));
    });
}

#[test]
fn active_prefix_matches_fresh_scene_through_churn_and_slot_reuse() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device request succeeds");
        let mut scene = SyntheticScene::grid(4).unwrap();
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
            .await
            .unwrap();
        let target = OffscreenTarget::new(&device, &prepared, 128, 128).unwrap();
        let original = pixels(&device, &queue, &prepared, &target);
        assert_eq!(prepared.active_instance_count(), 4);

        for slot in [0, 2] {
            scene.instances[slot].ids[3] = 0;
        }
        prepared
            .upload_instances(&queue, &scene.instances, &[0, 2])
            .unwrap();
        assert_eq!(prepared.active_instance_count(), 2);
        let compacted = pixels(&device, &queue, &prepared, &target);
        let reference = PreparedSynthetic::prepare(&device, &queue, 1, &scene)
            .await
            .unwrap();
        let reference_target = OffscreenTarget::new(&device, &reference, 128, 128).unwrap();
        assert_eq!(
            compacted,
            pixels(&device, &queue, &reference, &reference_target)
        );

        for slot in [0, 2] {
            scene.instances[slot].ids[3] = 1;
        }
        prepared
            .upload_instances(&queue, &scene.instances, &[0, 2])
            .unwrap();
        assert_eq!(prepared.active_instance_count(), 4);
        assert_eq!(original, pixels(&device, &queue, &prepared, &target));
    });
}

#[test]
fn mirrored_row_uses_two_sided_fallback_and_keeps_near_surface() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device request succeeds");
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_light(
            &device,
            &queue,
            1,
            &scene,
            SyntheticLight {
                direction_to_light: [0.0, 0.0, 1.0],
                irradiance: [4.0; 3],
            },
        )
        .await
        .unwrap();
        let target = OffscreenTarget::new(&device, &prepared, 65, 65).unwrap();
        let original = pixels(&device, &queue, &prepared, &target);
        assert_eq!(prepared.two_sided_instance_count(), 0);
        scene.instances[0].current[0][0] *= -1.0;
        scene.instances[0].normal[0][0] *= -1.0;
        scene.instances[0].previous = scene.instances[0].current;
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .unwrap();
        assert_eq!(prepared.two_sided_instance_count(), 1);
        let mirrored = pixels(&device, &queue, &prepared, &target);
        let center = (32 * 65 + 32) * 4;
        assert!(original[center] > 30, "near cube surface must be lit");
        assert_eq!(original[center..center + 4], mirrored[center..center + 4]);
        scene.instances[0].current[0][0] *= -1.0;
        scene.instances[0].normal[0][0] *= -1.0;
        scene.instances[0].previous = scene.instances[0].current;
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .unwrap();
        assert_eq!(prepared.two_sided_instance_count(), 0);
        assert_eq!(original, pixels(&device, &queue, &prepared, &target));
    });
}

#[test]
fn front_raster_surface_repairs_a_smoothed_normal_behind_the_view() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for shading-normal regression");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .unwrap();
        let row = &mut scene.instances[0];
        row.current = [
            [0.6, 0.0, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 0.0, 0.6, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        row.previous = row.current;
        // At the exact centre pixel V is +Z. These two shading normals are
        // symmetric about its tangent plane; neither changes raster winding.
        row.normal = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.02, 0.0],
        ];
        scene.materials[0].base_rgba = [0.65; 4];
        scene.materials[0].surface = [1.0, 0.8, 0.0, 1.0];
        let mut prepared = PreparedSynthetic::prepare_with_light(
            &device,
            &queue,
            1,
            &scene,
            SyntheticLight {
                direction_to_light: [1.0, 0.0, 0.0],
                irradiance: [1.0; 3],
            },
        )
        .await
        .unwrap();
        let target = OffscreenTarget::new(&device, &prepared, 65, 65).unwrap();
        let offset = (32 * 65 + 32) * 4;
        let reference = pixels(&device, &queue, &prepared, &target);
        assert!(
            reference[offset] > 40,
            "positive-hemisphere reference must be lit"
        );
        scene.instances[0].normal[2][2] = -0.02;
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .unwrap();
        let repaired = pixels(&device, &queue, &prepared, &target);
        for channel in 0..3 {
            assert!(
                reference[offset + channel].abs_diff(repaired[offset + channel]) <= 2,
                "front surface must not turn black from a slightly negative shading NoV"
            );
        }
        // Mirroring reverses the geometric winding and selects the existing
        // two-sided fallback. A real back-facing raster surface is not repaired.
        scene.instances[0].current[0][0] *= -1.0;
        scene.instances[0].previous = scene.instances[0].current;
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .unwrap();
        assert_eq!(prepared.two_sided_instance_count(), 1);
        let back = pixels(&device, &queue, &prepared, &target);
        assert_eq!(&back[offset..offset + 3], &[0_u8; 3]);
    });
}
