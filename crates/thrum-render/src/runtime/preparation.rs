//! One preparation body for initial and replacement scene resources.

use super::{FrameError, FrameSlot, IN_FLIGHT, SlotState};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::ContributionPasses,
    present::SurfacePresenter,
    synthetic::{
        CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, SceneTimingLayout,
        SyntheticScene,
    },
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
        bind_presenter(&self.prepared, &mut self.slots, presenter)
    }
}

pub(super) fn bind_presenter(
    prepared: &PreparedSynthetic,
    slots: &mut [FrameSlot],
    presenter: Option<&SurfacePresenter>,
) -> Result<(), FrameError> {
    let device = prepared.capabilities().device();
    for slot in slots {
        if let Some(presenter) = presenter {
            slot.target
                .bind_surface_graph(prepared, presenter.profile())
                .map_err(|error| FrameError::Gpu(format!("merged surface graph: {error:?}")))?;
        }
        slot.presentation = presenter
            .map(|presenter| presenter.bind_target(device, &slot.target))
            .transpose()
            .map_err(|error| FrameError::Gpu(format!("replacement surface binding: {error:?}")))?;
    }
    Ok(())
}

pub(super) async fn prepare_scene(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    lighting: FrameLighting,
    exposure: f32,
    extent: [u32; 2],
    surface: bool,
    contribution_passes: ContributionPasses,
    effect_graph: crate::contribution::EffectGraph,
) -> Result<PreparedScene, FrameError> {
    let mut prepared = PreparedSynthetic::prepare_with_capabilities(
        capabilities,
        queue,
        scene,
        lighting,
        exposure,
    )
    .await
    .map_err(|error| FrameError::Gpu(format!("scene prepare: {error:?}")))?;
    prepared.set_contribution_passes(contribution_passes);
    prepared.set_effect_graph(effect_graph);
    let slots = prepare_slots(&prepared, queue, extent, surface)?;
    Ok(PreparedScene { prepared, slots })
}

pub(super) fn prepare_slots(
    prepared: &PreparedSynthetic,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    surface: bool,
) -> Result<Vec<FrameSlot>, FrameError> {
    prepare_capture_slots(prepared, queue, extent, surface, CaptureSource::Sdr)
}

pub(super) fn prepare_capture_slots(
    prepared: &PreparedSynthetic,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    surface: bool,
    source: CaptureSource,
) -> Result<Vec<FrameSlot>, FrameError> {
    let capabilities = prepared.capabilities();
    let mut slots = Vec::with_capacity(IN_FLIGHT);
    for _ in 0..IN_FLIGHT {
        let target = OffscreenTarget::with_capture_source(
            capabilities.device(),
            prepared,
            extent[0],
            extent[1],
            source,
        )
        .map_err(|error| FrameError::Gpu(format!("scene target: {error:?}")))?;
        let timing_layout = SceneTimingLayout::from((prepared, &target));
        slots.push(FrameSlot {
            target,
            presentation: None,
            timings: GpuTimings::new(
                capabilities,
                queue,
                prepared.scene_pass_count() + u32::from(surface),
            )
            .map_err(|error| FrameError::Gpu(format!("scene timings: {error:?}")))?,
            timing_layout,
            timing_readback: false,
            render_done: Arc::new(AtomicBool::new(false)),
            state: SlotState::Free,
        });
    }
    Ok(slots)
}
