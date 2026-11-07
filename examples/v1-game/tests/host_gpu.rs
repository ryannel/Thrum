//! Fixed-clock simulation-to-GPU coverage, including an automatic Metal regression.
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

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
        let capture_directory =
            std::env::var_os("THRUM_V1_CAPTURE_DIR").map(std::path::PathBuf::from);
        if let Some(directory) = &capture_directory {
            std::fs::create_dir_all(directory).expect("capture directory");
        }
        let pixels = host.capture_pixels().expect("game atmosphere pixels");
        assert_eq!(pixels.len(), 640 * 360 * 4);
        let (rgba, remainder) = pixels.as_chunks::<4>();
        assert_eq!(remainder, [0_u8; 0]);
        assert!(rgba.iter().all(|pixel| pixel[3] == 255));
        assert!(rgba.iter().any(|pixel| pixel[..3] != [0; 3]));
        if let Some(path) = std::env::var_os("THRUM_V1_CAPTURE_PPM") {
            write_ppm(path, rgba, [640, 360]);
        }
        assert!(host.set_time_of_day(f32::NAN).is_err());
        assert!(host.set_camera_pitch(f32::NAN).is_err());
        assert!(host.set_camera_pitch(0.7).is_err());
        assert!(host.set_camera_yaw(f32::NAN).is_err());
        assert!(host.set_camera_yaw(4.0).is_err());
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
        host.set_time_of_day(0.77).expect("twilight control");
        let twilight = host.capture_pixels().expect("adapted twilight pixels");
        let sky = &twilight[(80 * 640 + 20) * 4..][..3];
        let brightness = sky.iter().map(|channel| u16::from(*channel)).sum::<u16>();
        assert!(
            brightness >= 80,
            "authored twilight camera must retain visible sky colour: {sky:?}"
        );
        for (label, horizon, direction) in [("dusk", 0.75_f32, 1.0), ("dawn", 0.25, -1.0)] {
            for step in 0..=16 {
                let time = horizon + direction * step as f32 * 0.001_875;
                host.set_time_of_day(time).expect("fine solar step");
                let pixels = host.capture_pixels().expect("fine solar pixels");
                let sky = &pixels[(80 * 640 + 20) * 4..][..3];
                let brightness = sky.iter().map(|channel| u16::from(*channel)).sum::<u16>();
                if let Some(directory) = &capture_directory {
                    write_ppm(
                        directory.join(format!("{label}_{step:02}.ppm")),
                        pixels.as_chunks::<4>().0,
                        [640, 360],
                    );
                }
                // A display sample advances by one of the game's 18,000
                // ticks/day, not the 33.75 ticks between these probe locations.
                host.set_time_of_day(time + direction / 18_000.0)
                    .expect("adjacent game tick");
                let adjacent = host.capture_pixels().expect("adjacent solar pixels");
                let next = adjacent[(80 * 640 + 20) * 4..][..3]
                    .iter()
                    .map(|channel| u16::from(*channel))
                    .sum::<u16>();
                assert!(
                    brightness.abs_diff(next) <= 8,
                    "{label} camera flashed over one tick at {time}: before={brightness}, after={next}"
                );
            }
        }
        if let Some(directory) = capture_directory {
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
            full.set_camera_yaw(-std::f32::consts::FRAC_PI_2)
                .expect("face the setting Sun");
            full.set_camera_pitch(0.0).expect("sunset horizon");
            full.set_time_of_day(0.745).expect("Sun above horizon");
            let pixels = full.capture_pixels().expect("full-resolution sunset");
            write_ppm(
                directory.join("sunset-sun-1080p.ppm"),
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
            host.set_camera_yaw(-std::f32::consts::FRAC_PI_2)
                .expect("Sun-facing solar sequence");
            for (name, time) in [("sunset-sun", 0.745), ("twilight-sun", 0.77)] {
                host.set_time_of_day(time).expect("Sun-facing solar time");
                let pixels = host.capture_pixels().expect("Sun-facing solar view");
                write_ppm(
                    directory.join(format!("{name}.ppm")),
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
        let initial = host.controlled_position().expect("seeded player");
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
        let moved = render
            .controlled_position()
            .expect("published controlled actor");
        assert!(
            moved[0] > initial[0],
            "the admitted axis moved the committed actor"
        );
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
        let pixels = host.capture_pixels().expect("visible capture");
        assert_eq!(pixels.len(), 640 * 360 * 4);
        if let Some(path) = std::env::var_os("THRUM_V1_DYNAMIC_CAPTURE_PPM") {
            write_ppm(path, pixels.as_chunks::<4>().0, [640, 360]);
        }
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
