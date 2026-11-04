//! Fixed N1 application envelope. Connection state, rate limits, role and
//! per-kind payload validation belong to the session owner above this codec.

/// Exact byte width of the protocol header.
pub const HEADER_BYTES: usize = 64;
/// Maximum complete first-profile application message.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const MAGIC: [u8; 4] = *b"ENG1";
const MAJOR: u16 = 1;

/// First-version message kinds; all other ordinals are reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// Initial client negotiation request.
    Hello = 1,
    /// Host identity and limits response.
    Welcome = 2,
    /// Explicit negotiation or application rejection.
    Reject = 3,
    /// Client has verified required gameplay content.
    ContentReady = 4,
    /// Begin an exact baseline image transfer.
    BaselineBegin = 5,
    /// One numbered baseline image chunk.
    BaselineChunk = 6,
    /// Finish the baseline image transfer.
    BaselineEnd = 7,
    /// Confirm atomic admission of the complete baseline.
    BaselineAck = 8,
    /// Bounded player command records.
    InputBatch = 9,
    /// One atomic live state publication.
    StateGroup = 10,
    /// Clock sample and optional application acknowledgement.
    Clock = 11,
    /// Explicit session teardown.
    Close = 12,
    /// Staging acknowledgement for one baseline chunk.
    BaselineChunkAck = 13,
    /// Selective acknowledgement for one admitted state image.
    StateAck = 14,
}

impl TryFrom<u8> for Kind {
    type Error = EnvelopeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Hello,
            2 => Self::Welcome,
            3 => Self::Reject,
            4 => Self::ContentReady,
            5 => Self::BaselineBegin,
            6 => Self::BaselineChunk,
            7 => Self::BaselineEnd,
            8 => Self::BaselineAck,
            9 => Self::InputBatch,
            10 => Self::StateGroup,
            11 => Self::Clock,
            12 => Self::Close,
            13 => Self::BaselineChunkAck,
            14 => Self::StateAck,
            _ => return Err(EnvelopeError::ReservedKind),
        })
    }
}

/// Exact first-version header fields. The session owner validates epoch,
/// sender sequence, acknowledgement and state before payload admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// One of the fourteen first-version application kinds.
    pub kind: Kind,
    /// Match-scoped identity; zero for initial Hello.
    pub match_id: [u8; 16],
    /// Connection incarnation; zero for initial Hello.
    pub connection_epoch: u64,
    /// Per-sender application sequence; zero for initial Hello.
    pub message_sequence: u64,
    /// Associated server tick; zero for initial Hello.
    pub server_tick: u64,
    /// Highest contiguous dispositioned application sequence; zero means none.
    pub acknowledged_sequence: u64,
}

/// Borrowed, structurally decoded message. No payload array is allocated.
pub struct Message<'a> {
    /// Validated fixed envelope header.
    pub header: Header,
    /// Exact declared payload bytes, still awaiting kind and semantic checks.
    pub payload: &'a [u8],
}

/// Framing or first-version header refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Header bytes are absent or payload length differs from the declaration.
    Length,
    /// Complete application message exceeds 64 KiB.
    MessageTooLarge,
    /// Destination cannot hold the complete message.
    DestinationTooSmall,
    /// The four-byte `ENG1` magic is absent.
    Magic,
    /// Protocol major is not one.
    Version,
    /// Kind is outside the fourteen selected first-version values.
    ReservedKind,
    /// Flags or reserved header bytes are nonzero.
    ReservedField,
    /// Initial Hello carries a nonzero identity, sequence, tick or ACK.
    HelloIdentity,
}

fn total_len(payload_len: usize) -> Result<usize, EnvelopeError> {
    let total = HEADER_BYTES
        .checked_add(payload_len)
        .ok_or(EnvelopeError::MessageTooLarge)?;
    if total > MAX_MESSAGE_BYTES {
        return Err(EnvelopeError::MessageTooLarge);
    }
    Ok(total)
}

