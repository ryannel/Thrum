//! The link's own framing on the reliable stream, the only bytes from a peer this crate parses.
//!
//! The stream carries the four-byte link hello, then length-prefixed messages: a little-endian `u32` length of
//! 1 to [`RELIABLE_LIMIT`] bytes, then that many bytes. Every adapter decodes through these functions, and the
//! property test `tests/untrusted.rs` and the stdin targets in `examples/` drive the same ones.
use crate::{HELLO, RELIABLE_LIMIT};

/// Bytes in a length prefix.
pub const PREFIX_BYTES: usize = 4;

/// Why a peer's bytes broke the link's rules. Each closes the link as `Protocol`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    /// The first frame was not the link hello.
    Hello,
    /// A length prefix of zero.
    Empty,
    /// A length prefix over the reliable limit.
    TooLarge,
}

/// Check a complete hello frame.
/// # Errors
/// Returns `Hello` for any other four bytes.
pub fn hello(bytes: [u8; 4]) -> Result<(), WireError> {
    if &bytes == HELLO {
        Ok(())
    } else {
        Err(WireError::Hello)
    }
}

/// Decode a complete length prefix to a message length, before anything is allocated for the body.
/// # Errors
/// Returns `Empty` for zero and `TooLarge` above the reliable limit.
pub fn prefix(bytes: [u8; PREFIX_BYTES]) -> Result<usize, WireError> {
    let length = u32::from_le_bytes(bytes);
    if length == 0 {
        return Err(WireError::Empty);
    }
    match usize::try_from(length) {
        Ok(length) if length <= RELIABLE_LIMIT => Ok(length),
        _ => Err(WireError::TooLarge),
    }
}

/// Encode a message's length prefix. The caller has already refused empty and oversize messages.
/// # Panics
/// Panics for a length outside 1 to the reliable limit, which the link's send refuses first.
pub fn encode_prefix(length: usize) -> [u8; PREFIX_BYTES] {
    assert!(
        (1..=RELIABLE_LIMIT).contains(&length),
        "unchecked message length"
    );
    u32::try_from(length).expect("bounded length").to_le_bytes()
}

/// One decoded unit of the reliable stream.
#[derive(Debug, PartialEq, Eq)]
pub enum Frame {
    /// The peer's link hello, always first.
    Hello,
    /// A whole message.
    Message(Vec<u8>),
}

#[derive(Debug)]
enum Stage {
    Hello {
        have: usize,
    },
    Prefix {
        bytes: [u8; PREFIX_BYTES],
        have: usize,
    },
    Body {
        bytes: Vec<u8>,
        length: usize,
    },
    Failed(WireError),
}

/// Incremental decoder for a reliable stream that arrives in chunks of any size.
///
/// It holds at most one partial frame. A body is allocated only after its prefix is checked, at exactly the
/// prefix's length, so no input makes it allocate more than the reliable limit. A refused stream stays refused.
#[derive(Debug)]
pub struct Decoder {
    stage: Stage,
}
impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Decoder {
    /// A decoder expecting the hello first.
    pub const fn new() -> Self {
        Self {
            stage: Stage::Hello { have: 0 },
        }
    }
    /// Consume bytes up to the end of the next whole frame. Returns how many bytes of `input` were used and the
    /// frame if one completed; call again with the rest. A hello is refused at its first wrong byte.
    /// # Errors
    /// Returns the rule the stream broke, and the same error on every later call.
    pub fn decode(&mut self, input: &[u8]) -> Result<(usize, Option<Frame>), WireError> {
        let mut used = 0;
        loop {
            match &mut self.stage {
                Stage::Failed(error) => return Err(*error),
                Stage::Hello { have } => {
                    while *have < HELLO.len() {
                        let Some(&byte) = input.get(used) else {
                            return Ok((used, None));
                        };
                        if byte != HELLO[*have] {
                            self.stage = Stage::Failed(WireError::Hello);
                            return Err(WireError::Hello);
                        }
                        *have += 1;
                        used += 1;
                    }
                    self.stage = Stage::Prefix {
                        bytes: [0; PREFIX_BYTES],
                        have: 0,
                    };
                    return Ok((used, Some(Frame::Hello)));
                }
                Stage::Prefix { bytes, have } => {
                    let take = (PREFIX_BYTES - *have).min(input.len() - used);
                    bytes[*have..*have + take].copy_from_slice(&input[used..used + take]);
                    *have += take;
                    used += take;
                    if *have < PREFIX_BYTES {
                        return Ok((used, None));
                    }
                    match prefix(*bytes) {
                        Ok(length) => {
                            self.stage = Stage::Body {
                                bytes: Vec::with_capacity(length),
                                length,
                            };
                        }
                        Err(error) => {
                            self.stage = Stage::Failed(error);
                            return Err(error);
                        }
                    }
                }
                Stage::Body { bytes, length } => {
                    let take = (*length - bytes.len()).min(input.len() - used);
                    bytes.extend_from_slice(&input[used..used + take]);
                    used += take;
                    if bytes.len() < *length {
                        return Ok((used, None));
                    }
                    let message = std::mem::take(bytes);
                    self.stage = Stage::Prefix {
                        bytes: [0; PREFIX_BYTES],
                        have: 0,
                    };
                    return Ok((used, Some(Frame::Message(message))));
                }
            }
        }
    }
    /// Bytes of a partial frame held, which a stalled peer can leave here: at most the reliable limit.
    pub fn held(&self) -> usize {
        match &self.stage {
            Stage::Hello { have } | Stage::Prefix { have, .. } => *have,
            Stage::Body { bytes, .. } => bytes.len(),
            Stage::Failed(_) => 0,
        }
    }
}

/// Encode a hello and messages as one stream, for tests and proof tools.
/// # Panics
/// Panics for an empty or oversize message.
pub fn encode_stream<'a>(messages: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut stream = HELLO.to_vec();
    for message in messages {
        stream.extend_from_slice(&encode_prefix(message.len()));
        stream.extend_from_slice(message);
    }
    stream
}
impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "link rule broken: {self:?}")
    }
}
impl std::error::Error for WireError {}
