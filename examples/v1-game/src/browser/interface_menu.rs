use std::time::Duration;

use thrum_app::{Game, KeyCode};
use thrum_ui::{Event, TextSource};
use wasm_bindgen::prelude::*;

use super::{BrowserGame, js_error};

#[wasm_bindgen]
impl BrowserGame {
    /// Deliver a physical browser key to the interface's router.
    /// # Errors
    /// Returns a refused game input.
    pub fn menu_key(
        &mut self,
        code: &str,
        down: bool,
        repeat: bool,
        shift: bool,
        now: f64,
    ) -> Result<(), JsValue> {
        if let Some(code) = KeyCode::from_code(code) {
            self.menu_event(thrum_app::interface_key(code, down, repeat, shift), now)?;
        }
        Ok(())
    }

    /// Deliver a browser pointer or touch contact, in canvas backing pixels.
    /// # Errors
    /// Returns a malformed event or a refused game input.
    pub fn menu_pointer(
        &mut self,
        source: &str,
        phase: &str,
        id: u32,
        point: Vec<f64>,
        now: f64,
    ) -> Result<(), JsValue> {
        let point = point
            .try_into()
            .map_err(|_| js_error("pointer needs x and y"))?;
        let event =
            thrum_app::web::interface_pointer(source, phase, id, point).map_err(js_error)?;
        self.menu_event(event, now)
    }

    /// Deliver the whole location fragment or paste value.
    /// # Errors
    /// Returns a refused game input.
    pub fn menu_text(&mut self, text: String, paste: bool, now: f64) -> Result<(), JsValue> {
        if paste {
            self.menu_event(Event::Text(text, TextSource::Paste), now)
        } else {
            if let Some(menu) = &mut self.page.game.0.menu {
                menu.launch(text, TextSource::Link, event_time(now)?);
            }
            Ok(())
        }
    }

    /// Cancel all held contacts when browser focus or visibility is lost.
    /// # Errors
    /// Returns a refused game input.
    pub fn menu_interrupt(&mut self, now: f64) -> Result<(), JsValue> {
        self.menu_event(Event::Interrupted, now)
    }

    /// Retain the proof's submitted frames, game input and activations for this page.
    pub fn record_menu_events(&mut self) {
        if let Some(menu) = &mut self.page.game.0.menu {
            menu.record();
        }
    }

    /// Latest menu proof record.
    pub fn menu_report(&self) -> Option<String> {
        Some(self.page.game.0.menu.as_ref()?.report().to_string())
    }

    /// Trace routed copies of the real event and its pre-event state in this renderer.
    pub fn profile_menu_routing(&self) -> Option<String> {
        Some(
            self.page
                .game
                .0
                .menu
                .as_ref()?
                .profile_routes()?
                .to_string(),
        )
    }
}

impl BrowserGame {
    fn menu_event(&mut self, event: Event<KeyCode>, now: f64) -> Result<(), JsValue> {
        self.page
            .game
            .interface_input(event, event_time(now)?)
            .map(|_| ())
            .map_err(js_error)
    }
}

fn event_time(now: f64) -> Result<Duration, JsValue> {
    Duration::try_from_secs_f64(now / 1000.0).map_err(js_error)
}
