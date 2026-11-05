//! Fixed-clock simulation-to-GPU coverage, including an automatic Metal regression.
#![cfg(not(target_arch = "wasm32"))]

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use thrum_render::{present::SurfaceProfile, timing::GpuTimings};
use v1_game::{FrameHost, FrameSample, SurfaceOutput};

fn await_sample(host: &mut FrameHost, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(sample) = host.poll_completed().expect("GPU progress") {
            return sample;
        }
        assert!(Instant::now() < deadline, "GPU completion timed out");
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
    }
}

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn committed_ticks_drive_latest_gpu_frame_and_overload_is_explicit() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: GpuTimings::requested_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("device request succeeds");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("prepared 10k scene");
        host.enable_projection()
            .expect("bounded same-game projection");
        let idle = host.drive(Duration::ZERO, None).expect("idle callback");
        assert_eq!(idle.completed_ticks, 0);
        assert!(!idle.submitted);
        assert_eq!(host.tick(), 0);
        let first = host
            .drive(Duration::from_nanos(16_666_667), None)
            .expect("first due callback");
        assert_eq!(
            (
                first.tick,
                first.completed_ticks,
                first.queued_ticks,
                first.submitted
            ),
            (1, 1, 0, true)
        );
        let catch_up = host
            .drive(Duration::from_nanos(50_000_001), None)
            .expect("bounded catch-up");
        assert_eq!(
            (
                catch_up.tick,
                catch_up.completed_ticks,
                catch_up.queued_ticks,
                catch_up.submitted,
            ),
            (3, 2, 0, true)
        );
        assert_eq!(host.tick(), 3);
        let mut projected_ticks = Vec::new();
        while host
            .poll_projection(|projection| {
                projected_ticks.push((
                    projection.id().expect("queued source").tick,
                    projection.is_baseline(),
                ));
            })
            .expect("four in-order projections fit the outbox")
            .expect("projection output enabled")
        {}
        assert_eq!(
            projected_ticks,
            [(0, true), (1, false), (2, false), (3, false)]
        );
        assert_eq!(catch_up.projection_lost_ticks, 0);
        let deadline = Instant::now() + Duration::from_secs(10);
        for expected_tick in [1, 3] {
            let sample = loop {
                if let Some(sample) = host.poll_completed().expect("GPU progress") {
                    break sample;
                }
                assert!(Instant::now() < deadline, "GPU completion timed out");
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .expect("GPU wait");
            };
            assert_eq!(sample.tick, expected_tick);
            assert_eq!(sample.uploaded_rows, 1_500);
            assert_eq!(sample.dropped_presentation, 0);
            assert!(!sample.timing_fault);
            assert!(sample.gpu_pass_ns.is_some());
        }
        assert_eq!(host.pending_frames(), 0);
        assert_eq!(host.dropped_views(), 0);
        assert_eq!(
            host.capture_pixels().expect("visible capture").len(),
            640 * 360 * 4
        );
        let overload = host
            .drive(Duration::from_millis(200), None)
            .expect("nine due updates are reported as overload");
        assert_eq!(overload.overload, Some(9));
        assert_eq!(overload.queued_ticks, 0);
        assert_eq!(host.tick(), 3, "overload cannot skip or repeat a tick");
        assert!(host.next_due_in().is_none());

        let (plain_device, plain_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device without timestamp queries");
        let mut plain =
            FrameHost::new(&plain_device, &plain_queue, v1_game::FrameConfig::default())
                .await
                .expect("same prepared scene without timestamps");
        assert!(
            plain
                .drive(Duration::from_nanos(16_666_667), None)
                .expect("plain first tick")
                .submitted
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let plain_sample = loop {
            if let Some(sample) = plain.poll_completed().expect("plain GPU progress") {
                break sample;
            }
            assert!(Instant::now() < deadline, "plain GPU completion timed out");
            plain_device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .expect("plain GPU wait");
        };
        assert_eq!(plain_sample.tick, 1);
        assert_eq!(plain_sample.gpu_pass_ns, None);
        assert!(!plain_sample.timing_fault);
        assert_eq!(plain.pending_frames(), 0);
    });
}

#[test]
#[ignore = "requires a real native GPU adapter for the local InputFrame-to-world route"]
fn local_controls_use_validated_input_and_hold_until_replaced() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("native device");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("prepared game host");
        let initial = host.controlled_position().expect("seeded player");
        host.set_controls([1.0, 0.0, 0.0])
            .expect("normalized local control");
        assert_eq!(
            host.drive(Duration::from_nanos(16_666_667), None)
                .expect("first controlled tick")
                .completed_ticks,
            1
        );
        assert!(host.set_controls([2.0, 0.0, 0.0]).is_err());
        assert_eq!(
            host.drive(Duration::from_nanos(33_333_334), None)
                .expect("held control tick")
                .completed_ticks,
            1
        );
        let moving = host.controlled_position().expect("same player");
        assert!((moving[0] - initial[0] - 0.2).abs() < 1e-9);
        assert_eq!(moving[1], initial[1]);
        host.set_controls([0.0; 3]).expect("release control");
        assert_eq!(
            host.drive(Duration::from_nanos(50_000_001), None)
                .expect("released control tick")
                .completed_ticks,
            1
        );
        assert_eq!(host.controlled_position(), Some(moving));
    });
}

