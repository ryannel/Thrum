//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

#[cfg(target_arch = "wasm32")]
mod browser;
mod camera;
#[cfg(not(target_arch = "wasm32"))]
mod content;
pub mod cooked;
mod environment;
mod host;
mod projection;
mod scene;
mod showcase;
pub mod simulation;

#[cfg(target_arch = "wasm32")]
pub use browser::{BrowserGame, CookedValley, create_browser_game, fetch_cooked_valley};
pub use camera::SceneCamera;
#[cfg(not(target_arch = "wasm32"))]
pub use content::attach_cooked_scene;
pub use host::{
    AdvancedFrame, DriveReport, EncodedFrame, FrameConfig, FrameHost, FrameSample, SurfaceOutput,
};
#[cfg(not(target_arch = "wasm32"))]
pub use host::{ThreadedFrameHost, WorldControls};
pub use projection::{GameProjection, ProjectionGap};
#[doc(hidden)]
pub use scene::simulation_with_count;
