//! Bounded, game-schema-neutral input records inside an authenticated N1 envelope.
//! Structural decoding borrows checked fields; the session supplies peer/epoch
//! and a game decoder before any frame enters the future timing window.

use thrum_core::{
    Origin,
    sim::{InputFrame, Ownership},
};

/// Most input records accepted in one batch.
pub const MAX_RECORDS: usize = 8;
/// Most actions accepted in one input record.
pub const MAX_ACTIONS: usize = 8;
/// Largest game-declared action payload.
pub const MAX_ACTION_BYTES: usize = 256;
/// First V1 application datagram budget, including its separate envelope.
pub const DATAGRAM_BYTES: usize = 1_000;
const ENVELOPE_BYTES: usize = 64;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 - ENVELOPE_BYTES;

/// Structural or game-schema input refusal. Scratch output may contain partial
/// bytes after an encode fault, but no packet is sent or W1 state mutated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputCodecError {
    /// A batch has zero or more than eight records.
    RecordCount,
    /// A record declares more than eight actions.
    ActionCount,
    /// A declared action exceeds 256 bytes.
    ActionTooLarge,
    /// An axis is nonfinite or outside [-1, 1], or uses reserved wire -32768.
    InvalidAxis,
    /// Encoded bytes end before a declared field or action.
    Truncated,
    /// Bytes remain after the final declared record.
    TrailingBytes,
    /// Payload exceeds the complete 64 KiB application-message bound.
    PayloadTooLarge,
    /// Caller-provided output cannot hold the complete batch.
    DestinationTooSmall,
    /// A game schema, value or exact action length was refused.
    InvalidAction,
    /// Session actor mapping does not match the authenticated binding.
    WrongActor,
}

/// Borrowed, structurally checked game action.
#[derive(Clone, Copy, Debug)]
pub struct ActionView<'a> {
    /// Negotiated game action schema ordinal.
    pub schema_id: u32,
    /// Exact bounded action bytes, still awaiting game validation.
    pub payload: &'a [u8],
}

/// Game-owned action layout. Implementations validate semantic values and
/// write/read only their declared schema. Engine N1 stores no game enum.
pub trait ActionWire<A> {
    /// Write one action into at most 256 destination bytes; return schema ID
    /// and exact byte count. Do not retain the destination.
    ///
    /// # Errors
    /// Returns an explicit schema, value or destination refusal.
    fn encode(&self, action: &A, destination: &mut [u8]) -> Result<(u32, usize), InputCodecError>;

    /// Decode one structurally checked action into the game's typed value.
    ///
    /// # Errors
    /// Returns `InvalidAction` for unknown schema, length or value.
    fn decode(&self, action: ActionView<'_>) -> Result<A, InputCodecError>;
}

/// One owned frame and its session-mapped wire actor ID for encoding.
#[derive(Clone, Copy)]
pub struct EncodeRecord<'a, A> {
    /// Exact input sequence, target tick, normalized axes and typed actions.
    pub frame: &'a InputFrame<A>,
    /// Match-scoped wire actor ID, resolved from the session actor table.
    pub actor_wire_id: u64,
}

/// One complete borrowed input record with already-framed actions.
#[derive(Clone, Copy, Debug)]
pub struct RecordView<'a> {
    /// Connection-scoped input sequence.
    pub sequence: u64,
    /// Match-scoped wire actor ID.
    pub actor_wire_id: u64,
    /// Intended authority tick.
    pub target_tick: u64,
    /// Decoded, normalized three-axis control.
    pub axes: [f32; 3],
    actions: [Option<ActionView<'a>>; MAX_ACTIONS],
    action_count: usize,
}

