//! Typed demo controls: one model of sliders, toggles, text fields and readouts that native
//! and browser views present identically, so a demo can expose its mechanism.
//!
//! A game declares controls once, with a stable dotted key (`sky.solar_time`), a display label,
//! a canonical engine value and a [`Display`] mapping. It gets back typed handles
//! ([`Slider`], [`Toggle`], [`Text`], [`Readout`]) and reads values through them. Views and
//! files edit the same model, and the game applies what changed since its last look:
//!
//! ```
//! use thrum_controls::{Controls, Display};
//! let mut controls = Controls::new();
//! let time = controls
//!     .slider("sky.solar_time", "Solar time", 0.0..=1.0)
//!     .default(0.5)
//!     .display(Display::clock())
//!     .add()
//!     .unwrap();
//! let seen = controls.revision();
//! controls.edit_display(time.id(), 18.0); // what a panel does when dragged to 18:00
//! for id in controls.changed_since(seen) {
//!     assert_eq!(id, time.id());
//!     assert!((controls.get(time) - 0.75).abs() < 1e-6);
//! }
//! ```
//!
//! Edits from people (panels) clamp and snap into range and are counted, because presentation
//! never fails a frame. Programmatic edits and parameter files refuse invalid values with a
//! [`ControlError`] that names the control, the cause and a safe next step. The model holds no
//! game schema and performs no I/O; views live in [`macos`] and `web/thrum-controls.js`.

mod display;
mod file;
mod json;
#[cfg(target_os = "macos")]
pub mod macos;

use std::{fmt, ops::RangeInclusive};

pub use display::Display;
pub use file::FileReport;

/// Longest accepted control key and text value, in bytes.
pub const MAX_KEY_BYTES: usize = 64;
/// Longest accepted label, in bytes.
pub const MAX_LABEL_BYTES: usize = 48;
/// Longest accepted text value, in bytes.
pub const MAX_TEXT_BYTES: usize = 256;

/// Index of one declared control within its [`Controls`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ControlId(u16);

impl ControlId {
    /// Position of the control in declaration order.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// Typed handle to a numeric slider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Slider(ControlId);
/// Typed handle to an on/off toggle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Toggle(ControlId);
/// Typed handle to an editable text field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Text(ControlId);
/// Typed handle to a read-only value the game or engine publishes, such as a field's reading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Readout(ControlId);

macro_rules! handle_id {
    ($($handle:ident),*) => {$(
        impl $handle {
            /// Untyped identity shared with views and change lists.
            #[must_use]
            pub fn id(self) -> ControlId {
                self.0
            }
        }
    )*};
}
handle_id!(Slider, Toggle, Text, Readout);

/// What kind of control a declaration is, with its static shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// A bounded numeric value in engine units.
    Slider {
        /// Inclusive engine-unit range.
        min: f32,
        /// Inclusive engine-unit range.
        max: f32,
        /// Engine-unit step that people's edits snap to; zero means continuous.
        step: f32,
        /// How the value is shown and edited.
        display: Display,
    },
    /// An on/off value.
    Toggle,
    /// A bounded single-line string.
    Text,
    /// A value shown but never edited by a person.
    Readout {
        /// How the value is shown.
        display: Display,
    },
}

/// Current value of one control.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Slider or numeric readout value in engine units.
    Number(f32),
    /// Toggle state.
    Flag(bool),
    /// Text field contents.
    Text(String),
}

/// One declared control.
#[derive(Clone, Debug)]
pub struct Spec {
    key: String,
    label: String,
    kind: Kind,
}

