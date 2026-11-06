//! One preparation body for initial and replacement scene resources.

use super::{FrameError, FrameSlot, IN_FLIGHT, SlotState};
use crate::{
    capabilities::DeviceCapabilities,
    present::SurfacePresenter,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene},
    timing::GpuTimings,
};
use std::sync::{Arc, atomic::AtomicBool};

pub(super) struct PreparedScene {
    pub(super) prepared: PreparedSynthetic,
    pub(super) slots: Vec<FrameSlot>,
}

impl PreparedScene {
    pub(super) fn bind_presenter(
        &mut self,
        presenter: Option<&SurfacePresenter>,
    ) -> Result<(), FrameError> {
        let device = self.prepared.capabilities().device();
        for slot in &mut self.slots {
            slot.presentation = presenter
                .map(|presenter| presenter.bind_target(device, &slot.target))
                .transpose()
                .map_err(|error| {
                    FrameError::Gpu(format!("replacement surface binding: {error:?}"))
                })?;
        }
        Ok(())
    }
}

pub(super) async fn prepare_scene(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    lighting: FrameLighting,
    exposure: f32,
    extent: [u32; 2],
    surface: bool,
) -> Result<PreparedScene, FrameError> {
    let prepared = PreparedSynthetic::prepare_with_capabilities(
        capabilities,
        queue,
        scene,
        lighting,
        exposure,
    )
    .await
    .map_err(|error| FrameError::Gpu(format!("scene prepare: {error:?}")))?;
    let mut slots = Vec::with_capacity(IN_FLIGHT);
    for _ in 0..IN_FLIGHT {
        slots.push(FrameSlot {
            target: OffscreenTarget::new(capabilities.device(), &prepared, extent[0], extent[1])
                .map_err(|error| FrameError::Gpu(format!("scene target: {error:?}")))?,
            presentation: None,
            timings: GpuTimings::new(
                capabilities,
                queue,
                prepared.scene_pass_count() + u32::from(surface),
            )
            .map_err(|error| FrameError::Gpu(format!("scene timings: {error:?}")))?,
            render_done: Arc::new(AtomicBool::new(false)),
            state: SlotState::Free,
        });
    }
    Ok(PreparedScene { prepared, slots })
}
