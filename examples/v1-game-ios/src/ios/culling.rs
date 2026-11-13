//! Same-input still readbacks of the running simulator host and explicit Direct.

use std::fs;

use serde_json::json;
use thrum_render::{
    capabilities::DeviceCapabilities,
    synthetic::{CullMode, CullingSelection},
};
use v1_game::{FrameConfig, FrameHost, GameSetup};

fn selection(value: CullingSelection) -> serde_json::Value {
    json!({
        "requested": format!("{:?}", value.requested),
        "resolved": format!("{:?}", value.resolved),
        "reason": format!("{:?}", value.reason),
    })
}

pub(super) fn record(
    host: &mut FrameHost,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
    config: FrameConfig,
    setup: GameSetup,
) -> Result<(), String> {
    let expected = GameSetup {
        camera_culling: CullMode::Occlusion,
        ..GameSetup::default()
    };
    if setup != expected || std::env::var_os("THRUM_ASSET_STORE").is_some() {
        return Err(
            "culling oracle requires the ordinary diagnostic grid with Occlusion requested".into(),
        );
    }
    let direct_setup = GameSetup {
        camera_culling: CullMode::Direct,
        ..setup
    };
    let mut direct = pollster::block_on(FrameHost::new_with_setup(
        device,
        queue,
        capabilities,
        config,
        direct_setup,
    ))?;
    // Both ordinary hosts retain the initial committed world. The captured
    // fallback host then becomes the app's resident, presenting host.
    let fallback_pixels = host.capture_pixels()?;
    let direct_pixels = direct.capture_pixels()?;
    let path = std::env::temp_dir().join("thrum-culling-oracle");
    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    fs::write(path.join("fallback.rgba"), fallback_pixels).map_err(|error| error.to_string())?;
    fs::write(path.join("direct.rgba"), direct_pixels).map_err(|error| error.to_string())?;
    let identity = json!({
        "mode": "still",
        "extent": config.render_extent,
        "tick": host.tick(),
        "direct_tick": direct.tick(),
        "alpha": 1.0,
        "expected_instances": setup.content.entity_count(),
        "setup": format!("{setup:?}"),
        "direct_setup": format!("{direct_setup:?}"),
        "fallback": selection(host.culling_selection()),
        "direct": selection(direct.culling_selection()),
        "downlevel": format!("{:?}", capabilities.downlevel()),
        "scope": "same initial committed game input, final SDR engine readback on the simulator device",
    });
    fs::write(path.join("identity.json"), identity.to_string()).map_err(|error| error.to_string())
}
