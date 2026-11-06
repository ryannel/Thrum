use std::fmt;

use crate::AssetError;

/// BLAKE3-256 digest naming an immutable product (or a source file) by its exact bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hash bytes. BLAKE3 selects NEON/AVX2/AVX-512 internally where the CPU has them.
    pub fn of(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Wrap a digest produced elsewhere, such as an incremental hasher.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lower-case hexadecimal form used for store paths and the dev server.
    pub fn to_hex(self) -> String {
        let mut text = String::with_capacity(64);
        for byte in self.0 {
            text.push(char::from(HEX[usize::from(byte >> 4)]));
            text.push(char::from(HEX[usize::from(byte & 15)]));
        }
        text
    }

    /// Parse exactly 64 lower-case hexadecimal digits; anything else refuses.
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 64 {
            return None;
        }
        let mut out = [0; 32];
        for (index, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
            out[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Some(Self(out))
    }

    /// Check that `bytes` are exactly the content this hash names.
    ///
    /// # Errors
    /// [`AssetError::HashMismatch`] when they differ.
    pub fn verify(&self, bytes: &[u8]) -> Result<(), AssetError> {
        if Self::of(bytes) == *self {
            Ok(())
        } else {
            Err(AssetError::HashMismatch)
        }
    }

    /// First eight bytes as a little-endian integer, for compact keyed lookups.
    pub fn prefix_u64(&self) -> u64 {
        let mut prefix = [0; 8];
        prefix.copy_from_slice(&self.0[..8]);
        u64::from_le_bytes(prefix)
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", &self.to_hex()[..16])
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::ContentHash;

    #[test]
    fn hex_round_trips_and_refuses_bad_text() {
        let hash = ContentHash::of(b"thrum");
        assert_eq!(ContentHash::from_hex(&hash.to_hex()), Some(hash));
        assert!(hash.verify(b"thrum").is_ok());
        assert!(hash.verify(b"thrun").is_err());
        assert_eq!(ContentHash::from_hex("00"), None);
        assert_eq!(ContentHash::from_hex(&"G".repeat(64)), None);
        assert_eq!(
            ContentHash::from_hex(&"A".repeat(64)),
            None,
            "upper case is not canonical"
        );
    }
}
