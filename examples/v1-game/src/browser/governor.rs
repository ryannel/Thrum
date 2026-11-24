//! Browser qualification of both production timing sources.

use super::{GameControls, HostedGame, game, js_error};
use crate::governor_load::DebugLoad;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// The ordinary browser frame host with a scheduled render-only load.
#[wasm_bindgen]
pub struct GovernorBrowser {
    shell: game::Shell,
}

/// Prepare the real canvas, optionally removing timestamp queries from its device.
///
/// # Errors
/// Returns a WebGPU, world or scene preparation failure.
#[wasm_bindgen]
pub async fn create_governor_game(
    canvas: HtmlCanvasElement,
    timestamps: bool,
    rounds: u32,
) -> Result<GovernorBrowser, JsValue> {
    if rounds > DebugLoad::MAX_ROUNDS {
        return Err(js_error("debug load rounds exceed the qualification limit"));
    }
    let mut hosted = HostedGame(game::PageGame {
        demo: GameControls::new().map_err(js_error)?,
        setup: crate::GameSetup {
            reconstruction: thrum_render::synthetic::Reconstruction::Taau,
            camera_culling: thrum_render::synthetic::CullMode::Frustum,
            route_overlay: crate::RouteOverlay::Proof,
            ..Default::default()
        },
        shapes: None,
        initial_downlevel: String::new(),
    });
    let mask = if timestamps {
        wgpu::Features::empty()
    } else {
        wgpu::Features::TIMESTAMP_QUERY
    };
    let mut shell = thrum_app::web::create_game_with_feature_mask(
        canvas,
        game::DEVICE_GENERATION,
        &mut hosted,
        mask,
    )
    .await
    .map_err(js_error)?;
    shell
        .set_scene_contribution(Box::new(DebugLoad::new(75..168, rounds)))
        .map_err(js_error)?;
    if !shell.enable_governor() {
        return Err(js_error("governor requires a prepared TAAU scene"));
    }
    Ok(GovernorBrowser { shell })
}

#[wasm_bindgen]
impl GovernorBrowser {
    /// Drive a display frame with the timestamp provided by requestAnimationFrame.
    /// # Errors
    /// Returns a browser frame-host fault.
    pub fn frame(&mut self, timestamp: f64) -> Result<(), JsValue> {
        self.shell.frame(timestamp).map_err(js_error)
    }

    /// Collect the timing source, state and complete timestamped trace.
    pub fn trace(&self) -> String {
        let rows = self.shell.samples().iter().filter_map(|sample| {
            let quality = sample.governor.as_ref()?;
            Some(serde_json::json!({
                "tick": sample.tick,
                "submission": sample.submission_order,
                "time_ms": sample.observation.as_secs_f64() * 1000.0,
                "scale": quality.state.scale.percent(),
                "tier": quality.state.tier,
                "reason": format!("{:?}", quality.state.reason),
                "source": quality.source.map(|source| format!("{source:?}")),
                "budget_ms": quality.budget.as_secs_f64() * 1000.0,
                "gpu_ns": sample.gpu_pass_ns.as_ref().map(|gpu| gpu.sum() + sample.gpu_present_ns.unwrap_or_default()),
                "debug_load_ns": sample.gpu_pass_ns.as_ref().and_then(|gpu| gpu.passes().iter().find(|pass| pass.name.ends_with("debug load")).map(|pass| pass.nanoseconds)),
            }))
        }).collect::<Vec<_>>();
        serde_json::json!({
            "tick": self.shell.host().world().tick(),
            "presented": self.shell.presented(),
            "rows": rows,
        })
        .to_string()
    }
}
