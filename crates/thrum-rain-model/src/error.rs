use std::fmt;

/// Rain content or state that cannot be prepared or stepped.
#[derive(Clone, Debug, PartialEq)]
pub enum RainError {
    /// The ground's grid is malformed or holds a nonfinite height.
    Ground(&'static str),
    /// The profile holds a value that cannot be used.
    Profile(&'static str),
    /// The body holds more cells or hollows than the profile allows.
    OverLimit {
        /// What is counted: "cells" or "hollows".
        what: &'static str,
        /// How many the body holds.
        count: usize,
        /// The profile's limit.
        limit: usize,
    },
    /// A shelter is malformed, or one more than the canopy's layers crosses a column.
    Shelter {
        /// The shelter's name.
        name: String,
        /// World x/z of the column, or of the shelter where no column applies.
        place: [f64; 2],
        /// Why it was refused.
        reason: &'static str,
    },
    /// A tick's rain, drying or ground class is negative or nonfinite.
    Input(&'static str),
    /// Encoded rain state that does not match its tree.
    State(&'static str),
}

impl fmt::Display for RainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ground(reason) => write!(f, "ground refused: {reason}"),
            Self::Profile(reason) => write!(f, "rain profile refused: {reason}"),
            Self::OverLimit { what, count, limit } => {
                write!(
                    f,
                    "hollows body holds {count} {what}, over the profile's {limit}"
                )
            }
            Self::Shelter {
                name,
                place,
                reason,
            } => write!(
                f,
                "shelter {name:?} refused at x {:.3} z {:.3}: {reason}",
                place[0], place[1]
            ),
            Self::Input(reason) => write!(f, "rain input refused: {reason}"),
            Self::State(reason) => write!(f, "rain state refused: {reason}"),
        }
    }
}

impl std::error::Error for RainError {}
