//! Opt-in native GPU test of the scene and present graph submission into an
//! acquired-format texture. A real window/swapchain is exercised separately.
#![cfg(not(target_arch = "wasm32"))]

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use thrum_render::present::SurfaceProfile;
use v1_game::{FrameHost, SurfaceOutput};

const SIZE: u32 = 640;
const ROW_BYTES: u32 = SIZE * 4;

#[test]
fn surface_pass_letterboxes_and_preserves_sdr_bytes_across_srgb_formats() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features:
                    thrum_render::capabilities::DeviceCapabilities::requested_timing_features(
                        &adapter,
                    ),
                ..Default::default()
            })
            .await
            .expect("device with optional timestamps");
        let (raw, source) = render_surface(&device, &queue, wgpu::TextureFormat::Bgra8Unorm).await;
        let (srgb, _) = render_surface(&device, &queue, wgpu::TextureFormat::Bgra8UnormSrgb).await;
        let mut occupied = 0;
        for y in 0..SIZE as usize {
            for x in 0..SIZE as usize {
                let offset = y * ROW_BYTES as usize + x * 4;
                if !(140..500).contains(&y) {
                    assert_eq!(&raw[offset..offset + 3], &[0, 0, 0]);
                    assert_eq!(&srgb[offset..offset + 3], &[0, 0, 0]);
                } else if raw[offset..offset + 3] != [0, 0, 0] {
                    occupied += 1;
                }
                if (140..500).contains(&y) {
                    let source_offset = ((y - 140) * SIZE as usize + x) * 4;
                    assert_eq!(
                        &raw[offset..offset + 3],
                        &[
                            source[source_offset + 2],
                            source[source_offset + 1],
                            source[source_offset]
                        ],
                        "surface must preserve source orientation and SDR bytes"
                    );
                }
                for (&left, &right) in raw[offset..offset + 3]
                    .iter()
                    .zip(&srgb[offset..offset + 3])
                {
                    assert!(
                        left.abs_diff(right) <= 2,
                        "sRGB surface double-encoded a pixel"
                    );
                }
            }
        }
        assert!(occupied > 40_000, "the 10k scene must reach the surface");
    });
}

async fn render_surface(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
) -> (Vec<u8>, Vec<u8>) {
    let mut host = FrameHost::new(
        device,
        queue,
        v1_game::FrameConfig {
            surface: Some(SurfaceProfile {
                format,
                extent: [SIZE, SIZE],
            }),
            ..Default::default()
        },
    )
    .await
    .expect("prepared scene passes");
    // Compare the surface with the actual submitted source, including its
    // jitter sample, rather than a separate unjittered endpoint redraw.
    host.enable_submitted_capture_from(1, thrum_render::synthetic::CaptureSource::Sdr)
        .expect("submitted source tap");
    // Sample immediately before the second tick, as on the ordinary frame path.
    let advanced = host
        .advance(Duration::from_nanos(33_333_333))
        .expect("world advances before surface acquisition");
    assert!(advanced.can_acquire_serial());
    let encoded = advanced
        .encode_scene()
        .expect("scene encodes before surface acquisition");
    let surface = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface-format test target"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
    let report = encoded
        .submit(Some(&SurfaceOutput {
            view: &view,
            extent: [SIZE, SIZE],
        }))
        .expect("one committed presented frame");
    assert!(report.submitted && report.surface_presented);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface-format test readback"),
        size: u64::from(ROW_BYTES) * u64::from(SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &surface,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        let _ = send.send(result);
    });
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("surface copy completion");
    receive
        .recv_timeout(Duration::from_secs(10))
        .expect("map callback")
        .expect("map result");
    let bytes = buffer
        .slice(..)
        .get_mapped_range()
        .expect("mapped surface bytes")
        .to_vec();
    buffer.unmap();
    let completed = loop {
        if let Some(sample) = host.poll_completed().expect("timing progress") {
            break sample;
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("timestamp completion");
    };
    assert!(completed.surface_presented);
    if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        assert!(completed.gpu_pass_ns.is_some());
        assert!(completed.gpu_present_ns.is_some());
    }
    assert!(!completed.timing_fault);
    let deadline = Instant::now() + Duration::from_secs(10);
    let source = loop {
        if let Some(image) = host
            .try_submitted_image()
            .expect("submitted source progress")
        {
            assert_eq!(image.sample.submission_order, completed.submission_order);
            assert!(image.sample.surface_presented);
            let pixels = image.rgba.clone();
            host.recycle_submitted_image(image);
            break pixels;
        }
        assert!(Instant::now() < deadline, "submitted source readback");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("source copy completion");
    };
    assert_eq!(source.len(), 640 * 360 * 4);
    (bytes, source)
}

#[test]
fn unavailable_surface_drops_only_views_and_resize_reuses_preparation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut host = FrameHost::new(
            &device,
            &queue,
            v1_game::FrameConfig {
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: [SIZE, SIZE],
                }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared host");
        let missed = host
            .drive(Duration::from_nanos(16_666_667), None)
            .expect("committed tick without drawable");
        assert_eq!(missed.tick, 1);
        assert_eq!(missed.completed_ticks, 1);
        assert!(missed.dropped_view);
        assert!(!missed.submitted);
        assert_eq!(host.dropped_surface(), 1);

        host.resize_surface([1000, 1000])
            .expect("recompiled surface graph");
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("resized surface target"),
            size: wgpu::Extent3d {
                width: 1000,
                height: 1000,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let resumed = host
            .drive(
                Duration::from_nanos(33_333_334),
                Some(&SurfaceOutput {
                    view: &view,
                    extent: [1000, 1000],
                }),
            )
            .expect("resized surface frame");
        assert_eq!(resumed.tick, 2);
        assert_eq!(resumed.completed_ticks, 1);
        assert!(resumed.surface_presented);
        assert_eq!(host.dropped_surface(), 1);
    });
}
