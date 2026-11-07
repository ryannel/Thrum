//! JSON shapes that browser views build controls from. Cold: called when a view is built
//! and when it syncs after the model's revision advances, never per entity.

use serde_json::{Map, Value as Json, json};

use crate::{Controls, Kind, Value};

impl Controls {
    /// Every declared control as JSON for a view to build widgets from.
    ///
    /// Each entry has `key`, `group`, `label`, `kind` (`slider`, `toggle`, `text` or
    /// `readout`), the current `value` and `text`, and for numbers a `display` object with
    /// `scale`, `decimals`, `unit` and `clock`. Slider `min`, `max` and `step` are in display
    /// units, so a view never converts anything except through `display.scale`.
    #[must_use]
    pub fn describe_json(&self) -> String {
        let entries: Vec<Json> = self
            .specs
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                let mut fields = Map::new();
                fields.insert("key".into(), spec.key.clone().into());
                fields.insert("group".into(), spec.group().into());
                fields.insert("label".into(), spec.label.clone().into());
                match &spec.kind {
                    Kind::Slider {
                        min,
                        max,
                        step,
                        display,
                    } => {
                        fields.insert("kind".into(), "slider".into());
                        fields.insert("min".into(), tidy(display.to_display(*min)).into());
                        fields.insert("max".into(), tidy(display.to_display(*max)).into());
                        fields.insert("step".into(), tidy(display.to_display(*step)).into());
                        fields.insert("display".into(), display_json(*display));
                    }
                    Kind::Toggle => {
                        fields.insert("kind".into(), "toggle".into());
                    }
                    Kind::Text => {
                        fields.insert("kind".into(), "text".into());
                    }
                    Kind::Readout { display } => {
                        fields.insert("kind".into(), "readout".into());
                        fields.insert("display".into(), display_json(*display));
                    }
                }
                self.insert_value(&mut fields, index);
                Json::Object(fields)
            })
            .collect();
        json!({ "revision": self.revision, "controls": entries }).to_string()
    }

    /// Values changed after `revision`, as `{"revision": r, "changed": [{key, value, text}]}`.
    #[must_use]
    pub fn changes_json(&self, revision: u64) -> String {
        let changed: Vec<Json> = self
            .changed_since(revision)
            .map(|id| {
                let mut fields = Map::new();
                fields.insert("key".into(), self.specs[id.index()].key.clone().into());
                self.insert_value(&mut fields, id.index());
                Json::Object(fields)
            })
            .collect();
        json!({ "revision": self.revision, "changed": changed }).to_string()
    }

    fn insert_value(&self, fields: &mut Map<String, Json>, index: usize) {
        let value = match (&self.specs[index].kind, &self.values[index]) {
            (Kind::Slider { display, .. } | Kind::Readout { display }, Value::Number(number)) => {
                tidy(display.to_display(*number)).into()
            }
            (_, Value::Number(number)) => tidy(f64::from(*number)).into(),
            (_, Value::Flag(on)) => (*on).into(),
            (_, Value::Text(text)) => text.clone().into(),
        };
        fields.insert("value".into(), value);
        if let Ok(index) = u16::try_from(index) {
            fields.insert(
                "text".into(),
                self.formatted(crate::ControlId(index)).into(),
            );
        }
    }
}

/// Round a display number to six significant digits. Engine values are `f32`, so `1°` in
/// radians shows as `0.99999994` without this; a view would then build a slider whose step
/// or range drifts from the authored one.
pub(crate) fn tidy(value: f64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let shown = format!("{value:.5e}");
    shown.parse().unwrap_or(value)
}

fn display_json(display: crate::Display) -> Json {
    json!({
        "scale": display.scale(),
        "decimals": display.decimals(),
        "unit": display.unit(),
        "clock": display.is_clock(),
    })
}
