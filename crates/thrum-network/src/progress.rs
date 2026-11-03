//! Bounded client input-result progress for event-free R4 groups. A future
//! exact event owner must extend this same atomic admission boundary.

use crate::r4::GroupView;

/// Maximum unresolved or individually resolved inputs retained per peer.
pub const MAX_PENDING_INPUTS: usize = 256;

/// One outstanding input represented in an admitted baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingInput {
    /// Connection-scoped input sequence.
    pub sequence: u64,
    /// Manifest action type expected in the matching result.
    pub action_type: u32,
    /// Whether an individually resolved higher result is already represented.
    pub resolved: bool,
}

/// A result-progress admission refusal before any state or progress mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressError {
    /// Baseline input sequence inventory is invalid or too large.
    InvalidBaseline,
    /// Input admission has no retained slot.
    Capacity,
    /// Sequence exhaustion requires a new connection incarnation.
    SequenceExhausted,
    /// Event-bearing groups await an exact event-dedup owner.
    UnsupportedEvent,
    /// Results or watermark appeared outside the controlled partition.
    WrongPartition,
    /// The full controller state is absent from a result-bearing group.
    MissingController,
    /// A result is unknown, duplicate, already resolved or mismatches input.
    InvalidResult,
    /// Watermark differs from the exactly contiguous resolved prefix.
    InvalidWatermark,
}

/// One client's accepted input inventory and contiguous resolution cursor.
/// All Vec capacity is reserved once from the admitted baseline.
#[derive(Debug, PartialEq, Eq)]
pub struct Progress {
    controlled_group_id: u32,
    controlled_actor_id: u64,
    resolved_through: u64,
    next_sequence: u64,
    pending: Vec<PendingInput>,
}

#[derive(Clone, Copy)]
pub(crate) struct Candidate {
    newly_resolved: [u64; 4],
    prefix: usize,
}

impl Progress {
    /// Construct from an externally verified same-epoch baseline. `pending`
    /// must enumerate every sequence immediately above `resolved_through`,
    /// including individually resolved results above a pending gap.
    pub fn from_baseline(
        controlled_group_id: u32,
        controlled_actor_id: u64,
        resolved_through: u64,
        pending: &[PendingInput],
    ) -> Result<Self, ProgressError> {
        if controlled_actor_id == 0
            || pending.len() > MAX_PENDING_INPUTS
            || pending.first().is_some_and(|first| first.resolved)
        {
            return Err(ProgressError::InvalidBaseline);
        }
        let mut expected = resolved_through
            .checked_add(1)
            .ok_or(ProgressError::SequenceExhausted)?;
        for input in pending {
            if input.sequence != expected {
                return Err(ProgressError::InvalidBaseline);
            }
            expected = expected
                .checked_add(1)
                .ok_or(ProgressError::SequenceExhausted)?;
        }
        let mut retained = Vec::with_capacity(MAX_PENDING_INPUTS);
        retained.extend_from_slice(pending);
        Ok(Self {
            controlled_group_id,
            controlled_actor_id,
            resolved_through,
            next_sequence: expected,
            pending: retained,
        })
    }

    /// Reserve one client input sequence only after finite retained capacity
    /// is available. The caller still validates the action and connection.
    pub fn reserve_input(&mut self, action_type: u32) -> Result<u64, ProgressError> {
        if self.pending.len() == MAX_PENDING_INPUTS {
            return Err(ProgressError::Capacity);
        }
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(ProgressError::SequenceExhausted)?;
        let sequence = self.next_sequence;
        self.pending.push(PendingInput {
            sequence,
            action_type,
            resolved: false,
        });
        self.next_sequence = next;
        Ok(sequence)
    }

    /// Highest exactly contiguous input sequence represented by admitted state.
    pub fn resolved_through(&self) -> u64 {
        self.resolved_through
    }

    /// Number of retained pending or individually resolved higher inputs.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Reserved Rust value and Vec element-capacity bytes, excluding allocator metadata.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.pending.capacity() * std::mem::size_of::<PendingInput>()
    }

    pub(crate) fn preflight(
        &self,
        group: &GroupView<'_>,
        prior_sample_tick: u64,
    ) -> Result<Candidate, ProgressError> {
        if group.events().len() != 0 || group.results().any(|result| result.event_count != 0) {
            return Err(ProgressError::UnsupportedEvent);
        }
        if group.header.group_id != self.controlled_group_id {
            if group.results().len() != 0 || group.header.resolved_through != 0 {
                return Err(ProgressError::WrongPartition);
            }
            return Ok(Candidate {
                newly_resolved: [0; 4],
                prefix: 0,
            });
        }
        if (group.results().len() != 0 || group.header.resolved_through != 0)
            && !group
                .actors()
                .any(|actor| actor.wire_id == self.controlled_actor_id)
        {
            return Err(ProgressError::MissingController);
        }
        let mut bits = [0u64; 4];
        for result in group.results() {
            let offset = result
                .input_sequence
                .checked_sub(self.resolved_through)
                .and_then(|value| value.checked_sub(1))
                .and_then(|value| usize::try_from(value).ok())
                .ok_or(ProgressError::InvalidResult)?;
            let input = self
                .pending
                .get(offset)
                .ok_or(ProgressError::InvalidResult)?;
            let bit = 1u64 << (offset % 64);
            if input.resolved
                || bits[offset / 64] & bit != 0
                || input.action_type != result.action_type
                || result.resolved_tick <= prior_sample_tick
                || result.resolved_tick > group.header.commit_tick
            {
                return Err(ProgressError::InvalidResult);
            }
            bits[offset / 64] |= bit;
        }
        let prefix = self
            .pending
            .iter()
            .enumerate()
            .take_while(|(i, input)| input.resolved || bits[i / 64] & (1u64 << (i % 64)) != 0)
            .count();
        let expected_watermark = self.resolved_through + prefix as u64;
        if group.header.resolved_through != expected_watermark {
            return Err(ProgressError::InvalidWatermark);
        }
        Ok(Candidate {
            newly_resolved: bits,
            prefix,
        })
    }

    pub(crate) fn commit(&mut self, candidate: Candidate) {
        for (i, input) in self.pending.iter_mut().enumerate() {
            if candidate.newly_resolved[i / 64] & (1u64 << (i % 64)) != 0 {
                input.resolved = true;
            }
        }
        self.pending.drain(..candidate.prefix);
        self.resolved_through += candidate.prefix as u64;
    }
}
