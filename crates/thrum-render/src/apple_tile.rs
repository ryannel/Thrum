//! Native provider interface and the whole-encoder segment.
pub(crate) mod metalfx;
mod segment;
pub(crate) use segment::{
    NativeBinding, NativeFrame, NativeFrameValues, NativeGraph, NativeGraphInputs, NativePolicy,
    NativePreparation, NativeProvider, NativeState, PreparedProvider,
};
pub use segment::{
    NativeOutcome, NativeProviderReport, NativeSegment, NativeTwinSelection, TwinReason,
};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod metal;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use metal::{CommandBuffer, Texture, texture};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod witness;
