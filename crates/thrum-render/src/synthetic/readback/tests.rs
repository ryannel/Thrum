use super::*;
use crate::synthetic::{PreparedSynthetic, SyntheticScene};

#[test]
fn metal_readback_preserves_sdr_and_linear_channels_and_removes_row_padding() {
    pollster::block_on(async {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real Metal adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let scene = SyntheticScene::grid(1).unwrap();
        let prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .unwrap();
        for (source, expected) in [
            (CaptureSource::Sdr, vec![255, 0, 128, 255]),
            (
                CaptureSource::SceneLinear,
                vec![0, 0x3c, 0, 0, 0, 0x38, 0, 0x3c],
            ),
        ] {
            for width in [1, 3, 65, 1920] {
                let target =
                    OffscreenTarget::with_capture_source(&device, &prepared, width, 3, source)
                        .unwrap();
                let scene = target.textures.texture(&target.ids.scene).unwrap();
                assert_eq!(
                    scene.usage().contains(wgpu::TextureUsages::COPY_SRC),
                    source == CaptureSource::SceneLinear
                );
                let resource = match source {
                    CaptureSource::Sdr => &target.ids.output,
                    CaptureSource::SceneLinear => &target.ids.scene,
                };
                let texture = target.textures.texture(resource).unwrap();
                // queue.write_texture requires COPY_DST, which the ordinary output
                // intentionally lacks. A render clear provides known RGBA bytes.
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 1.0,
                                    g: 0.0,
                                    b: 0.5,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                }
                let mut readback = target.readback(&device);
                readback.copy_into(&mut encoder, &target, None).unwrap();
                queue.submit([encoder.finish()]);
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let pixels = readback.try_collect(41).unwrap().unwrap();
                assert_eq!(pixels.len(), width as usize * 3 * expected.len());
                assert!(
                    pixels
                        .chunks_exact(expected.len())
                        .all(|pixel| pixel == expected)
                );
            }
        }
    });
}
