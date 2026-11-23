//! Real Metal completion and frame identity, plus deterministic counter refusals.
mod support;

use std::time::Duration;
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    registry_audit::counter_refusal,
    runtime::{FrameConfig, SurfaceOutput},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
    timing::{TimingFrame, TimingRefusal, TimingStats},
};

fn accounted(stats: TimingStats) {
    assert_eq!(
        stats.gathered,
        stats.published + stats.total_refused() + stats.pending as u64
    );
}

#[test]
fn malformed_or_previously_seen_pairs_are_refused_by_reason() {
    for (start, end, reason) in [
        (0, 101, TimingRefusal::Zero),
        (101, 0, TimingRefusal::Zero),
        (u64::MAX, 101, TimingRefusal::Sentinel),
        (101, u64::MAX, TimingRefusal::Sentinel),
        (110, 109, TimingRefusal::Inverted),
        (90, 100, TimingRefusal::Repeated),
        (90, 110, TimingRefusal::Repeated),
    ] {
        assert_eq!(counter_refusal(start, end, 100), Some(reason));
    }
    assert_eq!(counter_refusal(101, 101, 100), None);
    assert_eq!(counter_refusal(100, 101, 100), None);
}

#[test]
fn submitted_frames_publish_their_own_batch_and_discarded_frames_publish_nothing() {
    let Some(gpu) = thrum_test_support::gpu("frame timestamp validity", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    pollster::block_on(async {
        let (device, queue) = gpu.portable(None);
        assert!(device.features().contains(wgpu::Features::TIMESTAMP_QUERY));
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 7);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.sun_shadows = false;
        let surface = SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent: [32, 32],
        };
        let view = support::drawable(&device, surface.format, surface.extent);
        let output = SurfaceOutput {
            view: &view,
            extent: surface.extent,
        };
        let mut host = support::host_with_config(
            &capabilities,
            &queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            FrameConfig {
                render_extent: surface.extent,
                surface: Some(surface),
            },
        )
        .await;
        let mut history = Vec::new();
        for order in 0..12 {
            assert!(host.drive(Duration::ZERO, Some(&output)).unwrap().submitted);
            let pending = host.timing_stats();
            assert_eq!(pending.pending, 1);
            accounted(pending);

            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            assert!(
                host.poll_completed().unwrap().is_none(),
                "render completion only submits the resolve"
            );
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let mut sample = host
                .poll_completed()
                .unwrap()
                .expect("resolved timing report");
            let timing = sample.gpu_pass_ns.as_ref().expect("valid Metal counters");
            assert_eq!(sample.submission_order, order);
            assert_eq!(
                timing.frame(),
                Some(TimingFrame {
                    device_generation: 7,
                    submission_order: order
                })
            );
            assert!(timing.sum() > 0.0);
            assert_eq!(host.timing_stats().published, order + 1);
            accounted(host.timing_stats());
            sample.detach_gpu_timing();
            history.push(sample);
        }
        assert_eq!(
            history[0]
                .gpu_pass_ns
                .as_ref()
                .unwrap()
                .frame()
                .unwrap()
                .submission_order,
            0
        );

        drop(
            host.advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap(),
        );
        let discarded = host.timing_stats();
        assert_eq!(discarded.refused(TimingRefusal::Discarded), 1);
        assert!(host.poll_completed().unwrap().is_none());
        accounted(discarded);

        host.set_timing_requested(false);
        drop(
            host.advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap(),
        );
        assert!(host.drive(Duration::ZERO, Some(&output)).unwrap().submitted);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let disabled = host.poll_completed().unwrap().expect("untimed frame");
        assert!(
            !disabled.timing_requested && !disabled.timing_fault && disabled.gpu_pass_ns.is_none()
        );
        assert_eq!(host.timing_stats(), discarded);

        let next_gpu = gpu.fresh_adapter();
        let (next_device, next_queue) = next_gpu.portable(None);
        let next_caps = DeviceCapabilities::capture(&next_gpu.adapter, &next_device, 8);
        let mut replacement = host
            .device_prepare_inputs()
            .unwrap()
            .prepare(&next_caps, &next_queue, Some(surface))
            .await
            .unwrap();
        host.set_timing_requested(true);
        assert!(host.drive(Duration::ZERO, Some(&output)).unwrap().submitted);
        assert_eq!(host.timing_stats().pending, 1);
        host.publish_device(&mut replacement).unwrap();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        assert!(
            host.poll_completed().unwrap().is_none(),
            "retired callbacks cannot publish"
        );
        let retired = host.timing_stats();
        assert_eq!(retired.refused(TimingRefusal::RetiredDevice), 1);
        assert_eq!(retired.published, 12);
        assert_eq!(retired.pending, 0);
        accounted(retired);
        println!("{retired:?}");
    });
}