#[test]
#[ignore = "requires Metal; ./dev check runs this regression on macOS"]
fn ready_worker_view_does_not_skip_the_next_committed_tick() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
        {
            Ok(adapter) => adapter,
            Err(error) => {
                eprintln!("SKIP ready-view tick regression: no Metal adapter ({error})");
                return;
            }
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("native device");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("prepared game");
        let (wake, woke) = mpsc::sync_channel(2);
        host.enable_native_worker(move || {
            let _ = wake.try_send(());
        })
        .expect("worker");
        let first_time = Duration::from_nanos(16_666_667);
        let second_time = Duration::from_nanos(33_333_334);
        let first = host.drive(first_time, None).expect("tick one dispatches");
        assert_eq!(first.tick, 1);
        assert!(!first.submitted);
        woke.recv_timeout(Duration::from_secs(2))
            .expect("tick one extract is ready");

        let second = host
            .drive(second_time, None)
            .expect("tick two commits while tick one view is ready");
        assert_eq!((second.tick, second.completed_ticks), (2, 1));
        assert!(second.submitted, "tick one can still be shown");
        woke.recv_timeout(Duration::from_secs(2))
            .expect("tick two extract must be dispatched and ready");

        let third = host
            .drive(second_time, None)
            .expect("tick two view submits without a third world tick");
        assert_eq!((third.tick, third.completed_ticks), (2, 0));
        assert!(third.submitted, "tick two was not silently skipped");
        assert_eq!(await_sample(&mut host, &device).tick, 1);
        assert_eq!(await_sample(&mut host, &device).tick, 2);
    });
}

