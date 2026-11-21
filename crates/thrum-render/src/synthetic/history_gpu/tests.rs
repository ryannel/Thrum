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
    submitted_motion_x_at_alpha(device, queue, prepared, target, scene, changed, x, 1.0)
}

#[cfg(target_os = "macos")]
fn submitted_motion_x_at_alpha(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    changed: &[u32],
    x: u32,
    alpha: f32,
) -> f32 {
    prepared
        .update_frame(
            queue,
            target,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            alpha,
        )
        .expect("live view");
    prepared
        .upload_instances(queue, &scene.instances, changed)
        .expect("instance upload");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 41, target, None)
        .expect("scene with motion");
    let readback = thrum_test_support::readback::Readback::texture(
        device,
        &mut encoder,
        target
            .textures
            .texture(&target.ids.motion)
            .expect("motion target"),
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let submission = queue.submit([encoder.finish()]);
    prepared.commit_frame();
    let mapped = readback.wait(device, submission);
    let offset = 32 * 256 + x as usize * 4;
    half_to_f32(u16::from_le_bytes(
        mapped[offset..offset + 2].try_into().unwrap(),
    ))
}

#[cfg(target_os = "macos")]
#[test]
fn metal_motion_history_uses_interpolated_display_samples() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
            "metal_motion_history_uses_interpolated_display_samples",
            None,
            crate::capabilities::DeviceProfile::Portable,
            &tests::support::NO_FEATURES,
        ) else {
            return;
        };
        let mut scene = SyntheticScene::grid(1).expect("one instance");
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .expect("scene pipeline");
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
        prepared.synchronize_history(&queue, 1, 0);
        submitted_motion_x(&device, &queue, &mut prepared, &mut target, &scene, &[], 32);
        scene.instances[0].previous[3][0] = 0.0;
        scene.instances[0].current[3][0] = 0.2;
        let half = submitted_motion_x_at_alpha(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[0],
            35,
            0.5,
        );
        assert!((half + 3.2).abs() < 0.5, "half-step motion: {half}");
        let three_quarters = submitted_motion_x_at_alpha(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            &[],
            37,
            0.75,
        );
        assert!(
            (three_quarters + 1.6).abs() < 0.5,
            "motion must follow the last submitted display sample: {three_quarters}"
        );
    });
}

#[cfg(target_os = "macos")]
#[test]
fn metal_motion_history_follows_submitted_frames_and_resets() {
    pollster::block_on(async {
        let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
            "metal_motion_history_follows_submitted_frames_and_resets",
            None,
            crate::capabilities::DeviceProfile::Portable,
            &tests::support::NO_FEATURES,
        ) else {
            return;
        };
        let mut scene = SyntheticScene::grid(1).expect("one instance");
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .expect("scene pipeline");
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
        prepared.synchronize_history(&queue, 1, 0);
        assert_eq!(
            submitted_motion_x(&device, &queue, &mut prepared, &mut target, &scene, &[], 32)
                .to_bits(),
            65504.0_f32.to_bits()
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
        scene.view = scene.view.with_history_reset(true);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
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
        scene.view = scene.view.with_history_reset(false);
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
            "a dropped cut must preserve submitted history: {after_drop}"
        );
        scene.instances[0].set_generation(2);
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
            reused.to_bits() == 65504.0_f32.to_bits(),
            "reused generation resets motion: {reused}"
        );
        prepared.synchronize_history(&queue, 2, 0);
        let reset =
            submitted_motion_x(&device, &queue, &mut prepared, &mut target, &scene, &[], 50);
        assert!(
            reset.to_bits() == 65504.0_f32.to_bits(),
            "world epoch resets motion: {reset}"
        );
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
        assert!(
            cut.to_bits() == 65504.0_f32.to_bits(),
            "camera cut resets motion: {cut}"
        );

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
        compacted_scene.instances[0].deactivate();
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
            after_compaction.to_bits() == 65504.0_f32.to_bits(),
            "physical-slot compaction resets motion: {after_compaction}"
        );
    });
}
