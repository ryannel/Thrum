//! Fixed-clock simulation-to-GPU coverage, including an automatic Metal regression.
#![cfg(not(target_arch = "wasm32"))]

use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use thrum_render::{capabilities::DeviceCapabilities, present::SurfaceProfile};
use v1_game::{FrameConfig, FrameHost, FrameSample, SurfaceOutput};

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

fn write_ppm(path: impl AsRef<std::path::Path>, rgba: &[[u8; 4]], extent: [u32; 2]) {
    let mut ppm = Vec::with_capacity(rgba.len() * 3 + 32);
    ppm.extend_from_slice(format!("P6\n{} {}\n255\n", extent[0], extent[1]).as_bytes());
    for pixel in rgba {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).expect("write explicit visual capture");
}

#[test]
fn game_atmosphere_capture_has_visible_finite_pixels() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real Metal adapter required for game atmosphere capture");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
            .await
            .expect("prepared game scene");
        let pixels = host.capture_pixels().expect("game atmosphere pixels");
        assert_eq!(pixels.len(), 640 * 360 * 4);
        let (rgba, remainder) = pixels.as_chunks::<4>();
        assert_eq!(remainder, []);
        assert!(rgba.iter().all(|pixel| pixel[3] == 255));
        assert!(rgba.iter().any(|pixel| pixel[..3] != [0; 3]));
        if let Some(path) = std::env::var_os("THRUM_V1_CAPTURE_PPM") {
            write_ppm(path, rgba, [640, 360]);
        }
        assert!(host.set_time_of_day(f32::NAN).is_err());
        assert!(host.set_camera_pitch(f32::NAN).is_err());
        assert!(host.set_camera_pitch(0.7).is_err());
        host.set_time_of_day(0.25).expect("sunrise control");
        let sunrise = host.capture_pixels().expect("sunrise pixels");
        let changed = rgba
            .iter()
            .zip(sunrise.as_chunks::<4>().0)
            .filter(|(noon, sunrise)| noon[..3] != sunrise[..3])
            .count();
        assert!(
            changed > 640 * 360 / 10,
            "time control changes the game image"
        );
        if let Some(directory) = std::env::var_os("THRUM_V1_CAPTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("capture directory");
            for (name, time) in [
                ("midnight", 0.0),
                ("sunrise", 0.25),
                ("noon", 0.5),
                ("sunset", 0.75),
                ("twilight", 0.77),
            ] {
                host.set_time_of_day(time).expect("fixed solar time");
                let pixels = host.capture_pixels().expect("fixed game camera");
                write_ppm(
                    directory.join(format!("{name}.ppm")),
                    pixels.as_chunks::<4>().0,
                    [640, 360],
                );
            }
            host.set_time_of_day(0.5).expect("Sun-facing noon");
            host.set_camera_pitch(0.35).expect("Sun-facing view");
            let pixels = host.capture_pixels().expect("Sun-facing camera");
            write_ppm(
                directory.join("sun-facing.ppm"),
                pixels.as_chunks::<4>().0,
                [640, 360],
            );
            let mut full = FrameHost::new(
                &device,
                &queue,
                FrameConfig {
                    render_extent: [1920, 1080],
                    surface: None,
                },
            )
            .await
            .expect("prepared full-resolution game scene");
            full.set_time_of_day(0.5).expect("full-resolution noon");
            full.set_camera_pitch(0.35)
                .expect("full-resolution Sun view");
            let pixels = full.capture_pixels().expect("full-resolution Sun capture");
            write_ppm(
                directory.join("sun-facing-1080p.ppm"),
                pixels.as_chunks::<4>().0,
                [1920, 1080],
            );
            host.set_camera_pitch(0.0).expect("level sweep camera");
            for sample in 0..96 {
                host.set_time_of_day(sample as f32 / 96.0)
                    .expect("sweep time");
                let pixels = host.capture_pixels().expect("sweep camera");
                write_ppm(
                    directory.join(format!("sweep_{sample:03}.ppm")),
                    pixels.as_chunks::<4>().0,
                    [640, 360],
                );
            }
        }
    });
}

