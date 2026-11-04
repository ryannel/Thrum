use super::{FrameSample, IN_FLIGHT, retain_sample};
use std::{collections::VecDeque, time::Duration};

fn sample(tick: u64) -> FrameSample {
    FrameSample {
        tick,
        completed_ticks: 1,
        queued_ticks: 0,
        tick_cpu: Duration::ZERO,
        extract_cpu: Duration::ZERO,
        worker_queue_ns: 0,
        worker_ready_ns: 0,
        visual_age_ticks: 0,
        upload_cpu: Duration::ZERO,
        encode_cpu: Duration::ZERO,
        wall: Duration::ZERO,
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
        visual_revision: None,
    }
}

#[test]
fn full_report_queue_drops_the_oldest_tick_even_when_it_finishes_late() {
    let mut queue = VecDeque::with_capacity(IN_FLIGHT);
    let mut dropped = 0;
    for tick in [2, 3, 4] {
        retain_sample(&mut queue, &mut dropped, sample(tick));
    }
    retain_sample(&mut queue, &mut dropped, sample(1));
    assert_eq!(dropped, 1);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [2, 3, 4]
    );
    retain_sample(&mut queue, &mut dropped, sample(5));
    assert_eq!(dropped, 2);
    assert_eq!(
        queue.iter().map(|entry| entry.tick).collect::<Vec<_>>(),
        [3, 4, 5]
    );
}

#[test]
#[ignore = "requires a real native GPU adapter to compare stale and refreshed captures"]
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
        let mut host = super::FrameHost::new(&device, &queue, None)
            .await
            .expect("prepared scene");
        let before = host.capture_pixels().expect("initial capture");
        for _ in 0..100 {
            host.sim.step(&[]).expect("committed world update");
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
#[ignore = "requires a real native GPU adapter for capture after worker extraction"]
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
        let mut host = super::FrameHost::new(&device, &queue, None)
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
#[ignore = "requires a real native GPU adapter for a changed visual generation"]
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
        let mut host = super::FrameHost::new(&device, &queue, None)
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
