//! Prepared replacement resources, host-bound until GPU-safe installation.

use super::{
    FrameError,
    creation::{CreationPhase, Detached},
    preparation::{PreparedScene, prepare_scene},
};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::ContributionPasses,
    synthetic::{FrameLighting, SyntheticScene},
};
use std::sync::Arc;

/// A detached copy of the scene and executor handles for preparing one cooked
/// visual generation without borrowing the ticking host.
pub struct VisualPrepareInputs {
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) scene: SyntheticScene,
    pub(super) surface: bool,
    pub(super) contribution_passes: ContributionPasses,
    pub(super) render_extent: [u32; 2],
    pub(super) owner: Arc<()>,
    pub(super) creations: Detached,
}

/// Prepared product and GPU targets awaiting an extraction-barrier selection.
/// Keep this value across a busy frame ring and retry publication later.
pub struct PreparedVisual {
    pub(super) lighting: FrameLighting,
    pub(super) revision: Option<[u8; 32]>,
    pub(super) scene: Option<PreparedScene>,
    pub(super) owner: Arc<()>,
}

impl VisualPrepareInputs {
    /// Build a complete presentation generation off the authoritative tick path.
    /// # Errors
    /// Returns a GPU preparation fault without affecting the running host.
    pub async fn prepare(
        self,
        lighting: FrameLighting,
        exposure: f32,
        revision: Option<[u8; 32]>,
    ) -> Result<PreparedVisual, FrameError> {
        let region = self
            .creations
            .region(CreationPhase::Transition, "prepare-visual");
        let scene = prepare_scene(
            &self.capabilities,
            &self.queue,
            &self.scene,
            lighting,
            exposure,
            self.render_extent,
            self.surface,
            self.contribution_passes,
        )
        .await?;
        drop(region);
        Ok(PreparedVisual {
            lighting,
            revision,
            scene: Some(scene),
            owner: self.owner,
        })
    }
}
