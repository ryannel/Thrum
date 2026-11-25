//! Browser diagnostics for the sprite scene's authored texels and frame reports.

use super::BrowserGame;
use thrum_render::runtime::SceneKind;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl BrowserGame {
    /// Select the next camera pose of the diagnostic sprite sweep.
    ///
    /// # Errors
    /// Refuses an inactive sweep or a pose outside 0..256.
    pub fn set_sprite_sweep_step(&mut self, step: u32) -> Result<(), JsValue> {
        super::game::set_sprite_sweep_step(&mut self.page.shell, step).map_err(super::js_error)
    }

    /// Authored card texels used by the check's independent CPU rasterizer.
    pub fn sprite_card_texels(&self) -> Option<Vec<u8>> {
        (self.page.shell.host().scene_kind() == SceneKind::TwoD).then(crate::sprite_card::texels)
    }

    /// Report the sprite scene's compiled plan and completed presentation samples.
    /// # Errors
    /// Refuses a different scene or a graph that cannot be inspected.
    pub fn sprite_scene_report(&self) -> Result<String, JsValue> {
        let host = self.page.shell.host();
        if host.scene_kind() != SceneKind::TwoD {
            return Err(super::js_error("sprite scene is not selected"));
        }
        let graph = host.scene_graph().map_err(super::js_error)?;
        let limits = host.device_capabilities().limits();
        let samples: Vec<_> = self
            .page
            .shell
            .samples()
            .iter()
            .filter_map(|sample| {
                let timings = sample.gpu_pass_ns.as_ref()?;
                let present = sample.gpu_present_ns?;
                Some(serde_json::json!({
                    "submission": sample.submission_order,
                    "tick_cpu_ns": sample.tick_cpu.as_nanos(),
                    "extract_cpu_ns": sample.extract_cpu.as_nanos(),
                    "upload_cpu_ns": sample.upload_cpu.as_nanos(),
                    "encode_cpu_ns": sample.encode_cpu.as_nanos(),
                    "render_cpu_ns": sample.render_cpu.as_nanos(),
                    "wall_ns": sample.wall.as_nanos(),
                    "pass_ns": timings.values(),
                    "interface": sample.interface.map(|stats| serde_json::json!({
                        "upload_cpu_ns": stats.upload_cpu.as_nanos(),
                        "encode_cpu_ns": stats.encode_cpu.as_nanos(),
                        "uploaded_bytes": stats.uploaded_bytes,
                        "dropped_rows": stats.dropped_rows,
                        "extent_mismatch": stats.extent_mismatch,
                        "gpu_ns": timings.passes().iter()
                            .find(|pass| pass.name.as_ref() == "interface stage")
                            .map(|pass| pass.nanoseconds),
                    })),
                    "present_ns": present,
                    "gpu_span_ns": timings.timed_pass_span_nanoseconds(),
                    "rows": sample.uploaded_rows,
                    "sprites": crate::sprite_field::sample_stats(sample),
                }))
            })
            .collect();
        Ok(serde_json::json!({
            "kind": "TwoD",
            "generation": graph.device_generation,
            "passes": graph.pass_names().collect::<Vec<_>>(),
            "reconstructor": host.resolved_reconstructor().map(|provider| format!("{provider:?}")),
            "sampled_textures": limits.max_sampled_textures_per_shader_stage,
            "array_layers": limits.max_texture_array_layers,
            "storage_buffers": limits.max_storage_buffers_per_shader_stage,
            "storage": host.storage_report(),
            "samples": samples,
        })
        .to_string())
    }
}