impl Spec {
    /// Stable dotted key, such as `sky.solar_time`; the part before the first dot is its group.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Group shown as a panel section: the key up to its first dot, or empty.
    #[must_use]
    pub fn group(&self) -> &str {
        self.key.split_once('.').map_or("", |(group, _)| group)
    }
    /// Label shown to a person.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Kind and static shape.
    #[must_use]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

/// A refused declaration or edit. Each message names the control, the cause and a next step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlError {
    /// The key is empty, too long, not lowercase `a-z0-9_.`, or starts/ends with a dot.
    InvalidKey(String),
    /// Two controls share a key.
    DuplicateKey(String),
    /// The label is empty or longer than [`MAX_LABEL_BYTES`].
    InvalidLabel(String),
    /// A slider range, step or default is not finite or not ordered.
    InvalidRange(String),
    /// More controls than one panel can hold.
    TooMany,
    /// An edit named no declared control.
    UnknownKey {
        /// The key that was not found.
        key: String,
        /// The closest declared key, if one is similar.
        suggestion: Option<String>,
    },
    /// An edit's value does not fit the control.
    InvalidValue {
        /// The control's key.
        key: String,
        /// What was wrong and what is accepted.
        cause: String,
    },
    /// A parameter file is not a JSON object of keys to values.
    InvalidFile(String),
    /// An edit named a control ID this model never declared (for example one from another model).
    UnknownId(usize),
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(key) => write!(
                f,
                "control key {key:?} is invalid; use 1–{MAX_KEY_BYTES} lowercase letters, digits, \
                 '_' and '.', such as \"sky.solar_time\""
            ),
            Self::DuplicateKey(key) => {
                write!(
                    f,
                    "control {key:?} is declared twice; give each control its own key"
                )
            }
            Self::InvalidLabel(key) => write!(
                f,
                "control {key:?} needs a label of 1–{MAX_LABEL_BYTES} bytes that a person can read"
            ),
            Self::InvalidRange(key) => write!(
                f,
                "slider {key:?} needs finite min < max, a non-negative step, a default inside the range and a positive display scale"
            ),
            Self::TooMany => {
                f.write_str("too many controls; group related values or remove unused ones")
            }
            Self::UnknownKey { key, suggestion } => match suggestion {
                Some(near) => write!(f, "no control is named {key:?}; did you mean {near:?}?"),
                None => write!(
                    f,
                    "no control is named {key:?}; check the keys the game declares"
                ),
            },
            Self::InvalidValue { key, cause } => write!(f, "control {key:?}: {cause}"),
            Self::InvalidFile(cause) => write!(
                f,
                "parameter file {cause}; write one JSON object of control keys to display values, \
                 such as {{\"sky.solar_time\": 18.5}}"
            ),
            Self::UnknownId(index) => write!(
                f,
                "control ID {index} is not declared in this model; look the control up by key on the model it belongs to"
            ),
        }
    }
}

impl std::error::Error for ControlError {}

/// The game's declared controls and their current values.
///
/// Every edit that changes a value advances one model revision and stamps the control with it,
/// so a consumer applies only what changed: [`Controls::changed_since`].
#[derive(Clone, Debug, Default)]
pub struct Controls {
    specs: Vec<Spec>,
    values: Vec<Value>,
    revisions: Vec<u64>,
    revision: u64,
    clamped_edits: u64,
}

/// Declares one slider; finish with [`SliderBuilder::add`].
#[must_use = "a slider is only declared by add()"]
pub struct SliderBuilder<'a> {
    controls: &'a mut Controls,
    key: String,
    label: String,
    range: RangeInclusive<f32>,
    step: f32,
    default: Option<f32>,
    display: Display,
}

impl SliderBuilder<'_> {
    /// Starting value in engine units; defaults to the range minimum.
    pub fn default(mut self, value: f32) -> Self {
        self.default = Some(value);
        self
    }
    /// Engine-unit step that people's edits snap to; zero (the default) is continuous.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }
    /// How the value is shown; defaults to two decimal places without a unit.
    pub fn display(mut self, display: Display) -> Self {
        self.display = display;
        self
    }
    /// Declare the slider.
    /// # Errors
    /// Refuses an invalid key or label, a duplicate key, a non-finite or unordered range, or a
    /// display scale that is not finite and positive.
    pub fn add(self) -> Result<Slider, ControlError> {
        let (min, max) = (*self.range.start(), *self.range.end());
        let default = self.default.unwrap_or(min);
        let ordered = min.is_finite() && max.is_finite() && min < max;
        let step_ok = self.step.is_finite() && self.step >= 0.0;
        // Edits divide by the scale, so zero, negative or non-finite would admit NaN or
        // reverse the range a person drags.
        let scale_ok = self.display.scale().is_finite() && self.display.scale() > 0.0;
        if !ordered || !step_ok || !scale_ok || !(min..=max).contains(&default) {
            return Err(ControlError::InvalidRange(self.key));
        }
        let kind = Kind::Slider {
            min,
            max,
            step: self.step,
            display: self.display,
        };
        self.controls
            .declare(self.key, self.label, kind, Value::Number(default))
            .map(Slider)
    }
}

