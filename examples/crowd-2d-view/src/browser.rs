use thrum_app::web::{WebShell, create_game};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{CrowdScene, CrowdView, CrowdWorld, proof};

/// Browser adapter for the ordinary Crowd scene and its submitted-output tap.
#[wasm_bindgen]
pub struct BrowserCrowd {
    game: CrowdView,
    shell: WebShell<CrowdWorld, CrowdScene>,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
impl BrowserCrowd {
    /// Prepare a canvas using the portable WebGPU recipe.
    pub async fn open(
        canvas: HtmlCanvasElement,
        count: u32,
        close: bool,
        exact: bool,
        timed: bool,
    ) -> Result<BrowserCrowd, JsValue> {
        let mut game = CrowdView::new(count, close)
            .map_err(js_error)?
            .with_timing(timed);
        if exact {
            game = game.exact_steps();
        }
        let shell = create_game(canvas, 1, &mut game).await.map_err(js_error)?;
        Ok(Self { game, shell })
    }

    /// Drive one canvas callback through the normal browser shell.
    pub fn step(&mut self, timestamp_ms: f64) -> Result<bool, JsValue> {
        let previous = self.shell.presented();
        self.shell
            .frame_game(&mut self.game, timestamp_ms)
            .map_err(js_error)?;
        Ok(self.shell.presented() > previous)
    }

    /// Bounded set of retired frame samples, drained by the proof.
    pub fn samples(&mut self) -> Result<String, JsValue> {
        self.shell.poll_completed().map_err(js_error)?;
        let collected: Vec<_> = self.shell.samples().iter().map(proof::sample).collect();
        let tick = self.shell.samples().last().map(|sample| sample.tick + 1);
        if let Some(tick) = tick {
            self.shell.discard_report_samples_before(tick);
        }
        Ok(serde_json::to_string(&collected).map_err(js_error)?)
    }

    /// The publication associated with the most recent submission.
    pub fn publication(&self, digest: bool) -> String {
        let mut publication = proof::publication(self.shell.host(), digest);
        publication["submission"] = serde_json::json!(self.shell.host().last_submission());
        publication.to_string()
    }

    /// The compiled graph and backend used by this canvas.
    pub fn identity(&self) -> Result<String, JsValue> {
        let host = self.shell.host();
        Ok(serde_json::json!({
            "passes": proof::passes(host).map_err(js_error)?,
            "adapter": self.shell.status().adapter_name,
            "recipe": format!("{:?}", host.device_capabilities().recipe()),
        })
        .to_string())
    }

    /// Refusal counters for timing records requested on this canvas.
    pub fn timings(&self) -> String {
        use thrum_render::timing::TimingRefusal as Refusal;
        let stats = self.shell.host().timing_stats();
        serde_json::json!({
            "gathered": stats.gathered,
            "published": stats.published,
            "zero": stats.refused(Refusal::Zero),
            "sentinel": stats.refused(Refusal::Sentinel),
            "inverted": stats.refused(Refusal::Inverted),
            "repeated": stats.refused(Refusal::Repeated),
            "layout": stats.refused(Refusal::Layout),
            "readback": stats.refused(Refusal::Readback),
            "reports_held": stats.refused(Refusal::ReportsHeld),
        })
        .to_string()
    }

    /// Start the T2 submitted-output observer on the running canvas.
    pub fn begin_capture(&mut self) -> Result<(), JsValue> {
        self.shell.begin_submitted_capture(1).map_err(js_error)
    }

    /// Poll a captured frame and pass its pixels to the caller.
    pub fn capture(&mut self) -> Result<Option<BrowserImage>, JsValue> {
        let Some(image) = self.shell.next_submitted_image().map_err(js_error)? else {
            return Ok(None);
        };
        let result = BrowserImage {
            metadata: proof::image_sample(&image).to_string(),
            rgba: image.rgba.clone(),
        };
        self.shell.recycle_submitted_image(image);
        Ok(Some(result))
    }
}

/// One completed T2 image, in the same submission's timing and extent.
#[wasm_bindgen]
pub struct BrowserImage {
    metadata: String,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl BrowserImage {
    /// Timing and submission identity for these exact pixels.
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }

    /// Copy the original tightly packed SDR output into the page.
    pub fn pixels(&self) -> Vec<u8> {
        self.rgba.clone()
    }
}
