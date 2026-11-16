//! Compare every stored volume channel against the ordinary atmosphere pass,
//! including the half of the level view which never crosses the plane.

use super::*;
use crate::{
    atmosphere::AtmosphereFrame, capabilities::DeviceCapabilities, synthetic::FrameLighting,
};

fn camera(height: f32, pitch: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, height, 0.0],
        [0.0, height + pitch.sin(), -pitch.cos()],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        5000.0,
    )
    .expect("air camera")
}

fn lighting(height: f32) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], height, 1)
            .expect("atmosphere"),
    )
}

#[cfg(target_os = "macos")]
#[test]
fn plane_volume_matches_every_channel_of_the_ordinary_observer() {
    pollster::block_on(async {
        let (_, device, queue) =
            crate::synthetic::tests::support::device(wgpu::Backends::METAL).await;
        let capabilities = DeviceCapabilities::capture_device(&device, 1);
        for height in [0.0, 50.0] {
            for below in [2.0, 40.0] {
                for pitch in [0.0, 30.0_f32.to_radians()] {
                    let mut scene = SyntheticScene::grid(1).expect("scene");
                    scene.view = camera(100.0, 0.0);
                    let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                        &capabilities,
                        &queue,
                        &scene,
                        lighting(100.0),
                        1.0,
                    )
                    .await
                    .expect("scene");
                    prepared
                        .prepare_secondary_views(
                            &queue,
                            &[SecondaryViewSlot::new(1.0).expect("slot")],
                            [64, 64],
                            &[],
                        )
                        .expect("view");
                    prepared.set_contribution_passes(ContributionPasses {
                        upload: true,
                        compute: false,
                    });
                    let target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
                    let request = SecondaryViewRequest::new(
                        &camera(height - below, pitch),
                        [64, 64],
                        Some(
                            crate::contribution::ViewClipPlane::new([0.0, 1.0, 0.0, -height])
                                .expect("plane"),
                        ),
                        ViewAir::Plane,
                    )
                    .expect("request");
                    let mut asker = Asker {
                        slot: prepared.secondary_handles[0].clone(),
                        request: Some(request),
                    };
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    prepared
                        .encode_contributed(
                            &mut encoder,
                            1,
                            &target,
                            None,
                            None,
                            Some(&mut asker),
                            None,
                            &mut Vec::new(),
                        )
                        .expect("view graph");
                    queue.submit([encoder.finish()]);
                    let actual = volume(
                        &device,
                        &queue,
                        prepared.secondary[0]
                            .aerial
                            .as_ref()
                            .expect("aerial")
                            .textures(),
                    );
                    scene.view = camera(height, pitch);
                    let reference = PreparedSynthetic::prepare_with_capabilities(
                        &capabilities,
                        &queue,
                        &scene,
                        lighting(height),
                        1.0,
                    )
                    .await
                    .expect("ordinary reference");
                    let target = OffscreenTarget::new(&device, &reference, 64, 64)
                        .expect("reference target");
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    reference
                        .encode(&mut encoder, 1, &target, None)
                        .expect("ordinary graph");
                    queue.submit([encoder.finish()]);
                    let expected = volume(
                        &device,
                        &queue,
                        reference
                            .atmosphere
                            .as_ref()
                            .expect("atmosphere")
                            .aerial_textures(),
                    );
                    assert_eq!(actual.len(), expected.len());
                    if height > 0.0 && pitch.to_bits() == 0.0_f32.to_bits() {
                        let never_crossing = (32 * 32 + 24 * 32 + 16) * 4;
                        assert!(
                            half::f16::from_bits(expected[never_crossing]).to_f32() > 0.0,
                            "the reference's never-crossing row must carry scattering"
                        );
                    }
                    let mut maximum = 0.0_f32;
                    for (&a, &e) in actual.iter().zip(&expected) {
                        let value = half::f16::from_bits(e).to_f32();
                        let step = (half::f16::from_bits(e + 1).to_f32() - value).abs();
                        let difference = (half::f16::from_bits(a).to_f32() - value).abs();
                        maximum = maximum.max(difference / step);
                        assert!(
                            difference <= step,
                            "height {height} below {below} pitch {pitch}: difference {difference}, step {step}"
                        );
                    }
                    eprintln!(
                        "plane aerial height={height} below={below} pitch={pitch}: {} stored channels, max steps {maximum}",
                        actual.len()
                    );
                }
            }
        }
    });
}

pub(super) fn volume(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: [&wgpu::Texture; 2],
) -> Vec<u16> {
    let bytes = u64::from(textures[0].width())
        * u64::from(textures[0].height())
        * u64::from(textures[0].depth_or_array_layers())
        * 8;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("aerial comparison readback"),
        size: bytes * 2,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (index, texture) in (0_u64..).zip(textures) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: bytes * index,
                    bytes_per_row: Some(texture.width() * 8),
                    rows_per_image: Some(texture.height()),
                },
            },
            texture.size(),
        );
    }
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&output, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("callback receiver");
    });
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("readback completion");
    receive.recv().expect("callback").expect("map");
    output
        .slice(..)
        .get_mapped_range()
        .expect("mapped volume")
        .as_chunks::<2>()
        .0
        .iter()
        .map(|word| u16::from_le_bytes(*word))
        .collect()
}
