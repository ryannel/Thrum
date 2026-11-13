//! Capability fallback must prepare Direct, rather than merely report it.
use thrum_render::{
    capabilities::DeviceCapabilities,
    synthetic::{
        CullMode, CullingReason, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight,
        SyntheticScene,
    },
};

#[test]
fn capability_record_selects_the_prepared_path() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let known = DeviceCapabilities::capture(&adapter, &device, 1);
        let masked = known.clone().without_indirect_execution();
        let unknown = DeviceCapabilities::capture_device(&device, 1);
        let mut direct_image = None;
        for requested in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
            for (caps, fallback) in [
                (&known, CullingReason::Supported),
                (&masked, CullingReason::MissingIndirectExecution),
                (&unknown, CullingReason::UnknownCapabilities),
            ] {
                let mut scene = SyntheticScene::grid(100).expect("grid");
                scene.camera_culling = requested;
                let prepared = PreparedSynthetic::prepare_with_capabilities(
                    caps,
                    &queue,
                    &scene,
                    FrameLighting::Diagnostic(SyntheticLight::default()),
                    1.0,
                )
                .await
                .expect("prepare");
                let selection = prepared.culling_selection();
                assert_eq!(selection.requested, requested);
                let reason = if requested == CullMode::Direct {
                    CullingReason::RequestedDirect
                } else {
                    fallback
                };
                assert_eq!(selection.reason, reason);
                assert_eq!(
                    selection.resolved,
                    if reason == CullingReason::Supported {
                        requested
                    } else {
                        CullMode::Direct
                    }
                );
                assert_eq!(prepared.camera_culling(), selection.resolved);
                // Exercise the selected pipelines and graph, including both fallback records.
                let target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
                let mut pixels = target.readback(&device);
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode_with_readback(&mut encoder, 1, &target, None, &mut pixels)
                    .expect("encode");
                let submission = queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(std::time::Duration::from_secs(30)),
                    })
                    .expect("wait");
                let image = pixels.try_collect(1).expect("readback").expect("pixels");
                assert!(
                    image
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| pixel != &image[..4])
                );
                if selection.resolved == CullMode::Direct {
                    if let Some(reference) = &direct_image {
                        assert_eq!(
                            &image, reference,
                            "fallback draws the complete Direct scene"
                        );
                    } else {
                        direct_image = Some(image);
                    }
                }
            }
        }
    });
}
