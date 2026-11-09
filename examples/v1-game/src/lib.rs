//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

#[cfg(target_arch = "wasm32")]
mod browser;
mod camera;
#[cfg(not(target_arch = "wasm32"))]
mod content;
pub mod controls;
pub mod cooked;
mod environment;
mod field;
mod field_meshes;
mod host;
mod projection;
mod scene;
mod showcase;
pub mod simulation;
#[cfg(target_os = "macos")]
mod window;

#[cfg(target_arch = "wasm32")]
pub use browser::{
    BrowserGame, CookedValley, create_browser_game, create_field_game, create_route_game,
    fetch_cooked_valley,
};
pub use camera::{CameraRoute, RouteCheckpoint, RouteKey, RoutePose, SceneCamera};
#[cfg(feature = "draw-probe")]
pub use content::attach_cooked_scene_with_probe;
#[cfg(not(target_arch = "wasm32"))]
pub use content::{attach_cooked_scene, selected_setup};
#[doc(hidden)]
pub use host::field_in_view_share;
pub use host::{
    AdvancedFrame, DriveReport, EncodedFrame, FrameConfig, FrameHost, FrameSample, GameSetup,
    SurfaceOutput,
};
#[cfg(not(target_arch = "wasm32"))]
pub use host::{ThreadedFrameHost, WorldControls};
pub use projection::{GameProjection, ProjectionGap};
#[doc(hidden)]
pub use scene::simulation_with_count;
pub use scene::{GameContent, RouteOverlay};
#[cfg(target_os = "macos")]
pub use window::{capture_game, run_runtime_capture, run_window};
