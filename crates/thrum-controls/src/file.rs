//! Parameter files: authored values that hot reload into a running demo.

use crate::{ControlError, Controls, Kind, Value};

/// Outcome of applying a parameter file. Valid entries apply; each invalid one is reported
/// with its key, cause and next step, and leaves that control unchanged.
#[derive(Debug, Default, PartialEq)]
pub struct FileReport {
    /// Entries whose value changed.
    pub changed: usize,
    /// Entries that already held the file's value.
    pub unchanged: usize,
    /// Entries refused, sorted by key.
    pub problems: Vec<ControlError>,
}

impl std::fmt::Display for FileReport {
    /// One line for a status bar or log, such as `2 changed, 1 unchanged, 1 refused`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} changed, {} unchanged, {} refused",
            self.changed,
            self.unchanged,
            self.problems.len()
        )
    }
}

impl Controls {
    /// Apply a parameter file: one JSON object of control keys to **display** values (the
    /// numbers a person sees in the panel, such as hours or degrees), booleans or strings.
    ///
    /// Authored content validates with tolerances: a slider value within half a step of the
    /// range is clamped; anything else invalid is reported and skipped.
    ///
    /// # Errors
    /// Refuses text that is not a JSON object; nothing is applied.
    pub fn apply_file(&mut self, json: &str) -> Result<FileReport, ControlError> {
        let entries: serde_json::Map<String, serde_json::Value> = serde_json::from_str(json)
            .map_err(|error| {
                ControlError::InvalidFile(format!("is not a JSON object ({error})"))
            })?;
        let mut report = FileReport::default();
        for (key, value) in entries {
            match self.apply_entry(&key, &value) {
                Ok(true) => report.changed += 1,
                Ok(false) => report.unchanged += 1,
                Err(problem) => report.problems.push(problem),
            }
        }
        Ok(report)
    }

    fn apply_entry(&mut self, key: &str, value: &serde_json::Value) -> Result<bool, ControlError> {
        let id = self.lookup(key)?;
        let invalid = |cause: String| ControlError::InvalidValue {
            key: key.to_owned(),
            cause,
        };
        match (self.specs[id.index()].kind.clone(), value) {
            (
                Kind::Slider {
                    min,
                    max,
                    step,
                    display,
                },
                serde_json::Value::Number(number),
            ) => {
                let shown = number.as_f64().unwrap_or(f64::NAN);
                let engine = display.to_engine(shown);
                let tolerance = (step * 0.5).max((max - min) * 1e-4);
                if !engine.is_finite() || engine < min - tolerance || engine > max + tolerance {
                    return Err(invalid(format!(
                        "{shown} is outside {}–{} (display units); choose a value in that range",
                        crate::json::tidy(display.to_display(min)),
                        crate::json::tidy(display.to_display(max))
                    )));
                }
                self.edit_display(id, shown)
            }
            (Kind::Toggle, serde_json::Value::Bool(on)) => self.edit_value(id, Value::Flag(*on)),
            (Kind::Text, serde_json::Value::String(text)) => {
                if text.len() > crate::MAX_TEXT_BYTES || text.contains(['\n', '\r']) {
                    return Err(invalid(format!(
                        "text must be one line of at most {} bytes",
                        crate::MAX_TEXT_BYTES
                    )));
                }
                self.edit_value(id, Value::Text(text.clone()))
            }
            (Kind::Readout { .. }, _) => Err(invalid(
                "is a readout the game publishes; remove it from the file".to_owned(),
            )),
            (Kind::Slider { .. }, _) => Err(invalid("needs a number in display units".to_owned())),
            (Kind::Toggle, _) => Err(invalid("needs true or false".to_owned())),
            (Kind::Text, _) => Err(invalid("needs a string".to_owned())),
        }
    }
}
