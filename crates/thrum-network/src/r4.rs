//! Fixed-size R4 StateGroup payload records and bounded, borrowed decoding.
//! The 64-byte application envelope and game manifest are separate owners.

/// Largest first-profile StateGroup, including its separate 64-byte envelope.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;
/// The R4 StateGroup payload header length.
pub const HEADER_BYTES: usize = 32;
/// An R4 actor record's encoded length.
pub const ACTOR_BYTES: usize = 56;
/// An R4 lifecycle record's encoded length.
pub const LIFECYCLE_BYTES: usize = 16;
/// An R4 action-result record's encoded length.
pub const RESULT_BYTES: usize = 32;
/// An R4 event record's encoded length.
pub const EVENT_BYTES: usize = 40;
/// Exact maximum fixed R4 payload under all four record-count caps.
pub const MAX_PAYLOAD_BYTES: usize = HEADER_BYTES
    + MAX_ACTORS * ACTOR_BYTES
    + MAX_LIFECYCLE * LIFECYCLE_BYTES
    + MAX_RESULTS * RESULT_BYTES
    + MAX_EVENTS * EVENT_BYTES;

const ENVELOPE_BYTES: usize = 64;
const MAX_ACTORS: usize = 32;
const MAX_LIFECYCLE: usize = 32;
const MAX_RESULTS: usize = 64;
const MAX_EVENTS: usize = 64;

/// A malformed or unsupported fixed R4 StateGroup payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupError {
    /// A record count exceeds the first-profile bound.
    CountExceeded,
    /// Framing length is wrong for the declared counts.
    LengthMismatch,
    /// The encoded group would exceed the complete 8 KiB message limit.
    MessageTooLarge,
    /// The supplied destination cannot hold the complete payload.
    DestinationTooSmall,
    /// A result has an invalid disposition, reason or immediate-event range.
    InvalidResult,
}

/// The fixed R4 group header. Partition identity is stable across publications.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupHeader {
    /// Baseline-bound partition identity.
    pub group_id: u32,
    /// Incarnation of this partition's interest membership.
    pub interest_generation: u32,
    /// World tick of this group's exact state sample.
    pub commit_tick: u64,
    /// Contiguous input-result watermark for the controlled partition.
    pub resolved_through: u64,
}

/// One exact 56-byte R4 actor projection, with integer world units from the manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actor {
    /// Match-scoped, never-reused wire entity ID.
    pub wire_id: u64,
    /// Three integer world sectors.
    pub sector: [i32; 3],
    /// Offset inside the declared sector scale.
    pub offset: [i32; 3],
    /// Quantized quaternion components.
    pub quaternion: [i16; 4],
    /// Quantized velocity components.
    pub velocity: [i16; 3],
    /// Game-manifest mode ordinal.
    pub mode: u8,
    /// Game-manifest actor flags.
    pub flags: u8,
    /// Quantized health.
    pub health: u16,
    /// Inventory revision; the match resets before aliasing after wrap.
    pub inventory_revision: u16,
    /// Owner ordinal in the negotiated manifest.
    pub owner: u16,
    /// R4 actor visibility generation; rebaseline before aliasing after wrap.
    pub visibility_generation: u16,
}

/// One exact 16-byte R4 lifecycle entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lifecycle {
    /// Entity affected by the transition.
    pub wire_id: u64,
    /// Full visibility generation carried by lifecycle records.
    pub visibility_generation: u32,
    /// Entity schema ordinal in the protocol manifest.
    pub schema_id: u16,
    /// Manifest-declared spawn/despawn/leave/re-entry kind.
    pub kind: u8,
    /// Manifest-declared transition flags.
    pub flags: u8,
}

/// One exact 32-byte committed or rejected action result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionResult {
    /// Connection-scoped input sequence.
    pub input_sequence: u64,
    /// Authority tick at which the result resolved.
    pub resolved_tick: u64,
    /// First contiguous immediate-event ID, or zero when there are none.
    pub first_event_id: u64,
    /// Manifest-declared action type.
    pub action_type: u32,
    /// Zero on commit; game-declared rejection reason otherwise.
    pub reason: u16,
    /// Number of contiguous immediate events for this result.
    pub event_count: u8,
    /// Zero for committed, one for rejected.
    pub disposition: u8,
}

