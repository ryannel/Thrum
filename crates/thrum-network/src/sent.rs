//! Physical ordinary StateGroup sent frames and exact admission retirement.
//! Replacement-baseline framing and transport/session policy are later gates.

use crate::envelope::{self, Header, Kind};
use crate::outbound::{TransferError, UnsentQueue};
use crate::r4;
use crate::retirement::{AuthorityRetirement, PeerKey, RetirementError};

/// Ordinary frame arena; baseline framing has a separate reserved owner.
pub const ORDINARY_BYTES: usize = 160 * 1024;
/// Maximum outstanding ordinary application frames.
pub const ORDINARY_ENTRIES: usize = 112;
/// Exact pending result identity capacity.
pub const RESULT_IDS: usize = 256;
/// Exact pending event identity capacity.
pub const EVENT_IDS: usize = 128;
const FRAME_CHARGE: usize = 12;

/// Refusal before an application sequence is consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    /// The peer/epoch differs from the retained owner.
    WrongIdentity,
    /// A transport write has uncertain disposition.
    Poisoned,
    /// Fixed R4 framing failed.
    Framing(r4::GroupError),
    /// An identity is zero, pending-duplicate, or a result is previously
    /// admitted or outside the result window.
    DuplicateOrInvalidIdentity,
    /// Exact ordinary frame bytes or entry slots are exhausted.
    FrameCapacity,
    /// Result/event journal capacity is exhausted.
    JournalCapacity,
    /// No more application sequences can be assigned in this epoch.
    SequenceExhausted,
    /// A rejected ordinary group requires a replacement baseline or close.
    ReplacementRequired,
}

/// Refusal of a client StateGroup Reject before changing retained state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectError {
    /// Wrong match, peer or connection epoch.
    WrongIdentity,
    /// An uncertain write or authority callback requires peer teardown.
    Poisoned,
    /// The named sequence does not identify an outstanding ordinary frame.
    UnknownSequence,
    /// The named frame was not fully submitted to the ordered transport.
    NotWritten,
    /// A client admission ACK for this sequence already has an occupied
    /// result receipt awaiting authority disposition.
    ConflictingAdmission,
    /// A rejected frame is already awaiting replacement.
    ReplacementRequired,
}

/// Exact known-byte transport advance refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// An uncertain write already quarantined this peer.
    Poisoned,
    /// A rejected ordinary group prevents starting another frame; this
    /// slice requires close until integrated baseline replacement exists.
    ReplacementRequired,
    /// Only the first incompletely written frame may advance.
    WrongSequence,
    /// Accepted-byte count exceeds the remaining exact frame bytes.
    InvalidCount,
}

/// Refusal of a cumulative client admission acknowledgement.
#[derive(Debug, PartialEq, Eq)]
pub enum AckError<E> {
    /// Wrong peer, match or epoch.
    WrongIdentity,
    /// An uncertain transport write requires peer teardown.
    Poisoned,
    /// Sequence is not a complete outstanding submitted prefix.
    InvalidSequence,
    /// Cumulative admission cannot pass a rejected StateGroup.
    RejectedSequence,
    /// A prior exact ACK receipt is still awaiting authority disposition.
    ReceiptOccupied,
    /// The result frontier refused or the authority callback failed.
    Retirement(RetirementError<E>),
}

#[derive(Clone, Copy)]
struct Entry {
    sequence: u64,
    start: usize,
    frame_len: usize,
    charged_len: usize,
    written: usize,
    result_count: usize,
    event_count: usize,
}

/// Borrowed exact encoded frame, possibly split at the arena's physical end.
pub struct FrameParts<'a> {
    /// Assigned application sequence.
    pub sequence: u64,
    /// Already accepted bytes of this frame, if a qualified transport reports them.
    pub accepted_bytes: usize,
    /// First physical slice of the immutable frame.
    pub first: &'a [u8],
    /// Second physical slice after wrap; empty for a contiguous frame.
    pub second: &'a [u8],
}

