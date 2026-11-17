/// Refusal to configure or derive a cooked draw probe.
#[derive(Debug)]
pub enum DrawProbeError {
    /// Configure the probe before preparing the contribution.
    AlreadyPrepared,
    /// Supply one complete command list per placement.
    PlacementCoverage,
    /// Supply contiguous nonempty triangle ranges, base vertex zero and one instance.
    InvalidCommands,
    /// A command range cannot be represented.
    RangeOverflow,
    /// Select an omitted placement that exists.
    PlacementOutOfRange,
    /// Publish a reference store before deriving its commands.
    MissingHead,
    /// Supply the named portable mesh product.
    MissingMesh(String),
    /// The reference commands do not cover the mesh's entire index section.
    IncompleteCoverage(String),
    /// A reference product could not be loaded or validated.
    Product(thrum_asset::AssetError),
}

impl std::fmt::Display for DrawProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyPrepared => f.write_str("draw probe must precede preparation"),
            Self::PlacementCoverage => f.write_str("reference placement coverage mismatch"),
            Self::InvalidCommands => f.write_str("reference must cover contiguous nonempty triangle ranges with base vertex zero and one instance"),
            Self::RangeOverflow => f.write_str("reference index range overflows"),
            Self::PlacementOutOfRange => f.write_str("omitted placement is out of range"),
            Self::MissingHead => f.write_str("reference store HEAD is missing"),
            Self::MissingMesh(name) => write!(f, "reference mesh {name} is missing"),
            Self::IncompleteCoverage(name) => write!(f, "incomplete reference coverage for {name}"),
            Self::Product(error) => write!(f, "reference product refused: {error}"),
        }
    }
}
impl std::error::Error for DrawProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Product(error) => Some(error),
            _ => None,
        }
    }
}
impl From<thrum_asset::AssetError> for DrawProbeError {
    fn from(error: thrum_asset::AssetError) -> Self {
        Self::Product(error)
    }
}
