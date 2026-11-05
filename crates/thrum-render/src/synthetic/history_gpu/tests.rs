#[cfg(target_os = "macos")]
use super::super::*;

#[cfg(target_os = "macos")]
fn half_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = u32::from((bits >> 10) & 0x1f);
    let fraction = u32::from(bits & 0x03ff);
    if exponent == 0 {
        let value = f32::from(bits & 0x03ff) * 2.0_f32.powi(-24);
        return if sign == 0 { value } else { -value };
    }
    f32::from_bits(sign | ((exponent + 112) << 23) | (fraction << 13))
}

#[cfg(target_os = "macos")]
fn submitted_motion_x(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    changed: &[u32],
    x: u32,
) -> f32 {
    prepared
        .update_frame(
            queue,
            target,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
        )
        .expect("live view");
    prepared
        .upload_instances(queue, &scene.instances, changed)
        .expect("instance upload");
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("motion test readback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 41, target, None)
        .expect("scene with motion");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target
                .textures
                .texture(&target.ids.motion)
                .expect("motion target"),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("readback receiver");
    });
    let submission = queue.submit([encoder.finish()]);
    prepared.commit_frame(queue);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("motion submission");
    receive
        .recv()
        .expect("readback callback")
        .expect("map motion");
    let mapped = buffer.slice(..).get_mapped_range().expect("mapped motion");
    let offset = 32 * 256 + x as usize * 4;
    let value = half_to_f32(u16::from_le_bytes(
        mapped[offset..offset + 2].try_into().unwrap(),
    ));
    drop(mapped);
    buffer.unmap();
    value
}

#[cfg(target_os = "macos")]
#[test]
fn metal_motion_history_follows_submitted_frames_and_resets() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
        else {
            eprintln!("SKIP motion history: no Metal adapter");
            return;
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Metal device");
        let mut scene = SyntheticScene::grid(1).expect("one instance");
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .expect("scene pipeline");
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
        prepared.synchronize_history(&queue, 1, 0);
        assert!(
            submitted_motion_x(&device, &queue, &mut prepared, &mut target, &scene, &[], 32).abs()
                < 0.1
        );
        scene.instances[0].current[3][0] = 0.2;
        let moved = submitted_motion_x(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[0],
            38,
        );
        assert!(
            (moved + 6.4).abs() < 0.5,
            "history-pixel minus current-pixel: {moved}"
        );
        scene.instances[0].current[3][0] = 0.4;
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
            )
            .expect("abandoned view update");
        prepared
            .upload_instances(&queue, &scene.instances, &[0])
            .expect("abandoned instance upload");
        let mut abandoned =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut abandoned, 41, &target, None)
            .expect("abandoned encode");
        drop(abandoned);
        queue.submit([]);
        scene.instances[0].current[3][0] = 0.6;
        let after_drop = submitted_motion_x(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[0],
            50,
        );
        assert!(
            (after_drop + 12.8).abs() < 0.5,
            "dropped frame must not advance history: {after_drop}"
        );
        scene.instances[0].ids[3] = 2;
        let reused = submitted_motion_x(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[0],
            50,
        );
        assert!(
            reused.abs() < 0.1,
            "reused generation resets motion: {reused}"
        );
        prepared.synchronize_history(&queue, 2, 0);
        let reset =
            submitted_motion_x(&device, &queue, &mut prepared, &mut target, &scene, &[], 50);
        assert!(reset.abs() < 0.1, "world epoch resets motion: {reset}");
        scene.instances[0].current[3][0] = 0.7;
        scene.view = SyntheticView::default().with_history_reset(true);
        let cut = submitted_motion_x(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[0],
            54,
        );
        assert!(cut.abs() < 0.1, "camera cut resets motion: {cut}");

        let mut compacted_scene = SyntheticScene::grid(2).expect("two instances");
        for instance in &mut compacted_scene.instances {
            instance.current[3][1] = 0.0;
            instance.previous = instance.current;
        }
        let mut compacted = PreparedSynthetic::prepare(&device, &queue, 41, &compacted_scene)
            .await
            .expect("two-instance scene");
        let mut compacted_target =
            OffscreenTarget::new(&device, &compacted, 64, 64).expect("two-instance target");
        submitted_motion_x(
            &device,
            &queue,
            &mut compacted,
            &mut compacted_target,
            &compacted_scene,
            &[],
            46,
        );
        compacted_scene.instances[0].ids[3] = 0;
        let after_compaction = submitted_motion_x(
            &device,
            &queue,
            &mut compacted,
            &mut compacted_target,
            &compacted_scene,
            &[0],
            46,
        );
        assert!(
            after_compaction.abs() < 0.1,
            "physical-slot compaction resets motion: {after_compaction}"
        );
    });
}