impl ActionResult {
    fn validate(self) -> Result<(), GroupError> {
        let event_range_valid = (self.event_count == 0 && self.first_event_id == 0)
            || (self.event_count != 0
                && self.first_event_id != 0
                && self
                    .first_event_id
                    .checked_add(u64::from(self.event_count) - 1)
                    .is_some());
        if !event_range_valid
            || (self.disposition == 0 && self.reason != 0)
            || (self.disposition == 1 && self.event_count != 0)
            || self.disposition > 1
        {
            return Err(GroupError::InvalidResult);
        }
        Ok(())
    }
}

/// One exact 40-byte R4 event with an optional input origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// Match-scoped event ID.
    pub event_id: u64,
    /// Authority commit tick of this event.
    pub commit_tick: u64,
    /// Origin connection epoch, or zero with an absent origin.
    pub origin_epoch: u64,
    /// Origin input sequence, or zero with an absent origin.
    pub origin_input_sequence: u64,
    /// Manifest-declared effect kind.
    pub kind: u32,
    /// Ordinal within the origin action.
    pub ordinal: u16,
    /// Manifest-declared event flags.
    pub flags: u16,
}

/// Borrowed input to the fixed R4 payload encoder.
pub struct GroupInput<'a> {
    /// Partition/tick metadata.
    pub header: GroupHeader,
    /// Complete actor records selected for this group.
    pub actors: &'a [Actor],
    /// Ordered lifecycle transitions.
    pub lifecycle: &'a [Lifecycle],
    /// Individually resolved input outcomes.
    pub results: &'a [ActionResult],
    /// Identified events.
    pub events: &'a [Event],
}

/// A parsed, zero-allocation view of an R4 StateGroup payload.
pub struct GroupView<'a> {
    /// Parsed partition and tick metadata.
    pub header: GroupHeader,
    actor_bytes: &'a [u8],
    lifecycle_bytes: &'a [u8],
    result_bytes: &'a [u8],
    event_bytes: &'a [u8],
}

fn payload_len(counts: [usize; 4]) -> Result<usize, GroupError> {
    if counts[0] > MAX_ACTORS
        || counts[1] > MAX_LIFECYCLE
        || counts[2] > MAX_RESULTS
        || counts[3] > MAX_EVENTS
    {
        return Err(GroupError::CountExceeded);
    }
    let size = HEADER_BYTES
        + counts[0] * ACTOR_BYTES
        + counts[1] * LIFECYCLE_BYTES
        + counts[2] * RESULT_BYTES
        + counts[3] * EVENT_BYTES;
    if size + ENVELOPE_BYTES > MAX_MESSAGE_BYTES {
        return Err(GroupError::MessageTooLarge);
    }
    Ok(size)
}

fn put<const N: usize>(target: &mut [u8], cursor: &mut usize, value: [u8; N]) {
    target[*cursor..*cursor + N].copy_from_slice(&value);
    *cursor += N;
}

fn get<const N: usize>(source: &[u8], cursor: &mut usize) -> [u8; N] {
    let value = source[*cursor..*cursor + N]
        .try_into()
        .expect("validated fixed record");
    *cursor += N;
    value
}

impl Actor {
    fn write(self, target: &mut [u8]) {
        let mut cursor = 0;
        put(target, &mut cursor, self.wire_id.to_le_bytes());
        for value in self.sector {
            put(target, &mut cursor, value.to_le_bytes());
        }
        for value in self.offset {
            put(target, &mut cursor, value.to_le_bytes());
        }
        for value in self.quaternion {
            put(target, &mut cursor, value.to_le_bytes());
        }
        for value in self.velocity {
            put(target, &mut cursor, value.to_le_bytes());
        }
        put(target, &mut cursor, [self.mode]);
        put(target, &mut cursor, [self.flags]);
        for value in [
            self.health,
            self.inventory_revision,
            self.owner,
            self.visibility_generation,
        ] {
            put(target, &mut cursor, value.to_le_bytes());
        }
        debug_assert_eq!(cursor, ACTOR_BYTES);
    }

    fn read(source: &[u8]) -> Self {
        let mut cursor = 0;
        let wire_id = u64::from_le_bytes(get(source, &mut cursor));
        let sector = std::array::from_fn(|_| i32::from_le_bytes(get(source, &mut cursor)));
        let offset = std::array::from_fn(|_| i32::from_le_bytes(get(source, &mut cursor)));
        let quaternion = std::array::from_fn(|_| i16::from_le_bytes(get(source, &mut cursor)));
        let velocity = std::array::from_fn(|_| i16::from_le_bytes(get(source, &mut cursor)));
        let mode = get::<1>(source, &mut cursor)[0];
        let flags = get::<1>(source, &mut cursor)[0];
        let health = u16::from_le_bytes(get(source, &mut cursor));
        let inventory_revision = u16::from_le_bytes(get(source, &mut cursor));
        let owner = u16::from_le_bytes(get(source, &mut cursor));
        let visibility_generation = u16::from_le_bytes(get(source, &mut cursor));
        debug_assert_eq!(cursor, ACTOR_BYTES);
        Self {
            wire_id,
            sector,
            offset,
            quaternion,
            velocity,
            mode,
            flags,
            health,
            inventory_revision,
            owner,
            visibility_generation,
        }
    }
}

