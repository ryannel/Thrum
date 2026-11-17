use std::{fmt, path::PathBuf};

/// A native texture encoder refusal. The upstream ASTC code has no `Error` or `Display`
/// implementation, so this adapter retains its typed code without stringifying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureEncodeError {
    /// Inspect the code, repair the configuration/input or select another profile.
    Astc(astcenc_rs::Error),
}

impl fmt::Display for TextureEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Astc(code) => write!(f, "ASTC encoder refused: {code:?}"),
        }
    }
}
impl std::error::Error for TextureEncodeError {}

/// An origin cannot be admitted to the development server's allow list.
#[derive(Debug, PartialEq, Eq)]
pub enum OriginError {
    /// Supply exactly the browser's HTTP or HTTPS origin, without a path or credentials.
    InvalidOrigin(String),
}

impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrigin(origin) => write!(
                f,
                "allowed origin `{origin}` must be http://host[:port] or https://host[:port], exactly as the page's Origin header (no path or trailing slash)"
            ),
        }
    }
}
impl std::error::Error for OriginError {}

/// A measured run regressed, or its passing record could not be saved.
#[derive(Debug)]
pub enum TripwireError {
    /// Inspect the report and investigate the metrics that crossed their tripwires.
    Regression {
        /// All metrics and their outcomes, including every regression.
        report: String,
    },
    /// Repair access to the history file before recording the passing run again.
    History {
        /// History file that could not be appended.
        path: PathBuf,
        /// Passing metrics that were not recorded.
        report: String,
        /// Failure from appending the record.
        source: std::io::Error,
    },
}

impl fmt::Display for TripwireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regression { report } => f.write_str(report),
            Self::History {
                path,
                report,
                source,
            } => write!(
                f,
                "{report}history {} not appended: {source}",
                path.display()
            ),
        }
    }
}
impl std::error::Error for TripwireError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::History { source, .. } => Some(source),
            Self::Regression { .. } => None,
        }
    }
}
