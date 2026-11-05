use super::{FrameSample, IN_FLIGHT, pop_report_before_pending, retain_sample};
use std::{collections::VecDeque, time::Duration};

fn sample(tick: u64) -> FrameSample {
    FrameSample {
        tick,
        presentation_alpha: 1.0,
        submission_order: tick,
        completed_ticks: 1,
        queued_ticks: 0,
        tick_cpu: Duration::ZERO,
        extract_cpu: Duration::ZERO,
        worker_queue_ns: 0,
        worker_ready_ns: 0,
        visual_age_ticks: 0,
        reused_visual: false,
        upload_cpu: Duration::ZERO,
        view_update_ns: 0,
        encode_cpu: Duration::ZERO,
        wall: Duration::ZERO,
        render_cpu: Duration::ZERO,
        gpu_pass_ns: None,
        gpu_present_ns: None,
        surface_presented: false,
        timing_fault: false,
        uploaded_rows: 0,
        uploaded_bytes: 0,
        moving: 0,
        dropped_presentation: 0,
        dropped_light_references: 0,
        clamped_light_channels: 0,
        clamped_shadow_casters: 0,
        visual_revision: None,
    }
}

#[test]
fn reused_source_ticks_do_not_hide_earlier_completed_submissions() {
    let mut queue = VecDeque::with_capacity(IN_FLIGHT);
    let mut dropped = 0;
    let mut first = sample(1);
    first.submission_order = 7;
    let mut second = sample(1);
    second.submission_order = 8;
    retain_sample(&mut queue, &mut dropped, first);
    assert_eq!(
        pop_report_before_pending(&mut queue, Some(second.submission_order))
            .expect("earlier completed submission")
            .submission_order,
        first.submission_order
    );
    assert!(queue.is_empty());
    retain_sample(&mut queue, &mut dropped, second);
    assert!(pop_report_before_pending(&mut queue, Some(7)).is_none());
    assert_eq!(
        pop_report_before_pending(&mut queue, None)
            .expect("remaining submission")
            .submission_order,
        second.submission_order
    );
    assert_eq!(dropped, 0);
}

#[test]
fn full_report_queue_drops_the_oldest_tick_even_when_it_finishes_late() {
    assert_eq!(IN_FLIGHT, 2);
    let mut queue = VecDeque::with_capacity(IN_FLIGHT);
    let mut dropped = 0;
    for tick in [2, 3] {
        retain_sample(&mut queue, &mut dropped, sample(tick));
    }
    retain_sample(&mut queue, &mut dropped, sample(1));
    assert_eq!(dropped, 1);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [2, 3]
    );
    retain_sample(&mut queue, &mut dropped, sample(4));
    assert_eq!(dropped, 2);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [3, 4]
    );
}

#[test]
fn capture_refreshes_the_latest_committed_world_after_an_unsent_view() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let before = host.capture_pixels().expect("initial capture");
        for _ in 0..100 {
            host.world
                .inline_mut()
                .sim
                .step(&[])
                .expect("committed world update");
        }
        assert_ne!(
            host.extractor
                .as_ref()
                .expect("serial extractor")
                .frame()
                .source()
                .map(|source| source.tick),
            Some(100)
        );
        let after = host.capture_pixels().expect("refreshed capture");
        assert_ne!(
            before, after,
            "moving presentation must use committed tick 100"
        );
        assert_eq!(
            host.extractor
                .as_ref()
                .expect("serial extractor")
                .frame()
                .source()
                .map(|source| source.tick),
            Some(100)
        );
        assert_eq!(host.dropped_capture_rows(), 0);
    });
}

#[test]
fn capture_applies_a_reclaimed_worker_view_even_when_its_tick_is_current() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let before = host.scene.instances[0].current[3][0];
        host.enable_native_worker(|| {}).expect("worker");
        let dispatch = host
            .drive(Duration::from_nanos(16_666_667), None)
            .expect("committed tick dispatches extraction");
        assert_eq!(dispatch.tick, 1);
        assert!(!dispatch.submitted);
        host.capture_pixels().expect("capture waits and refreshes");
        let after = host.scene.instances[0].current[3][0];
        assert_ne!(
            before, after,
            "same-tick worker columns reach the GPU scene"
        );
    });
}

