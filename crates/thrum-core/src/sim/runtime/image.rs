use crate::profile::{Digest, StateDecodeError, StateEncode, StateReader};
use crate::sim::ComponentSchemaId;

use super::{GameRuntime, TickGame};

const MAGIC: [u8; 8] = *b"THRWIMG1";
const LENGTH_AT: usize = 40;
const HEADER_BYTES: usize = 48;

/// Target-independent identity derived from the rule set and its build inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleSetIdentity(pub [u8; 32]);

/// A world image could not be written or installed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldImageError {
    /// A registered column has no portable codec.
    Schema(ComponentSchemaId),
    /// The image was written under a different rule-set identity.
    RuleSet,
    /// The destination already holds authority state.
    Destination,
    /// A field or allocation request failed validation.
    Decode(StateDecodeError),
}

impl std::fmt::Display for WorldImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(schema) => write!(f, "no portable codec for component {}", schema.0),
            Self::RuleSet => f.write_str("world image belongs to another rule set"),
            Self::Destination => f.write_str("world images require a fresh unseeded runtime"),
            Self::Decode(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for WorldImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<StateDecodeError> for WorldImageError {
    fn from(error: StateDecodeError) -> Self {
        Self::Decode(error)
    }
}

impl<G: TickGame> GameRuntime<G> {
    /// Write this committed endpoint into a reusable buffer. The image contains
    /// the digest inputs, including each column before its digest is folded.
    ///
    /// # Errors
    /// A component has no portable codec. The output is unchanged on refusal.
    ///
    /// # Panics
    /// A tick previously unwound while committing.
    pub fn write_world_image(
        &self,
        identity: RuleSetIdentity,
        resources: &impl StateEncode,
        output: &mut Vec<u8>,
    ) -> Result<(), WorldImageError> {
        self.assert_usable();
        self.world.image_schema().map_err(WorldImageError::Schema)?;
        output.clear();
        output.extend_from_slice(&MAGIC);
        output.extend_from_slice(&identity.0);
        output.extend_from_slice(&0_u64.to_le_bytes());
        let mut sink = Digest::record(output);
        self.tick.encode(&mut sink);
        self.world.encode_image(&mut sink);
        resources.encode(&mut sink);
        let payload = u64::try_from(output.len() - HEADER_BYTES).expect("image size fits u64");
        output[LENGTH_AT..HEADER_BYTES].copy_from_slice(&payload.to_le_bytes());
        Ok(())
    }

    /// Install a complete image in an unseeded runtime before publishing its
    /// first endpoint. Use a new world epoch when constructing that runtime.
    /// Publish the installed endpoint with `publish_initial` before stepping.
    ///
    /// The limit independently caps the input size and the aggregate heap
    /// allocation of decoding, including temporary columns. Game codecs must
    /// charge any allocations they make against that same budget.
    ///
    /// # Errors
    /// Byte, schema, rule-set or limit validation failed, or the runtime has
    /// already holds a world. Neither world nor resources are changed on refusal.
    pub fn restore_world_image<R: StateEncode>(
        &mut self,
        identity: RuleSetIdentity,
        resources: &mut R,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<(), WorldImageError> {
        let mut reader = StateReader::new(bytes, byte_limit)?;
        if reader.read::<8>()? != MAGIC {
            return Err(StateDecodeError::Invalid.into());
        }
        if reader.read::<32>()? != identity.0 {
            return Err(WorldImageError::RuleSet);
        }
        let payload = u64::decode(&mut reader)?;
        if payload == 0 || usize::try_from(payload).ok() != Some(reader.remaining()) {
            return Err(StateDecodeError::Invalid.into());
        }
        let tick = u64::decode(&mut reader)?;
        let world = self.world.decode_image(&mut reader, tick)?;
        let decoded_resources = R::decode(&mut reader)?;
        if reader.remaining() != 0 || !decoded_resources.is_finite() {
            return Err(StateDecodeError::Invalid.into());
        }
        if self.initial_published || self.tick != 0 || self.world.len() != 0 {
            return Err(WorldImageError::Destination);
        }
        self.world.install_image(world);
        self.tick = tick;
        drop(std::mem::replace(resources, decoded_resources));
        Ok(())
    }
}
