//! Assembled fixed-clock simulation and offscreen GPU scene used to qualify
//! Thrum's first V1 game workload. Platform surfaces and async publication
//! remain future consumers of the same engine owners.

mod host;
mod scene;

pub use host::{FrameHost, FrameSample};
