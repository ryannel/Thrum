//! Browser qualification of both production timing sources.

use super::{GameControls, HostedGame, game, js_error};
use crate::governor_load::DebugLoad;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const WARMUP_TICKS: u64 = 60;
const LOAD_START: u64 = WARMUP_TICKS + 60;
const LOAD_END: u64 = LOAD_START + 93;

/// The ordinary browser frame host with a scheduled render-only load.
#[wasm_bindgen]
pub struct GovernorBrowser {
    shell: game::Shell,
    governor_enabled: bool,
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
        sprite_store: None,
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
        .set_scene_contribution(Box::new(DebugLoad::new(LOAD_START..LOAD_END, rounds)))
        .map_err(js_error)?;
    Ok(GovernorBrowser {
        shell,
        governor_enabled: false,
    })
}

#[wasm_bindgen]
impl GovernorBrowser {
    /// Drive a display frame with the timestamp provided by requestAnimationFrame.
    /// # Errors
    /// Returns a browser frame-host fault or a refusal to enable the governor.
    pub fn frame(&mut self, timestamp: f64) -> Result<(), JsValue> {
        // Let first-use rendering settle before the policy admits timing samples.
        if !self.governor_enabled && self.shell.host().world().tick() >= WARMUP_TICKS {
            if !self.shell.enable_governor() {
                return Err(js_error("governor requires a prepared TAAU scene"));
            }
            self.governor_enabled = true;
        }
        self.shell.frame(timestamp).map_err(js_error)
    }

    /// Take the completed trace rows while leaving space for subsequent samples.
    pub fn take_trace(&mut self) -> String {
        let through = self.shell.samples().last().map(|sample| sample.tick);
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
        if let Some(tick) = through {
            self.shell.discard_report_samples_before(tick + 1);
        }
        serde_json::json!({
            "tick": self.shell.host().world().tick(),
            "presented": self.shell.presented(),
            "load_start": LOAD_START,
            "load_end": LOAD_END,
            "rows": rows,
        })
        .to_string()
    }
}