#[test]
fn held_publication_drops_one_view_then_recovers_on_a_paused_zero_tick_callback() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let zero = host
            .world
            .inline_mut()
            .sim
            .published_page()
            .expect("baseline lease");
        host.world
            .inline_mut()
            .sim
            .step(&[])
            .expect("first held tick");
        let one = host
            .world
            .inline_mut()
            .sim
            .published_page()
            .expect("first lease");
        host.world
            .inline_mut()
            .sim
            .step(&[])
            .expect("second held tick");
        let two = host
            .world
            .inline_mut()
            .sim
            .published_page()
            .expect("second lease");
        host.world
            .inline_mut()
            .sim
            .step(&[])
            .expect("third held tick");
        let three = host
            .world
            .inline_mut()
            .sim
            .published_page()
            .expect("third lease");

        let now = Duration::from_nanos(16_666_667);
        let busy = host.drive(now, None).expect("busy page is optional");
        assert_eq!((busy.tick, busy.completed_ticks), (4, 1));
        assert!(busy.dropped_view);
        assert!(!busy.submitted);
        assert_eq!(host.dropped_views(), 1);
        host.pause();
        drop(zero);
        let recovered = host.drive(now, None).expect("paused recovery");
        assert_eq!((recovered.tick, recovered.completed_ticks), (4, 0));
        assert!(recovered.submitted);
        assert_eq!(
            host.world
                .inline_mut()
                .sim
                .published_page()
                .expect("current page")
                .id()
                .tick,
            4
        );
        assert_eq!((one.id().tick, two.id().tick, three.id().tick), (1, 2, 3));
    });
}

#[test]
fn worker_result_from_a_retired_visual_generation_resets_pose_history() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        host.enable_native_worker(|| {}).expect("worker");
        let now = Duration::from_nanos(16_666_667);
        host.drive(now, None).expect("dispatch first visual page");
        host.visual_generation += 1;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let report = host.drive(now, None).expect("retired result handled");
            if report.submitted {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not return"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(host.dropped_stale_views(), 1);
        assert!(
            host.extractor
                .as_ref()
                .expect("returned extractor")
                .frame()
                .history_valid()
                .iter()
                .all(|valid| !valid),
            "retired generation cannot carry prior poses into the replacement"
        );
    });
}

#[test]
fn worker_result_from_an_old_timeline_is_replaced_after_restore() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let capture = host.world.inline_mut().sim.capture();
        host.enable_native_worker(|| {}).expect("worker");
        let now = Duration::from_nanos(16_666_667);
        let dispatched = host.drive(now, None).expect("dispatch old timeline");
        assert_eq!((dispatched.tick, dispatched.completed_ticks), (1, 1));
        host.world
            .inline_mut()
            .sim
            .restore(&capture)
            .expect("restore tick zero");
        assert_eq!(host.world.inline_mut().sim.tick(), 0);
        assert_eq!(host.world.inline_mut().sim.timeline(), 1);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let report = host.drive(now, None).expect("replace stale worker result");
            if report.submitted {
                assert_eq!(report.tick, 0);
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not return"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(host.dropped_stale_views(), 1);
        assert_eq!(host.last_view_source.map(|source| source.timeline), Some(1));
        assert_eq!(host.last_view_source.map(|source| source.tick), Some(0));
    });
}

#[test]
fn stale_frame_source_drops_the_view_without_touching_the_scene() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let original = host.scene.instances[0].current;
        let capture = host.world.inline_mut().sim.capture();
        host.world
            .inline_mut()
            .sim
            .step(&[])
            .expect("advance world");
        host.world
            .inline_mut()
            .sim
            .restore(&capture)
            .expect("replay same tick");
        let mut report = super::DriveReport {
            tick: 0,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            submitted: false,
            surface_presented: false,
            dropped_view: false,
            wall: Duration::ZERO,
        };
        host.submit_extracted(
            0,
            &mut report,
            None,
            super::SubmitFrame {
                extracted: thrum_render::extract::ExtractReport::default(),
                reused_source: false,
                extract_cpu: Duration::ZERO,
                worker_queue_cpu: Duration::ZERO,
                worker_ready_cpu: Duration::ZERO,
                tick_cpu: Duration::ZERO,
                completed_ticks: 0,
                queued_ticks: 0,
                frame_tick: 0,
                presentation_alpha: 1.0,
            },
            std::time::Instant::now(),
        )
        .expect("stale presentation degrades");
        assert!(report.dropped_view);
        assert!(!report.submitted);
        assert_eq!(host.dropped_stale_views(), 1);
        assert_eq!(host.scene.instances[0].current, original);
        assert_eq!(host.pending_frames(), 0);
    });
}

