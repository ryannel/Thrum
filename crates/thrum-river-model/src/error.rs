use std::fmt;

/// Why a solve was refused. Every refusal names what it is about and, where it has one, the
/// world x/z place.
#[derive(Clone, Debug, PartialEq)]
pub enum RiverError {
    /// A declaration, grid or profile value is invalid.
    Invalid {
        /// What is invalid.
        name: String,
        /// Why.
        reason: &'static str,
    },
    /// The water reached the edge of the solve's rectangle.
    ReachesEdge {
        /// The river.
        name: String,
        /// Where, world x/z.
        place: [f64; 2],
    },
    /// A source has no way down to a declared body: its water would leave over the rectangle's edge.
    NoWayDown {
        /// The spring or source body.
        name: String,
        /// Its place, world x/z.
        place: [f64; 2],
    },
    /// A source body's level somewhere in its range lies at or under its outlet's sill.
    BelowSill {
        /// The source body.
        name: String,
        /// The sill's place, world x/z.
        place: [f64; 2],
        /// The sill's height, metres.
        sill: f64,
        /// The lowest level in the body's range, metres.
        level: f64,
    },
    /// The solve did not reach its stopping rule inside its step budget, and wrote nothing.
    NotConverged {
        /// The river.
        name: String,
        /// Steps taken.
        steps: u64,
        /// The largest level rate at the last check, metres a second.
        level_rate: f64,
        /// The largest imbalance of discharge at the last check, as a share of the inflow.
        imbalance: f64,
    },
    /// A mouth or source end does not lie along one row or column of cells.
    EndNotStraight {
        /// The joined body.
        name: String,
    },
}

impl RiverError {
    pub(crate) fn invalid(name: &str, reason: &'static str) -> Self {
        Self::Invalid {
            name: name.into(),
            reason,
        }
    }

    /// The name of what the refusal is about.
    pub fn name(&self) -> &str {
        match self {
            Self::Invalid { name, .. }
            | Self::ReachesEdge { name, .. }
            | Self::NoWayDown { name, .. }
            | Self::BelowSill { name, .. }
            | Self::NotConverged { name, .. }
            | Self::EndNotStraight { name } => name,
        }
    }
}

impl fmt::Display for RiverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { name, reason } => write!(f, "{name}: {reason}"),
            Self::ReachesEdge { name, place } => write!(
                f,
                "{name}: water reaches the rectangle's edge at ({:.2}, {:.2})",
                place[0], place[1]
            ),
            Self::NoWayDown { name, place } => write!(
                f,
                "{name}: no way down to a declared body from ({:.2}, {:.2})",
                place[0], place[1]
            ),
            Self::BelowSill {
                name,
                place,
                sill,
                level,
            } => write!(
                f,
                "{name}: level {level:.3} m lies at or under the outlet's sill, {sill:.3} m at ({:.2}, {:.2})",
                place[0], place[1]
            ),
            Self::NotConverged {
                name,
                steps,
                level_rate,
                imbalance,
            } => write!(
                f,
                "{name}: not steady after {steps} steps (level rate {level_rate:.2e} m/s, imbalance {imbalance:.2e})"
            ),
            Self::EndNotStraight { name } => {
                write!(
                    f,
                    "{name}: the end does not lie along one row or column of cells"
                )
            }
        }
    }
}

impl std::error::Error for RiverError {}
