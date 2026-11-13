//! A full 32-phase raster jitter cycle, with committed temporal history.
use std::time::Duration;
use thrum_render::{
    capabilities::DeviceCapabilities,
    synthetic::{
        CullMode, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
    },
};

#[test]
fn culled_modes_equal_direct_over_full_temporal_cycle() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let caps = DeviceCapabilities::capture(&adapter, &device, 1);
        let mut scene = SyntheticScene::grid(10_000).expect("scene");
        scene.view = thrum_render::synthetic::SyntheticView::perspective(
            [0.4, 0.1, 0.9],
            [0.7, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            16.0 / 9.0,
            0.01,
            10.0,
        )
        .expect("perspective")
        .with_temporal_inputs(true)
        .with_jitter_phase(0, [320, 180]);
        scene.sun_shadows = true;
        let light = FrameLighting::Diagnostic(SyntheticLight::default());
        let mut modes = Vec::new();
        for mode in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
            scene.camera_culling = mode;
            let prepared =
                PreparedSynthetic::prepare_with_capabilities(&caps, &queue, &scene, light, 1.0)
                    .await
                    .expect("prepare");
            assert_eq!(prepared.camera_culling(), mode);
            let target = OffscreenTarget::new(&device, &prepared, 320, 180).expect("target");
            modes.push((prepared, target));
        }
        for phase in 0..32 {
            scene.view = scene.view.with_jitter_phase(phase, [320, 180]);
            let mut images = Vec::new();
            for (prepared, target) in &mut modes {
                prepared
                    .update_frame(&queue, target, &scene, light, 0.5)
                    .expect("update");
                let mut readback = target.readback(&device);
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode_with_readback(&mut encoder, 1, target, None, &mut readback)
                    .expect("encode");
                queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(30)),
                    })
                    .expect("wait");
                images.push(readback.try_collect(1).expect("collect").expect("pixels"));
                prepared.commit_frame();
            }
            assert_eq!(images[0].len(), 320 * 180 * 4);
            for index in 1..3 {
                assert_eq!(
                    images[0]
                        .iter()
                        .zip(&images[index])
                        .filter(|(a, b)| a != b)
                        .count(),
                    0,
                    "mode {index} phase {phase}"
                );
            }
        }
    });
}
