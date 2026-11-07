//! How presentation edits travel from the main thread to the render owner, and which
//! render replies may update the game's controls from their readback.
//!
//! Every reason to draw again (a display target, input, an edit, device loss) advances the
//! request sequence; a frame is wanted while it is ahead of the last submission. A frame
//! already in flight when the person edits a control reads back the state before that edit,
//! so its reply is not observed: it would snap the panel back until the next reply, and a
//! recovery replay could then resend the stale value. A frame cancelled by device loss takes
//! its edits with it; recovery queues the game's whole desired state again instead.

use crate::Edits;

#[derive(Debug, Default)]
pub(super) struct EditFlow<E> {
    /// Edits for the next frame request.
    pending: E,
    /// Advanced by every reason to draw again; unique per submitted frame.
    requested: u64,
    /// The request sequence of the latest submitted frame.
    submitted: u64,
    /// Request sequence at which the latest edits were queued; the frame that carries them
    /// is submitted at this sequence or later.
    edits_queued_at: u64,
}

impl<E: Edits> EditFlow<E> {
    /// Something other than an edit changed, so a new frame is wanted.
    pub(super) fn request(&mut self) {
        self.requested = self.requested.saturating_add(1);
    }

    /// Queue edits for the next frame request; returns whether there were any.
    pub(super) fn queue(&mut self, edits: E) -> bool {
        if edits.is_empty() {
            return false;
        }
        self.pending = self.pending.merge(edits);
        self.request();
        self.edits_queued_at = self.requested;
        true
    }

    /// A replacement device is ready: queue the whole desired state, which also covers
    /// edits carried by a frame cancelled during device loss.
    pub(super) fn recovered(&mut self, replay: E) {
        self.pending = self.pending.merge(replay);
        self.request();
    }

    /// Whether anything changed since the latest submitted frame.
    pub(super) fn wants_frame(&self) -> bool {
        self.requested != self.submitted
    }

    /// The edits the next frame request carries.
    pub(super) fn pending(&self) -> E {
        self.pending
    }

    /// The frame carrying `pending` was accepted; returns its request sequence.
    pub(super) fn submitted(&mut self) -> u64 {
        self.pending = E::default();
        self.submitted = self.requested;
        self.submitted
    }

    /// Whether the reply to the frame submitted at `submitted_sequence` reflects the latest edits.
    pub(super) fn admits(&self, submitted_sequence: u64) -> bool {
        submitted_sequence >= self.edits_queued_at
    }
}

#[cfg(test)]
#[path = "edit_flow_tests.rs"]
mod tests;
