use super::*;
use crate::{
    capabilities::DeviceCapabilities,
    synthetic::{CaptureSource, PreparedSynthetic, SyntheticScene},
};

#[test]
fn metal_recipe_replacement_retains_old_target_and_exact_channels() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        assert_eq!(capabilities.unified_memory(), Some(true));
        assert_eq!(
            capabilities.registry_digest(),
            capabilities.clone().registry_digest()
        );
        let unknown = DeviceCapabilities::capture_device(&device, 41);
        assert!(unknown.with_recipe_override("apple-tile").is_err());
        let scene = SyntheticScene::grid(1).unwrap();
        for source in [CaptureSource::Sdr, CaptureSource::SceneLinear] {
            let portable = capabilities
                .clone()
                .with_recipe_override("portable")
                .unwrap();
            let apple = DeviceCapabilities::capture(&adapter, &device, 42)
                .with_recipe_override("apple-tile")
                .unwrap();
            let old = PreparedSynthetic::prepare_with_capabilities(
                &portable,
                &queue,
                &scene,
                crate::synthetic::FrameLighting::Diagnostic(
                    crate::synthetic::SyntheticLight::default(),
                ),
                1.0,
            )
            .await
            .unwrap();
            let new = PreparedSynthetic::prepare_with_capabilities(
                &apple,
                &queue,
                &scene,
                crate::synthetic::FrameLighting::Diagnostic(
                    crate::synthetic::SyntheticLight::default(),
                ),
                1.0,
            )
            .await
            .unwrap();
            let old_target =
                OffscreenTarget::with_capture_source(&device, &old, 65, 33, source).unwrap();
            let new_target =
                OffscreenTarget::with_capture_source(&device, &new, 65, 33, source).unwrap();
            assert!(
                new_target.day_depth.is_none(),
                "no stored override-only twin without stars"
            );
            let old_depth = old_target.textures.texture(&old_target.ids.depth).unwrap();
            let new_depth = new_target.textures.texture(&new_target.ids.depth).unwrap();
            assert_eq!(
                crate::capabilities::apple::storage_mode(old_depth),
                Some("Private")
            );
            assert_eq!(
                crate::capabilities::apple::storage_mode(new_depth),
                Some("Memoryless")
            );
            let (old_count, old_bytes) = old_target.texture_allocation();
            let (new_count, new_bytes) = new_target.texture_allocation();
            println!(
                "driver-resident bytes with both complete generations and shared/unique resources={:?}",
                capabilities.resident_device_bytes()
            );
            assert_eq!(old_count, new_count);
            assert_eq!(
                old_bytes, new_bytes,
                "nominal texture footprint counts memoryless allocations too"
            );
            println!(
                "source={source:?} old={old_count}/{old_bytes} new={new_count}/{new_bytes} retained overlap textures={} nominal_bytes={} fallback_only_bytes=0",
                old_count + new_count,
                old_bytes + new_bytes
            );
            let mut pixels = Vec::new();
            for (prepared, target, generation) in [(&old, &old_target, 41), (&new, &new_target, 42)]
            {
                let mut readback = target.readback(&device);
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode_with_readback(&mut encoder, generation, target, None, &mut readback)
                    .unwrap();
                queue.submit([encoder.finish()]);
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                pixels.push(readback.try_collect(generation).unwrap().unwrap());
            }
            assert_eq!(pixels[0], pixels[1], "same-target raw channels are exact");
        }
    });
}
