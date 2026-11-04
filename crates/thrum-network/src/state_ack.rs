//! Exact selective state-image ACK payload. Session identity belongs to the
//! surrounding envelope and the connection owner.

use crate::state_delta::Scope;

/// Fixed selective ACK payload width.
pub const BYTES: usize = 24;

/// The one received quantized image admitted by a replica.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateAck {
    /// Partition incarnation that produced the image.
    pub scope: Scope,
    /// Connection-wide image ID, never zero.
    pub image_id: u64,
    /// Authority sample tick represented by that image, never zero.
    pub tick: u64,
}

/// Exact framing and identity refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AckError {
    /// Payload is not exactly twenty-four bytes or output is too small.
    Length,
    /// An image or interest incarnation identity is zero.
    Identity,
}

fn valid(ack: StateAck) -> bool {
    ack.scope.interest_generation != 0 && ack.image_id != 0 && ack.tick != 0
}

/// Encode one selective ACK into caller-owned bytes.
///
/// # Errors
/// Refuses invalid identity or insufficient destination space.
pub fn encode(ack: StateAck, destination: &mut [u8]) -> Result<usize, AckError> {
    if !valid(ack) {
        return Err(AckError::Identity);
    }
    let bytes = destination.get_mut(..BYTES).ok_or(AckError::Length)?;
    bytes[..4].copy_from_slice(&ack.scope.partition_id.to_le_bytes());
    bytes[4..8].copy_from_slice(&ack.scope.interest_generation.to_le_bytes());
    bytes[8..16].copy_from_slice(&ack.image_id.to_le_bytes());
    bytes[16..24].copy_from_slice(&ack.tick.to_le_bytes());
    Ok(BYTES)
}

/// Decode exactly one selective ACK payload.
///
/// # Errors
/// Refuses wrong size or zero identities.
///
/// # Panics
/// Panics only if an already checked fixed-width slice cannot convert to its array.
pub fn decode(bytes: &[u8]) -> Result<StateAck, AckError> {
    let bytes: &[u8; BYTES] = bytes.try_into().map_err(|_| AckError::Length)?;
    let ack = StateAck {
        scope: Scope {
            partition_id: u32::from_le_bytes(bytes[..4].try_into().expect("checked width")),
            interest_generation: u32::from_le_bytes(bytes[4..8].try_into().expect("checked width")),
        },
        image_id: u64::from_le_bytes(bytes[8..16].try_into().expect("checked width")),
        tick: u64::from_le_bytes(bytes[16..24].try_into().expect("checked width")),
    };
    if !valid(ack) {
        return Err(AckError::Identity);
    }
    Ok(ack)
}