impl RecordView<'_> {
    /// Iterate each already-framed action in declared order.
    ///
    /// # Panics
    /// Panics only if the private decoder failed to fill a declared action slot.
    pub fn actions(&self) -> impl Iterator<Item = ActionView<'_>> {
        self.actions[..self.action_count]
            .iter()
            .map(|action| action.expect("checked action slot"))
    }

    /// Decode a whole typed frame after the session has checked its actor map.
    /// Call this for every record before staging any frame from the batch.
    ///
    /// # Errors
    /// Returns a mapping or game-schema refusal without changing N1/W1 state.
    pub fn decode_frame<A>(
        &self,
        binding: Ownership,
        actor_wire_id: u64,
        wire: &impl ActionWire<A>,
    ) -> Result<InputFrame<A>, InputCodecError> {
        if actor_wire_id != self.actor_wire_id {
            return Err(InputCodecError::WrongActor);
        }
        let mut actions = Vec::with_capacity(self.action_count);
        for action in self.actions() {
            actions.push(wire.decode(action)?);
        }
        Ok(InputFrame {
            origin: Origin {
                peer: binding.peer,
                connection_epoch: binding.connection_epoch,
                sequence: self.sequence,
            },
            actor: binding.actor,
            tick: self.target_tick,
            axes: self.axes,
            actions,
        })
    }
}

/// All structurally valid records in one borrowed payload. No heap allocation.
pub struct BatchView<'a> {
    records: [Option<RecordView<'a>>; MAX_RECORDS],
    count: usize,
}

impl BatchView<'_> {
    /// Traverse the complete checked batch in sender order.
    ///
    /// # Panics
    /// Panics only if the private decoder failed to fill a declared record slot.
    pub fn records(&self) -> impl Iterator<Item = RecordView<'_>> {
        self.records[..self.count]
            .iter()
            .map(|record| record.expect("checked record slot"))
    }

    /// Number of declared, checked records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// A checked batch is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

fn take<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], InputCodecError> {
    let end = cursor.checked_add(N).ok_or(InputCodecError::Truncated)?;
    let data = bytes.get(*cursor..end).ok_or(InputCodecError::Truncated)?;
    *cursor = end;
    Ok(data.try_into().expect("checked field width"))
}

fn put<const N: usize>(
    destination: &mut [u8],
    cursor: &mut usize,
    value: [u8; N],
) -> Result<(), InputCodecError> {
    let end = cursor
        .checked_add(N)
        .ok_or(InputCodecError::DestinationTooSmall)?;
    destination
        .get_mut(*cursor..end)
        .ok_or(InputCodecError::DestinationTooSmall)?
        .copy_from_slice(&value);
    *cursor = end;
    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "finite normalized axis and 32767 scale prove the rounded value fits i16"
)]
fn quantize(axis: f32) -> Result<i16, InputCodecError> {
    if !axis.is_finite() || axis.abs() > 1.0 {
        return Err(InputCodecError::InvalidAxis);
    }
    Ok((f64::from(axis) * 32767.0).round() as i16)
}

fn dequantize(value: i16) -> Result<f32, InputCodecError> {
    if value == i16::MIN {
        return Err(InputCodecError::InvalidAxis);
    }
    Ok(f32::from(value) / 32767.0)
}

