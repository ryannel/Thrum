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
    /// The dev server answered with a status the protocol does not allow at that point.
    Status(u16),
    /// A dev-server request did not complete (connection refused, reset or aborted).
    Transport(String),
    /// Browser fetch or body reading rejected with an original JavaScript value.
    #[cfg(target_arch = "wasm32")]
    BrowserTransport(BrowserTransportError),
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
            Self::Status(status) => write!(f, "dev server answered HTTP {status}"),
            Self::Transport(what) => write!(f, "dev server request failed: {what}"),
            #[cfg(target_arch = "wasm32")]
            Self::BrowserTransport(error) => write!(f, "dev server request failed: {error}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            #[cfg(target_arch = "wasm32")]
            Self::BrowserTransport(error) => Some(error),
            _ => None,
        }
    }
}

/// A browser transport rejection. JavaScript can reject with any value, which has no
/// Rust `Error` implementation; this adapter retains its identity and diagnostic properties.
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub enum BrowserTransportError {
    /// The original promise rejection, available for inspection after downcasting the source.
    Rejected(wasm_bindgen::JsValue),
}

#[cfg(target_arch = "wasm32")]
impl fmt::Display for BrowserTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(value) => match value.as_string() {
                Some(message) => f.write_str(&message),
                None => write!(f, "{value:?}"),
            },
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl std::error::Error for BrowserTransportError {}

impl From<std::io::Error> for AssetError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
