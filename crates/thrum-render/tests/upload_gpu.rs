//! Opt-in real-device proof that changed instance rows reach rendered pixels.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use thrum_render::synthetic::{OffscreenTarget, PreparedSynthetic, RenderError, SyntheticScene};

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
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
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