#[test]
#[ignore = "requires a real native GPU adapter for the worker-to-frame handoff"]
fn native_extract_wake_submits_same_tick_and_a_stalled_worker_does_not_hold_world_ticks() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut host = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("prepared scene");
        let (wake, awakened) = mpsc::sync_channel::<()>(0);
        host.enable_native_worker(move || {
            let _ = wake.send(());
        })
        .expect("native extraction worker");
        let first_time = Duration::from_nanos(16_666_667);
        let first = host.drive(first_time, None).expect("tick and dispatch");
        assert_eq!(first.tick, 1);
        assert!(!first.submitted);
        let deadline = Instant::now() + Duration::from_secs(2);
        let first_submit = loop {
            let report = host.drive(first_time, None).expect("ready wake callback");
            if report.submitted {
                break report;
            }
            assert!(
                Instant::now() < deadline,
                "worker failed to return its page"
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(first_submit.completed_ticks, 0);
        assert_eq!(first_submit.tick, 1);
        assert_eq!(host.tick(), 1);

        let second = host
            .drive(Duration::from_nanos(33_333_334), None)
            .expect("tick while first wake is stalled");
        let third = host
            .drive(Duration::from_nanos(50_000_001), None)
            .expect("next tick while extraction remains queued");
        assert_eq!((second.tick, third.tick), (2, 3));
        assert!(!second.submitted && !third.submitted);
        assert!(third.dropped_view);
        assert_eq!(host.dropped_views(), 1);
        awakened
            .recv_timeout(Duration::from_secs(2))
            .expect("release first wake");
        let deadline = Instant::now() + Duration::from_secs(2);
        let delayed_submit = loop {
            let report = host
                .drive(Duration::from_nanos(50_000_001), None)
                .expect("consume delayed view");
            if report.submitted {
                break report;
            }
            assert!(
                Instant::now() < deadline,
                "queued extraction did not finish"
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(delayed_submit.completed_ticks, 0);
        assert_eq!(host.tick(), 3);
        awakened
            .recv_timeout(Duration::from_secs(2))
            .expect("release second wake");
        assert_eq!(host.dropped_stale_views(), 0);
        let native_pixels = host.capture_pixels().expect("native world capture");
        awakened
            .recv_timeout(Duration::from_secs(2))
            .expect("release the latest worker wake before teardown");
        let mut serial = FrameHost::new(&device, &queue, v1_game::FrameConfig::default())
            .await
            .expect("same serial fixture");
        serial
            .drive(Duration::from_nanos(50_000_001), None)
            .expect("serial world reaches tick three");
        assert_eq!(serial.tick(), host.tick());
        assert_eq!(
            native_pixels,
            serial.capture_pixels().expect("serial world capture"),
            "worker extraction cannot change the authoritative or visual result"
        );
    });
}

#[test]
#[ignore = "requires a real native GPU adapter for stale-view refresh"]
fn native_view_beyond_two_ticks_refreshes_from_the_latest_committed_world() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let extent = [640, 360];
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("stale-view presentation target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut host = FrameHost::new(
            &device,
            &queue,
            v1_game::FrameConfig {
                surface: Some(SurfaceProfile { format, extent }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared scene and presenter");
        host.enable_native_worker(|| {}).expect("worker");
        let first_time = Duration::from_nanos(16_666_667);
        host.drive(first_time, None).expect("dispatch first view");
        let deadline = Instant::now() + Duration::from_secs(2);
        while !host.has_ready_view() {
            host.drive(first_time, None).expect("poll first view");
            assert!(Instant::now() < deadline, "worker did not return");
            std::thread::sleep(Duration::from_millis(1));
        }
        for tick in 2..=4 {
            let report = host
                .drive(Duration::from_nanos(tick * 16_666_667), None)
                .expect("world progresses without a surface");
            assert_eq!(report.tick, tick);
            assert!(report.dropped_view);
        }
        let report = host
            .drive(
                Duration::from_nanos(4 * 16_666_667),
                Some(&SurfaceOutput {
                    view: &view,
                    extent,
                }),
            )
            .expect("stale view refreshes before presentation");
        assert_eq!(report.completed_ticks, 0);
        assert!(report.submitted && report.surface_presented);
        assert_eq!(host.dropped_stale_views(), 1);
        let deadline = Instant::now() + Duration::from_secs(10);
        let sample = loop {
            if let Some(sample) = host.poll_completed().expect("GPU progress") {
                break sample;
            }
            assert!(Instant::now() < deadline, "GPU completion timed out");
            device.poll(wgpu::PollType::Poll).expect("GPU poll");
        };
        assert_eq!(sample.tick, 4);
        assert_eq!(sample.visual_age_ticks, 0);
    });
}

#[test]
#[ignore = "requires a real native GPU adapter for persistent slow-worker presentation"]
fn slow_native_extraction_keeps_presenting_the_last_scene_with_explicit_age() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: GpuTimings::requested_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("device");
        let extent = [640, 360];
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("slow-worker presentation target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent,
        };
        let mut host = FrameHost::new(
            &device,
            &queue,
            v1_game::FrameConfig {
                surface: Some(SurfaceProfile { format, extent }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared scene and presenter");
        let (wake, awakened) = mpsc::sync_channel::<()>(0);
        host.enable_native_worker(move || {
            let _ = wake.send(());
        })
        .expect("worker");

        let first_time = Duration::from_nanos(16_666_667);
        host.drive(first_time, None).expect("first tick dispatches");
        let deadline = Instant::now() + Duration::from_secs(2);
        while !host.has_ready_view() {
            host.drive(first_time, None).expect("poll worker result");
            assert!(Instant::now() < deadline, "first extraction did not finish");
            std::thread::sleep(Duration::from_millis(1));
        }
        let first = host
            .drive(first_time, Some(&output))
            .expect("first fresh surface frame");
        assert!(first.submitted && first.surface_presented);
        assert!(!first.dropped_view);
        let first_sample = await_sample(&mut host, &device);
        assert_eq!(first_sample.tick, 1);
        assert_eq!(first_sample.visual_age_ticks, 0);
        assert!(!first_sample.reused_visual);
        host.drive(Duration::from_nanos(33_333_334), None)
            .expect("second extraction queues behind stalled callback");

        for tick in 3_u64..=602 {
            let now = Duration::from_nanos(tick * 16_666_667);
            let world = host
                .drive(now, None)
                .expect("world ticks while worker is stalled");
            assert_eq!(world.tick, tick);
            assert!(world.dropped_view);
            assert!(host.has_reusable_view());
            let fallback = host
                .drive(now, Some(&output))
                .expect("reuse prepared scene on a zero-tick callback");
            assert_eq!(fallback.completed_ticks, 0);
            assert!(fallback.submitted && fallback.surface_presented);
            assert_eq!(host.reused_views(), tick - 2);
            assert_eq!(host.tick(), tick);
            let sample = await_sample(&mut host, &device);
            assert_eq!(sample.tick, 1);
            assert_eq!(
                sample.visual_age_ticks,
                u32::try_from(tick - 1).expect("small age")
            );
            assert!(sample.reused_visual);
            assert!(sample.surface_presented);
            assert_eq!(sample.uploaded_rows, 0);
            assert!(!sample.timing_fault);
            if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                assert_eq!(sample.gpu_pass_ns, Some([0.0; 5]));
                assert!(sample.gpu_present_ns.is_some());
            }
        }
        assert_eq!(host.dropped_views(), 600);
        assert_eq!(host.reused_views(), 600);
        awakened
            .recv_timeout(Duration::from_secs(2))
            .expect("release first completion callback");
        let now = Duration::from_nanos(602 * 16_666_667);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !host.has_ready_view() {
            host.drive(now, None).expect("poll delayed worker result");
            assert!(
                Instant::now() < deadline,
                "delayed extraction did not return"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let recovered = host
            .drive(now, Some(&output))
            .expect("over-horizon worker result refreshes latest world");
        assert!(recovered.submitted && recovered.surface_presented);
        assert_eq!(host.dropped_stale_views(), 1);
        let sample = await_sample(&mut host, &device);
        assert_eq!(sample.tick, 602);
        assert_eq!(sample.visual_age_ticks, 0);
        assert!(!sample.reused_visual);
        awakened
            .recv_timeout(Duration::from_secs(2))
            .expect("release second completion callback");
    });
}
