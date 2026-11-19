//! Coherent target-ring and surface transitions on the render owner.

use super::{FrameError, FrameHost, FrameWorld, SceneSource, creation::CreationPhase, preparation};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use {
    super::{
        FrameSlot, SlotState,
        creation::Ledger,
        submitted_capture::{Tap, TapSpec},
    },
    crate::{
        present::{PresenterInputs, SurfacePresenter},
        synthetic::CaptureSource,
    },
    std::sync::atomic::{AtomicBool, Ordering},
};

/// Detached preparation for a new render and output extent. The inputs
/// retain handles to the current device's pipelines and extent-independent
/// resources, and own no world, queue submission state or surface.
#[cfg(not(target_arch = "wasm32"))]
pub struct ExtentPrepareInputs {
    extent: [u32; 2],
    device: wgpu::Device,
    queue: wgpu::Queue,
    creations: Ledger,
    targets: preparation::TargetPreparation,
    presenter: PresenterInputs,
    owner: Arc<()>,
    generation: u64,
    source: CaptureSource,
    capture: Option<TapSpec>,
}

/// A fully bound generation that can be taken after the surface configures.
#[cfg(not(target_arch = "wasm32"))]
pub struct PreparedExtent {
    capture: Option<Tap>,
    extent: [u32; 2],
    slots: Vec<FrameSlot>,
    presenter: SurfacePresenter,
    owner: Arc<()>,
    generation: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl ExtentPrepareInputs {
    /// Build entirely on the preparation thread. Cancellation discards the
    /// detached resources there while the host may keep presenting.
    /// # Errors
    /// Returns a preparation, allocation or validation fault for this request.
    pub fn build(self, cancel: &AtomicBool) -> Result<Option<PreparedExtent>, FrameError> {
        if cancel.load(Ordering::Acquire) {
            return Ok(None);
        }
        let _region = self
            .creations
            .region(CreationPhase::Transition, "prepare scene extent");
        let allocation = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = (|| {
            let presenter = self
                .presenter
                .prepare(self.extent, self.extent)
                .map_err(|error| FrameError::Gpu(format!("scene presenter extent: {error:?}")))?;
            let mut slots = self.targets.prepare_slots(
                &self.queue,
                self.extent,
                true,
                Some(presenter.profile()),
                self.source,
                Some(cancel),
            )?;
            if cancel.load(Ordering::Acquire) {
                return Ok(None);
            }
            self.targets.bind_presenter(&mut slots, &presenter)?;
            let capture = self
                .capture
                .map(|spec| {
                    Tap::prepare(
                        spec,
                        self.generation,
                        self.extent,
                        &slots[0].target,
                        self.targets.capabilities(),
                        &self.queue,
                    )
                })
                .transpose()?;
            Ok(Some(PreparedExtent {
                capture,
                extent: self.extent,
                slots,
                presenter,
                owner: self.owner,
                generation: self.generation,
            }))
        })();
        let validation_error = pollster::block_on(validation.pop());
        let allocation_error = pollster::block_on(allocation.pop());
        if cancel.load(Ordering::Acquire) {
            return Ok(None);
        }
        if let Some(error) = validation_error.or(allocation_error) {
            return Err(FrameError::Gpu(format!("extent preparation: {error}")));
        }
        result
    }
}

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
    /// Retain the preparation inputs for the current scene and device.
    /// # Errors
    /// Refuses a host without a presenter.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn extent_prepare_inputs(
        &self,
        extent: [u32; 2],
    ) -> Result<ExtentPrepareInputs, FrameError> {
        let presenter = self
            .presenter
            .as_ref()
            .ok_or(FrameError::InvalidState("host has no surface presenter"))?;
        Ok(ExtentPrepareInputs {
            extent,
            device: self.device.clone(),
            queue: self.queue.clone(),
            creations: self.creations.clone(),
            targets: preparation::TargetPreparation::new(&self.prepared),
            presenter: presenter.inputs(),
            owner: Arc::clone(&self.visual_owner),
            generation: self.generation,
            source: self.slots[0].target.capture_source(),
            capture: self.submitted_capture.as_ref().map(|tap| tap.spec),
        })
    }

    /// Validate a finished generation before the platform enters configure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn admits_extent(&self, candidate: &PreparedExtent) -> bool {
        candidate.generation == self.generation && Arc::ptr_eq(&self.visual_owner, &candidate.owner)
    }

    /// Take a generation after the matching configure succeeds. The old
    /// ring remains alive until its submission and timestamp work is collected.
    /// Returns false for a stale device or scene preparation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_extent(&mut self, candidate: PreparedExtent) -> bool {
        if !self.admits_extent(&candidate) {
            return false;
        }
        let previous = std::mem::replace(&mut self.slots, candidate.slots);
        self.retired_slots.extend(
            previous
                .into_iter()
                .filter(|slot| !matches!(slot.state, SlotState::Free)),
        );
        self.presenter = Some(candidate.presenter);
        self.switch_capture_extent(candidate.capture);
        self.render_extent = candidate.extent;
        self.presenter_generation += 1;
        self.visual_owner = Arc::new(());
        self.reset_replacement_history();
        true
    }

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
        let view = if self.scene_kind() == super::SceneKind::ThreeD {
            Some(
                self.source
                    .frame(
                        self.world.source(),
                        self.presentation_alpha(),
                        extent,
                        self.visual_lighting,
                    )
                    .map_err(FrameError::Scene)?
                    .view,
            )
        } else {
            self.source
                .sprites(self.world.source(), self.presentation_alpha(), extent)
                .map_err(FrameError::Scene)?
                .view
                .clip_scale(extent)
                .map_err(|error| FrameError::Scene(format!("resized sprite view: {error:?}")))?;
            None
        };
        let presenter = presenter
            .refit(extent, extent)
            .map_err(|error| FrameError::Gpu(format!("scene presenter resize: {error:?}")))?;
        let mut slots = preparation::prepare_slots(
            &self.prepared,
            &self.queue,
            extent,
            Some(presenter.profile()),
        )?;
        preparation::bind_presenter(&self.prepared, &mut slots, Some(&presenter))?;
        if !current() {
            return Ok(false);
        }
        self.slots = slots;
        self.presenter = Some(presenter);
        self.render_extent = extent;
        if let Some(view) = view {
            self.scene.view = view;
        }
        self.presenter_generation += 1;
        self.visual_owner = Arc::new(());
        self.reset_replacement_history();
        if self
            .prepared
            .three_d()
            .is_some_and(|scene| scene.secondary_view_count() != 0)
        {
            // A declared scale resolves against the new scene extent. Owners
            // must receive the replacement attachments before the next ask.
            self.rebuild_contribution();
        }
        // With no declared slot, SceneSurface has size-independent inputs;
        // ordinary contributions keep their pipelines through a rotation.
        Ok(true)
    }
}
