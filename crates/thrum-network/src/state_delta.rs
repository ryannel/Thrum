//! Allocation-free V1 state-delta bytes over a game-declared field schema.
//! This first codec handles existing-actor datagrams. Session, lifecycle,
//! result closure and image retention remain separate owners.

mod bit;

use bit::{Reader, Writer};

/// Maximum actors in one partition datagram.
pub const MAX_ACTORS: usize = 32;
/// Maximum fields declared by one game projection schema.
pub const MAX_FIELDS: usize = 64;
/// Fixed V1 state-group payload header width.
pub const HEADER_BYTES: usize = 56;
/// Largest complete state-group application message, including its envelope.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024;
const ENVELOPE_BYTES: usize = 64;
const DELTA_MODE: u8 = 0;

/// Refusal before a candidate becomes confirmed state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeltaError {
    /// Schema ID, field count or field width is invalid.
    Schema,
    /// An authoritative field does not fit its declared bits or range.
    FieldRange,
    /// Output cannot hold a complete packet.
    DestinationTooSmall,
    /// Input ended before a declared field.
    Truncated,
    /// Input has bytes after its final actor.
    TrailingBytes,
    /// A varint overflows or is not shortest-form.
    Varint,
    /// Reserved or record-end padding bits are nonzero.
    Padding,
    /// Actor IDs repeat, regress or differ from the named base image.
    ActorOrder,
    /// Packet scope, base image, sample identity or sample tick is invalid.
    Base,
    /// A mode or record count is unsupported by this first delta codec.
    Mode,
    /// The group exceeds the complete 8 KiB application-message limit.
    MessageTooLarge,
}

/// Game-owned quantized projection. The engine controls bit widths, mask and
/// canonical framing; the game maps checked field ordinals to typed values.
/// A schema declaration is pinned by the session protocol hash.
pub trait FieldCodec<T: Copy> {
    /// Negotiated, nonzero game schema ordinal.
    const SCHEMA_ID: u32;
    /// Field widths in stable schema order, each in 1..=64.
    const FIELD_BITS: &'static [u8];

    /// Read one already-quantized field as an unsigned bit pattern.
    ///
    /// # Errors
    /// Refuses a projection outside the game's declared numeric envelope.
    fn field(&self, actor: &T, ordinal: usize) -> Result<u64, DeltaError>;

    /// Set one checked-width field in scratch, validating its game meaning.
    ///
    /// # Errors
    /// Refuses a reserved or out-of-range bit pattern.
    fn set_field(&self, actor: &mut T, ordinal: usize, bits: u64) -> Result<(), DeltaError>;

    /// Validate cross-field meaning after a whole actor has decoded.
    ///
    /// # Errors
    /// Refuses an invalid decoded projection.
    fn validate(&self, actor: &T) -> Result<(), DeltaError>;
}

/// Generate the ordinal-to-field mapping for a game-owned integer projection.
/// The declaration is part of the session's pinned protocol schema; callers
/// still quantize physical values and check their game range before encoding.
#[macro_export]
macro_rules! state_schema {
    ($vis:vis struct $name:ident for $actor:ty, schema = $id:literal {
        $( $ordinal:literal => $field:ident : $bits:literal ),+ $(,)?
    }) => {
        const _: () = {
            const ORDINALS: &[usize] = &[$($ordinal),+];
            let mut index = 0;
            while index < ORDINALS.len() {
                assert!(ORDINALS[index] == index, "state schema ordinals must be consecutive");
                index += 1;
            }
        };

        #[derive(Clone, Copy)]
        $vis struct $name;

        impl $crate::state_delta::FieldCodec<$actor> for $name {
            const SCHEMA_ID: u32 = $id;
            const FIELD_BITS: &'static [u8] = &[$($bits),+];

            fn field(
                &self,
                actor: &$actor,
                ordinal: usize,
            ) -> Result<u64, $crate::state_delta::DeltaError> {
                match ordinal {
                    $( $ordinal => Ok(u64::from(actor.$field)), )+
                    _ => Err($crate::state_delta::DeltaError::Schema),
                }
            }

            fn set_field(
                &self,
                actor: &mut $actor,
                ordinal: usize,
                bits: u64,
            ) -> Result<(), $crate::state_delta::DeltaError> {
                match ordinal {
                    $( $ordinal => {
                        actor.$field = bits
                            .try_into()
                            .map_err(|_| $crate::state_delta::DeltaError::FieldRange)?;
                        Ok(())
                    }, )+
                    _ => Err($crate::state_delta::DeltaError::Schema),
                }
            }

            fn validate(&self, _actor: &$actor) -> Result<(), $crate::state_delta::DeltaError> {
                Ok(())
            }
        }
    };
}

