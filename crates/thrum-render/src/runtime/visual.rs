//! Prepared replacement resources, host-bound until GPU-safe installation.

use super::{
    FrameError,
    creation::{CreationPhase, Detached},
    preparation::{self, PreparedScene, prepare_scene},
};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::ContributionPasses,
    present::{SurfacePresenter, SurfaceProfile},
    synthetic::{FrameLighting, RetainedRayMeshes, SyntheticScene},
};
use std::sync::Arc;

/// A detached copy of the scene and executor handles for preparing one cooked
/// visual generation without borrowing the ticking host.
pub struct VisualPrepareInputs {
    pub(super) supports_candidates: bool,
    pub(super) kind: super::SceneKind,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) scene: SyntheticScene,
    pub(super) retained_ray_meshes: Option<RetainedRayMeshes>,
    pub(super) surface: bool,
    pub(super) contribution_passes: ContributionPasses,
    pub(super) effect_graph: crate::contribution::EffectGraph,
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
    pub(super) extent: Option<([u32; 2], SurfacePresenter)>,
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
        Box::pin(self.prepare_bound(lighting, exposure, revision, None)).await
    }

    async fn prepare_bound(
        self,
        lighting: FrameLighting,
        exposure: f32,
        revision: Option<[u8; 32]>,
        surface_profile: Option<SurfaceProfile>,
    ) -> Result<PreparedVisual, FrameError> {
        if self.kind == super::SceneKind::TwoD {
            return Err(FrameError::InvalidState(
                "2D scene refuses a 3D visual replacement",
            ));
        }
        let region = self
            .creations
            .region(CreationPhase::Transition, "prepare-visual");
        let admission = preparation::contribution_admission(
            self.scene.ray_admission(&self.capabilities),
            self.supports_candidates,
        );
        let (scene, presenter) = preparation::prepare_3d_transition(admission, async |selection| {
            preparation::validate_attempt(&self.capabilities, selection, async || {
                let (passes, graph) = if preparation::is_candidate_transition(selection) {
                    (
                        ContributionPasses::default(),
                        crate::contribution::EffectGraph::default(),
                    )
                } else {
                    (self.contribution_passes, self.effect_graph.clone())
                };
                let mut scene = prepare_scene(
                    &self.capabilities,
                    &self.queue,
                    &self.scene,
                    lighting,
                    exposure,
                    self.render_extent,
                    self.surface || surface_profile.is_some(),
                    passes,
                    graph,
                    self.retained_ray_meshes.clone(),
                    selection,
                )
                .await?;
                let presenter = if let Some(profile) = surface_profile {
                    let presenter =
                        SurfacePresenter::prepare(&self.capabilities, self.render_extent, profile)
                            .await
                            .map_err(|error| {
                                FrameError::gpu("extent presenter", error)
                            })?;
                    scene.bind_presenter(Some(&presenter))?;
                    Some(presenter)
                } else {
                    None
                };
                Ok((scene, presenter))
            })
            .await
        })
        .await?;
        drop(region);
        Ok(PreparedVisual {
            lighting,
            revision,
            scene: Some(scene),
            owner: self.owner,
            extent: presenter.map(|presenter| (self.render_extent, presenter)),
        })
    }

    /// Prepare a bounded scene/provider extent replacement on a worker, including
    /// its presenter. The active host remains usable throughout.
    /// # Errors
    /// Refuses invalid extents or GPU preparation without changing the live view.
    pub async fn prepare_extent(
        mut self,
        extent: [u32; 2],
        mut profile: SurfaceProfile,
        lighting: FrameLighting,
        exposure: f32,
    ) -> Result<PreparedVisual, FrameError> {
        if extent.contains(&0) {
            return Err(FrameError::InvalidState("extent must be nonzero"));
        }
        self.render_extent = extent;
        profile.extent = extent;
        Box::pin(self.prepare_bound(lighting, exposure, None, Some(profile))).await
    }
}
