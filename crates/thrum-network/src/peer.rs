//! One ordinary outbound peer owns its unsent and submitted N1 state.
//! Baseline replacement, authenticated session admission and transport are
//! separate construction gates; this owner closes on a capture fault or Reject.

use crate::outbound::{
    OutboundError, OutboundValidator, PartitionSeed, TransferError, UnsentQueue,
};
use crate::retirement::PeerKey;
use crate::sent::{AckError, FrameParts, OrdinaryLedger, RejectError, SubmitError, WriteError};

/// Ordinary publication is permitted only while both owned stores are live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerPhase {
    /// Capture, submit and exact admission may continue.
    Ordinary,
    /// Drop this peer; no replacement-baseline path exists in this slice.
    RequiresClose,
}

/// Construction refusal before any peer owner is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewPeerError {
    /// The baseline partition set or peer identity is invalid.
    Outbound(OutboundError),
    /// The initial sent-ledger identity is invalid.
    Ledger(SubmitError),
}

/// A retained owner is closed or an operation on one of its stores refused.
#[derive(Debug, PartialEq, Eq)]
pub enum PeerError<E> {
    /// This ordinary-only peer must be torn down before further publication.
    RequiresClose,
    /// The named operation refused without disguising its exact reason.
    Operation(E),
}

/// A private pair of queue and exact ordinary sent-frame ledger for one
/// match/peer/epoch. No caller may exchange either store for another peer's.
pub struct OrdinaryPeer {
    unsent: UnsentQueue,
    sent: OrdinaryLedger,
    phase: PeerPhase,
}

impl OrdinaryPeer {
    /// Construct from one externally admitted baseline incarnation. Sequence
    /// zero is reserved for initial Hello; any nonzero seed must come from a
    /// verified same-epoch control prefix.
    pub fn from_baseline(
        key: PeerKey,
        partitions: &[PartitionSeed],
        last_sequence: u64,
        authority_retired_through: u64,
    ) -> Result<Self, NewPeerError> {
        let unsent = UnsentQueue::from_baseline(key, partitions).map_err(NewPeerError::Outbound)?;
        let sent = OrdinaryLedger::new(key, last_sequence, authority_retired_through)
            .map_err(NewPeerError::Ledger)?;
        Ok(Self {
            unsent,
            sent,
            phase: PeerPhase::Ordinary,
        })
    }

    /// Exact match/peer/connection incarnation bound at construction.
    pub fn identity(&self) -> PeerKey {
        self.unsent.identity()
    }

    /// Capture is permanently stopped after any incomplete tick plan, an
    /// exact Reject or an uncertain external write/authority callback.
    pub fn phase(&self) -> PeerPhase {
        if self.phase == PeerPhase::RequiresClose
            || self.sent.is_poisoned()
            || self.sent.rejected_sequence().is_some()
        {
            PeerPhase::RequiresClose
        } else {
            PeerPhase::Ordinary
        }
    }

    /// Number of N1-owned groups not yet assigned an envelope sequence.
    pub fn queued_groups(&self) -> usize {
        self.unsent.len()
    }

    /// Number of submitted frames still awaiting exact admission.
    pub fn outstanding_frames(&self) -> usize {
        self.sent.outstanding_entries()
    }

    /// Last sequence assigned by the only sent owner in this ordinary slice.
    pub fn last_sequence(&self) -> u64 {
        self.sent.last_sequence()
    }

    /// Last result prefix acknowledged by the authority callback.
    pub fn result_retired_through(&self) -> u64 {
        self.sent.result_retired_through()
    }

    /// Reserved Rust value and arena/Vec capacity bytes, excluding allocator
    /// bookkeeping and the caller's game projection scratch.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            - std::mem::size_of::<UnsentQueue>()
            - std::mem::size_of::<OrdinaryLedger>()
            + self.unsent.reserved_capacity_bytes()
            + self.sent.reserved_capacity_bytes()
    }

    /// Capture up to four chronological R4 groups from one peer publication.
    /// The external producer must prove the whole-peer partition set; the
    /// game validator must certify complete schema/dependency closure. Any
    /// refusal or hook unwind leaves exact queue bytes/cursors untouched and
    /// requires closing this ordinary-only peer; authority time continues.
    pub fn capture_batch(
        &mut self,
        payloads: &[&[u8]],
        validator: &impl OutboundValidator,
    ) -> Result<(), PeerError<OutboundError>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        // Set before the game hook: if it unwinds through the queue's atomic
        // preflight, a caller catching the unwind cannot resume this peer.
        self.phase = PeerPhase::RequiresClose;
        let result = self.unsent.capture_batch(payloads, validator);
        if result.is_ok() {
            self.phase = PeerPhase::Ordinary;
        }
        result.map_err(PeerError::Operation)
    }

    /// Transfer the earliest N1-owned group only after exact frame/journal
    /// reservation and sequence commit. Capacity pressure preserves queue
    /// bytes and permits an exact later retry; other failures require close.
    pub fn submit_next(
        &mut self,
        acknowledged_client_sequence: u64,
    ) -> Result<Option<u64>, PeerError<TransferError<SubmitError>>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        let key = self.identity();
        self.sent
            .submit_next_from_queue(&mut self.unsent, key, acknowledged_client_sequence)
            .map_err(|error| {
                if !matches!(
                    error,
                    TransferError::Sink(SubmitError::FrameCapacity | SubmitError::JournalCapacity)
                ) {
                    self.phase = PeerPhase::RequiresClose;
                }
                PeerError::Operation(error)
            })
    }

    /// Borrow the earliest incomplete exact frame. A qualified ordered
    /// transport may advance only its known accepted-byte offset.
    pub fn next_write(&self) -> Result<Option<FrameParts<'_>>, PeerError<WriteError>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        self.sent.next_write().map_err(PeerError::Operation)
    }

    /// Record only exact bytes known accepted for the next frame.
    pub fn accept_written(
        &mut self,
        sequence: u64,
        count: usize,
    ) -> Result<(), PeerError<WriteError>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        self.sent
            .accept_written(sequence, count)
            .map_err(PeerError::Operation)
    }

    /// Stop all further peer publication after an uncertain external write.
    pub fn quarantine_uncertain_write(&mut self) {
        self.sent.quarantine_uncertain_write();
    }

    /// Admit an exact fully written cumulative prefix. The caller must bind
    /// `key` to the authenticated session and recheck current authority
    /// incarnation inside its callback; a callback refusal retains the exact
    /// receipt for retry without retiring any frame or mandatory identity.
    pub fn acknowledge_through<E>(
        &mut self,
        key: PeerKey,
        through: u64,
        authority_ack: impl FnOnce(PeerKey, u64) -> Result<(), E>,
    ) -> Result<u64, PeerError<AckError<E>>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        self.sent
            .acknowledge_through(key, through, authority_ack)
            .map_err(PeerError::Operation)
    }

    /// Verify one exact Reject. A valid Reject retains all frames and
    /// mandatory identities but requires close until baseline replacement
    /// joins this same sent owner.
    pub fn reject_state_group(
        &mut self,
        key: PeerKey,
        sequence: u64,
    ) -> Result<(), PeerError<RejectError>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(PeerError::RequiresClose);
        }
        self.sent
            .reject_state_group(key, sequence)
            .map_err(PeerError::Operation)
    }
}
