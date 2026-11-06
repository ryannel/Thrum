use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
mod gpu_support;
#[cfg(not(target_arch = "wasm32"))]
mod lifecycle_gpu;
#[cfg(not(target_arch = "wasm32"))]
mod recovery_gpu;

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
            host.inner
                .world_mut()
                .inline_mut()
                .world
                .game_mut()
                .sim
                .step(&[])
                .expect("committed world update");
        }
        let after = host.capture_pixels().expect("refreshed capture");
        assert_ne!(
            before, after,
            "moving presentation must use committed tick 100"
        );
        assert_eq!(host.dropped_capture_rows(), 0);
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
            .inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .published_page()
            .expect("baseline lease");
        host.inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .step(&[])
            .expect("first held tick");
        let one = host
            .inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .published_page()
            .expect("first lease");
        host.inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .step(&[])
            .expect("second held tick");
        let two = host
            .inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .published_page()
            .expect("second lease");
        host.inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .step(&[])
            .expect("third held tick");
        let three = host
            .inner
            .world_mut()
            .inline_mut()
            .world
            .game_mut()
            .sim
            .published_page()
            .expect("third lease");

        let now = Duration::from_nanos(16_666_667);
        let busy = host.drive(now, None).expect("busy page is optional");
        assert_eq!((busy.tick, busy.completed_ticks), (4, 1));
        assert!(busy.dropped_view);
        assert!(!busy.submitted);
        assert_eq!(host.dropped_views(), 1);
        host.pause().expect("inline world pause");
        drop(zero);
        assert!(
            thrum_core::sim::WorldGame::publication(
                host.inner.world_mut().inline_mut().world.game_mut()
            )
            .expect("lease-only publication")
            .is_none(),
            "leasing must not recover a page outside the clock's recovery hook"
        );
        let recovered = host.drive(now, None).expect("paused recovery");
        assert_eq!((recovered.tick, recovered.completed_ticks), (4, 0));
        assert!(recovered.submitted);
        assert_eq!(
            host.inner
                .world_mut()
                .inline_mut()
                .world
                .game_mut()
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
