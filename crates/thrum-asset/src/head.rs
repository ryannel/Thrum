use crate::layout::{HEAD_FORMAT, HEAD_LEN, HEAD_MAGIC};
use crate::{AssetError, ContentHash};

/// The current publication: a generation and the manifest product it selects.
///
/// The cooker replaces the store's `HEAD` file atomically (write, sync, rename), so a reader sees
/// either the previous record or the new one, never a mixture. The record carries its own short
/// BLAKE3 check so a damaged file is refused rather than misread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    /// Identity of the store instance. A store that is deleted and recreated gets a new epoch, so
    /// readers accept its generation 1 instead of ignoring it as older than what they saw.
    pub epoch: u32,
    /// Strictly increasing publication number within one epoch.
    pub generation: u64,
    /// Content hash of the selected manifest product.
    pub manifest: ContentHash,
}

impl Head {
    /// Encode the exact 64-byte record.
    pub fn encode(&self) -> [u8; HEAD_LEN] {
        let mut out = [0; HEAD_LEN];
        out[..8].copy_from_slice(&HEAD_MAGIC);
        out[8..12].copy_from_slice(&HEAD_FORMAT.to_le_bytes());
        out[12..16].copy_from_slice(&self.epoch.to_le_bytes());
        out[16..24].copy_from_slice(&self.generation.to_le_bytes());
        out[24..56].copy_from_slice(self.manifest.as_bytes());
        let check = blake3::hash(&out[..56]);
        out[56..].copy_from_slice(&check.as_bytes()[..8]);
        out
    }

    /// Decode and check a record.
    ///
    /// # Errors
    /// Refuses a wrong length, magic, version or check value.
    pub fn decode(bytes: &[u8]) -> Result<Self, AssetError> {
        let bytes: &[u8; HEAD_LEN] = bytes
            .try_into()
            .map_err(|_| AssetError::Malformed("HEAD length"))?;
        if bytes[..8] != HEAD_MAGIC {
            return Err(AssetError::Malformed("HEAD magic"));
        }
        let format = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        if format != HEAD_FORMAT {
            return Err(AssetError::UnsupportedVersion(format));
        }
        if blake3::hash(&bytes[..56]).as_bytes()[..8] != bytes[56..] {
            return Err(AssetError::Malformed("HEAD check"));
        }
        let mut generation = [0; 8];
        generation.copy_from_slice(&bytes[16..24]);
        let mut manifest = [0; 32];
        manifest.copy_from_slice(&bytes[24..56]);
        Ok(Self {
            epoch: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            generation: u64::from_le_bytes(generation),
            manifest: ContentHash::from_bytes(manifest),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Head;
    use crate::ContentHash;

    #[test]
    fn head_round_trips_and_refuses_damage() {
        let head = Head {
            epoch: 0x5eed,
            generation: 7,
            manifest: ContentHash::of(b"manifest"),
        };
        let bytes = head.encode();
        assert_eq!(Head::decode(&bytes).ok(), Some(head));
        for index in [0, 9, 13, 17, 30, 60] {
            let mut damaged = bytes;
            damaged[index] ^= 1;
            assert!(Head::decode(&damaged).is_err(), "byte {index}");
        }
        assert!(Head::decode(&bytes[..63]).is_err());
    }
}
