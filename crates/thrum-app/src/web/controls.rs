//! Binding of the typed control model to its browser view.

use thrum_controls::{Controls, FileReport, Value};
use wasm_bindgen::JsValue;

/// Tracks the last revision sent to a page's control view.
#[derive(Default)]
pub struct ControlBinding {
    seen: u64,
}

impl ControlBinding {
    /// Describe the controls and bring the view up to the current revision.
    pub fn describe(&mut self, model: &Controls) -> String {
        self.seen = model.revision();
        model.describe_json()
    }

    /// Report edits made since the view last refreshed.
    pub fn changes(&mut self, model: &Controls) -> Option<String> {
        if self.seen == model.revision() {
            return None;
        }
        let changes = model.changes_json(self.seen);
        self.seen = model.revision();
        Some(changes)
    }

    /// Apply a value in display units through the typed model.
    /// # Errors
    /// Refuses unknown keys and values of the wrong type or range.
    pub fn edit(model: &mut Controls, key: &str, value: &JsValue) -> Result<bool, JsValue> {
        let id = model.lookup(key).map_err(js_error)?;
        if let Some(number) = value.as_f64() {
            return model.edit_display(id, number).map_err(js_error);
        }
        let value = if let Some(on) = value.as_bool() {
            Value::Flag(on)
        } else if let Some(text) = value.as_string() {
            Value::Text(text)
        } else {
            return Err(js_error(format!(
                "control \"{key}\" needs a number, boolean or string"
            )));
        };
        model.edit_value(id, value).map_err(js_error)
    }

    /// Apply a parameter file with the same rules as the native panel.
    /// # Errors
    /// Refuses an invalid parameter document.
    pub fn parameters(model: &mut Controls, text: &str) -> Result<FileReport, JsValue> {
        model.apply_file(text).map_err(js_error)
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
