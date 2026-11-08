use super::{Host, Policy, gpu, host};
use std::time::{Duration, Instant};

fn retire_scene(host: &mut Host) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let timeout = deadline.saturating_duration_since(Instant::now());
        assert!(
            !timeout.is_zero(),
            "camera-history test scene did not retire"
        );
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .expect("wait for scene or its timing readback");
        if host.poll_completed().expect("collect scene").is_some() {
            assert_eq!(host.pending_frames(), 0);
            return;
        }
    }
}

#[test]
fn a_skipped_camera_cut_resets_only_the_first_submitted_view_in_its_epoch() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                camera_cut_tick: Some(2),
                empty: true,
                ..Policy::default()
            },
        )
        .await;
        assert!(
            host.advance(Duration::ZERO)
                .expect("initial authority")
                .encode_scene()
                .expect("initial recording")
                .submit(None)
                .expect("initial submission")
                .submitted
        );
        retire_scene(&mut host);
        host.world.authority.tick = 3;
        host.world.publication.tick = 3;
        let abandoned = host
            .advance(Duration::ZERO)
            .expect("post-cut authority")
            .encode_scene()
            .expect("post-cut recording");
        assert!(abandoned.can_acquire_surface());
        drop(abandoned);
        assert!(host.scene.view.history_reset());
        assert_eq!(host.submitted_view_history_epoch, 0);

        let pixels = host.capture_pixels().expect("post-cut capture");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert!(host.scene.view.history_reset());
        assert_eq!(host.submitted_view_history_epoch, 0);

        for reset in [true, false] {
            assert!(
                host.advance(Duration::ZERO)
                    .expect("same authority")
                    .encode_scene()
                    .expect("same-epoch recording")
                    .submit(None)
                    .expect("same-epoch submission")
                    .submitted
            );
            assert_eq!(host.scene.view.history_reset(), reset);
            assert_eq!(host.submitted_view_history_epoch, 1);
            retire_scene(&mut host);
        }
    });
}
