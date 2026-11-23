//! The game and its browser controls, exposed through one page interface.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::{ControlBinding, WebHostError, WebShell, create_game};
use crate::{Game, InlineWorld, KeyCode, KeyResponse};

/// When browser edits reach the render runtime.
#[derive(Clone, Copy, Default)]
pub enum ControlTiming {
    /// Apply checked game edits during the input event, including pause.
    #[default]
    Immediate,
    /// Let the game read the latest control snapshot as it computes frame edits.
    FrameSnapshot,
}

/// A hosted browser game. A game-specific facade can retain this value to expose
/// its own scene operations alongside the shared page methods.
pub struct PageGame<G: Game<World: InlineWorld>> {
    /// The game's prepared browser shell.
    pub shell: WebShell<G::World, G::Scene>,
    /// The retained game and its controls.
    pub game: G,
    binding: ControlBinding,
    timing: ControlTiming,
    attachment_error: Option<Box<dyn FnMut(String)>>,
}

impl<G: Game<World: InlineWorld>> PageGame<G> {
    /// Retain an ordinary game and prepare it on the canvas.
    /// # Errors
    /// Returns the shell or game preparation failure.
    pub async fn new(
        canvas: HtmlCanvasElement,
        generation: u64,
        mut game: G,
        timing: ControlTiming,
    ) -> Result<Self, WebHostError> {
        let shell = create_game(canvas, generation, &mut game).await?;
        Ok(Self {
            shell,
            game,
            binding: ControlBinding::default(),
            timing,
            attachment_error: None,
        })
    }

    /// Receive a late attachment error from a replacement graphics device.
    pub fn on_attachment_error(&mut self, report: impl FnMut(String) + 'static) {
        self.attachment_error = Some(Box::new(report));
    }

    fn apply_controls(&mut self, now: f64) -> Result<(), JsValue> {
        match self.timing {
            ControlTiming::Immediate => {
                self.shell.apply_game_edits(&mut self.game, now).map_err(js)
            }
            ControlTiming::FrameSnapshot => {
                self.shell.update_profiler(self.game.controls_mut(), false);
                Ok(())
            }
        }
    }
}

/// Implementation boundary for the shared wasm exports.
#[doc(hidden)]
pub trait PageRuntime {
    fn controls_json(&mut self) -> String;
    fn control_changes(&mut self) -> Option<String>;
    fn edit_control(&mut self, key: &str, value: &JsValue, now: f64) -> Result<bool, JsValue>;
    fn parameters(&mut self, text: &str, now: f64) -> Result<thrum_controls::FileReport, JsValue>;
    fn frame(&mut self, now: f64) -> Result<(), JsValue>;
    fn set_visible(&mut self, visible: bool, now: f64) -> Result<(), JsValue>;
    fn set_paused(&mut self, paused: bool, now: f64) -> Result<(), JsValue>;
    fn paused(&self) -> bool;
    fn graphics_fault(&self) -> Option<String>;
    fn recovering(&self) -> bool;
    fn graphics_recovery_error(&self) -> Option<String>;
    fn request_graphics_recovery(&mut self) -> Result<(), JsValue>;
    fn key(&mut self, code: &str, pressed: bool, now: f64) -> Result<bool, JsValue>;
    fn focus_lost(&mut self) -> Result<(), JsValue>;
}

impl<G: Game<World: InlineWorld>> PageRuntime for PageGame<G> {
    fn controls_json(&mut self) -> String {
        match self.timing {
            ControlTiming::Immediate => self.binding.describe(self.game.controls()),
            ControlTiming::FrameSnapshot => self.game.controls().describe_json(),
        }
    }

    fn control_changes(&mut self) -> Option<String> {
        self.binding.changes(self.game.controls())
    }

    fn edit_control(&mut self, key: &str, value: &JsValue, now: f64) -> Result<bool, JsValue> {
        let changed = ControlBinding::edit(self.game.controls_mut(), key, value)?;
        self.apply_controls(now)?;
        Ok(changed)
    }

