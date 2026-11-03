//! Finite N1-owned unsent R4 payload slots. Submission sequence and exact
//! result/event ACK retirement belong to the later sent-frame owner.

use crate::r4::{self, GroupHeader, GroupView};

/// Maximum peer-owned unsent R4 payload units in the first profile.
pub const MAX_UNSENT_GROUPS: usize = 16;
const MAX_PARTITIONS: usize = 4;
const PAYLOAD_STORAGE_BYTES: usize = MAX_UNSENT_GROUPS * r4::MAX_PAYLOAD_BYTES;

/// One admitted baseline-bound partition's capture cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionSeed {
    /// Stable group identity from the admitted baseline.
    pub group_id: u32,
    /// Nonzero interest generation.
    pub interest_generation: u32,
    /// Exact baseline sample tick.
    pub sample_tick: u64,
    /// Controlled partition's contiguous input watermark, or zero.
    pub resolved_through: u64,
}

/// The producer's checked state coverage for this partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateCoverage {
    /// Every visible actor in the partition is present at this sample tick.
    CompletePartition,
    /// Only selected actors are present; this unit cannot be coalesced.
    Partial,
}

/// Game-specific schema, membership and dependency-closure check before
/// owned bytes enter the queue. It must be pure; refusal leaves queue state.
pub trait OutboundValidator {
    /// Validate all scalar fields and required state/event closure; certify
    /// whether actors form a complete partition replacement.
    fn group(&self, group: &GroupView<'_>) -> Option<StateCoverage>;
}

/// Capture refusal before bytes or a submission sequence are committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutboundError {
    /// An external transfer callback unwound; peer teardown is required.
    Poisoned,
    /// Baseline partition seed is duplicate, empty, invalid or too large.
    InvalidBaseline,
    /// R4 fixed framing or local result shape failed.
    Framing(r4::GroupError),
    /// Game schema or dependency closure refused.
    Schema,
    /// Group ID was not in the admitted baseline.
    UnknownPartition,
    /// Sample tick failed to advance in this partition.
    StaleTick,
    /// A group arrived behind an already captured authority barrier.
    OutOfOrderBarrier,
    /// Interest generation skipped, regressed or wrapped.
    InterestGeneration,
    /// An interest transition omitted complete current partition state.
    IncompleteGeneration,
    /// Contiguous result watermark regressed.
    Watermark,
    /// Sixteen unsent slots are occupied; caller must resync/close the peer.
    Capacity,
}

/// Transfer refusal. A poisoned queue must close its peer; a sink error is
/// retryable only when that sink made no external mutation before returning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferError<E> {
    /// A prior callback unwound after possibly publishing an external frame.
    Poisoned,
    /// The sink refused without mutating its own sent-frame owner.
    Sink(E),
}

#[derive(Clone, Copy)]
struct Slot {
    header: GroupHeader,
    len: u16,
    mandatory: bool,
}

#[derive(Clone, Copy)]
struct Partition {
    group_id: u32,
    interest_generation: u32,
    last_tick: u64,
    resolved_through: u64,
}

/// One borrowed, N1-owned payload awaiting exact sent-frame reservation.
pub struct QueuedGroup<'a> {
    /// Stable partition, interest generation, exact tick and watermark.
    pub header: GroupHeader,
    /// Copied fixed R4 payload bytes.
    pub payload: &'a [u8],
    /// Partial state, lifecycle, result, event, generation or watermark prevents
    /// coalescing this unit before submission.
    pub mandatory: bool,
}

/// One finite peer's pre-submission units. Payload bytes are in one fixed
/// backing allocation; metadata and chronological order are inline.
pub struct UnsentQueue {
    payloads: Box<[u8]>,
    slots: [Option<Slot>; MAX_UNSENT_GROUPS],
    order: [u8; MAX_UNSENT_GROUPS],
    len: usize,
    partitions: [Option<Partition>; MAX_PARTITIONS],
    last_barrier_tick: u64,
    transfer_in_progress: bool,
}

impl UnsentQueue {
    /// Reserve the exact payload arena after validating baseline cursors.
    /// No N1 queue allocation occurs when the capability is disabled.
    pub fn from_baseline(seeds: &[PartitionSeed]) -> Result<Self, OutboundError> {
        if seeds.is_empty() || seeds.len() > MAX_PARTITIONS {
            return Err(OutboundError::InvalidBaseline);
        }
        let mut partitions = [None; MAX_PARTITIONS];
        for (i, seed) in seeds.iter().enumerate() {
            if seed.interest_generation == 0
                || seeds[..i]
                    .iter()
                    .any(|prior| prior.group_id == seed.group_id)
                || seed.sample_tick != seeds[0].sample_tick
            {
                return Err(OutboundError::InvalidBaseline);
            }
            partitions[i] = Some(Partition {
                group_id: seed.group_id,
                interest_generation: seed.interest_generation,
                last_tick: seed.sample_tick,
                resolved_through: seed.resolved_through,
            });
        }
        Ok(Self {
            payloads: vec![0; PAYLOAD_STORAGE_BYTES].into_boxed_slice(),
            slots: [None; MAX_UNSENT_GROUPS],
            order: [0; MAX_UNSENT_GROUPS],
            len: 0,
            partitions,
            last_barrier_tick: seeds[0].sample_tick,
            transfer_in_progress: false,
        })
    }

