use super::*;

fn retire(host: &mut Host) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while host.pending_frames() != 0 {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(!timeout.is_zero(), "extent frames did not retire");
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .expect("complete frames");
        while host.poll_completed().expect("completion").is_some() {}
    }
}

fn verify(host: &mut Host, extent: [u32; 2]) {
    let encoded = host
        .advance(Duration::ZERO)
        .expect("advance")
        .encode_scene()
        .expect("encode");
    let frame = encoded.frame_extent().expect("actual frame");
    assert_eq!(frame.attachments, [extent; 4]);
    assert_eq!(frame.source, extent);
    assert_eq!(frame.destination, extent);
    assert_eq!(frame.viewport, [0, 0, extent[0], extent[1]]);
    // Policy's forward-facing camera gives a diagonal projection.
    assert!(
        (frame.view_projection[1][1] / frame.view_projection[0][0]
            - extent[0] as f32 / extent[1] as f32)
            .abs()
            < 1e-5
    );
    assert!((frame.view_projection[1][1] - 1.0 / 0.5_f32.tan()).abs() < 1e-5);
    encoded.submit(None).expect("submit");
    assert!(host.scene.view.history_reset());
    retire(host);
    assert!(!host.history_reset_pending);
}

#[test]
fn coupled_extent_changes_real_targets_projection_and_history_atomically() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let device = host.device.clone();
        let world = host.world.source();
        let mut stale_visual = host
            .visual_prepare_inputs()
            .prepare(
                FrameLighting::Diagnostic(SyntheticLight::default()),
                0.125,
                None,
            )
            .await
            .expect("old-grid visual");
        for extent in [[48, 96], [96, 48], [48, 96]] {
            assert!(host.resize_scene_surface(extent).expect("coupled resize"));
            assert!(host.history_reset_pending);
            let target = host.slots[0].target.output_view().clone();
            let revision = host.presenter_generation;
            assert!(host.resize_scene_surface(extent).expect("same extent"));
            assert_eq!(
                host.slots[0].target.output_view(),
                &target,
                "same grid reuses resources"
            );
            assert_eq!(host.presenter_generation, revision);
            verify(&mut host, extent);
            assert_eq!(host.world.source(), world);
            assert_eq!(host.device, device);
            assert!(host.prepared.capabilities().matches(&device));
        }
        assert!(
            host.publish_visual(&mut stale_visual).is_err(),
            "old-size candidates cannot publish"
        );
        let target = host.slots[0].target.output_view().clone();
        assert!(!host.resize_scene_surface([0, 96]).expect("zero defers"));
        assert!(host.resize_scene_surface([u32::MAX, 96]).is_err());
        assert!(
            !host
                .resize_scene_surface_if([96, 48], || false)
                .expect("stale preparation")
        );
        assert_eq!(host.render_extent, [48, 96]);
        assert_eq!(host.presenter.as_ref().unwrap().extent(), [48, 96]);
        assert_eq!(host.slots[0].target.output_view(), &target);
        assert!(
            host.advance(Duration::ZERO)
                .expect("pending advance")
                .encode_scene()
                .expect("pending encode")
                .submit(None)
                .expect("pending frame")
                .submitted
        );
        assert!(host.pending_frames() > 0);
        assert!(!host.resize_scene_surface([96, 48]).expect("busy defers"));
        assert_eq!(host.slots[0].target.output_view(), &target);
        retire(&mut host);
        assert!(
            host.resize_scene_surface([96, 48])
                .expect("retry after completion")
        );
        verify(&mut host, [96, 48]);
        host.resize_surface([48, 96])
            .expect("fixed-render surface-only caller");
        assert_eq!(host.render_extent, [96, 48]);
        assert_eq!(host.slots[0].target.attachment_extents(), [[96, 48]; 4]);
        assert_eq!(
            host.presenter.as_ref().unwrap().source_viewport(),
            ([96, 48], [0, 36, 48, 24])
        );
    });
}

#[test]
fn detached_device_refits_scene_and_presenter_and_refreshes_current_camera() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (first, queue) = gpu(&instance, 7).await;
        let mut host = host(&first, &queue, Policy::default()).await;
        let (second, queue) = gpu(&instance, 8).await;
        let mut candidate = host
            .device_prepare_inputs()
            .expect("inputs")
            .prepare(&second, &queue, Some(surface()))
            .await
            .expect("candidate");
        assert!(candidate.resize_scene_surface([0, 96]).is_err());
        assert!(candidate.resize_scene_surface([u32::MAX, 96]).is_err());
        candidate
            .resize_scene_surface([96, 48])
            .expect("landscape refit");
        candidate
            .resize_scene_surface([96, 48])
            .expect("same candidate grid");
        host.publish_device(&mut candidate).expect("publication");
        assert_eq!(host.render_extent, [96, 48]);
        assert!(host.history_reset_pending);
        verify(&mut host, [96, 48]);
        assert!(host.publish_device(&mut candidate).is_err());
        assert!(candidate.resize_scene_surface([48, 96]).is_err());
    });
}