/// One match-scoped wire ID and game-declared projection value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Actor<T> {
    /// Never-reused ID within a match.
    pub wire_id: u64,
    /// Already-quantized, game-owned projection.
    pub value: T,
}

/// A stable partition incarnation. A lifecycle transition changes generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scope {
    /// Stable partition ID.
    pub partition_id: u32,
    /// Nonzero interest generation.
    pub interest_generation: u32,
}

/// An immutable, already-admitted quantized base image.
#[derive(Clone, Copy)]
pub struct Image<'a, T> {
    /// Partition identity and interest generation.
    pub scope: Scope,
    /// Unique connection-wide image ID.
    pub id: u64,
    /// Authority sample tick represented by this image.
    pub tick: u64,
    /// Complete sorted partition actor projection.
    pub actors: &'a [Actor<T>],
}

/// Checked information from one encoded or decoded delta.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaReport {
    /// Unique sample image ID.
    pub image_id: u64,
    /// Authority tick represented by the sample.
    pub tick: u64,
    /// Number of changed actors in the packet.
    pub changed_actors: usize,
    /// Payload bytes, excluding the separate application envelope.
    pub bytes: usize,
}

fn schema_valid<T: Copy, S: FieldCodec<T>>() -> Result<(), DeltaError> {
    let bits = S::FIELD_BITS;
    if S::SCHEMA_ID == 0
        || bits.is_empty()
        || bits.len() > MAX_FIELDS
        || bits.iter().any(|width| !(1..=64).contains(width))
    {
        return Err(DeltaError::Schema);
    }
    Ok(())
}

fn actors_valid<T>(actors: &[Actor<T>]) -> bool {
    actors.len() <= MAX_ACTORS
        && actors.first().is_none_or(|actor| actor.wire_id != 0)
        && actors
            .windows(2)
            .all(|pair| pair[0].wire_id < pair[1].wire_id)
}

fn put_u16(bytes: &mut [u8], start: usize, value: u16) {
    bytes[start..start + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], start: usize, value: u32) {
    bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], start: usize, value: u64) {
    bytes[start..start + 8].copy_from_slice(&value.to_le_bytes());
}

fn get_u16(bytes: &[u8], start: usize) -> u16 {
    u16::from_le_bytes(bytes[start..start + 2].try_into().expect("checked header"))
}

fn get_u32(bytes: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(bytes[start..start + 4].try_into().expect("checked header"))
}

fn get_u64(bytes: &[u8], start: usize) -> u64 {
    u64::from_le_bytes(bytes[start..start + 8].try_into().expect("checked header"))
}

fn packet_limit(bytes: usize) -> Result<(), DeltaError> {
    if bytes
        .checked_add(ENVELOPE_BYTES)
        .is_none_or(|total| total > MAX_MESSAGE_BYTES)
    {
        Err(DeltaError::MessageTooLarge)
    } else {
        Ok(())
    }
}

/// Encode all changed fields of existing sorted actors against one admitted
/// image. The returned current projection is the exact candidate image only
/// when every changed actor fits this packet. A capacity refusal sends nothing.
///
/// # Errors
/// Refuses invalid schema, identity, range, sample order or destination size.
pub fn encode_delta<T: Copy, S: FieldCodec<T>>(
    schema: &S,
    base: Image<'_, T>,
    current: &[Actor<T>],
    image_id: u64,
    tick: u64,
    destination: &mut [u8],
) -> Result<DeltaReport, DeltaError> {
    schema_valid::<T, S>()?;
    if !actors_valid(base.actors)
        || base.actors.len() != current.len()
        || base.scope.interest_generation == 0
        || base.id == 0
        || image_id <= base.id
        || tick <= base.tick
    {
        return Err(DeltaError::Base);
    }
    if destination.len() < HEADER_BYTES {
        return Err(DeltaError::DestinationTooSmall);
    }
    let mut writer = Writer::new(destination, HEADER_BYTES * 8);
    let mask_bytes = S::FIELD_BITS.len().div_ceil(8);
    let mut previous_id = 0u64;
    let mut changed = 0usize;
    for (before, after) in base.actors.iter().zip(current) {
        if before.wire_id != after.wire_id {
            return Err(DeltaError::ActorOrder);
        }
        schema.validate(&after.value)?;
        let mut mask = 0u64;
        let mut values = [0u64; MAX_FIELDS];
        for (index, &width) in S::FIELD_BITS.iter().enumerate() {
            let old = schema.field(&before.value, index)?;
            let new = schema.field(&after.value, index)?;
            if (width < 64 && (old >> width != 0 || new >> width != 0)) || width == 0 {
                return Err(DeltaError::FieldRange);
            }
            values[index] = new;
            if old != new {
                mask |= 1u64 << index;
            }
        }
        if mask == 0 {
            continue;
        }
        writer.put_var(after.wire_id - previous_id)?;
        previous_id = after.wire_id;
        for index in 0..mask_bytes {
            writer.put_bits((mask >> (index * 8)) & 0xff, 8)?;
        }
        for (index, &width) in S::FIELD_BITS.iter().enumerate() {
            if mask & (1u64 << index) != 0 {
                writer.put_bits(values[index], width)?;
            }
        }
        writer.align()?;
        changed += 1;
    }
    let bytes = writer.position();
    packet_limit(bytes)?;
    destination[0] = DELTA_MODE;
    put_u16(
        destination,
        2,
        u16::try_from(changed).map_err(|_| DeltaError::Mode)?,
    );
    put_u32(destination, 12, base.scope.partition_id);
    put_u32(destination, 16, base.scope.interest_generation);
    put_u32(destination, 20, S::SCHEMA_ID);
    put_u64(destination, 24, base.id);
    put_u64(destination, 32, image_id);
    put_u64(destination, 40, tick);
    Ok(DeltaReport {
        image_id,
        tick,
        changed_actors: changed,
        bytes,
    })
}

