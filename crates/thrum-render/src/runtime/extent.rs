//! Coherent target-ring and surface transitions on the render owner.

use super::{FrameError, FrameHost, FrameWorld, SceneSource, creation::CreationPhase, preparation};

/// Resource and camera state of one encoded frame, without GPU readback.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameExtent {
    /// Depth, scene, motion and output resource dimensions.
    pub attachments: [[u32; 2]; 4],
    /// Unique graph textures per slot, including fixed-size shadow maps.
    pub target_textures: u32,
    /// Unpadded texture bytes per slot, excluding driver padding and buffers.
    pub target_bytes: u64,
    /// Matrix packed for the scene's vertex shader, column major.
    pub view_projection: [[f32; 4]; 4],
    /// Previous unjittered matrix, from the same submitted state used for packing.
    pub previous_view_projection: [[f32; 4]; 4],
    /// Presenter source grid.
    pub source: [u32; 2],
    /// Presenter destination grid.
    pub destination: [u32; 2],
    /// Viewport actually selected by the presentation recorder.
    pub viewport: [u32; 4],
    /// Submission that this encoded token will receive on success.
    pub submission: u64,
    /// Presenter/target transition identity, stable between resizes.
    pub revision: u64,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Change the scene grid and surface together between completed frames.
    /// Zero extents and occupied slots defer with `false`. Retry with the latest
    /// extent after normal completion polling. Identical extents reuse resources.
    /// Camera policy, content, world and device remain owned by this host.
    /// # Errors
    /// Refuses an absent presenter, invalid extent or failed preparation before
    /// publishing either targets or presenter. The prior grid remains coherent.
    pub fn resize_scene_surface(&mut self, extent: [u32; 2]) -> Result<bool, FrameError> {
        self.resize_scene_surface_if(extent, || true)
    }

    /// As [`Self::resize_scene_surface`], with a platform epoch check immediately
    /// before publication. Stale preparations drop without changing the host.
    /// # Errors
    /// Returns the same preparation faults as [`Self::resize_scene_surface`].
    pub fn resize_scene_surface_if(
        &mut self,
        extent: [u32; 2],
        current: impl FnOnce() -> bool,
    ) -> Result<bool, FrameError> {
        if extent.contains(&0) {
            return Ok(false);
        }
        let presenter = self
            .presenter
            .as_ref()
            .ok_or(FrameError::InvalidState("host has no surface presenter"))?;
        if extent == self.render_extent && extent == presenter.extent() {
            return Ok(true);
        }
        if self.pending_frames() != 0 {
            return Ok(false);
        }
        let _region = self
            .creations
            .region(CreationPhase::Transition, "resize-scene-surface");
        let view = self
            .source
            .frame(
                self.world.source(),
                self.presentation_alpha(),
                extent,
                self.visual_lighting,
            )
            .map_err(FrameError::Scene)?
            .view;
        let presenter = presenter
            .refit(extent, extent)
            .map_err(|error| FrameError::Gpu(format!("scene presenter resize: {error:?}")))?;
        let mut slots = preparation::prepare_slots(&self.prepared, &self.queue, extent, true)?;
        preparation::bind_presenter(&self.prepared, &mut slots, Some(&presenter))?;
        if !current() {
            return Ok(false);
        }
        self.slots = slots;
        self.presenter = Some(presenter);
        self.render_extent = extent;
        self.scene.view = view;
        self.presenter_generation += 1;
        self.visual_owner = std::sync::Arc::new(());
        self.reset_replacement_history();
        // SceneSurface has only size-independent device/layout inputs. Contributions
        // receive the new matrix and history reset through ContributionFrame.
        // Rebuilding here would compile identical cooked pipelines on every rotation.
        Ok(true)
    }
}