#[test]
fn abandoned_advanced_frame_keeps_its_tick_and_counts_the_skipped_view() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        let advanced = host
            .advance(Duration::from_nanos(16_666_667))
            .expect("admit tick before presentation");
        drop(advanced);
        assert_eq!(host.tick(), 1);
        assert_eq!(host.dropped_views(), 1);
        assert_eq!(host.pending_frames(), 0);
        let next = host
            .drive(Duration::from_nanos(33_333_334), None)
            .expect("next tick still advances and presents");
        assert_eq!((next.tick, next.completed_ticks), (2, 1));
        assert!(next.submitted);
    });
}

#[test]
fn abandoned_encoded_scene_reuses_slot_and_timestamp_batch() {
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
            .expect("device");
        let extent = [640, 360];
        let mut host = super::FrameHost::new(
            &device,
            &queue,
            super::FrameConfig {
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared scene");
        for tick in 1..=64 {
            let encoded = host
                .advance(Duration::from_nanos(tick * 16_666_667))
                .expect("admitted tick")
                .encode_scene()
                .expect("scene encoded before acquisition");
            let report = encoded.skip();
            assert_eq!(report.tick, tick);
            assert!(report.dropped_view && !report.submitted);
            assert_eq!(host.pending_frames(), 0);
        }
        assert_eq!((host.tick(), host.dropped_views()), (64, 64));
        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("late surface test target"),
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
        let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
        let report = host
            .advance(Duration::from_nanos(65 * 16_666_667))
            .expect("next tick admitted")
            .encode_scene()
            .expect("timestamp labels reusable")
            .submit(Some(&super::SurfaceOutput {
                view: &view,
                extent,
            }))
            .expect("scene and surface submit once");
        assert!(report.submitted && report.surface_presented);
        assert_eq!(host.tick(), 65);
        let between = host
            .advance(Duration::from_nanos(65 * 16_666_667 + 8_333_333))
            .expect("display callback between world ticks");
        assert!(between.can_acquire_serial());
        let report = between
            .encode_scene()
            .expect("same committed page has another display sample")
            .submit(Some(&super::SurfaceOutput {
                view: &view,
                extent,
            }))
            .expect("second display image submits");
        assert_eq!((report.tick, report.completed_ticks), (65, 0));
        assert!(report.submitted && report.surface_presented);
    });
}

#[test]
fn capture_after_abandoned_fractional_view_uses_last_submitted_sample() {
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
        let extent = [640, 360];
        let mut host = super::FrameHost::new(
            &device,
            &queue,
            super::FrameConfig {
                surface: Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent,
                }),
                ..Default::default()
            },
        )
        .await
        .expect("prepared scene");
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fractional capture surface"),
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
        host.advance(Duration::from_nanos(16_666_667))
            .expect("first tick")
            .encode_scene()
            .expect("first view")
            .submit(Some(&super::SurfaceOutput {
                view: &view,
                extent,
            }))
            .expect("first submit");
        let baseline = host
            .capture_pixels()
            .expect("last submitted sample capture");
        host.advance(Duration::from_millis(25))
            .expect("half tick")
            .encode_scene()
            .expect("fractional view")
            .skip();
        let after = host
            .capture_pixels()
            .expect("capture restores submitted sample");
        assert_eq!(after, baseline);
    });
}

#[test]
fn abandoned_overload_frame_counts_its_completed_queued_ticks() {
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
        let mut host = super::FrameHost::new(&device, &queue, super::FrameConfig::default())
            .await
            .expect("prepared scene");
        drop(
            host.advance(Duration::from_nanos(133_333_334))
                .expect("eight due, four completed"),
        );
        assert_eq!((host.tick(), host.dropped_views()), (4, 1));
        drop(
            host.advance(Duration::from_nanos(233_333_334))
                .expect("new work refused but queued ticks complete"),
        );
        assert_eq!((host.tick(), host.dropped_views()), (8, 2));
    });
}
