use std::fmt;

/// Failure to read, map or validate a cooked product. Runtime callers keep their previous
/// resident generation when any of these occur.
#[derive(Debug)]
pub enum AssetError {
    /// Filesystem failure while reading `HEAD` or mapping a product.
    Io(std::io::Error),
    /// The bytes do not have the expected container, header or section structure.
    Malformed(&'static str),
    /// The format version is newer or older than this runtime understands.
    UnsupportedVersion(u32),
    /// The product bytes do not hash to the content address that named them.
    HashMismatch,
    /// A KTX2 file uses a format, supercompression or shape this runtime does not load.
    UnsupportedTexture(&'static str),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "asset store I/O failed: {error}"),
            Self::Malformed(what) => write!(f, "malformed cooked product: {what}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported product format version {version}")
            }
            Self::HashMismatch => f.write_str("product bytes do not match their content hash"),
            Self::UnsupportedTexture(what) => write!(f, "unsupported KTX2 texture: {what}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AssetError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