impl Lifecycle {
    fn write(self, target: &mut [u8]) {
        let mut cursor = 0;
        put(target, &mut cursor, self.wire_id.to_le_bytes());
        put(
            target,
            &mut cursor,
            self.visibility_generation.to_le_bytes(),
        );
        put(target, &mut cursor, self.schema_id.to_le_bytes());
        put(target, &mut cursor, [self.kind]);
        put(target, &mut cursor, [self.flags]);
        debug_assert_eq!(cursor, LIFECYCLE_BYTES);
    }

    fn read(source: &[u8]) -> Self {
        let mut cursor = 0;
        let record = Self {
            wire_id: u64::from_le_bytes(get(source, &mut cursor)),
            visibility_generation: u32::from_le_bytes(get(source, &mut cursor)),
            schema_id: u16::from_le_bytes(get(source, &mut cursor)),
            kind: get::<1>(source, &mut cursor)[0],
            flags: get::<1>(source, &mut cursor)[0],
        };
        debug_assert_eq!(cursor, LIFECYCLE_BYTES);
        record
    }
}

impl ActionResult {
    fn write(self, target: &mut [u8]) {
        let mut cursor = 0;
        for value in [self.input_sequence, self.resolved_tick, self.first_event_id] {
            put(target, &mut cursor, value.to_le_bytes());
        }
        put(target, &mut cursor, self.action_type.to_le_bytes());
        put(target, &mut cursor, self.reason.to_le_bytes());
        put(target, &mut cursor, [self.event_count]);
        put(target, &mut cursor, [self.disposition]);
        debug_assert_eq!(cursor, RESULT_BYTES);
    }

    fn read(source: &[u8]) -> Self {
        let mut cursor = 0;
        let record = Self {
            input_sequence: u64::from_le_bytes(get(source, &mut cursor)),
            resolved_tick: u64::from_le_bytes(get(source, &mut cursor)),
            first_event_id: u64::from_le_bytes(get(source, &mut cursor)),
            action_type: u32::from_le_bytes(get(source, &mut cursor)),
            reason: u16::from_le_bytes(get(source, &mut cursor)),
            event_count: get::<1>(source, &mut cursor)[0],
            disposition: get::<1>(source, &mut cursor)[0],
        };
        debug_assert_eq!(cursor, RESULT_BYTES);
        record
    }
}

impl Event {
    fn write(self, target: &mut [u8]) {
        let mut cursor = 0;
        for value in [
            self.event_id,
            self.commit_tick,
            self.origin_epoch,
            self.origin_input_sequence,
        ] {
            put(target, &mut cursor, value.to_le_bytes());
        }
        put(target, &mut cursor, self.kind.to_le_bytes());
        put(target, &mut cursor, self.ordinal.to_le_bytes());
        put(target, &mut cursor, self.flags.to_le_bytes());
        debug_assert_eq!(cursor, EVENT_BYTES);
    }

    fn read(source: &[u8]) -> Self {
        let mut cursor = 0;
        let record = Self {
            event_id: u64::from_le_bytes(get(source, &mut cursor)),
            commit_tick: u64::from_le_bytes(get(source, &mut cursor)),
            origin_epoch: u64::from_le_bytes(get(source, &mut cursor)),
            origin_input_sequence: u64::from_le_bytes(get(source, &mut cursor)),
            kind: u32::from_le_bytes(get(source, &mut cursor)),
            ordinal: u16::from_le_bytes(get(source, &mut cursor)),
            flags: u16::from_le_bytes(get(source, &mut cursor)),
        };
        debug_assert_eq!(cursor, EVENT_BYTES);
        record
    }
}

