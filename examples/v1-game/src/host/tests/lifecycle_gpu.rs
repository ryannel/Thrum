//! The real game keeps rendering/camera control while its resident world pauses.

use std::time::{Duration, Instant};

use thrum_core::sim::{WorldLifecycle, WorldLifecycleReply};

use super::super::{
    FrameConfig, FrameHost, FrameSample, SurfaceOutput, ThreadedFrameHost, WorldControls,
};
use thrum_render::present::SurfaceProfile;

fn transition(
    host: &mut ThreadedFrameHost,
    controls: &WorldControls,
    device: &wgpu::Device,
    request: WorldLifecycle,
) -> WorldLifecycleReply {
    controls
        .request_lifecycle(request)
        .expect("accept lifecycle");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        // GPU completion and world acknowledgement are independent services.
        while host
            .poll_completed()
            .expect("retire submitted GPU work")
            .is_some()
        {}
        if let Some(reply) = controls.poll_lifecycle_reply().expect("healthy world") {
            assert_eq!(reply.request, request);
            return reply;
        }
        assert!(Instant::now() < deadline, "world did not acknowledge");
        device.poll(wgpu::PollType::Poll).expect("GPU progress");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn draw(
    host: &mut ThreadedFrameHost,
    origin: Instant,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &wgpu::Texture,
) -> (Vec<u8>, FrameSample) {
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: [640, 360],
    };
    let report = host
        .drive(origin.elapsed(), Some(&output))
        .expect("draw current world");
    assert!(report.submitted && report.surface_presented);
    // Copy the actual submitted blit target, not a separate diagnostic render.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("submitted pause frame"),
        size: 640 * 360 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(640 * 4),
                rows_per_image: Some(360),
            },
        },
        target.size(),
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("readback receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("submitted pixels complete");
    receive
        .recv()
        .expect("mapping callback")
        .expect("pixels mapped");
    let deadline = Instant::now() + Duration::from_secs(10);
    let sample = loop {
        if let Some(sample) = host.poll_completed().expect("retire frame") {
            break sample;
        }
        assert!(Instant::now() < deadline, "submitted sample did not retire");
        device
            .poll(wgpu::PollType::Poll)
            .expect("completion callback");
        std::thread::yield_now();
    };
    assert_eq!(host.pending_frames(), 0);
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("submitted pixels");
    let pixels: Vec<_> = mapped
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == 255)
    );
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != [0; 3])
    );
    (pixels, sample)
}

fn retain(name: &str, pixels: &[u8]) {
    if let Some(path) = std::env::var_os("THRUM_PAUSE_CAPTURE_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).expect("explicit capture directory");
        assert_eq!(pixels.len(), 640 * 360 * 4);
        let mut ppm = b"P6\n640 360\n255\n".to_vec();
        for pixel in pixels.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(path.join(format!("{name}.ppm")), ppm).expect("retain capture");
    }
}

#[test]
fn threaded_pause_keeps_pixels_still_camera_live_and_resume_advances_the_same_world() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for threaded pause rendering");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("game rendering device");
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen presentation proxy, not a drawable"),
            size: wgpu::Extent3d {
                width: 640,
                height: 360,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let host = FrameHost::new(
            &device,
            &queue,
            FrameConfig {
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: [640, 360],
                }),
                ..FrameConfig::default()
            },
        )
        .await
        .expect("10k game scene");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
        let paused = transition(&mut host, &controls, &device, WorldLifecycle::Pause);
        let (before, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.presentation_alpha, 1.0);
        std::thread::sleep(Duration::from_millis(40));
        let (still, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.completed_ticks, 0);
        assert_eq!(sample.presentation_alpha, 1.0);
        assert_eq!(
            still, before,
            "paused world is stable across submitted frames"
        );
        host.set_camera_yaw(0.1).expect("camera while paused");
        let (camera, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.presentation_alpha, 1.0);
        assert_ne!(camera, still, "presentation remains interactive");
        host.set_camera_yaw(0.0).expect("restore fixed camera");
        controls
            .set_controls([1.0, 0.0, 0.0])
            .expect("held input while paused");
        let resumed = transition(&mut host, &controls, &device, WorldLifecycle::Resume);
        assert_eq!(resumed.id, paused.id, "resume excludes the hidden interval");
        assert_eq!(resumed.timeline, paused.timeline);
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("world progresses") == paused.id.tick {
            assert!(Instant::now() < deadline, "resumed world did not advance");
            std::thread::sleep(Duration::from_millis(1));
        }
        let next = transition(&mut host, &controls, &device, WorldLifecycle::Pause);
        assert_eq!(next.id.epoch, paused.id.epoch);
        assert_eq!(next.timeline, paused.timeline);
        assert!(next.id.tick > paused.id.tick);
        let (after, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, next.id.tick);
        assert_ne!(after, before, "the same game resumes its moving entities");
        retain("paused", &before);
        retain("camera-paused", &camera);
        retain("resumed", &after);
    });
}
