//! Timed N1 staging before exact next-tick W1 admission.

use std::num::NonZeroUsize;

use thrum_core::{
    ActorId, Origin, PeerId,
    sim::{FrameError, InputFrame, Ownership},
};
use thrum_network::future_input::{FutureInputError, FutureInputWindow, StageResult};

fn binding(peer: u64, epoch: u64) -> Ownership {
    Ownership {
        peer: PeerId(peer),
        connection_epoch: epoch,
        actor: ActorId(peer),
    }
}

fn frame(peer: u64, epoch: u64, tick: u64, sequence: u64, axis: f32) -> InputFrame<()> {
    InputFrame::new(
        Origin {
            peer: PeerId(peer),
            connection_epoch: epoch,
            sequence,
        },
        ActorId(peer),
        tick,
        [axis, 0.0, 0.0],
    )
}

fn stage(
    window: &mut FutureInputWindow<()>,
    frame: InputFrame<()>,
) -> Result<StageResult, FutureInputError> {
    window.stage(frame, |_| Ok(()))
}

#[test]
fn future_frames_stage_out_of_order_and_leave_w1_due_order_explicit() {
    let owners = [binding(1, 7), binding(2, 9)];
    let mut window = FutureInputWindow::new(&owners, NonZeroUsize::new(8).expect("nonzero"), 10)
        .expect("distinct peers");
    assert_eq!(
        stage(&mut window, frame(2, 9, 13, 13, -1.0)),
        Ok(StageResult::Queued)
    );
    assert_eq!(
        stage(&mut window, frame(1, 7, 11, 11, 1.0)),
        Ok(StageResult::Queued)
    );
    assert_eq!(
        stage(&mut window, frame(2, 9, 13, 13, -1.0)),
        Ok(StageResult::Duplicate)
    );
    assert_eq!(
        stage(&mut window, frame(2, 9, 13, 14, 0.0)),
        Err(FutureInputError::ConflictingFrame)
    );
    assert_eq!(
        stage(&mut window, frame(1, 7, 11, 11, -0.0)),
        Err(FutureInputError::ConflictingFrame)
    );
    assert_eq!(window.commit_tick(11), Err(FutureInputError::Undrained));
    assert_eq!(
        window
            .take_due(PeerId(1))
            .expect("known peer")
            .expect("due input")
            .origin
            .sequence,
        11
    );
    assert!(window.take_due(PeerId(2)).expect("known peer").is_none());
    assert_eq!(
        stage(&mut window, frame(1, 7, 11, 15, 1.0)),
        Err(FutureInputError::Late)
    );
    assert_eq!(
        stage(&mut window, frame(2, 9, 11, 15, 1.0)),
        Err(FutureInputError::Late)
    );
    window.commit_tick(11).expect("drained tick");
    assert_eq!(window.committed_tick(), 11);
    assert_eq!(
        stage(&mut window, frame(1, 7, 11, 11, 1.0)),
        Err(FutureInputError::Late)
    );
    assert!(window.take_due(PeerId(2)).expect("known peer").is_none());
    window.commit_tick(12).expect("no due input");
    assert_eq!(
        window
            .take_due(PeerId(2))
            .expect("known peer")
            .expect("future input")
            .origin
            .sequence,
        13
    );
    window.commit_tick(13).expect("drained tick");
    assert_eq!(window.commit_tick(13), Err(FutureInputError::WrongCommit));
}

#[test]
fn invalid_identity_axes_and_reconnect_do_not_replace_a_future_frame() {
    let mut window =
        FutureInputWindow::new(&[binding(1, 7)], NonZeroUsize::new(4).expect("nonzero"), 3)
            .expect("one peer");
    assert_eq!(
        stage(&mut window, frame(1, 7, 4, 1, 1.0)),
        Ok(StageResult::Queued)
    );
    assert_eq!(
        stage(&mut window, frame(2, 7, 5, 1, 1.0)),
        Err(FutureInputError::UnknownPeer)
    );
    assert_eq!(
        stage(&mut window, frame(1, 6, 5, 1, 1.0)),
        Err(FutureInputError::WrongConnection)
    );
    let mut wrong_actor = frame(1, 7, 5, 1, 1.0);
    wrong_actor.actor = ActorId(2);
    assert_eq!(
        stage(&mut window, wrong_actor),
        Err(FutureInputError::WrongActor)
    );
    assert_eq!(
        stage(&mut window, frame(1, 7, 5, 1, f32::NAN)),
        Err(FutureInputError::InvalidAxis)
    );
    assert_eq!(
        window.stage(frame(1, 7, 6, 3, 1.0), |_| Err(FrameError::InvalidAction)),
        Err(FutureInputError::InvalidPayload(FrameError::InvalidAction))
    );
    assert_eq!(
        stage(&mut window, frame(1, 7, 6, 3, 1.0)),
        Ok(StageResult::Queued)
    );
    assert_eq!(
        stage(&mut window, frame(1, 7, 8, 1, 1.0)),
        Err(FutureInputError::TooEarly)
    );
    assert_eq!(
        window.reconnect(binding(1, 7)),
        Err(FutureInputError::StaleConnection)
    );
    assert_eq!(
        window
            .take_due(PeerId(1))
            .expect("known peer")
            .expect("old frame")
            .origin
            .sequence,
        1
    );
    window.commit_tick(4).expect("drained old tick");
    stage(&mut window, frame(1, 7, 5, 2, -1.0)).expect("old epoch frame");
    window.reconnect(binding(1, 8)).expect("new epoch");
    assert!(window.take_due(PeerId(1)).expect("known peer").is_none());
    assert_eq!(
        stage(&mut window, frame(1, 7, 5, 2, -1.0)),
        Err(FutureInputError::WrongConnection)
    );
    assert_eq!(
        stage(&mut window, frame(1, 8, 6, 1, 0.0)),
        Ok(StageResult::Queued)
    );
}