#[test]
fn repeated_display_target_reuses_world_columns_but_advances_interpolation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real Metal adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let extent = [320, 180];
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("repeat display target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
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
        let output = SurfaceOutput {
            view: &view,
            extent,
        };
        let capabilities =
            DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig {
                render_extent: extent,
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
            },
        )
        .await
        .expect("prepared V1 scene");
        let first = host
            .drive(Duration::from_nanos(16_666_667), Some(&output))
            .expect("first tick and display target");
        assert!(first.submitted);
        let first = await_sample(&mut host, &device);
        let second = host
            .drive(Duration::from_millis(25), Some(&output))
            .expect("same world tick at later display target");
        assert!(second.submitted);
        let second = await_sample(&mut host, &device);
        assert_eq!((first.tick, second.tick), (1, 1));
        assert_eq!(second.completed_ticks, 0);
        assert_eq!(second.uploaded_rows, 0);
        assert_eq!(second.moving, first.moving);
        assert!((second.presentation_alpha - 0.5).abs() < 1e-4);
        assert!(second.surface_presented);
    });
}

#[test]
fn autonomous_world_page_drives_the_existing_gpu_scene() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real native adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let host = FrameHost::new(&device, &queue, FrameConfig::default())
            .await
            .expect("prepared scene");
        let origin = Instant::now();
        let (mut render, controls) = host
            .into_threaded(origin, || {})
            .expect("one autonomous world");
        controls
            .set_controls([1.0, 0.0, 0.0])
            .expect("validated input");
        std::thread::sleep(Duration::from_millis(85));
        let world_deadline = Instant::now() + Duration::from_secs(2);
        let report = loop {
            let report = render
                .drive(origin.elapsed(), None)
                .expect("render latest world page");
            if report.submitted {
                break report;
            }
            assert!(
                Instant::now() < world_deadline,
                "world page did not reach GPU"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(report.tick >= 1, "world advanced while GPU host was idle");
        let deadline = Instant::now() + Duration::from_secs(10);
        let sample = loop {
            if let Some(sample) = render.poll_completed().expect("GPU progress") {
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
        assert_eq!(sample.tick, report.tick);
        assert_eq!(sample.uploaded_rows, 1_501);
        let next = render
            .drive(origin.elapsed(), None)
            .expect("next independent world observation");
        assert_eq!(
            u64::from(next.completed_ticks),
            next.tick - report.tick,
            "each autonomous world tick is reported once across render callbacks"
        );
    });
}

#[test]
fn committed_ticks_drive_latest_gpu_frame_and_overload_is_explicit() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("device request succeeds");
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
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
            // Each new world tick uploads mover endpoints once; alpha zero
            // selects the previous endpoint on the GPU without another upload.
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
        let mut plain = FrameHost::new(&plain_device, &plain_queue, FrameConfig::default())
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
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
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
#[cfg(target_os = "macos")]
fn ready_worker_view_does_not_skip_the_next_committed_tick() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter required for ready-view tick regression");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("native device");
        let capabilities =
            DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
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
            .drive(second_time + Duration::from_nanos(8_333_333), None)
            .expect("tick two view submits without a third world tick");
        assert_eq!((third.tick, third.completed_ticks), (2, 0));
        assert!(third.submitted, "tick two was not silently skipped");
        assert_eq!(await_sample(&mut host, &device).tick, 1);
        let second_sample = await_sample(&mut host, &device);
        assert_eq!(second_sample.tick, 2);
        assert!(
            (second_sample.presentation_alpha - 0.5).abs() < 0.001,
            "native presentation must use current clock alpha: {second_sample:?}"
        );
    });
}

#[test]
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
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
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
        let mut serial = FrameHost::new(&device, &queue, FrameConfig::default())
            .await
            .expect("same serial fixture");
        serial
            .drive(Duration::from_nanos(50_000_001), None)
            .expect("serial world reaches tick three");
        assert_eq!(serial.tick(), host.tick());
        let serial_pixels = serial.capture_pixels().expect("serial world capture");
        let first_difference = native_pixels
            .iter()
            .zip(&serial_pixels)
            .position(|(native, serial)| native != serial);
        assert_eq!(
            first_difference, None,
            "worker extraction cannot change the authoritative or visual result; first byte differs at {first_difference:?}"
        );
    });
}

#[test]
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
            FrameConfig {
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
fn slow_native_extraction_keeps_presenting_the_last_scene_with_explicit_age() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
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
            FrameConfig {
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
                assert_eq!(sample.gpu_pass_ns, Some([0.0; 11]));
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
