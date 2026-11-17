//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

#[cfg(target_arch = "wasm32")]
mod browser;
mod camera;
pub mod clouds;
pub use clouds::CloudSelection;
#[cfg(not(target_arch = "wasm32"))]
mod content;
pub mod controls;
pub mod cooked;
pub mod effect;
mod environment;
mod exit_scene;
mod field;
mod field_meshes;
mod host;
pub mod lent;
mod projection;
mod scene;
mod showcase;
pub mod simulation;
#[cfg(target_os = "macos")]
mod window;

#[cfg(target_arch = "wasm32")]
pub use browser::{
    BrowserGame, CookedValley, create_browser_game, create_field_game, create_route_game,
    fetch_cooked_valley, qualify_effect_after_opaque, qualify_effect_state, qualify_lent_products,
};
#[cfg(all(target_arch = "wasm32", feature = "draw-probe"))]
pub use browser::{BrowserLightProbe, create_light_list_probe};
pub use camera::{CameraRoute, RouteCheckpoint, RouteKey, RoutePose, SceneCamera};
#[cfg(all(feature = "draw-probe", not(target_arch = "wasm32")))]
pub use content::attach_cooked_scene_with_probe;
#[cfg(not(target_arch = "wasm32"))]
pub use content::{CloudQualityError, apply_cloud_quality, attach_cooked_scene, selected_setup};
#[doc(hidden)]
pub use host::field_in_view_share;
pub use host::{
    AdvancedFrame, DriveReport, EncodedFrame, FrameConfig, FrameHost, FrameSample, GameSetup,
    MeshRowSource, SurfaceOutput,
};
#[cfg(not(target_arch = "wasm32"))]
pub use host::{ThreadedFrameHost, WorldControls};
pub use projection::{GameProjection, ProjectionGap};
pub use scene::{GameContent, RouteOverlay};
#[doc(hidden)]
pub use scene::{exit_authority, simulation_with_count};
#[doc(hidden)]
pub use showcase::prepare_showcase as prepare_cooked_valley;
#[cfg(target_os = "macos")]
pub use window::{
    NATIVE_RECOVERY_VARIABLE, NativeRecovery, capture_game, run_runtime_capture, run_window,
};
