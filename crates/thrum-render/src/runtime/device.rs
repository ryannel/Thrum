//! Detached GPU-only reconstruction on a platform-selected replacement device.

use super::{
    FrameError, FrameHost, FrameWorld, SceneSource,
    creation::{CreationPhase, Ledger},
    preparation::{self, PreparedRenderer, PreparedScene, prepare_scene, prepare_sprites},
};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::ContributionPasses,
    present::{SurfacePresenter, SurfaceProfile},
    synthetic::{FrameLighting, SyntheticScene},
};
use std::sync::Arc;

/// Retained CPU presentation inputs for one device reconstruction attempt.
/// Creating or consuming this snapshot never transfers or advances the world.
/// Camera and dynamic lighting policy are sampled again at publication.
pub struct DevicePrepareInputs {
    base_device: wgpu::Device,
    base_generation: u64,
    owner: Arc<()>,
    scene: RetainedScene,
    render_extent: [u32; 2],
}

enum RetainedScene {
    ThreeD(Box<RetainedThreeD>),
    TwoD(crate::sprite::SpriteScene),
}

struct RetainedThreeD {
    scene: SyntheticScene,
    lighting: FrameLighting,
    exposure: f32,
    contribution_passes: ContributionPasses,
    supports_candidates: bool,
}

/// Complete replacement resources, bound to one host and its base generation.
/// The platform must reject a lost candidate device before publication.
pub struct PreparedDevice {
    base_generation: u64,
    owner: Arc<()>,
    capabilities: DeviceCapabilities,
    queue: wgpu::Queue,
    scene: Option<PreparedScene>,
    presenter: Option<SurfacePresenter>,
    creations: Ledger,
    render_extent: [u32; 2],
}

impl PreparedDevice {
    /// Fit the candidate to a surface resized during detached preparation.
    /// This touches only the candidate device and retains it on failure.
    /// # Errors
    /// Refuses an invalid extent, consumed candidate or absent presenter.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), FrameError> {
        if self.scene.is_none() {
            return Err(FrameError::InvalidState(
                "device replacement already published",
            ));
        }
        if self
            .scene
            .as_ref()
            .is_some_and(|scene| matches!(scene.prepared, PreparedRenderer::TwoD(_)))
        {
            return self.resize_scene_surface(extent);
        }
        let _region = self.creations.region(CreationPhase::Build, "device-resize");
        self.presenter
            .as_mut()
            .ok_or(FrameError::InvalidState(
                "candidate has no surface presenter",
            ))?
            .resize(self.capabilities.device(), extent)
            .map_err(|error| FrameError::gpu("replacement surface resize", error))
    }
    /// Refit both candidate scene targets and presenter after an oriented resize.
    /// # Errors
    /// Refuses zero/invalid extents or a consumed candidate without publication.
    pub fn resize_scene_surface(&mut self, extent: [u32; 2]) -> Result<(), FrameError> {
        let scene = self.scene.as_mut().ok_or(FrameError::InvalidState(
            "device replacement already published",
        ))?;
        let presenter = self.presenter.as_ref().ok_or(FrameError::InvalidState(
            "candidate has no surface presenter",
        ))?;
        if extent == self.render_extent && extent == presenter.extent() {
            return Ok(());
        }
        let _region = self
            .creations
            .region(CreationPhase::Build, "device-scene-resize");
        let presenter = presenter
            .refit(extent, extent)
            .map_err(|error| FrameError::gpu("replacement scene resize", error))?;
        let mut slots = preparation::prepare_slots(
            &scene.prepared,
            &self.queue,
            extent,
            Some(presenter.profile()),
        )?;
        preparation::bind_presenter(&scene.prepared, &mut slots, Some(&presenter))?;
        scene.slots = slots;
        self.presenter = Some(presenter);
        self.render_extent = extent;
        Ok(())
    }
}