/// Encode complete records into caller-owned scratch. A small destination may
/// refuse; callers route the whole newest frame to a reliable lane or report
/// profile refusal rather than splitting its actions.
///
/// # Errors
/// Returns count, axis, action, payload or destination refusal.
pub fn encode<A>(
    records: &[EncodeRecord<'_, A>],
    wire: &impl ActionWire<A>,
    destination: &mut [u8],
) -> Result<usize, InputCodecError> {
    if records.is_empty() || records.len() > MAX_RECORDS {
        return Err(InputCodecError::RecordCount);
    }
    let mut cursor = 0;
    put(
        destination,
        &mut cursor,
        [u8::try_from(records.len()).map_err(|_| InputCodecError::RecordCount)?],
    )?;
    for record in records {
        if record.frame.actions.len() > MAX_ACTIONS {
            return Err(InputCodecError::ActionCount);
        }
        put(
            destination,
            &mut cursor,
            record.frame.origin.sequence.to_le_bytes(),
        )?;
        put(destination, &mut cursor, record.actor_wire_id.to_le_bytes())?;
        put(destination, &mut cursor, record.frame.tick.to_le_bytes())?;
        for &axis in &record.frame.axes {
            put(destination, &mut cursor, quantize(axis)?.to_le_bytes())?;
        }
        put(
            destination,
            &mut cursor,
            [
                u8::try_from(record.frame.actions.len())
                    .map_err(|_| InputCodecError::ActionCount)?,
            ],
        )?;
        for action in &record.frame.actions {
            let payload_start = cursor
                .checked_add(6)
                .ok_or(InputCodecError::DestinationTooSmall)?;
            let available = destination
                .len()
                .saturating_sub(payload_start)
                .min(MAX_ACTION_BYTES);
            let action_destination = destination
                .get_mut(payload_start..payload_start + available)
                .ok_or(InputCodecError::DestinationTooSmall)?;
            let (schema_id, payload_len) = wire.encode(action, action_destination)?;
            if payload_len > MAX_ACTION_BYTES {
                return Err(InputCodecError::ActionTooLarge);
            }
            if payload_len > available {
                return Err(InputCodecError::DestinationTooSmall);
            }
            put(destination, &mut cursor, schema_id.to_le_bytes())?;
            put(
                destination,
                &mut cursor,
                u16::try_from(payload_len)
                    .map_err(|_| InputCodecError::ActionTooLarge)?
                    .to_le_bytes(),
            )?;
            cursor += payload_len;
        }
        if cursor > MAX_PAYLOAD_BYTES {
            return Err(InputCodecError::PayloadTooLarge);
        }
    }
    Ok(cursor)
}

/// Structurally decode the complete payload before game-schema validation or
/// staging. Every action view borrows the original checked byte slice.
///
/// # Errors
/// Returns a count, length, axis or trailing-byte refusal.
pub fn decode(payload: &[u8]) -> Result<BatchView<'_>, InputCodecError> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(InputCodecError::PayloadTooLarge);
    }
    let mut cursor = 0;
    let count = usize::from(take::<1>(payload, &mut cursor)?[0]);
    if count == 0 || count > MAX_RECORDS {
        return Err(InputCodecError::RecordCount);
    }
    let mut records = [None; MAX_RECORDS];
    for slot in records.iter_mut().take(count) {
        let sequence = u64::from_le_bytes(take(payload, &mut cursor)?);
        let actor_wire_id = u64::from_le_bytes(take(payload, &mut cursor)?);
        let target_tick = u64::from_le_bytes(take(payload, &mut cursor)?);
        let mut axes = [0.0; 3];
        for axis in &mut axes {
            *axis = dequantize(i16::from_le_bytes(take(payload, &mut cursor)?))?;
        }
        let action_count = usize::from(take::<1>(payload, &mut cursor)?[0]);
        if action_count > MAX_ACTIONS {
            return Err(InputCodecError::ActionCount);
        }
        let mut actions = [None; MAX_ACTIONS];
        for action in actions.iter_mut().take(action_count) {
            let schema_id = u32::from_le_bytes(take(payload, &mut cursor)?);
            let action_len = usize::from(u16::from_le_bytes(take(payload, &mut cursor)?));
            if action_len > MAX_ACTION_BYTES {
                return Err(InputCodecError::ActionTooLarge);
            }
            let end = cursor
                .checked_add(action_len)
                .ok_or(InputCodecError::Truncated)?;
            let bytes = payload.get(cursor..end).ok_or(InputCodecError::Truncated)?;
            cursor = end;
            *action = Some(ActionView {
                schema_id,
                payload: bytes,
            });
        }
        *slot = Some(RecordView {
            sequence,
            actor_wire_id,
            target_tick,
            axes,
            actions,
            action_count,
        });
    }
    if cursor != payload.len() {
        return Err(InputCodecError::TrailingBytes);
    }
    Ok(BatchView { records, count })
}
