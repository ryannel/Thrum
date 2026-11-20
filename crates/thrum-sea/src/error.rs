use std::fmt;

/// Refusal to prepare or upload the ambient sea. A refused upload leaves it unchanged.
#[derive(Debug)]
pub enum AmbientError {
    /// Select a supported profile before preparing resources.
    UnsupportedProfile(ProfileField),
    /// The device cannot admit the named resource or operation.
    DeviceLimit {
        /// Resource or operation that cannot be admitted.
        resource: String,
        /// Device capability the caller must increase.
        limit: &'static str,
    },
    /// A prepared GPU buffer could not be mapped.
    Mapping(wgpu::MapRangeError),
    /// Supply finite inputs within the documented component and query counts.
    InvalidUpload,
    /// The CPU wave model refused the long-wave components.
    LongWaves(thrum_sea_model::SeaError),
    /// Reduce long-wave steepness below the GPU contraction bound.
    NonContractive,
    /// Supply finite positive cascade lengths and finite components.
    InvalidCascade,
    /// Select nonzero wave vectors inside the transform lattice.
    OutsideLattice,
    /// Supply finite nonnegative long-wave amplitudes and nonzero frequencies.
    InvalidLongWave,
    /// Move queries within 8192 metres of the anchor.
    QueryOutsideSupport,
}

/// Profile setting that must be corrected before preparing the sea.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileField {
    /// The transform size must be 256.
    TransformSize,
    /// Three cascades are supported.
    Cascades,
    /// Eight cache levels are supported.
    CacheLevels,
    /// Choppiness must be finite and within zero to four.
    Choppiness,
}

impl fmt::Display for AmbientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProfile(field) => f.write_str(match field {
                ProfileField::TransformSize => "sea transform_size: supported value is 256",
                ProfileField::Cascades => "sea cascades: supported value is 3",
                ProfileField::CacheLevels => "sea cache_levels: supported value is 8",
                ProfileField::Choppiness => "sea choppiness: expected finite 0..=4",
            }),
            Self::DeviceLimit { resource, limit } => write!(f, "sea {resource}: device limit {limit} is insufficient"),
            Self::Mapping(error) => write!(f, "sea twiddle mapping failed: {error}"),
            Self::InvalidUpload => f.write_str("sea upload requires finite time/anchor, <=192 components and <=4096 finite queries"),
            Self::LongWaves(error) => write!(f, "sea long waves refused: {error}"),
            Self::NonContractive => f.write_str("sea long-wave contraction bound must be below 0.9"),
            Self::InvalidCascade => f.write_str("sea cascade has invalid physical input"),
            Self::OutsideLattice => f.write_str("sea cascade exceeds the transform lattice or contains the zero mode"),
            Self::InvalidLongWave => f.write_str("sea long-wave component is invalid"),
            Self::QueryOutsideSupport => f.write_str("sea queries exceed local support of 8192 metres"),
        }
    }
}

impl std::error::Error for AmbientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Mapping(error) => Some(error),
            Self::LongWaves(error) => Some(error),
            _ => None,
        }
    }
}