/// One peer's ordinary sent-frame ring. All arrays are allocated or inlined
/// at construction; submission and ACK use bounded stack scratch only.
pub struct OrdinaryLedger {
    key: PeerKey,
    arena: Box<[u8]>,
    entries: [Option<Entry>; ORDINARY_ENTRIES],
    entry_head: usize,
    entry_len: usize,
    byte_head: usize,
    byte_len: usize,
    results: [u64; RESULT_IDS],
    result_head: usize,
    result_len: usize,
    events: [u64; EVENT_IDS],
    event_head: usize,
    event_len: usize,
    retirement: AuthorityRetirement,
    last_sequence: u64,
    acknowledged_through: u64,
    pending_ack: Option<u64>,
    rejected_sequence: Option<u64>,
    poisoned: bool,
    authority_call_in_progress: bool,
}

impl OrdinaryLedger {
    /// Start a new peer/epoch with no outstanding ordinary frames. Sequence
    /// zero is reserved for initial Hello; a nonzero seed is for a verified
    /// existing same-epoch baseline/control prefix.
    pub fn new(
        key: PeerKey,
        last_sequence: u64,
        authority_retired_through: u64,
    ) -> Result<Self, SubmitError> {
        if key.match_id == [0; 16] || key.connection_epoch == 0 {
            return Err(SubmitError::WrongIdentity);
        }
        Ok(Self {
            key,
            arena: vec![0; ORDINARY_BYTES].into_boxed_slice(),
            entries: [None; ORDINARY_ENTRIES],
            entry_head: 0,
            entry_len: 0,
            byte_head: 0,
            byte_len: 0,
            results: [0; RESULT_IDS],
            result_head: 0,
            result_len: 0,
            events: [0; EVENT_IDS],
            event_head: 0,
            event_len: 0,
            retirement: AuthorityRetirement::new(key, authority_retired_through)
                .expect("validated peer identity"),
            last_sequence,
            acknowledged_through: last_sequence,
            pending_ack: None,
            rejected_sequence: None,
            poisoned: false,
            authority_call_in_progress: false,
        })
    }