    fn parameters(&mut self, text: &str, now: f64) -> Result<thrum_controls::FileReport, JsValue> {
        let report = ControlBinding::parameters(self.game.controls_mut(), text)?;
        if matches!(self.timing, ControlTiming::Immediate) {
            self.apply_controls(now)?;
        }
        Ok(report)
    }

    fn frame(&mut self, now: f64) -> Result<(), JsValue> {
        if matches!(self.timing, ControlTiming::FrameSnapshot) {
            self.shell.update_profiler(self.game.controls_mut(), false);
        }
        let result = self.shell.frame_game(&mut self.game, now);
        if let Some(report) = self.attachment_error.as_mut()
            && let Some(error) = self.shell.take_attach_error()
        {
            report(error);
        }
        result.map_err(js)
    }

    fn set_visible(&mut self, visible: bool, now: f64) -> Result<(), JsValue> {
        self.shell.set_visible(visible, now).map_err(js)?;
        if matches!(self.timing, ControlTiming::FrameSnapshot) {
            self.shell.update_profiler(self.game.controls_mut(), false);
        }
        Ok(())
    }

    fn set_paused(&mut self, paused: bool, now: f64) -> Result<(), JsValue> {
        self.shell.set_paused(paused, now).map_err(js)?;
        self.game.observe_paused(self.shell.paused());
        Ok(())
    }

    fn paused(&self) -> bool {
        self.shell.paused()
    }

    fn graphics_fault(&self) -> Option<String> {
        self.shell.graphics_fault()
    }

    fn recovering(&self) -> bool {
        self.shell.recovering()
    }

    fn graphics_recovery_error(&self) -> Option<String> {
        self.shell.recovery_error().map(str::to_owned)
    }

    fn request_graphics_recovery(&mut self) -> Result<(), JsValue> {
        self.shell.request_graphics_recovery().map_err(js)
    }

    fn key(&mut self, code: &str, pressed: bool, now: f64) -> Result<bool, JsValue> {
        let Some(code) = KeyCode::from_code(code) else {
            return Ok(false);
        };
        let response = self.game.key(code, pressed).map_err(js)?;
        if response == KeyResponse::Edited {
            self.apply_controls(now)?;
        }
        Ok(response != KeyResponse::Ignored)
    }

    fn focus_lost(&mut self) -> Result<(), JsValue> {
        self.game.focus_lost().map_err(js)
    }
}

/// The page interface for any hosted game, ready to attach to the shared browser driver.
#[wasm_bindgen]
pub struct Page {
    runtime: Box<dyn PageRuntime>,
}

/// Prepare a game for the shared browser driver without writing a wasm facade.
/// A game exports only its launch function, returning this engine-owned value.
/// # Errors
/// Returns a device, surface or game preparation failure.
pub async fn create_page<G>(canvas: HtmlCanvasElement, game: G) -> Result<Page, WebHostError>
where
    G: Game<World: InlineWorld> + 'static,
{
    let runtime = PageGame::new(
        canvas,
        thrum_render::capabilities::FIRST_DEVICE_GENERATION,
        game,
        ControlTiming::Immediate,
    )
    .await?;
    Ok(Page {
        runtime: Box::new(runtime),
    })
}

impl Page {
    fn page_runtime(&self) -> &dyn PageRuntime {
        self.runtime.as_ref()
    }

    fn page_runtime_mut(&mut self) -> &mut dyn PageRuntime {
        self.runtime.as_mut()
    }
}

crate::export_page_methods!(Page);

/// Format a model report for a page or an explicit capture driver.
#[doc(hidden)]
pub fn parameter_json(report: &thrum_controls::FileReport) -> Result<String, JsValue> {
    let entries = js_sys::Array::new();
    for problem in &report.problems {
        entries.push(&JsValue::from_str(&problem.to_string()));
    }
    let problems = js_sys::JSON::stringify(&entries)?
        .as_string()
        .ok_or_else(|| js("control problems could not be serialized"))?;
    Ok(format!(
        "{{\"changed\":{},\"unchanged\":{},\"problems\":{problems}}}",
        report.changed, report.unchanged,
    ))
}

fn js(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