fn hello_valid(header: Header) -> bool {
    header.kind != Kind::Hello
        || (header.match_id == [0; 16]
            && header.connection_epoch == 0
            && header.message_sequence == 0
            && header.server_tick == 0
            && header.acknowledged_sequence == 0)
}

/// Encode one exact application message without allocation. The caller
/// separately validates the kind payload and live connection state.
///
/// # Errors
///
/// Returns an error for invalid framing, identity or destination capacity.
///
/// # Panics
///
/// Panics only if the checked 64 KiB message bound no longer fits a `u32`.
pub fn encode(
    header: Header,
    payload: &[u8],
    destination: &mut [u8],
) -> Result<usize, EnvelopeError> {
    let total = total_len(payload.len())?;
    if !hello_valid(header) {
        return Err(EnvelopeError::HelloIdentity);
    }
    if destination.len() < total {
        return Err(EnvelopeError::DestinationTooSmall);
    }
    destination[..4].copy_from_slice(&MAGIC);
    destination[4..6].copy_from_slice(&MAJOR.to_le_bytes());
    destination[6] = header.kind as u8;
    destination[7] = 0;
    destination[8..12].copy_from_slice(
        &u32::try_from(payload.len())
            .expect("bounded envelope payload")
            .to_le_bytes(),
    );
    destination[12..16].fill(0);
    destination[16..32].copy_from_slice(&header.match_id);
    destination[32..40].copy_from_slice(&header.connection_epoch.to_le_bytes());
    destination[40..48].copy_from_slice(&header.message_sequence.to_le_bytes());
    destination[48..56].copy_from_slice(&header.server_tick.to_le_bytes());
    destination[56..64].copy_from_slice(&header.acknowledged_sequence.to_le_bytes());
    destination[64..total].copy_from_slice(payload);
    Ok(total)
}

/// Decode fixed framing and first-version header rules before any payload
/// allocation or kind-specific parsing. State and rate checks are still due.
///
/// # Errors
///
/// Returns an error for invalid framing, identity or destination capacity.
///
/// # Panics
///
/// Panics only if a previously checked internal size or ownership invariant is broken.
pub fn decode(bytes: &[u8]) -> Result<Message<'_>, EnvelopeError> {
    if bytes.len() < HEADER_BYTES {
        return Err(EnvelopeError::Length);
    }
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(EnvelopeError::MessageTooLarge);
    }
    if bytes[..4] != MAGIC {
        return Err(EnvelopeError::Magic);
    }
    if u16::from_le_bytes(bytes[4..6].try_into().expect("checked fixed header length")) != MAJOR {
        return Err(EnvelopeError::Version);
    }
    let kind = Kind::try_from(bytes[6])?;
    if bytes[7] != 0 || bytes[12..16] != [0; 4] {
        return Err(EnvelopeError::ReservedField);
    }
    let payload_len = u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .expect("checked fixed header length"),
    ) as usize;
    if total_len(payload_len)? != bytes.len() {
        return Err(EnvelopeError::Length);
    }
    let header = Header {
        kind,
        match_id: bytes[16..32]
            .try_into()
            .expect("checked fixed header length"),
        connection_epoch: u64::from_le_bytes(
            bytes[32..40]
                .try_into()
                .expect("checked fixed header length"),
        ),
        message_sequence: u64::from_le_bytes(
            bytes[40..48]
                .try_into()
                .expect("checked fixed header length"),
        ),
        server_tick: u64::from_le_bytes(
            bytes[48..56]
                .try_into()
                .expect("checked fixed header length"),
        ),
        acknowledged_sequence: u64::from_le_bytes(
            bytes[56..64]
                .try_into()
                .expect("checked fixed header length"),
        ),
    };
    if !hello_valid(header) {
        return Err(EnvelopeError::HelloIdentity);
    }
    Ok(Message {
        header,
        payload: &bytes[HEADER_BYTES..],
    })
}
