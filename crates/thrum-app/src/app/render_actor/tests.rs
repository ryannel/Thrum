use super::{FrameError, RenderOutcome, drain_wait, ensure_running, frame_failure};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn only_gpu_failures_with_a_device_loss_signal_become_recovery() {
    assert!(matches!(
        frame_failure(FrameError::NativeCommandBuffer { generation: 7 }, false),
        RenderOutcome::DeviceLost
    ));
    for error in [
        FrameError::World("authority failure".into()),
        FrameError::Scene("scene failure".into()),
        FrameError::InvalidState("stale source"),
    ] {
        assert!(matches!(
            frame_failure(error, true),
            RenderOutcome::Frame(Err(_))
        ));
    }
    assert!(matches!(
        frame_failure(
            FrameError::gpu(
                "validation",
                thrum_render::error::RenderError::InvalidLighting
            ),
            false
        ),
        RenderOutcome::Frame(Err(_))
    ));
    assert!(matches!(
        frame_failure(FrameError::DeviceLost("lost".into()), false),
        RenderOutcome::DeviceLost
    ));
}

#[test]
fn shutdown_gate_refuses_work_without_a_gpu_or_world_operation() {
    let stopped = AtomicBool::new(false);
    ensure_running(&stopped).unwrap();
    stopped.store(true, Ordering::Release);
    assert!(matches!(
        ensure_running(&stopped),
        Err(FrameError::InvalidState(_))
    ));
}

#[test]
fn final_wait_reports_observed_loss_but_preserves_genuine_wait_failures() {
    assert!(!drain_wait(Ok(wgpu::PollStatus::QueueEmpty), false).unwrap());
    assert!(drain_wait(Ok(wgpu::PollStatus::QueueEmpty), true).unwrap());
    assert!(drain_wait(Err(wgpu::PollError::Timeout), true).unwrap());
    assert!(drain_wait(Err(wgpu::PollError::Timeout), false).is_err());
    assert!(drain_wait(Err(wgpu::PollError::WrongSubmissionIndex(2, 1)), true).is_err());
}

#[test]
fn tick_work_before_slot_wait_with_and_without_a_free_slot_before_deadline() {
    use super::{FrameSample, SlotWaitHost, wait_for_frame_slot};
    use std::{cell::RefCell, time::Duration};

    struct Host<'a> {
        calls: &'a RefCell<Vec<&'static str>>,
        free_on_poll: bool,
        free: bool,
    }

    impl SlotWaitHost for Host<'_> {
        fn ask_tick_stage(&mut self) -> Result<(), FrameError> {
            self.calls.borrow_mut().push("ask");
            Ok(())
        }

        fn has_free_slot(&self) -> bool {
            self.free
        }

        fn poll_completed(&mut self) -> Result<Option<FrameSample>, FrameError> {
            self.calls.borrow_mut().push("poll");
            self.free = self.free_on_poll;
            Ok(None)
        }
    }

    for (free_on_poll, expected) in [
        (true, &["ask", "poll"][..]),
        (false, &["ask", "poll", "sleep"][..]),
    ] {
        let calls = RefCell::new(Vec::new());
        let mut host = Host {
            calls: &calls,
            free_on_poll,
            free: false,
        };
        let mut remaining = Duration::from_micros(250);
        let stopped = AtomicBool::new(false);
        wait_for_frame_slot(
            &mut host,
            &mut [None],
            &stopped,
            || std::mem::take(&mut remaining),
            |_| calls.borrow_mut().push("sleep"),
        )
        .unwrap();
        assert_eq!(calls.borrow().as_slice(), expected);
        assert_eq!(host.free, free_on_poll);
    }
}