impl Controls {
    /// An empty set of controls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin declaring a slider over an inclusive engine-unit range.
    pub fn slider(
        &mut self,
        key: &str,
        label: &str,
        range: RangeInclusive<f32>,
    ) -> SliderBuilder<'_> {
        SliderBuilder {
            controls: self,
            key: key.to_owned(),
            label: label.to_owned(),
            range,
            step: 0.0,
            default: None,
            display: Display::plain(2, ""),
        }
    }

    /// Declare an on/off toggle.
    /// # Errors
    /// Refuses an invalid or duplicate key or an invalid label.
    pub fn toggle(
        &mut self,
        key: &str,
        label: &str,
        default: bool,
    ) -> Result<Toggle, ControlError> {
        self.declare(
            key.to_owned(),
            label.to_owned(),
            Kind::Toggle,
            Value::Flag(default),
        )
        .map(Toggle)
    }

    /// Declare an editable single-line text field.
    /// # Errors
    /// Refuses an invalid or duplicate key, an invalid label or an over-long default.
    pub fn text(&mut self, key: &str, label: &str, default: &str) -> Result<Text, ControlError> {
        check_text(key, default)?;
        self.declare(
            key.to_owned(),
            label.to_owned(),
            Kind::Text,
            Value::Text(default.to_owned()),
        )
        .map(Text)
    }

    /// Declare a read-only numeric value, such as a field reading or a frame time.
    /// # Errors
    /// Refuses an invalid or duplicate key or an invalid label.
    pub fn readout(
        &mut self,
        key: &str,
        label: &str,
        display: Display,
    ) -> Result<Readout, ControlError> {
        self.declare(
            key.to_owned(),
            label.to_owned(),
            Kind::Readout { display },
            Value::Number(0.0),
        )
        .map(Readout)
    }

    fn declare(
        &mut self,
        key: String,
        label: String,
        kind: Kind,
        value: Value,
    ) -> Result<ControlId, ControlError> {
        if !valid_key(&key) {
            return Err(ControlError::InvalidKey(key));
        }
        if label.trim().is_empty() || label.len() > MAX_LABEL_BYTES {
            return Err(ControlError::InvalidLabel(key));
        }
        if self.specs.iter().any(|spec| spec.key == key) {
            return Err(ControlError::DuplicateKey(key));
        }
        let id = ControlId(u16::try_from(self.specs.len()).map_err(|_| ControlError::TooMany)?);
        self.specs.push(Spec { key, label, kind });
        self.values.push(value);
        self.revisions.push(0);
        Ok(id)
    }

    /// Declared controls in declaration order; [`ControlId::index`] indexes this slice.
    #[must_use]
    pub fn specs(&self) -> &[Spec] {
        &self.specs
    }

    /// Current value of any control.
    #[must_use]
    pub fn value(&self, id: ControlId) -> &Value {
        &self.values[id.index()]
    }

    /// Controls grouped by key prefix, groups in order of first declaration and controls in
    /// declaration order within each, so every view lays them out the same way.
    #[must_use]
    pub fn groups(&self) -> Vec<(&str, Vec<ControlId>)> {
        let mut groups: Vec<(&str, Vec<ControlId>)> = Vec::new();
        for (index, spec) in self.specs.iter().enumerate() {
            let Ok(index) = u16::try_from(index) else {
                break;
            };
            let id = ControlId(index);
            match groups.iter_mut().find(|(group, _)| *group == spec.group()) {
                Some((_, ids)) => ids.push(id),
                None => groups.push((spec.group(), vec![id])),
            }
        }
        groups
    }

    /// Look up a control by key, explaining a miss.
    ///
    /// # Errors
    /// Returns [`ControlError::UnknownKey`] with the closest declared key, if any is close.
    pub fn lookup(&self, key: &str) -> Result<ControlId, ControlError> {
        self.find(key).ok_or_else(|| ControlError::UnknownKey {
            key: key.to_owned(),
            suggestion: self.suggestion(key),
        })
    }

    /// Look up a control by key.
    #[must_use]
    pub fn find(&self, key: &str) -> Option<ControlId> {
        self.specs
            .iter()
            .position(|spec| spec.key == key)
            .and_then(|index| u16::try_from(index).ok())
            .map(ControlId)
    }

    /// Current slider value in engine units.
    #[must_use]
    pub fn get(&self, slider: Slider) -> f32 {
        self.number(slider.0)
    }
    /// Current toggle state.
    #[must_use]
    pub fn is_on(&self, toggle: Toggle) -> bool {
        matches!(self.values[toggle.0.index()], Value::Flag(true))
    }
    /// Current text.
    #[must_use]
    pub fn text_of(&self, text: Text) -> &str {
        match &self.values[text.0.index()] {
            Value::Text(value) => value,
            _ => "",
        }
    }
    /// Current readout value.
    #[must_use]
    pub fn reading(&self, readout: Readout) -> f32 {
        self.number(readout.0)
    }

    fn number(&self, id: ControlId) -> f32 {
        match self.values[id.index()] {
            Value::Number(value) => value,
            _ => 0.0,
        }
    }

    /// Model revision; advances once per value-changing edit.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Revision at which a control last changed (zero: never since declaration).
    #[must_use]
    pub fn revision_of(&self, id: ControlId) -> u64 {
        self.revisions[id.index()]
    }

    /// Controls changed in code, or edited by a person or file, after `revision`, in
    /// declaration order. A person's edit counts even when it repeats the current value.
    pub fn changed_since(&self, revision: u64) -> impl Iterator<Item = ControlId> + '_ {
        self.revisions
            .iter()
            .enumerate()
            .filter(move |(_, changed)| **changed > revision)
            .filter_map(|(index, _)| u16::try_from(index).ok().map(ControlId))
    }

    /// People's edits that were clamped or snapped into range since declaration.
    #[must_use]
    pub fn clamped_edits(&self) -> u64 {
        self.clamped_edits
    }

    /// Set a slider from code.
    /// # Errors
    /// Refuses a non-finite value or one outside the declared range; the model is unchanged.
    pub fn set(&mut self, slider: Slider, value: f32) -> Result<bool, ControlError> {
        let Kind::Slider {
            min, max, display, ..
        } = self.specs[slider.0.index()].kind
        else {
            return Err(wrong_kind(&self.specs[slider.0.index()], "a number"));
        };
        if !value.is_finite() || !(min..=max).contains(&value) {
            let key = self.specs[slider.0.index()].key.clone();
            return Err(ControlError::InvalidValue {
                key,
                cause: format!(
                    "{value} is outside {}–{}; choose a value in that range",
                    display.format(min),
                    display.format(max)
                ),
            });
        }
        Ok(self.store(slider.0, Value::Number(value)))
    }

    /// Set a toggle from code.
    pub fn set_flag(&mut self, toggle: Toggle, on: bool) -> bool {
        self.store(toggle.0, Value::Flag(on))
    }

    /// Set a text field from code.
    /// # Errors
    /// Refuses text longer than [`MAX_TEXT_BYTES`] or containing a line break.
    pub fn set_text(&mut self, text: Text, value: &str) -> Result<bool, ControlError> {
        check_text(&self.specs[text.0.index()].key, value)?;
        Ok(self.store(text.0, Value::Text(value.to_owned())))
    }

    /// Publish a readout value; a non-finite reading shows as zero and is counted.
    pub fn show(&mut self, readout: Readout, value: f32) -> bool {
        let value = if value.is_finite() {
            value
        } else {
            self.clamped_edits = self.clamped_edits.saturating_add(1);
            0.0
        };
        self.store(readout.0, Value::Number(value))
    }

    /// Apply a person's slider edit in display units: snap to the step, clamp into range,
    /// count any repair and return whether the value changed.
    /// # Errors
    /// Refuses a control that is not a slider, naming what it takes instead.
    pub fn edit_display(
        &mut self,
        id: ControlId,
        display_value: f64,
    ) -> Result<bool, ControlError> {
        let spec = self.spec(id)?;
        let Kind::Slider {
            min,
            max,
            step,
            display,
        } = spec.kind.clone()
        else {
            return Err(wrong_kind(spec, "a number"));
        };
        let wanted = if display_value.is_finite() {
            display.to_engine(display_value)
        } else {
            min
        };
        let mut value = wanted.clamp(min, max);
        if step > 0.0 {
            value = (min + ((value - min) / step).round() * step).clamp(min, max);
        }
        if !display_value.is_finite() || (value - wanted).abs() > step.max(f32::EPSILON) {
            self.clamped_edits = self.clamped_edits.saturating_add(1);
        }
        Ok(self.store_edit(id, Value::Number(value)))
    }

    /// Apply a person's toggle or text edit and return whether the value changed.
    /// # Errors
    /// Refuses a value of the wrong kind, naming what the control takes, and text that is
    /// over-long or has a line break.
    pub fn edit_value(&mut self, id: ControlId, value: Value) -> Result<bool, ControlError> {
        let spec = self.spec(id)?;
        match (&spec.kind, &value) {
            (Kind::Toggle, Value::Flag(_)) => {}
            (Kind::Text, Value::Text(text)) => check_text(&spec.key, text)?,
            (_, Value::Flag(_)) => return Err(wrong_kind(spec, "true or false")),
            (_, Value::Text(_)) => return Err(wrong_kind(spec, "text")),
            (_, Value::Number(_)) => return Err(wrong_kind(spec, "a number")),
        }
        Ok(self.store_edit(id, value))
    }

    /// The declaration of `id`, refusing an ID this model never issued.
    fn spec(&self, id: ControlId) -> Result<&Spec, ControlError> {
        self.specs
            .get(id.index())
            .ok_or(ControlError::UnknownId(id.index()))
    }

    /// Store a person's or file's edit. Unlike a code edit, it always advances the control's
    /// revision: choosing 12:00 again is still a request the game must apply, for example
    /// after its clock has moved on from the value the model last showed.
    fn store_edit(&mut self, id: ControlId, value: Value) -> bool {
        let changed = self.store(id, value);
        if !changed {
            self.revision = self.revision.wrapping_add(1);
            self.revisions[id.index()] = self.revision;
        }
        changed
    }

    fn store(&mut self, id: ControlId, value: Value) -> bool {
        let slot = &mut self.values[id.index()];
        let same = match (&*slot, &value) {
            (Value::Number(old), Value::Number(new)) => old.to_bits() == new.to_bits(),
            (old, new) => old == new,
        };
        if same {
            return false;
        }
        *slot = value;
        self.revision = self.revision.wrapping_add(1);
        self.revisions[id.index()] = self.revision;
        true
    }

    /// Text a person reads for a control's current value.
    #[must_use]
    pub fn formatted(&self, id: ControlId) -> String {
        match (&self.specs[id.index()].kind, &self.values[id.index()]) {
            (Kind::Slider { display, .. } | Kind::Readout { display }, Value::Number(value)) => {
                display.format(*value)
            }
            (_, Value::Flag(on)) => if *on { "On" } else { "Off" }.to_owned(),
            (_, Value::Text(text)) => text.clone(),
            (_, Value::Number(value)) => value.to_string(),
        }
    }

    pub(crate) fn suggestion(&self, key: &str) -> Option<String> {
        self.specs
            .iter()
            .map(|spec| (edit_distance(&spec.key, key), &spec.key))
            .filter(|(distance, near)| *distance <= near.len().max(key.len()) / 3 + 1)
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, near)| near.clone())
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_BYTES
        && !key.starts_with('.')
        && !key.ends_with('.')
        && !key.contains("..")
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.'
        })
}

/// The refusal for an edit whose value does not suit the control's kind.
fn wrong_kind(spec: &Spec, given: &str) -> ControlError {
    let takes = match spec.kind {
        Kind::Slider { .. } => "is a slider; give a number in display units",
        Kind::Toggle => "is a toggle; give true or false",
        Kind::Text => "is a text field; give one line of text",
        Kind::Readout { .. } => "is a readout and cannot be edited",
    };
    ControlError::InvalidValue {
        key: spec.key.clone(),
        cause: format!("got {given}, but it {takes}"),
    }
}

fn check_text(key: &str, value: &str) -> Result<(), ControlError> {
    if value.len() > MAX_TEXT_BYTES || value.contains(['\n', '\r']) {
        return Err(ControlError::InvalidValue {
            key: key.to_owned(),
            cause: format!("text must be one line of at most {MAX_TEXT_BYTES} bytes"),
        });
    }
    Ok(())
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b = b.as_bytes();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.bytes().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous + usize::from(ca != *cb);
            previous = row[j + 1];
            row[j + 1] = substitute.min(row[j] + 1).min(previous + 1);
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests;
