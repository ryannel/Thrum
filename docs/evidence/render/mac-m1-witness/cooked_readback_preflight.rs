
#[test]
fn cooked_noon_readback_matches_after_device_destroy() {
    pollster::block_on(async {
        let output = std::path::PathBuf::from(
            std::env::var_os("THRUM_M1_READBACK_DIR").expect("fresh scratch output"),
        );
        std::fs::create_dir(&output).expect("new scratch directory");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let extent = [1920, 1080];
        let profile = SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent,
        };
        let (old, old_queue, old_caps) = cooked_readback_device(&instance, 1).await;
        let mut host = FrameHost::new_with_setup(
            &old,
            &old_queue,
            &old_caps,
            FrameConfig {
                render_extent: extent,
                surface: Some(profile),
            },
            crate::selected_setup().expect("valley setup"),
        )
        .await
        .expect("cooked valley host");
        let cooked = crate::attach_cooked_scene(&mut host)
            .expect("mount the immutable cooked valley")
            .expect("cooked stats");
        host.pause().expect("freeze the initial endpoint");
        host.set_time_of_day(0.5).expect("noon");
        host.set_camera_yaw(0.0).expect("noon yaw");
        host.set_camera_pitch(0.0).expect("noon pitch");
        let (mut threaded, controls) = host
            .into_threaded(Instant::now(), || {})
            .expect("autonomous world with the frozen seed");
        let pause = transition(&mut threaded, &controls, &old, WorldLifecycle::Pause);
        assert_eq!(pause.id.tick, 0);
        let mut host = threaded.inner;

        let generation = cooked_readback_resident(&mut host, &cooked, 0);
        let tick = host.tick();
        let control = host.capture_pixels().expect("uninterrupted readback");
        let before = host.capture_pixels().expect("pre-loss readback");
        std::fs::write(output.join("control.rgba"), &control).expect("save control");
        std::fs::write(output.join("before.rgba"), &before).expect("save pre-loss");
        assert_eq!(control.len(), 1920 * 1080 * 4);
        assert!(control == before, "uninterrupted readback changed");
        let inputs = host.device_prepare_inputs().expect("retain visual inputs");
        let (sender, receiver) = std::sync::mpsc::channel();
        old.set_device_lost_callback(move |reason, _| {
            let _ = sender.send(reason);
        });
        old.destroy();
        old.poll(wgpu::PollType::Poll).expect("deliver Metal loss callback");
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).expect("loss callback"),
            wgpu::DeviceLostReason::Destroyed
        );
        let (_replacement, replacement_queue, replacement_caps) =
            cooked_readback_device(&instance, 2).await;
        let mut candidate = inputs
            .prepare(&replacement_caps, &replacement_queue, Some(profile))
            .await
            .expect("rebuild exact recipe");
        host.publish_device(&mut candidate).expect("replace graphics only");
        assert_eq!(cooked_readback_resident(&mut host, &cooked, 1), generation);
        assert_eq!(host.tick(), tick);
        assert_eq!(controls.check_health().expect("same paused owner"), tick);
        let after = host.capture_pixels().expect("replacement readback");
        std::fs::write(output.join("after.rgba"), &after).expect("save replacement");
        let changed = before
            .chunks_exact(4)
            .zip(after.chunks_exact(4))
            .filter(|(first, second)| first != second)
            .count();
        eprintln!(
            "cooked generation={generation} epoch={:?} tick={tick} timeline={} changed_pixels={changed}",
            pause.id.epoch, pause.timeline,
        );
        assert_eq!(before.len(), after.len());
        assert_eq!(changed, 0);
    });
}

async fn cooked_readback_device(
    instance: &wgpu::Instance,
    generation: u64,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .expect("Metal adapter required");
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter)
                | DeviceCapabilities::requested_content_features(&adapter),
            ..Default::default()
        })
        .await
        .expect("cooked readback device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
    (device, queue, capabilities)
}

fn cooked_readback_resident(
    host: &mut FrameHost,
    cooked: &std::sync::Mutex<thrum_asset_gpu::CookedStats>,
    earlier_swaps: usize,
) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        {
            let stats = cooked.lock().expect("cooked stats");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            if stats.swaps.len() > earlier_swaps {
                return stats.drawn.expect("swapped cooked generation");
            }
        }
        assert!(Instant::now() < deadline, "cooked residency timed out");
        drop(host.capture_pixels().expect("drive cooked upload with explicit readback"));
    }
}
