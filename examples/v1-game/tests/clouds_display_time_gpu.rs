//! Submitted game frames interpolate shade without refreshing its products.
pub mod support;

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use thrum_render::runtime::{FrameConfig, FrameSample};
use v1_game::{CloudSelection, GameHost, GameScene, GameSetup, GameWorld};

type Host = thrum_render::runtime::FrameHost<GameWorld, GameScene>;

fn complete(host: &mut Host, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = host.poll_completed().expect("completion") {
            return sample;
        }
        assert!(Instant::now() < deadline);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
    }
}

fn mapping(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Buffer) -> Vec<f32> {
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("submitted shade mapping readback"),
        size: source.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(source, 0, &target, 0, source.size());
    queue.submit([encoder.finish()]);
    let (send, receive) = mpsc::channel();
    target.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).expect("receiver");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("Metal");
    receive.recv().expect("callback").expect("map");
    let bytes = target.slice(..).get_mapped_range().expect("readback");
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_mapping_moves_at_display_time_without_product_writes() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_mapping_moves_at_display_time_without_product_writes",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", true);
        let (view, profile) = support::presentation(&device, [160, 90]);
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: [160, 90],
        };
        let mut host = GameSetup {
            clouds: CloudSelection::Shade,
            ..GameSetup::default()
        }
        .prepare(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: [160, 90],
                surface: Some(profile),
            },
        )
        .await
        .expect("cloud game");
        host.drive(Duration::ZERO, Some(&surface)).expect("origin");
        complete(&mut host, &device);
        host.drive(Duration::from_nanos(16_666_667), Some(&surface))
            .expect("tick one");
        let first = complete(&mut host, &device);
        let (_, buffer) = host.cloud_ground_product().expect("GPU provider");
        let a = mapping(&device, &queue, &buffer);
        host.drive(Duration::from_millis(25), Some(&surface))
            .expect("same tick");
        let second = complete(&mut host, &device);
        let b = mapping(&device, &queue, &buffer);
        assert_eq!(first.tick, second.tick);
        let expected = 10.0 / 60.0 * (second.presentation_alpha - first.presentation_alpha);
        assert!(expected > 0.05, "distinct display times");
        // Near-window positions retain millimetre precision at this view.
        assert!((b[16] - a[16] - expected).abs() < 0.002);
        assert!((b[17] - a[17]).abs() < 0.002);
        assert!(
            !second
                .gpu_pass_ns
                .expect("GPU pass times")
                .passes()
                .iter()
                .any(|p| p.name.contains("cloud "))
        );
        host.set_scene_camera(
            v1_game::SceneCamera::perspective(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, 0.0],
                65.0_f32.to_radians(),
                0.1,
                20.0,
            )
            .expect("checked cut camera"),
        )
        .expect("camera cut");
        host.drive(Duration::from_millis(26), Some(&surface))
            .expect("cut frame");
        complete(&mut host, &device);
        let c = mapping(&device, &queue, &buffer);
        let expected_cut = 10.0 / 60.0 * (1.0 - second.presentation_alpha);
        assert!(
            (c[16] - b[16] - expected_cut).abs() < 0.002,
            "cut uses committed travel"
        );
    });
}
