/// A material hook could not be read from its source file.
#[derive(Debug)]
pub enum MaterialReadError {
    /// Repair the file's access or UTF-8 contents and request the reload again.
    Io {
        /// Requested source file.
        path: std::path::PathBuf,
        /// Filesystem or UTF-8 decoding failure.
        source: std::io::Error,
    },
}

impl std::fmt::Display for MaterialReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "material hook {}: {source}", path.display()),
        }
    }
}
impl std::error::Error for MaterialReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}
