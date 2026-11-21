//! Validation and loss callbacks from a real Metal device.

#[test]
fn metal_watch_records_validation_and_loss_and_a_fresh_generation_submits() {
    use std::{sync::mpsc, time::Duration};
    use thrum_render::capabilities::{DeviceRecipe, FIRST_DEVICE_GENERATION};

    let Some(gpu) = thrum_test_support::gpu(
        "metal_watch_records_validation_and_loss_and_a_fresh_generation_submits",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (send, receive) = mpsc::channel();
        let recipe = DeviceRecipe::default();
        let opened = recipe
            .open_with_wake(&gpu.instance, None, move |generation| {
                let _ = send.send(generation);
            })
            .await
            .unwrap();
        let invalid_buffer = |label| {
            opened.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 16,
                usage: wgpu::BufferUsages::empty(),
                mapped_at_creation: false,
            })
        };
        let _invalid = invalid_buffer("first invalid buffer");
        let first = opened
            .watch
            .uncaptured_error()
            .expect("uncaptured validation error");
        assert!(first.contains("first invalid buffer"), "{first}");
        let _invalid = invalid_buffer("second invalid buffer");
        assert_eq!(opened.watch.uncaptured_error(), Some(first));
        assert!(!opened.watch.is_lost());
        let submitted = opened.queue.submit([]);
        let wait = |submission_index| wgpu::PollType::Wait {
            submission_index: Some(submission_index),
            timeout: Some(Duration::from_secs(5)),
        };
        opened.device.poll(wait(submitted)).unwrap();

        opened.device.destroy();
        thrum_test_support::wait::until("destroy notification", Duration::from_secs(5), || {
            opened.device.poll(wgpu::PollType::Poll).unwrap();
            opened.watch.is_lost().then_some(())
        });
        assert_eq!(
            receive.recv_timeout(Duration::from_secs(1)).unwrap(),
            FIRST_DEVICE_GENERATION
        );
        let loss = opened.watch.loss_message().expect("first loss callback");
        assert!(loss.contains("Destroyed"), "{loss}");
        assert_eq!(opened.watch.uncaptured_error(), Some(first));

        let replacement = DeviceRecipe {
            generation: FIRST_DEVICE_GENERATION + 1,
            ..recipe
        }
        .open(&gpu.instance, None)
        .await
        .unwrap();
        assert!(!replacement.watch.is_lost());
        assert_eq!(replacement.watch.uncaptured_error(), None);
        assert_eq!(replacement.watch.loss_message(), None);
        assert_eq!(replacement.watch.generation, opened.watch.generation + 1);
        let submitted = replacement.queue.submit([]);
        replacement.device.poll(wait(submitted)).unwrap();
        eprintln!(
            "Metal generation {} recorded validation and {loss}; generation {} submitted",
            opened.watch.generation, replacement.watch.generation
        );
    });
}
