//! Runtime values staged by a caller and applied by the host at a frame boundary.

/// Why a quality declaration or staged request was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QualityError {
    /// A declaration repeats a name or has a default outside its supported range.
    InvalidDefinition {
        /// The control whose declaration is invalid.
        name: String,
    },
    /// No attached contribution owns the requested control.
    UnknownKnob {
        /// The requested control name.
        name: String,
    },
    /// The requested value lies outside the control's inclusive range.
    OutOfRange {
        /// The requested control name.
        name: String,
        /// The refused value.
        value: u32,
        /// The inclusive supported range.
        range: std::ops::RangeInclusive<u32>,
    },
    /// The host has no scene contribution to configure.
    NoContribution,
}

impl std::fmt::Display for QualityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDefinition { name } => {
                write!(
                    f,
                    "quality knob {name:?} repeats a name or has an invalid default/range"
                )
            }
            Self::UnknownKnob { name } => write!(f, "unknown quality knob {name:?}"),
            Self::OutOfRange { name, value, range } => {
                write!(
                    f,
                    "quality knob {name:?} value {value} is outside {range:?}"
                )
            }
            Self::NoContribution => {
                f.write_str("attach a scene contribution before setting quality")
            }
        }
    }
}

impl std::error::Error for QualityError {}

/// History affected by a quality change; ordered from least to most extensive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityInvalidation {
    /// Preserve histories.
    #[default]
    None,
    /// Retain the effect history and report its age.
    Age,
    /// Reset only the effect's owned histories.
    EffectHistory,
    /// Reset the view's reconstruction history as well.
    ViewHistory,
}

/// Discoverable integer runtime control, with units and effect declared by its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityKnob {
    /// Stable effect-local name.
    pub name: String,
    /// What changing the control affects, including its unit.
    pub effect: &'static str,
    /// Committed runtime value.
    pub value: u32,
    /// Inclusive supported range.
    pub range: std::ops::RangeInclusive<u32>,
    /// Invalidation when this value changes.
    pub invalidation: QualityInvalidation,
    /// Prepared values for the three governor tiers, if this control participates.
    pub governor_steps: Option<[u32; 3]>,
}

/// Bounded staging for an effect's quality controls. Repeated requests coalesce.
#[derive(Clone)]
pub struct QualitySettings {
    knobs: Vec<QualityKnob>,
    pending: Vec<Option<u32>>,
}
impl QualitySettings {
    /// Declare the controls once at construction.
    /// # Errors
    /// Refuses duplicate names, empty ranges and out-of-range defaults.
    pub fn new(knobs: Vec<QualityKnob>) -> Result<Self, QualityError> {
        let mut names = std::collections::BTreeSet::new();
        for knob in &knobs {
            if !names.insert(&knob.name)
                || !knob.range.contains(&knob.value)
                || knob.governor_steps.is_some_and(|steps| {
                    steps[0] != knob.value
                        || steps.iter().any(|value| !knob.range.contains(value))
                        || knob.invalidation == QualityInvalidation::ViewHistory
                })
            {
                return Err(QualityError::InvalidDefinition {
                    name: knob.name.clone(),
                });
            }
        }
        Ok(Self {
            pending: vec![None; knobs.len()],
            knobs,
        })
    }
    /// Current values, excluding staged changes.
    #[must_use]
    pub fn knobs(&self) -> &[QualityKnob] {
        &self.knobs
    }
    /// Stage one change without mutating live frame state.
    /// # Errors
    /// Refuses an unknown name or a value outside its declared range.
    pub fn set(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        let index = self
            .knobs
            .iter()
            .position(|knob| knob.name == name)
            .ok_or_else(|| QualityError::UnknownKnob { name: name.into() })?;
        if !self.knobs[index].range.contains(&value) {
            return Err(QualityError::OutOfRange {
                name: name.into(),
                value,
                range: self.knobs[index].range.clone(),
            });
        }
        self.pending[index] = Some(value);
        Ok(())
    }
    /// Apply staged changes at the next frame boundary, returning the required reset.
    pub fn commit(&mut self) -> QualityInvalidation {
        let mut invalidation = QualityInvalidation::None;
        for (knob, pending) in self.knobs.iter_mut().zip(&mut self.pending) {
            if let Some(value) = pending.take()
                && value != knob.value
            {
                knob.value = value;
                invalidation = invalidation.max(knob.invalidation);
            }
        }
        invalidation
    }
}

#[cfg(test)]
mod tests;
