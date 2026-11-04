//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Platform surfaces and worker snapshot
//! publication remain future consumers of the same engine owners.

mod host;
mod scene;

pub use host::{DriveReport, FrameHost, FrameSample, SurfaceOutput};