    /// Number of retained, still-unsubmitted units.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether no unit is awaiting submission.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Fixed Rust owner bytes and exact payload arena, excluding allocator metadata.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.payloads.len()
    }

    /// Validate and copy a complete R4 payload at the authority publication
    /// barrier. State-only units can replace only the last unit for their own
    /// partition incarnation, and move to the chronological tail. Mandatory
    /// bytes are never coalesced. No envelope sequence is assigned here.
    pub fn capture(
        &mut self,
        payload: &[u8],
        validator: &impl OutboundValidator,
    ) -> Result<(), OutboundError> {
        if self.transfer_in_progress {
            return Err(OutboundError::Poisoned);
        }
        let group = r4::decode(payload).map_err(OutboundError::Framing)?;
        let index = self
            .partitions
            .iter()
            .position(|partition| partition.is_some_and(|p| p.group_id == group.header.group_id))
            .ok_or(OutboundError::UnknownPartition)?;
        let partition = self.partitions[index].expect("located partition");
        if group.header.commit_tick <= partition.last_tick {
            return Err(OutboundError::StaleTick);
        }
        if group.header.commit_tick < self.last_barrier_tick {
            return Err(OutboundError::OutOfOrderBarrier);
        }
        let generation_changed = group.header.interest_generation != partition.interest_generation;
        if generation_changed
            && partition.interest_generation.checked_add(1)
                != Some(group.header.interest_generation)
        {
            return Err(OutboundError::InterestGeneration);
        }
        if group.header.resolved_through < partition.resolved_through {
            return Err(OutboundError::Watermark);
        }
        let coverage = validator.group(&group).ok_or(OutboundError::Schema)?;
        if generation_changed && coverage != StateCoverage::CompletePartition {
            return Err(OutboundError::IncompleteGeneration);
        }
        let mandatory = generation_changed
            || coverage == StateCoverage::Partial
            || group.lifecycle().len() != 0
            || group.results().len() != 0
            || group.events().len() != 0
            || group.header.resolved_through > partition.resolved_through;
        let previous =
            self.order[..self.len]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(position, slot)| {
                    self.slots[*slot as usize]
                        .filter(|meta| meta.header.group_id == group.header.group_id)
                        .map(|meta| (position, *slot as usize, meta))
                });
        let coalesced = if mandatory {
            None
        } else {
            previous.and_then(|(position, slot, meta)| {
                (!meta.mandatory
                    && meta.header.interest_generation == group.header.interest_generation)
                    .then_some((position, slot))
            })
        };
        let slot = if let Some((_, slot)) = coalesced {
            slot
        } else {
            if self.len == MAX_UNSENT_GROUPS {
                return Err(OutboundError::Capacity);
            }
            self.slots
                .iter()
                .position(Option::is_none)
                .expect("free slot under capacity")
        };
        let start = slot * r4::MAX_PAYLOAD_BYTES;
        self.payloads[start..start + payload.len()].copy_from_slice(payload);
        self.slots[slot] = Some(Slot {
            header: group.header,
            len: payload.len() as u16,
            mandatory,
        });
        if let Some((position, _)) = coalesced {
            self.order.copy_within(position + 1..self.len, position);
            self.order[self.len - 1] = slot as u8;
        } else {
            self.order[self.len] = slot as u8;
            self.len += 1;
        }
        self.partitions[index] = Some(Partition {
            group_id: partition.group_id,
            interest_generation: group.header.interest_generation,
            last_tick: group.header.commit_tick,
            resolved_through: group.header.resolved_through,
        });
        self.last_barrier_tick = group.header.commit_tick;
        Ok(())
    }

    /// Borrow the earliest chronological unit. Its bytes remain stable until
    /// a successful `transfer_next` or a same-partition state-only coalescing.
    pub fn peek_next(&self) -> Result<Option<QueuedGroup<'_>>, OutboundError> {
        if self.transfer_in_progress {
            return Err(OutboundError::Poisoned);
        }
        if self.len == 0 {
            return Ok(None);
        }
        Ok(Some(self.next_unit_unchecked()))
    }

    fn next_unit_unchecked(&self) -> QueuedGroup<'_> {
        let slot = self.order[0] as usize;
        let meta = self.slots[slot].expect("ordered occupied slot");
        let start = slot * r4::MAX_PAYLOAD_BYTES;
        QueuedGroup {
            header: meta.header,
            payload: &self.payloads[start..start + usize::from(meta.len)],
            mandatory: meta.mandatory,
        }
    }

    /// Transfer the earliest unit to an external sent-frame owner. The
    /// callback must reserve exact bytes and ACK metadata before assigning its
    /// envelope sequence, and may fail without mutating its own state. Only a
    /// successful callback frees this unsent slot. No remote ACK occurs here.
    pub fn transfer_next<E>(
        &mut self,
        transfer: impl FnOnce(QueuedGroup<'_>) -> Result<(), E>,
    ) -> Result<bool, TransferError<E>> {
        if self.transfer_in_progress {
            return Err(TransferError::Poisoned);
        }
        if self.len == 0 {
            return Ok(false);
        }
        // If the external sink unwinds after an uncertain publication, this
        // flag remains set and no later caller can reuse the queued bytes.
        self.transfer_in_progress = true;
        let result = transfer(self.next_unit_unchecked());
        if let Err(error) = result {
            self.transfer_in_progress = false;
            return Err(TransferError::Sink(error));
        }
        let slot = self.order[0] as usize;
        self.order.copy_within(1..self.len, 0);
        self.len -= 1;
        self.slots[slot] = None;
        self.transfer_in_progress = false;
        Ok(true)
    }
}
