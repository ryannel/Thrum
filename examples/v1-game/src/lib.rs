//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

#[cfg(target_arch = "wasm32")]
mod browser;
mod camera;
pub mod clouds;
pub use clouds::{CloudSelection, PlacedSky};
#[cfg(not(target_arch = "wasm32"))]
mod content;
#[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
mod content_preview;
pub mod controls;
#[cfg(feature = "proving")]
pub mod cooked;
#[cfg(feature = "proving")]
pub mod effect;
mod exit_scene;
#[cfg(feature = "proving")]
pub use exit_scene::SunProofGeometry;
mod field_meshes;
#[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
mod governor_load;
mod host;
#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
mod hosted;
#[cfg(feature = "proving")]
pub mod lent;
mod scene;
mod showcase;
pub mod simulation;
#[cfg(feature = "proving")]
mod sprite_card;
#[cfg(feature = "proving")]
mod sprite_probe;
#[cfg(feature = "proving")]
pub use sprite_probe::SpriteProbe;
#[cfg(feature = "proving")]
mod sprite_field;
#[cfg(target_os = "macos")]
mod window;

#[cfg(feature = "proving")]
use thrum_fixtures::mixed_field as field;
use thrum_fixtures::mixed_field::day_cycle as environment;

#[cfg(target_arch = "wasm32")]
pub use browser::{
    BrowserGame, CookedValley, create_browser_game, create_route_game, fetch_cooked_valley,
};
#[cfg(all(target_arch = "wasm32", feature = "draw-probe"))]
pub use browser::{BrowserLightProbe, create_light_list_probe};
#[cfg(all(target_arch = "wasm32", feature = "proving"))]
pub use browser::{
    GovernorBrowser, create_field_game, create_governor_game, create_placed_browser_game,
    create_sprite_game, create_sprite_probe, qualify_effect_after_opaque, qualify_effect_state,
    qualify_lent_products,
};
pub use camera::{CameraRoute, RouteCheckpoint, RouteKey, RoutePose, SceneCamera};
#[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
pub use content::attach_cooked_scene_with_probe;
#[cfg(all(feature = "proving", not(target_arch = "wasm32")))]
pub use content::{CloudQualityError, apply_cloud_quality};
#[cfg(not(target_arch = "wasm32"))]
pub use content::{attach_cooked_scene, selected_setup};
#[cfg(feature = "proving")]
pub use host::ExitEntities;
#[cfg(not(target_arch = "wasm32"))]
pub use host::WorldControls;
#[cfg(feature = "proving")]
#[doc(hidden)]
pub use host::field_in_view_share;
pub use host::{GameHost, GameReconstruction, GameScene, GameSetup, GameWorld, MeshRowSource};
pub use scene::{GameContent, RouteOverlay};
#[doc(hidden)]
pub use scene::{exit_authority, simulation_with_count};
#[doc(hidden)]
pub use showcase::{CookedValleyError, prepare_cooked_valley};
/// Authored valley configuration for qualification through the ordinary game host.
///
/// # Errors
/// Returns an invalid authored camera or placement.
#[cfg(all(target_os = "macos", feature = "draw-probe"))]
#[doc(hidden)]
pub fn valley_config(
    source: thrum_asset_gpu::CookedSource,
) -> Result<(SceneCamera, thrum_asset_gpu::CookedConfig), CookedValleyError> {
    showcase::valley_config(source)
}
#[cfg(all(target_os = "macos", feature = "proving"))]
pub use window::{
    CAPTURE_OBSERVER_VARIABLE, NATIVE_RECOVERY_VARIABLE, NativeRecovery, RenderScaleTrace,
    RuntimeCaptureOutput, SceneCaptureRequest, capture_game, run_runtime_capture,
    run_scene_capture,
};
#[cfg(target_os = "macos")]
pub use window::{WindowGovernor, run_window};