/// Decode one datagram into scratch copied from its named immutable base.
/// Scratch is **not** confirmed on error; the caller publishes it only after
/// this function and its session checks succeed. This first codec requires
/// unchanged actor membership, so lifecycle uses the separate reliable path.
///
/// # Errors
/// Refuses malformed, stale, misbound, out-of-range or incomplete input.
pub fn decode_delta<T: Copy, S: FieldCodec<T>>(
    schema: &S,
    base: Image<'_, T>,
    bytes: &[u8],
    scratch: &mut [Actor<T>],
) -> Result<DeltaReport, DeltaError> {
    schema_valid::<T, S>()?;
    if bytes.len() < HEADER_BYTES {
        return Err(DeltaError::Truncated);
    }
    packet_limit(bytes.len())?;
    if bytes[0] != DELTA_MODE
        || get_u16(bytes, 4) != 0
        || get_u16(bytes, 6) != 0
        || get_u16(bytes, 8) != 0
    {
        return Err(DeltaError::Mode);
    }
    if bytes[1] != 0 || get_u16(bytes, 10) != 0 || get_u64(bytes, 48) != 0 {
        return Err(DeltaError::Padding);
    }
    let count = usize::from(get_u16(bytes, 2));
    if count > MAX_ACTORS {
        return Err(DeltaError::Mode);
    }
    let image_id = get_u64(bytes, 32);
    let tick = get_u64(bytes, 40);
    if !actors_valid(base.actors)
        || scratch.len() != base.actors.len()
        || base.scope.interest_generation == 0
        || get_u32(bytes, 12) != base.scope.partition_id
        || get_u32(bytes, 16) != base.scope.interest_generation
        || get_u32(bytes, 20) != S::SCHEMA_ID
        || get_u64(bytes, 24) != base.id
        || image_id <= base.id
        || tick <= base.tick
    {
        return Err(DeltaError::Base);
    }
    scratch.copy_from_slice(base.actors);
    let mut reader = Reader::new(bytes, HEADER_BYTES * 8);
    let mask_bytes = S::FIELD_BITS.len().div_ceil(8);
    let mut previous_id = 0u64;
    for _ in 0..count {
        let difference = reader.get_var()?;
        if difference == 0 {
            return Err(DeltaError::ActorOrder);
        }
        let wire_id = previous_id
            .checked_add(difference)
            .ok_or(DeltaError::ActorOrder)?;
        previous_id = wire_id;
        let index = base
            .actors
            .binary_search_by_key(&wire_id, |actor| actor.wire_id)
            .map_err(|_| DeltaError::ActorOrder)?;
        let mut mask = 0u64;
        for byte in 0..mask_bytes {
            mask |= reader.get_bits(8)? << (byte * 8);
        }
        if mask == 0 || (S::FIELD_BITS.len() < 64 && mask >> S::FIELD_BITS.len() != 0) {
            return Err(DeltaError::Padding);
        }
        for (field, &width) in S::FIELD_BITS.iter().enumerate() {
            if mask & (1u64 << field) != 0 {
                let value = reader.get_bits(width)?;
                schema.set_field(&mut scratch[index].value, field, value)?;
            }
        }
        schema.validate(&scratch[index].value)?;
        reader.align()?;
    }
    if reader.position() != bytes.len() {
        return Err(DeltaError::TrailingBytes);
    }
    Ok(DeltaReport {
        image_id,
        tick,
        changed_actors: count,
        bytes: bytes.len(),
    })
}
