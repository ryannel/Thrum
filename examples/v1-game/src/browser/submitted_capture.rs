//! Typed-array export of the ordinary renderer submission, with bounded leases.
use thrum_app::Game;

use super::BrowserGame;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl BrowserGame {
    /// Select a motion capture without the optional GPU pass timing observer.
    pub fn capture_without_pass_timings(&mut self) {
        self.shell.update_profiler(self.game.controls_mut(), false);
    }

    /// Start submitted-output capture with three GPU slots and eight CPU buffers.
    /// # Errors
    /// Refuses a lost device, an active session or allocation failure.
    pub fn begin_submitted_capture(&mut self, session: u32) -> Result<(), JsValue> {
        self.shell
            .begin_submitted_capture(u64::from(session))
            .map_err(super::js_error)
    }

    /// Start a held SDR capture with a single staging slot and a single CPU lease.
    ///
    /// # Errors
    /// Refuses a lost device, an active session or an extent beyond the byte allowance.
    pub fn begin_fixed_submission(&mut self, session: u32) -> Result<(), JsValue> {
        self.shell
            .begin_submitted_capture_with_limits(
                u64::from(session),
                thrum_render::runtime::CaptureLimits {
                    staging_slots: 1,
                    cpu_buffers: 1,
                    ..Default::default()
                },
            )
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
    /// Copy into a correctly sized caller-owned array or allocate one for this
    /// image. The renderer lease returns immediately.
    /// # Errors
    /// Refuses loss, retired generation, mapping or JS object construction failure.
    pub fn next_submitted_image(
        &mut self,
        reuse: Option<js_sys::Uint8Array>,
    ) -> Result<JsValue, JsValue> {
        let Some(image) = self.shell.next_submitted_image().map_err(super::js_error)? else {
            return Ok(JsValue::UNDEFINED);
        };
        let mut metadata = serde_json::json!({
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
        if let Some(stats) = image.sample.sprites {
            metadata["sprite_movers"] = image.sample.moving.into();
            metadata["sprite_draws"] = stats.draws.into();
        }
        let pixels = if let Some(reuse) =
            reuse.filter(|buffer| buffer.length() as usize == image.rgba.len())
        {
            reuse.copy_from(&image.rgba);
            reuse
        } else {
            js_sys::Uint8Array::from(image.rgba.as_slice())
        };
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
