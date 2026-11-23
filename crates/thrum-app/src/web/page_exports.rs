//! Shared wasm methods for the engine page and game-specific diagnostic facades.

/// Add the engine's page methods to a facade that also exports game-specific
/// diagnostics. Ordinary games can return [`crate::web::Page`] without a facade.
#[macro_export]
macro_rules! export_page_methods {
    ($facade:ident) => {
        #[wasm_bindgen]
        impl $facade {
            /// Describe the game's typed controls in display units.
            pub fn controls_json(&mut self) -> String {
                self.page_runtime_mut().controls_json()
            }

            /// Read control changes since the last refresh.
            pub fn control_changes(&mut self) -> Option<String> {
                self.page_runtime_mut().control_changes()
            }

            /// Apply a typed control edit in display units.
            /// # Errors
            /// Returns the model's validation error or a refused lifecycle transition.
            pub fn edit_control(
                &mut self,
                key: &str,
                value: &JsValue,
                timestamp_ms: Option<f64>,
            ) -> Result<bool, JsValue> {
                self.page_runtime_mut()
                    .edit_control(key, value, timestamp_ms.unwrap_or_default())
            }

            /// Apply a parameter document and return its JSON report.
            /// # Errors
            /// Returns an invalid document or a refused game edit.
            pub fn apply_parameters(&mut self, text: &str, now: f64) -> Result<String, JsValue> {
                let report = self.page_runtime_mut().parameters(text, now)?;
                $crate::web::parameter_json(&report)
            }

            /// Apply a parameter document and report refused entries as text.
            /// # Errors
            /// Returns an invalid document or a refused game edit.
            pub fn apply_params(&mut self, text: &str) -> Result<String, JsValue> {
                let report = self.page_runtime_mut().parameters(text, 0.0)?;
                Ok(report
                    .problems
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"))
            }

            /// Drive a frame at the supplied browser timestamp.
            /// # Errors
            /// Returns a game or graphics fault.
            pub fn frame(&mut self, now: f64) -> Result<(), JsValue> {
                self.page_runtime_mut().frame(now)
            }

            /// Stop admitting time while hidden and resume from a fresh timestamp.
            /// # Errors
            /// Returns a clock or lifecycle refusal.
            pub fn set_visible(&mut self, visible: bool, now: f64) -> Result<(), JsValue> {
                self.page_runtime_mut().set_visible(visible, now)
            }

            /// Apply a local pause preference independently of page visibility.
            /// # Errors
            /// Returns a clock or lifecycle refusal.
            pub fn set_paused(&mut self, paused: bool, now: f64) -> Result<(), JsValue> {
                self.page_runtime_mut().set_paused(paused, now)
            }

            /// The local pause preference.
            pub fn paused(&self) -> bool {
                self.page_runtime().paused()
            }

            /// Persistent device-loss diagnostic.
            pub fn graphics_fault(&self) -> Option<String> {
                self.page_runtime().graphics_fault()
            }

            /// Whether a replacement is underway.
            pub fn recovering(&self) -> bool {
                self.page_runtime().recovering()
            }

            /// Why the most recent replacement attempt failed.
            pub fn graphics_recovery_error(&self) -> Option<String> {
                self.page_runtime().graphics_recovery_error()
            }

            /// Request a replacement for the lost graphics device.
            /// # Errors
            /// Returns a refusal if the device is healthy or a replacement is underway.
            pub fn request_graphics_recovery(&mut self) -> Result<(), JsValue> {
                self.page_runtime_mut().request_graphics_recovery()
            }

            /// Request a replacement for the lost graphics device.
            /// # Errors
            /// Returns a refusal if the device is healthy or a replacement is underway.
            pub fn recover_graphics(&mut self) -> Result<(), JsValue> {
                self.request_graphics_recovery()
            }

            /// Deliver a physical key. Returns whether the game handled it.
            /// # Errors
            /// Returns the game's input or lifecycle refusal.
            pub fn key(&mut self, code: &str, pressed: bool, now: f64) -> Result<bool, JsValue> {
                self.page_runtime_mut().key(code, pressed, now)
            }

            /// Release held input when focus leaves the game.
            /// # Errors
            /// Returns the game's input refusal.
            pub fn focus_lost(&mut self) -> Result<(), JsValue> {
                self.page_runtime_mut().focus_lost()
            }
        }
    };
}
