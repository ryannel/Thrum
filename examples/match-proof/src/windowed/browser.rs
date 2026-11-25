use crowd_2d_view::{CrowdScene, proof};
use thrum_app::web::{WebShell, create_game};
use thrum_core::sim::DrivenWorld;
use thrum_render::runtime::CaptureLimits;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::{Driver, Peer, Replica, WindowMatch};

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

#[wasm_bindgen]
pub struct WindowedMatch {
    game: WindowMatch,
    shell: WebShell<DrivenWorld<Driver>, CrowdScene>,
}

#[wasm_bindgen]
impl WindowedMatch {
    /// Connect the real Chrome WebTransport and prepare its WebGPU canvas.
    pub async fn open(canvas: HtmlCanvasElement, ticket: &str) -> Result<Self, JsError> {
        let ticket = ticket.parse().map_err(js_error)?;
        let replica = Replica::connect(&ticket).map_err(js_error)?;
        let mut game = WindowMatch::new(Driver {
            peer: Peer::Replica(Box::new(replica)),
            rows: std::cell::RefCell::default(),
            last_frame: None,
        });
        let shell = create_game(canvas, 1, &mut game).await.map_err(js_error)?;
        Ok(Self { game, shell })
    }

    /// Advance through the ordinary WebGPU page shell.
    pub fn frame(&mut self, timestamp: f64) -> Result<(), JsError> {
        self.shell
            .frame_game(&mut self.game, timestamp)
            .map_err(js_error)
    }

    /// Follow the browser's actual document visibility.
    pub fn visible(&mut self, visible: bool, timestamp: f64) -> Result<(), JsError> {
        self.shell.set_visible(visible, timestamp).map_err(js_error)
    }

    /// Driver and frame witnesses produced since the preceding call.
    pub fn rows(&self) -> String {
        let driver = self.shell.host().world().driver();
        serde_json::Value::Array(std::mem::take(&mut *driver.rows.borrow_mut())).to_string()
    }

    /// Enable the T2 tap on the canvas submissions.
    pub fn begin_capture(&mut self) -> Result<(), JsError> {
        self.shell
            .begin_submitted_capture_with_limits(
                1,
                CaptureLimits {
                    staging_slots: 8,
                    cpu_buffers: 12,
                    byte_allowance: 128 * 1024 * 1024,
                },
            )
            .map_err(js_error)
    }

    /// Transfer one T2 frame to a reusable JavaScript buffer.
    pub fn capture(&mut self, pixels: &js_sys::Uint8Array) -> Result<Option<String>, JsError> {
        let Some(image) = self.shell.next_submitted_image().map_err(js_error)? else {
            return Ok(None);
        };
        if image.rgba.len() != pixels.length() as usize {
            return Err(JsError::new("capture extent changed"));
        }
        pixels.copy_from(&image.rgba);
        let mut metadata = proof::image_sample(&image);
        metadata["event"] = serde_json::json!("capture");
        self.shell.recycle_submitted_image(image);
        Ok(Some(metadata.to_string()))
    }

    /// Cumulative omissions of submitted images.
    pub fn omissions(&self) -> u64 {
        self.shell.host().submitted_capture_omissions()
    }

    /// Stop capture admission without dropping queued images.
    pub fn stop_capture(&mut self) {
        self.shell.stop_submitted_capture();
    }

    /// Number of image copies still pending.
    pub fn pending_capture(&self) -> usize {
        self.shell
            .host()
            .submitted_capture_usage()
            .map_or(0, |(pending, _)| pending)
    }

    /// Adapter used by the canvas.
    pub fn adapter(&self) -> String {
        self.shell.status().adapter_name
    }
}