impl DevicePrepareInputs {
    /// Prepare on a different device with a strictly newer platform generation.
    /// `queue` must be the queue returned with `capabilities`' device. As with
    /// initial preparation, this caller-owned pairing is not privately probed.
    /// `surface` describes a valid nonzero prepared extent; the platform may
    /// defer configuring an actual zero-sized surface until it becomes usable.
    /// # Errors
    /// Returns stale device/generation, invalid scene/profile or GPU preparation
    /// failure. The source host retains its world and CPU inputs for retry.
    pub async fn prepare(
        self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        surface: Option<SurfaceProfile>,
    ) -> Result<PreparedDevice, FrameError> {
        if capabilities.generation() <= self.base_generation
            || capabilities.matches(&self.base_device)
        {
            return Err(FrameError::InvalidState(
                "device replacement requires a different, newer device",
            ));
        }
        let creations = capabilities.device().ledger();
        let build = creations.region(CreationPhase::Build, "device");
        let (scene, presenter, render_extent) = match self.scene {
            RetainedScene::ThreeD(inputs) => {
                let extent = self.render_extent;
                let selection = preparation::contribution_admission(
                    inputs.scene.ray_admission(capabilities),
                    inputs.supports_candidates,
                );
                let (scene, presenter) =
                    preparation::prepare_3d_transition(selection, async |selection| {
                        preparation::validate_attempt(capabilities, selection, async || {
                            let mut scene = prepare_scene(
                                capabilities,
                                queue,
                                &inputs.scene,
                                inputs.lighting,
                                inputs.exposure,
                                extent,
                                surface.is_some(),
                                inputs.contribution_passes,
                                crate::contribution::EffectGraph::default(),
                                None,
                                selection,
                            )
                            .await?;
                            let presenter =
                                prepare_presenter(capabilities, extent, surface).await?;
                            scene.bind_presenter(presenter.as_ref())?;
                            Ok((scene, presenter))
                        })
                        .await
                    })
                    .await?;
                (scene, presenter, extent)
            }
            RetainedScene::TwoD(sprites) => {
                let extent = surface.map_or(self.render_extent, |profile| profile.extent);
                let mut scene =
                    prepare_sprites(capabilities, queue, sprites, extent, surface).await?;
                let presenter = prepare_presenter(capabilities, extent, surface).await?;
                scene.bind_presenter(presenter.as_ref())?;
                (scene, presenter, extent)
            }
        };
        drop(build);
        Ok(PreparedDevice {
            base_generation: self.base_generation,
            owner: self.owner,
            capabilities: capabilities.clone(),
            queue: queue.clone(),
            scene: Some(scene),
            presenter,
            creations,
            render_extent,
        })
    }
}