    /// Current exact reserved Rust value/arena bytes, excluding allocator metadata.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.arena.len() + self.retirement.reserved_capacity_bytes()
            - std::mem::size_of::<AuthorityRetirement>()
    }
    /// Exact retained frame bytes, including charged 12-byte per-frame padding.
    pub fn retained_frame_bytes(&self) -> usize {
        self.byte_len
    }
    /// Number of unacknowledged application entries.
    pub fn outstanding_entries(&self) -> usize {
        self.entry_len
    }
    /// Remaining exact identity slots available to this peer's unsent queue.
    pub(crate) fn journal_credits(&self) -> (usize, usize) {
        (RESULT_IDS - self.result_len, EVENT_IDS - self.event_len)
    }
    /// Last assigned application sequence.
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }
    /// Last contiguous sequence actually admitted by this ledger.
    pub fn acknowledged_through(&self) -> u64 {
        self.acknowledged_through
    }
    /// Result prefix acknowledged by the current authority callback.
    pub fn result_retired_through(&self) -> u64 {
        self.retirement.retired_through()
    }
    /// Client-admitted results still above an unresolved lower gap.
    pub fn admitted_higher_result_count(&self) -> usize {
        self.retirement.admitted_higher_count()
    }
    /// Whether an exact ACK receipt is awaiting authority disposition.
    pub fn receipt_occupied(&self) -> bool {
        self.retirement.receipt_occupied()
    }
    /// Exact ordinary sequence rejected by the peer, if any.
    pub fn rejected_sequence(&self) -> Option<u64> {
        self.rejected_sequence
    }
    /// Whether transport disposition has become uncertain.
    pub fn is_poisoned(&self) -> bool {
        self.poisoned || self.authority_call_in_progress
    }

    fn entry(&self, offset: usize) -> Entry {
        self.entries[(self.entry_head + offset) % ORDINARY_ENTRIES].expect("occupied prefix")
    }

    /// Move the earliest already schema-certified N1-owned payload into
    /// exact sent-frame storage. The queue releases its slot only after this
    /// ledger has reserved bytes and journals and assigned the sequence.
    /// On refusal, the queue retains the original unit and no sequence is
    /// consumed. This is the no-socket owner-to-owner boundary; the W1
    /// capture producer and authenticated session remain separate gates.
    pub fn submit_next_from_queue(
        &mut self,
        queue: &mut UnsentQueue,
        key: PeerKey,
        acknowledged_client_sequence: u64,
    ) -> Result<Option<u64>, TransferError<SubmitError>> {
        if queue.identity() != key || self.key != key {
            return Err(TransferError::Sink(SubmitError::WrongIdentity));
        }
        let mut committed = None;
        let transferred = queue.transfer_next(|unit| {
            committed =
                Some(self.submit_state_group(key, unit.payload, acknowledged_client_sequence)?);
            Ok(())
        })?;
        if transferred {
            Ok(Some(
                committed.expect("successful transfer committed sequence"),
            ))
        } else {
            Ok(None)
        }
    }

    /// Encode and retain one already schema-certified R4 payload. This
    /// structural boundary rechecks framing and exact identity/capacity; the
    /// caller must certify game schema, complete closure and match-lifetime
    /// event-ID policy before calling. This owner deduplicates pending events,
    /// while a future session/event owner handles admitted-history replay.
    pub fn submit_state_group(
        &mut self,
        key: PeerKey,
        payload: &[u8],
        acknowledged_client_sequence: u64,
    ) -> Result<u64, SubmitError> {
        if self.is_poisoned() {
            return Err(SubmitError::Poisoned);
        }
        if key != self.key {
            return Err(SubmitError::WrongIdentity);
        }
        if self.rejected_sequence.is_some() {
            return Err(SubmitError::ReplacementRequired);
        }
        let group = r4::decode(payload).map_err(SubmitError::Framing)?;
        let sequence = self
            .last_sequence
            .checked_add(1)
            .ok_or(SubmitError::SequenceExhausted)?;
        let frame_len = envelope::HEADER_BYTES + payload.len();
        let charged_len = frame_len + FRAME_CHARGE;
        if self.entry_len == ORDINARY_ENTRIES || charged_len > ORDINARY_BYTES - self.byte_len {
            return Err(SubmitError::FrameCapacity);
        }
        let result_count = group.results().len();
        let event_count = group.events().len();
        if result_count > RESULT_IDS - self.result_len || event_count > EVENT_IDS - self.event_len {
            return Err(SubmitError::JournalCapacity);
        }
        let mut new_results = [0u64; 64];
        for (index, result) in group.results().enumerate() {
            let id = result.input_sequence;
            if id == 0
                || self.retirement.is_admitted(id)
                || id.saturating_sub(self.retirement.retired_through()) > RESULT_IDS as u64
                || new_results[..index].contains(&id)
                || (0..self.result_len)
                    .any(|i| self.results[(self.result_head + i) % RESULT_IDS] == id)
            {
                return Err(SubmitError::DuplicateOrInvalidIdentity);
            }
            new_results[index] = id;
        }
        let mut new_events = [0u64; 64];
        for (index, event) in group.events().enumerate() {
            let id = event.event_id;
            if id == 0
                || new_events[..index].contains(&id)
                || (0..self.event_len).any(|i| self.events[(self.event_head + i) % EVENT_IDS] == id)
            {
                return Err(SubmitError::DuplicateOrInvalidIdentity);
            }
            new_events[index] = id;
        }
        let header = Header {
            kind: Kind::StateGroup,
            match_id: key.match_id,
            connection_epoch: key.connection_epoch,
            message_sequence: sequence,
            server_tick: group.header.commit_tick,
            acknowledged_sequence: acknowledged_client_sequence,
        };
        let mut encoded = [0u8; envelope::HEADER_BYTES + r4::MAX_PAYLOAD_BYTES];
        envelope::encode(header, payload, &mut encoded).expect("R4 maximum fits envelope");
        let start = (self.byte_head + self.byte_len) % ORDINARY_BYTES;
        let first_len = frame_len.min(ORDINARY_BYTES - start);
        self.arena[start..start + first_len].copy_from_slice(&encoded[..first_len]);
        self.arena[..frame_len - first_len].copy_from_slice(&encoded[first_len..frame_len]);
        let at = (self.entry_head + self.entry_len) % ORDINARY_ENTRIES;
        self.entries[at] = Some(Entry {
            sequence,
            start,
            frame_len,
            charged_len,
            written: 0,
            result_count,
            event_count,
        });
        for &id in &new_results[..result_count] {
            self.results[(self.result_head + self.result_len) % RESULT_IDS] = id;
            self.result_len += 1;
        }
        for &id in &new_events[..event_count] {
            self.events[(self.event_head + self.event_len) % EVENT_IDS] = id;
            self.event_len += 1;
        }
        self.entry_len += 1;
        self.byte_len += charged_len;
        self.last_sequence = sequence;
        Ok(sequence)
    }

    /// First incomplete frame in stream order. Both slices are immutable
    /// until an admission ACK or explicit peer teardown.
    pub fn next_write(&self) -> Result<Option<FrameParts<'_>>, WriteError> {
        if self.is_poisoned() {
            return Err(WriteError::Poisoned);
        }
        if self.rejected_sequence.is_some() {
            return Err(WriteError::ReplacementRequired);
        }
        let Some(entry) = (0..self.entry_len)
            .map(|i| self.entry(i))
            .find(|entry| entry.written < entry.frame_len)
        else {
            return Ok(None);
        };
        let first_len = entry.frame_len.min(ORDINARY_BYTES - entry.start);
        Ok(Some(FrameParts {
            sequence: entry.sequence,
            accepted_bytes: entry.written,
            first: &self.arena[entry.start..entry.start + first_len],
            second: &self.arena[..entry.frame_len - first_len],
        }))
    }

    /// Record only bytes whose transport adapter knows it accepted. The
    /// adapter must close/quarantine on uncertain write disposition.
    pub fn accept_written(&mut self, sequence: u64, count: usize) -> Result<(), WriteError> {
        if self.is_poisoned() {
            return Err(WriteError::Poisoned);
        }
        if self.rejected_sequence.is_some() {
            return Err(WriteError::ReplacementRequired);
        }
        let Some(offset) =
            (0..self.entry_len).find(|&i| self.entry(i).written < self.entry(i).frame_len)
        else {
            return Err(WriteError::WrongSequence);
        };
        let at = (self.entry_head + offset) % ORDINARY_ENTRIES;
        let mut entry = self.entries[at].expect("incomplete entry");
        if entry.sequence != sequence {
            return Err(WriteError::WrongSequence);
        }
        if count > entry.frame_len - entry.written {
            return Err(WriteError::InvalidCount);
        }
        entry.written += count;
        self.entries[at] = Some(entry);
        Ok(())
    }

    /// An unknown external write outcome cannot be retried under a new
    /// sequence or silently discarded; the peer must close.
    pub fn quarantine_uncertain_write(&mut self) {
        self.poisoned = true;
    }

    /// Record one externally authenticated Reject for an exact fully written
    /// ordinary frame. It is never admission ACK and frees no retained bytes
    /// or identities. Replacement framing is a separate later owner; until
    /// then the peer must close rather than resume ordinary submission.
    pub fn reject_state_group(&mut self, key: PeerKey, sequence: u64) -> Result<(), RejectError> {
        if self.is_poisoned() {
            return Err(RejectError::Poisoned);
        }
        if key != self.key {
            return Err(RejectError::WrongIdentity);
        }
        if self.rejected_sequence.is_some() {
            return Err(RejectError::ReplacementRequired);
        }
        if self.pending_ack.is_some_and(|through| sequence <= through) {
            return Err(RejectError::ConflictingAdmission);
        }
        let Some(entry) = (0..self.entry_len)
            .map(|index| self.entry(index))
            .find(|entry| entry.sequence == sequence)
        else {
            return Err(RejectError::UnknownSequence);
        };
        if entry.written != entry.frame_len {
            return Err(RejectError::NotWritten);
        }
        self.rejected_sequence = Some(sequence);
        Ok(())
    }

    /// Admit a complete cumulative StateGroup prefix and retire its exact
    /// frame/journal bytes only after the authority result frontier accepts
    /// its exact identities. A callback error retains the occupied receipt
    /// for an exact retry; an unwind poisons retirement and requires close.
    pub fn acknowledge_through<E>(
        &mut self,
        key: PeerKey,
        through: u64,
        authority_ack: impl FnOnce(PeerKey, u64) -> Result<(), E>,
    ) -> Result<u64, AckError<E>> {
        if self.is_poisoned() {
            return Err(AckError::Poisoned);
        }
        if key != self.key {
            return Err(AckError::WrongIdentity);
        }
        if through <= self.acknowledged_through && self.pending_ack.is_none() {
            return Ok(self.acknowledged_through);
        }
        if self.pending_ack.is_some_and(|pending| pending != through) {
            return Err(AckError::ReceiptOccupied);
        }
        let count = usize::try_from(through.saturating_sub(self.acknowledged_through))
            .map_err(|_| AckError::InvalidSequence)?;
        if count == 0 || count > self.entry_len {
            return Err(AckError::InvalidSequence);
        }
        if self
            .rejected_sequence
            .is_some_and(|rejected| rejected <= through)
        {
            return Err(AckError::RejectedSequence);
        }
        let mut result_count = 0;
        for index in 0..count {
            let entry = self.entry(index);
            if entry.sequence != self.acknowledged_through + index as u64 + 1
                || entry.written != entry.frame_len
            {
                return Err(AckError::InvalidSequence);
            }
            result_count += entry.result_count;
        }
        if self.pending_ack.is_none() {
            let mut ids = [0u64; RESULT_IDS];
            for (index, slot) in ids[..result_count].iter_mut().enumerate() {
                *slot = self.results[(self.result_head + index) % RESULT_IDS];
            }
            self.retirement
                .stage(key, &ids[..result_count])
                .map_err(|error| {
                    AckError::Retirement(match error {
                        RetirementError::WrongIdentity => RetirementError::WrongIdentity,
                        RetirementError::InvalidIdentity => RetirementError::InvalidIdentity,
                        RetirementError::Capacity => RetirementError::Capacity,
                        RetirementError::ReceiptOccupied => RetirementError::ReceiptOccupied,
                        RetirementError::NoReceipt => RetirementError::NoReceipt,
                        RetirementError::Poisoned => RetirementError::Poisoned,
                        RetirementError::Authority(()) => unreachable!("stage has no callback"),
                    })
                })?;
            self.pending_ack = Some(through);
        }
        self.authority_call_in_progress = true;
        let outcome = self.retirement.finish(key, authority_ack);
        self.authority_call_in_progress = false;
        outcome.map_err(AckError::Retirement)?;
        for _ in 0..count {
            let at = self.entry_head;
            let entry = self.entries[at].take().expect("validated entry");
            self.entry_head = (self.entry_head + 1) % ORDINARY_ENTRIES;
            self.entry_len -= 1;
            self.byte_head = (self.byte_head + entry.charged_len) % ORDINARY_BYTES;
            self.byte_len -= entry.charged_len;
            self.result_head = (self.result_head + entry.result_count) % RESULT_IDS;
            self.result_len -= entry.result_count;
            self.event_head = (self.event_head + entry.event_count) % EVENT_IDS;
            self.event_len -= entry.event_count;
        }
        self.acknowledged_through = through;
        self.pending_ack = None;
        Ok(through)
    }
}
