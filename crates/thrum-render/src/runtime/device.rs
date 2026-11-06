//! Detached GPU-only reconstruction on a platform-selected replacement device.

use super::{
    FrameError, FrameHost, FrameWorld, SceneSource,
    preparation::{PreparedScene, prepare_scene},
};
use crate::{
    capabilities::DeviceCapabilities,
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
    scene: SyntheticScene,
    lighting: FrameLighting,
    exposure: f32,
    render_extent: [u32; 2],
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
        self.presenter
            .as_mut()
            .ok_or(FrameError::InvalidState(
                "candidate has no surface presenter",
            ))?
            .resize(self.capabilities.device(), extent)
            .map_err(|error| FrameError::Gpu(format!("replacement surface resize: {error:?}")))
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
        let mut scene = prepare_scene(
            capabilities,
            queue,
            &self.scene,
            self.lighting,
            self.exposure,
            self.render_extent,
            surface.is_some(),
        )
        .await?;
        let presenter = match surface {
            Some(profile) => Some(
                SurfacePresenter::prepare(capabilities, self.render_extent, profile)
                    .await
                    .map_err(|error| {
                        FrameError::Gpu(format!("replacement presenter: {error:?}"))
                    })?,
            ),
            None => None,
        };
        scene.bind_presenter(presenter.as_ref())?;
        Ok(PreparedDevice {
            base_generation: self.base_generation,
            owner: self.owner,
            capabilities: capabilities.clone(),
            queue: queue.clone(),
            scene: Some(scene),
            presenter,
        })
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Snapshot retained scene/cooked selection for detached GPU reconstruction.
    /// This copies the finite CPU scene once per explicit recovery attempt.
    /// # Errors
    /// Returns a current camera/lighting policy refusal without changing the host.
    pub fn device_prepare_inputs(&self) -> Result<DevicePrepareInputs, FrameError> {
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
        Ok(DevicePrepareInputs {
            base_device: self.device.clone(),
            base_generation: self.generation,
            owner: Arc::clone(&self.visual_owner),
            scene,
            lighting: frame.lighting,
            exposure: self.prepared.fallback_exposure(),
            render_extent: self.render_extent,
        })
    }

    /// Install a complete replacement without polling/waiting on the old device.
    /// The platform supplies matching queue/capabilities during preparation and
    /// checks its candidate-loss flag and current surface lease before this call.
    /// Old slots/reports are abandoned; late callbacks retain only old mailboxes.
    /// Authority, controls, source policy and cooked selection remain owned here.
    /// # Errors
    /// Rejects stale/foreign/consumed candidates or current policy/packing refusal
    /// before replacing active resources. CPU/world inputs remain available.
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
        let scene = candidate.scene.as_mut().ok_or(FrameError::InvalidState(
            "device replacement already published",
        ))?;
        let view = self.refresh_replacement(scene, &candidate.queue, self.visual_lighting)?;
        let scene = candidate.scene.take().ok_or(FrameError::InvalidState(
            "device replacement already published",
        ))?;
        let discarded = self.pending_frames().saturating_add(self.completed.len());
        self.dropped_reports = self
            .dropped_reports
            .saturating_add(u64::try_from(discarded).unwrap_or(u64::MAX));
        self.completed.clear();
        self.device = candidate.capabilities.device().clone();
        self.queue = candidate.queue.clone();
        self.generation = candidate.capabilities.generation();
        self.prepared = scene.prepared;
        self.slots = scene.slots;
        self.presenter = candidate.presenter.take();
        self.scene.view = view;
        self.reset_replacement_history();
        self.visual_owner = Arc::new(());
        Ok(())
    }
}
