//! Deterministic 48 kHz stereo audio mixing with no device required.
mod handle;
mod mix;
mod record;
mod samples;
mod spatial;
mod types;

pub mod diagnostics;

#[cfg(all(target_arch = "wasm32", feature = "mix-module"))]
mod module;

pub use handle::Handle;
pub use mix::Mix;
pub use types::{
    Config, EmitterId, Error, Fate, Listener, Outcome, Place, Play, PlayNumber, Record, SoundId,
    VoiceSettings, Volumes,
};

/// Stereo frames generated per second.
pub const SAMPLE_RATE: u32 = 48_000;
