//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
pub mod conditioned;
mod host;
mod scene;

#[cfg(target_arch = "wasm32")]
pub use browser::{BrowserGame, create_browser_game};
pub use host::{DriveReport, FrameHost, FrameSample, SurfaceOutput};
