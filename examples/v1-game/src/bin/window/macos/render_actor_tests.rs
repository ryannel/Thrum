use super::{FrameError, RenderOutcome, drain_wait, ensure_running, frame_failure};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn only_gpu_failures_with_a_device_loss_signal_become_recovery() {
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
        frame_failure(FrameError::Gpu("validation".into()), false),
        RenderOutcome::Frame(Err(_))
    ));
    assert!(matches!(
        frame_failure(FrameError::Gpu("lost".into()), true),
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
