//! Prepared replacement resources, host-bound until GPU-safe installation.

use super::{FrameError, FrameSlot, IN_FLIGHT, SlotState};
use crate::{
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
    timing::GpuTimings,
};
use std::sync::{Arc, atomic::AtomicBool};

/// A detached copy of the scene and executor handles for preparing one cooked
/// visual generation without borrowing the ticking host.
#[cfg(not(target_arch = "wasm32"))]
pub struct VisualPrepareInputs {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) scene: SyntheticScene,
    pub(super) surface: bool,
    pub(super) render_extent: [u32; 2],
    pub(super) owner: Arc<()>,
}

/// Prepared product and GPU targets awaiting an extraction-barrier selection.
/// Keep this value across a busy frame ring and retry publication later.
#[cfg(not(target_arch = "wasm32"))]
pub struct PreparedVisual {
    pub(super) lighting: FrameLighting,
    pub(super) revision: Option<[u8; 32]>,
    pub(super) prepared: Option<PreparedSynthetic>,
    pub(super) slots: Option<Vec<FrameSlot>>,
    pub(super) owner: Arc<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl VisualPrepareInputs {
    /// Build a complete presentation generation off the authoritative tick path.
    ///
    /// # Errors
    ///
    /// Returns a GPU preparation fault without affecting the running host.
    pub async fn prepare(
        self,
        lighting: FrameLighting,
        exposure: f32,
        revision: Option<[u8; 32]>,
    ) -> Result<PreparedVisual, FrameError> {
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            &self.capabilities,
            &self.queue,
            &self.scene,
            lighting,
            exposure,
        )
        .await
        .map_err(|error| format!("cooked visual prepare: {error:?}"))?;
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            slots.push(FrameSlot {
                target: OffscreenTarget::new(
                    &self.device,
                    &prepared,
                    self.render_extent[0],
                    self.render_extent[1],
                )
                .map_err(|error| format!("cooked target: {error:?}"))?,
                presentation: None,
                timings: GpuTimings::new(
                    &self.capabilities,
                    &self.queue,
                    prepared.scene_pass_count() + u32::from(self.surface),
                )
                .map_err(|error| format!("cooked timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        Ok(PreparedVisual {
            lighting,
            revision,
            prepared: Some(prepared),
            slots: Some(slots),
            owner: self.owner,
        })
    }
}
