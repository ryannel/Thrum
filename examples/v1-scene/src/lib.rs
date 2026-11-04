//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Native worker extraction and a macOS surface
//! consume the committed visual page; browser execution retains the serial path.

mod host;
mod scene;

pub use host::{DriveReport, FrameHost, FrameSample, SurfaceOutput};
