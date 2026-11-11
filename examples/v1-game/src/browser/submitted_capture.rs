//! Typed-array export of the ordinary renderer submission, with bounded leases.
use super::BrowserGame;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl BrowserGame {
    /// Start submitted-output capture with three GPU slots and eight CPU buffers.
    /// # Errors
    /// Refuses a lost device, an active session or allocation failure.
    pub fn begin_submitted_capture(&mut self, session: u32) -> Result<(), JsValue> {
        self.shell
            .begin_submitted_capture(u64::from(session))
            .map_err(super::js_error)
    }

    /// Capture scene-linear RGBA16Float through the same submitted-output tap.
    /// # Errors
    /// Refuses loss, an active session or preparation failure.
    pub fn begin_linear_capture(&mut self, session: u32) -> Result<(), JsValue> {
        self.shell
            .begin_linear_capture(u64::from(session))
            .map_err(super::js_error)
    }

    /// Export one image and its immutable submission identity without a redraw.
    /// The JS typed array owns a copy, so the renderer lease returns immediately.
    /// # Errors
    /// Refuses loss, retired generation, mapping or JS object construction failure.
    pub fn next_submitted_image(&mut self) -> Result<JsValue, JsValue> {
        let Some(image) = self.shell.next_submitted_image().map_err(super::js_error)? else {
            return Ok(JsValue::UNDEFINED);
        };
        let metadata = serde_json::json!({
            "session": image.session,
            "pixel_source": format!("{:?}", image.pixel_source),
            "exposure_gpu_words": image.exposure_gpu_words,
            "submission_id": image.sample.submission_order,
            "tick": image.sample.tick,
            "source_tick": image.source.tick,
            "source_epoch": image.source.epoch.to_string(),
            "timeline": image.source.timeline,
            "alpha": image.sample.presentation_alpha,
            "history_epoch": image.history_epoch,
            "device_generation": image.device_generation,
            "presenter_generation": image.presenter_generation,
            "submit_ms": image.submitted_elapsed.as_secs_f64() * 1000.0,
            "surface_presented": image.sample.surface_presented,
        });
        let pixels = js_sys::Uint8Array::from(image.rgba.as_slice());
        self.shell.recycle_submitted_image(image);
        let result = js_sys::Object::new();
        js_sys::Reflect::set(&result, &"metadata".into(), &metadata.to_string().into())?;
        js_sys::Reflect::set(&result, &"pixels".into(), &pixels)?;
        Ok(result.into())
    }

    /// Capture omissions invalidate a continuous interval.
    #[must_use]
    pub fn submitted_capture_omissions(&self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "finite capture sessions remain below 2^53"
        )]
        {
            self.shell.host().submitted_capture_omissions() as f64
        }
    }

    /// Readbacks still owned by the GPU, used for a bounded final drain.
    #[must_use]
    pub fn submitted_capture_pending(&self) -> usize {
        self.shell
            .host()
            .submitted_capture_usage()
            .map_or(0, |usage| usage.0)
    }

    /// Cancel pending readbacks without waiting for the GPU.
    pub fn end_submitted_capture(&mut self) {
        self.shell.end_submitted_capture();
    }
}