async fn prepare_presenter(
    capabilities: &DeviceCapabilities,
    extent: [u32; 2],
    profile: Option<SurfaceProfile>,
) -> Result<Option<SurfacePresenter>, FrameError> {
    if let Some(profile) = profile {
        let presenter = SurfacePresenter::prepare(capabilities, extent, profile)
            .await
            .map_err(|error| FrameError::gpu("replacement presenter", error))?;
        Ok(Some(presenter))
    } else {
        Ok(None)
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Snapshot retained scene/cooked selection for detached GPU reconstruction.
    /// This copies the finite CPU scene once per explicit recovery attempt.
    /// # Errors
    /// Returns a current camera/lighting policy refusal without changing the host.
    pub fn device_prepare_inputs(&self) -> Result<DevicePrepareInputs, FrameError> {
        let scene = match &self.prepared {
            PreparedRenderer::ThreeD(prepared) => {
                let frame = self
                    .source
                    .frame(
                        self.world.source(),
                        self.presentation_alpha(),
                        self.render_extent,
                        self.visual_lighting,
                    )
                    .map_err(FrameError::Scene)?;
                let mut scene = self.scene.clone();
                scene.view = frame.view;
                RetainedScene::ThreeD(Box::new(RetainedThreeD {
                    scene,
                    lighting: frame.lighting,
                    exposure: prepared.fallback_exposure(),
                    contribution_passes: prepared.contribution_passes(),
                    supports_candidates: {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            preparation::supports_candidates(self.contribution.as_deref())
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            false
                        }
                    },
                }))
            }
            PreparedRenderer::TwoD(prepared) => RetainedScene::TwoD(prepared.scene.clone()),
        };
        Ok(DevicePrepareInputs {
            base_device: self.device.clone(),
            base_generation: self.generation,
            owner: Arc::clone(&self.visual_owner),
            scene,
            render_extent: self.render_extent,
        })
    }

    /// Install a complete replacement without polling/waiting on the old device.
    /// The platform supplies matching queue/capabilities during preparation and
    /// checks its candidate-loss flag and current surface lease before this call.
    /// Old slots/reports are abandoned; late callbacks retain only old mailboxes.
    /// Authority, controls, source policy and cooked selection remain owned here.
    /// # Errors
    /// Rejects stale/foreign/consumed candidates, current policy/packing refusal,
    /// or a failed Maps scene rebuild before replacing active resources.
    /// CPU/world inputs remain available.
    pub fn publish_device(&mut self, candidate: &mut PreparedDevice) -> Result<(), FrameError> {
        if !Arc::ptr_eq(&candidate.owner, &self.visual_owner)
            || candidate.base_generation != self.generation
            || candidate.capabilities.generation() <= self.generation
            || candidate.capabilities.matches(&self.device)
        {
            return Err(FrameError::InvalidState(
                "stale or foreign device replacement",
            ));
        }
        let creations = candidate.creations.clone();
        let build = creations.region(CreationPhase::Build, "publish-device");
        let scene = candidate.scene.as_mut().ok_or(FrameError::InvalidState(
            "device replacement already published",
        ))?;
        let (mut scene, view, contribution_prepared) = if let Some(previous) =
            self.prepared.three_d()
        {
            if !scene.prepared.has_current_mesh_rows(previous) {
                return Err(FrameError::InvalidState(
                    "stale mesh rows in device replacement",
                ));
            }
            let frame = self.replacement_frame(candidate.render_extent, self.visual_lighting)?;
            #[cfg(not(target_arch = "wasm32"))]
            let staged = {
                let selection = scene.prepared.ray_structure_selection();
                if preparation::is_candidate_transition(selection) {
                    let exposure = scene.prepared.fallback_exposure();
                    self.stage_attached_3d_transition(
                        selection,
                        candidate.scene.take(),
                        preparation::TransitionInputs {
                            capabilities: &candidate.capabilities,
                            queue: &candidate.queue,
                            extent: candidate.render_extent,
                            presenter: candidate.presenter.as_ref(),
                            frame: &frame,
                            exposure,
                        },
                        || true,
                    )?
                } else {
                    None
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            if let Some((scene, view)) = staged {
                (scene, Some(view), true)
            } else {
                let scene = candidate.scene.as_mut().ok_or(FrameError::InvalidState(
                    "device replacement already published",
                ))?;
                let view = self.refresh_replacement(scene, &candidate.queue, &frame)?.0;
                (
                    candidate.scene.take().ok_or(FrameError::InvalidState(
                        "device replacement already published",
                    ))?,
                    Some(view),
                    false,
                )
            }
            #[cfg(target_arch = "wasm32")]
            {
                let view = self.refresh_replacement(scene, &candidate.queue, &frame)?.0;
                (
                    candidate.scene.take().ok_or(FrameError::InvalidState(
                        "device replacement already published",
                    ))?,
                    Some(view),
                    false,
                )
            }
        } else {
            self.source
                .sprites(
                    self.world.source(),
                    self.presentation_alpha(),
                    candidate.render_extent,
                )
                .map_err(FrameError::Scene)?
                .view
                .clip_scale(candidate.render_extent)
                .map_err(FrameError::Render)?;
            (
                candidate.scene.take().ok_or(FrameError::InvalidState(
                    "device replacement already published",
                ))?,
                None,
                false,
            )
        };
        let discarded = self.pending_frames().saturating_add(self.completed.len());
        self.dropped_reports = self
            .dropped_reports
            .saturating_add(u64::try_from(discarded).unwrap_or(u64::MAX));
        self.completed.clear();
        self.device = wgpu::Device::clone(candidate.capabilities.device());
        self.queue = candidate.queue.clone();
        self.generation = candidate.capabilities.generation();
        self.submitted_clock = None;
        self.native_failed_generation = None;
        self.render_extent = candidate.render_extent;
        if let (PreparedRenderer::TwoD(current), PreparedRenderer::TwoD(next)) =
            (&mut self.prepared, &mut scene.prepared)
        {
            next.inherit_world(current);
        }
        self.prepared = scene.prepared;
        self.slots = scene.slots;
        self.retired_slots.clear();
        self.retired_capture.clear();
        self.presenter = candidate.presenter.take();
        self.presenter_generation += 1;
        if let Some(view) = view {
            self.scene.view = view;
        }
        self.reset_replacement_history();
        self.visual_owner = Arc::new(());
        if !contribution_prepared {
            self.rebuild_contribution();
        }
        drop(build);
        creations.ready();
        let retired = std::mem::replace(&mut self.creations, creations);
        #[cfg(feature = "creation-counters")]
        {
            self.retired_creations = Some(retired.retire());
        }
        drop(retired);
        Ok(())
    }
}