/// Encode one bounded R4 payload into caller-owned storage, without allocating.
/// The caller must separately validate manifest ordinals, partition membership,
/// actor ranges and dependency/result/event closure before publication.
pub fn encode(input: GroupInput<'_>, destination: &mut [u8]) -> Result<usize, GroupError> {
    let counts = [
        input.actors.len(),
        input.lifecycle.len(),
        input.results.len(),
        input.events.len(),
    ];
    let size = payload_len(counts)?;
    if destination.len() < size {
        return Err(GroupError::DestinationTooSmall);
    }
    for result in input.results {
        result.validate()?;
    }
    let mut cursor = 0;
    put(
        destination,
        &mut cursor,
        input.header.group_id.to_le_bytes(),
    );
    put(
        destination,
        &mut cursor,
        input.header.interest_generation.to_le_bytes(),
    );
    put(
        destination,
        &mut cursor,
        input.header.commit_tick.to_le_bytes(),
    );
    put(
        destination,
        &mut cursor,
        input.header.resolved_through.to_le_bytes(),
    );
    for count in counts {
        put(destination, &mut cursor, (count as u16).to_le_bytes());
    }
    for actor in input.actors {
        actor.write(&mut destination[cursor..cursor + ACTOR_BYTES]);
        cursor += ACTOR_BYTES;
    }
    for record in input.lifecycle {
        record.write(&mut destination[cursor..cursor + LIFECYCLE_BYTES]);
        cursor += LIFECYCLE_BYTES;
    }
    for result in input.results {
        result.write(&mut destination[cursor..cursor + RESULT_BYTES]);
        cursor += RESULT_BYTES;
    }
    for event in input.events {
        event.write(&mut destination[cursor..cursor + EVENT_BYTES]);
        cursor += EVENT_BYTES;
    }
    debug_assert_eq!(cursor, size);
    Ok(size)
}

/// Parse fixed framing and local result shape before any decoded allocation.
/// This is not replica admission: manifest ordinals, actor ranges, event
/// origins, lifecycle and result/event closure remain consumer checks.
pub fn decode(payload: &[u8]) -> Result<GroupView<'_>, GroupError> {
    if payload.len() < HEADER_BYTES {
        return Err(GroupError::LengthMismatch);
    }
    let mut cursor = 0;
    let header = GroupHeader {
        group_id: u32::from_le_bytes(get(payload, &mut cursor)),
        interest_generation: u32::from_le_bytes(get(payload, &mut cursor)),
        commit_tick: u64::from_le_bytes(get(payload, &mut cursor)),
        resolved_through: u64::from_le_bytes(get(payload, &mut cursor)),
    };
    let counts = std::array::from_fn(|_| u16::from_le_bytes(get(payload, &mut cursor)) as usize);
    let size = payload_len(counts)?;
    if payload.len() != size {
        return Err(GroupError::LengthMismatch);
    }
    let (actor_bytes, rest) = payload[HEADER_BYTES..].split_at(counts[0] * ACTOR_BYTES);
    let (lifecycle_bytes, rest) = rest.split_at(counts[1] * LIFECYCLE_BYTES);
    let (result_bytes, event_bytes) = rest.split_at(counts[2] * RESULT_BYTES);
    let view = GroupView {
        header,
        actor_bytes,
        lifecycle_bytes,
        result_bytes,
        event_bytes,
    };
    for result in view.results() {
        result.validate()?;
    }
    Ok(view)
}

impl<'a> GroupView<'a> {
    /// Borrowed record iterator; no decoded array is allocated.
    pub fn actors(&self) -> impl ExactSizeIterator<Item = Actor> + 'a {
        self.actor_bytes
            .as_chunks::<ACTOR_BYTES>()
            .0
            .iter()
            .map(|bytes| Actor::read(bytes))
    }

    /// Borrowed lifecycle iterator.
    pub fn lifecycle(&self) -> impl ExactSizeIterator<Item = Lifecycle> + 'a {
        self.lifecycle_bytes
            .as_chunks::<LIFECYCLE_BYTES>()
            .0
            .iter()
            .map(|bytes| Lifecycle::read(bytes))
    }

    /// Borrowed action-result iterator.
    pub fn results(&self) -> impl ExactSizeIterator<Item = ActionResult> + 'a {
        self.result_bytes
            .as_chunks::<RESULT_BYTES>()
            .0
            .iter()
            .map(|bytes| ActionResult::read(bytes))
    }

    /// Borrowed event iterator.
    pub fn events(&self) -> impl ExactSizeIterator<Item = Event> + 'a {
        self.event_bytes
            .as_chunks::<EVENT_BYTES>()
            .0
            .iter()
            .map(|bytes| Event::read(bytes))
    }
}
