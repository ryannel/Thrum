//! Runtime values staged by a caller and applied by the host at a frame boundary.

/// History affected by a quality change; ordered from least to most extensive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityInvalidation {
    /// Preserve histories.
    #[default]
    None,
    /// Reset only the effect's owned histories.
    EffectHistory,
    /// Reset the view's reconstruction history as well.
    ViewHistory,
}

/// Discoverable integer cadence control. Values are ticks or slices, as named by the effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityKnob {
    /// Stable effect-local name.
    pub name: String,
    /// Committed runtime value.
    pub value: u32,
    /// Inclusive supported range.
    pub range: std::ops::RangeInclusive<u32>,
    /// Invalidation when this value changes.
    pub invalidation: QualityInvalidation,
}

/// Bounded staging for an effect's quality controls. Repeated requests coalesce.
pub struct QualitySettings {
    knobs: Vec<QualityKnob>,
    pending: Vec<Option<u32>>,
}
impl QualitySettings {
    /// Declare the controls once at construction.
    /// # Errors
    /// Refuses duplicate names, empty ranges and out-of-range defaults.
    pub fn new(knobs: Vec<QualityKnob>) -> Result<Self, String> {
        let mut names = std::collections::BTreeSet::new();
        for knob in &knobs {
            if !names.insert(&knob.name) || !knob.range.contains(&knob.value) {
                return Err("InvalidQualityKnob".into());
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
    pub fn set(&mut self, name: &str, value: u32) -> Result<(), String> {
        let index = self
            .knobs
            .iter()
            .position(|knob| knob.name == name)
            .ok_or("UnknownQualityKnob")?;
        if !self.knobs[index].range.contains(&value) {
            return Err("QualityKnobOutOfRange".into());
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
