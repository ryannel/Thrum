//! One ordinary outbound peer owns its unsent and submitted N1 state.
//! Baseline replacement, authenticated session admission and transport are
//! separate construction gates; this owner closes on a capture fault or Reject.

use crate::outbound::{
    OutboundError, OutboundValidator, PartitionSeed, TransferError, UnsentQueue,
};
use crate::r4::{self, GroupInput};
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

/// Refusal while encoding into one reusable, bounded projection scratch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchError {
    /// More than four groups were emitted for one peer publication.
    Capacity,
    /// A group's structural R4 encoding failed before queue mutation.
    Framing(r4::GroupError),
}

/// Whole-peer projection-plan refusal after the game callback returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The game omitted or duplicated a baseline-bound partition.
    PartitionSet,
    /// Groups in this atomic publication do not name one committed tick.
    TickMismatch,
}

/// One projected publication failed before N1 queue bytes were committed.
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError<E> {
    /// This ordinary-only peer already requires teardown.
    RequiresClose,
    /// The game projection refused; its original error is retained.
    Projection(E),
    /// The game ignored a failed bounded scratch emission.
    Scratch(ScratchError),
    /// The emitted group set is not one complete first-profile tick.
    Plan(PlanError),
    /// The schema or N1 queue refused the complete emitted plan.
    Queue(OutboundError),
}

/// Reusable projection bytes shared sequentially across peers. The fixed
/// backing allocation is separate from each peer's owned unsent/sent state;
/// a failed peer cannot publish partial scratch to its queue.
pub struct CaptureScratch {
    bytes: Box<[u8]>,
    group_ids: [u32; 4],
    ticks: [u64; 4],
    lengths: [usize; 4],
    len: usize,
    fault: Option<ScratchError>,
}

/// Borrowed emission surface for one game projection callback. The callback
/// cannot replace or clear the owner's scratch fault after a failed emission.
///
/// ```compile_fail
/// use thrum_network::peer::{CaptureScratch, CaptureWriter};
/// fn erase_failure(writer: &mut CaptureWriter<'_>) {
///     *writer = CaptureScratch::new();
/// }
/// ```
pub struct CaptureWriter<'a> {
    scratch: &'a mut CaptureScratch,
}

impl CaptureWriter<'_> {
    /// Encode one complete structural R4 group without allocating. Any
    /// capacity or framing refusal remains recorded for the whole callback.
    pub fn push(&mut self, input: GroupInput<'_>) -> Result<(), ScratchError> {
        self.scratch.push(input)
    }
}

impl CaptureScratch {
    /// Allocate four maximum R4 payload slots once before any host callback.
    pub fn new() -> Self {
        Self {
            bytes: vec![0; 4 * r4::MAX_PAYLOAD_BYTES].into_boxed_slice(),
            group_ids: [0; 4],
            ticks: [0; 4],
            lengths: [0; 4],
            len: 0,
            fault: None,
        }
    }

    fn push(&mut self, input: GroupInput<'_>) -> Result<(), ScratchError> {
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.len == 4 {
            self.fault = Some(ScratchError::Capacity);
            return Err(ScratchError::Capacity);
        }
        let start = self.len * r4::MAX_PAYLOAD_BYTES;
        let header = input.header;
        let length = match r4::encode(input, &mut self.bytes[start..start + r4::MAX_PAYLOAD_BYTES])
        {
            Ok(length) => length,
            Err(error) => {
                let fault = ScratchError::Framing(error);
                self.fault = Some(fault);
                return Err(fault);
            }
        };
        self.group_ids[self.len] = header.group_id;
        self.ticks[self.len] = header.commit_tick;
        self.lengths[self.len] = length;
        self.len += 1;
        Ok(())
    }

    /// Fixed Rust value and scratch-arena bytes, excluding allocator metadata.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.bytes.len()
    }

    fn clear(&mut self) {
        self.len = 0;
        self.fault = None;
    }

    fn group(&self, index: usize) -> &[u8] {
        let start = index * r4::MAX_PAYLOAD_BYTES;
        &self.bytes[start..start + self.lengths[index]]
    }
}

impl Default for CaptureScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// A private pair of queue and exact ordinary sent-frame ledger for one
/// match/peer/epoch. No caller may exchange either store for another peer's.
pub struct OrdinaryPeer {
    unsent: UnsentQueue,
    sent: OrdinaryLedger,
    phase: PeerPhase,
    partition_ids: [u32; 4],
    partition_count: usize,
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
        let mut partition_ids = [0; 4];
        for (index, partition) in partitions.iter().enumerate() {
            partition_ids[index] = partition.group_id;
        }
        Ok(Self {
            unsent,
            sent,
            phase: PeerPhase::Ordinary,
            partition_ids,
            partition_count: partitions.len(),
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

    /// Project one complete first-profile authority tick into reusable
    /// scratch, then atomically capture every baseline-bound partition. The
    /// game callback may borrow a W1 publication only for this call; its
    /// scalar R4 bytes are copied into N1 before that publication is dropped.
    /// A refusal or unwind closes only this peer and commits no queue bytes.
    pub fn capture_projected<E>(
        &mut self,
        scratch: &mut CaptureScratch,
        project: impl FnOnce(&mut CaptureWriter<'_>) -> Result<(), E>,
        validator: &impl OutboundValidator,
    ) -> Result<(), ProjectError<E>> {
        if self.phase() != PeerPhase::Ordinary {
            return Err(ProjectError::RequiresClose);
        }
        // This guard remains set if the game projector or schema hook unwinds.
        self.phase = PeerPhase::RequiresClose;
        scratch.clear();
        {
            let mut writer = CaptureWriter { scratch };
            project(&mut writer).map_err(ProjectError::Projection)?;
        }
        if let Some(error) = scratch.fault {
            return Err(ProjectError::Scratch(error));
        }
        if scratch.len != self.partition_count {
            return Err(ProjectError::Plan(PlanError::PartitionSet));
        }
        for &expected in &self.partition_ids[..self.partition_count] {
            if scratch.group_ids[..scratch.len]
                .iter()
                .filter(|&&actual| actual == expected)
                .count()
                != 1
            {
                return Err(ProjectError::Plan(PlanError::PartitionSet));
            }
        }
        if scratch.ticks[..scratch.len]
            .iter()
            .any(|&tick| tick != scratch.ticks[0])
        {
            return Err(ProjectError::Plan(PlanError::TickMismatch));
        }
        let payloads: [&[u8]; 4] = std::array::from_fn(|index| {
            if index < scratch.len {
                scratch.group(index)
            } else {
                &[]
            }
        });
        self.unsent
            .capture_batch(&payloads[..scratch.len], validator)
            .map_err(ProjectError::Queue)?;
        self.phase = PeerPhase::Ordinary;
        Ok(())
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
