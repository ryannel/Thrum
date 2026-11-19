//! One preparation body for initial and replacement scene resources.

use super::{FrameError, FrameSlot, IN_FLIGHT, SlotState};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::ContributionPasses,
    present::{SurfacePresenter, SurfaceProfile},
    sprite::{PreparedSprites, SpriteTarget},
    synthetic::{
        CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, RetainedRayMeshes,
        SceneTimingLayout, SyntheticScene, SyntheticTargetInputs,
    },
    timing::GpuTimings,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct PreparedScene {
    pub(super) prepared: PreparedRenderer,
    pub(super) slots: Vec<FrameSlot>,
}

/// Rebuild the contribution against the supplied scene and its exact layouts.
/// A new slot set is needed only when the graph or secondary views changed.
pub(super) fn rebuild_3d_contribution(
    prepared: &mut PreparedRenderer,
    contribution: &mut dyn crate::contribution::SceneContribution,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    extent: [u32; 2],
    presenter: Option<&SurfacePresenter>,
) -> Result<Option<Vec<FrameSlot>>, String> {
    let had_views = prepared.secondary_view_count() != 0;
    let previous_views = prepared.take_secondary();
    let result = (|| {
        let declarations = contribution.secondary_view_slots();
        prepared
            .prepare_secondary_views(queue, &declarations, extent, &scene.local_lights)
            .map_err(|error| error.to_string())?;
        contribution.rebuild(
            &prepared.scene_surface_with_views(queue, contribution.secondary_views_enabled()),
        )?;
        let graph = contribution.effect_graph();
        graph.validate()?;
        let mut passes = contribution.passes();
        passes.upload |= prepared.secondary_view_count() != 0;
        if graph == *prepared.effect_graph()
            && passes == prepared.contribution_passes()
            && !had_views
            && prepared.secondary_view_count() == 0
        {
            return Ok(None);
        }
        let previous_graph = prepared.effect_graph().clone();
        let previous_passes = prepared.contribution_passes();
        prepared.set_effect_graph(graph);
        prepared.set_contribution_passes(passes);
        let slots = prepare_slots(
            prepared,
            queue,
            extent,
            presenter.map(SurfacePresenter::profile),
        )
        .and_then(|mut slots| {
            bind_presenter(prepared, &mut slots, presenter)?;
            Ok(slots)
        });
        if slots.is_err() {
            prepared.set_effect_graph(previous_graph);
            prepared.set_contribution_passes(previous_passes);
        }
        slots
            .map(Some)
            .map_err(|error| format!("rebuilt contribution graph: {error}"))
    })();
    if result.is_err() {
        prepared.restore_secondary(previous_views);
    }
    result
}

pub(super) enum PreparedRenderer {
    ThreeD(Box<PreparedSynthetic>),
    TwoD(Box<PreparedSprites>),
}

pub(super) enum TargetInputs {
    ThreeD(Box<SyntheticTargetInputs>),
    TwoD(Box<PreparedSprites>),
}

pub(super) struct TargetPreparation {
    inputs: TargetInputs,
    capabilities: DeviceCapabilities,
    timing: Option<SceneTimingLayout>,
    passes: u32,
}

impl TargetPreparation {
    pub(super) fn new(prepared: &PreparedRenderer) -> Self {
        let (inputs, timing) = match prepared {
            PreparedRenderer::ThreeD(scene) => (
                TargetInputs::ThreeD(Box::new(scene.target_inputs())),
                Some(SceneTimingLayout::from(&**scene)),
            ),
            PreparedRenderer::TwoD(scene) => {
                (TargetInputs::TwoD(Box::new(scene.target_inputs())), None)
            }
        };
        Self {
            inputs,
            timing,
            capabilities: prepared.capabilities().clone(),
            passes: prepared.scene_pass_count(),
        }
    }

    pub(super) fn bind_presenter(
        &self,
        slots: &mut [FrameSlot],
        presenter: &SurfacePresenter,
    ) -> Result<(), FrameError> {
        let device = self.capabilities.device();
        for slot in slots {
            slot.presentation =
                Some(
                    match (&self.inputs, &mut slot.target) {
                        (TargetInputs::TwoD(_), FrameTarget::TwoD(target)) => presenter
                            .bind_output(device, self.capabilities.generation(), target.output()),
                        (TargetInputs::ThreeD(inputs), FrameTarget::ThreeD(target)) => {
                            target
                                .bind_surface_graph_with_inputs(inputs, presenter.profile())
                                .map_err(|error| {
                                    FrameError::Gpu(format!("merged surface graph: {error:?}"))
                                })?;
                            presenter.bind_target(device, target)
                        }
                        _ => unreachable!("targets come from the same preparation"),
                    }
                    .map_err(|error| {
                        FrameError::Gpu(format!("replacement surface binding: {error:?}"))
                    })?,
                );
        }
        Ok(())
    }
}

impl PreparedRenderer {
    pub(super) fn three_d(&self) -> Option<&PreparedSynthetic> {
        match self {
            Self::ThreeD(scene) => Some(scene),
            Self::TwoD(_) => None,
        }
    }

    pub(super) fn capabilities(&self) -> &DeviceCapabilities {
        match self {
            Self::ThreeD(scene) => scene.capabilities(),
            Self::TwoD(scene) => &scene.capabilities,
        }
    }

    pub(super) fn scene_pass_count(&self) -> u32 {
        match self {
            Self::ThreeD(scene) => scene.scene_pass_count(),
            Self::TwoD(scene) => {
                2 + u32::from(scene.contribution_passes.compute)
                    + u32::try_from(scene.effects.passes.len()).unwrap_or(u32::MAX - 3)
            }
        }
    }
}

pub(super) enum FrameTarget {
    ThreeD(Box<OffscreenTarget>),
    TwoD(Box<SpriteTarget>),
}

impl FrameTarget {
    pub(super) fn take_culling_counts_if_3d(&mut self) -> Option<crate::synthetic::CullingCounts> {
        match self {
            Self::ThreeD(target) => target.take_culling_counts(),
            Self::TwoD(_) => None,
        }
    }

    pub(super) fn three_d(&self) -> Option<&OffscreenTarget> {
        match self {
            Self::ThreeD(target) => Some(target),
            Self::TwoD(_) => None,
        }
    }

    pub(super) fn capture_source(&self) -> CaptureSource {
        match self {
            Self::ThreeD(target) => target.capture_source(),
            Self::TwoD(target) => target.source,
        }
    }

    pub(super) fn readback(&self, device: &wgpu::Device) -> crate::synthetic::PixelReadback {
        match self {
            Self::ThreeD(target) => target.readback(device),
            Self::TwoD(target) => target.readback(device),
        }
    }

    pub(super) fn storage_report(&self) -> String {
        match self {
            Self::ThreeD(target) => target.storage_report(),
            Self::TwoD(target) => target.storage_report(),
        }
    }
}

// The private wrappers preserve the 3D-only call sites. The shared paths
// dispatch on the variant before accessing any 3D attachments or state.
macro_rules! three_d_access {
    ($wrapper:ty, $inner:ty) => {
        impl std::ops::Deref for $wrapper {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                match self {
                    Self::ThreeD(inner) => inner,
                    Self::TwoD(_) => unreachable!("3D operation on a 2D scene"),
                }
            }
        }
        impl std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                match self {
                    Self::ThreeD(inner) => inner,
                    Self::TwoD(_) => unreachable!("3D operation on a 2D scene"),
                }
            }
        }
    };
}
three_d_access!(PreparedRenderer, PreparedSynthetic);
three_d_access!(FrameTarget, OffscreenTarget);

impl PreparedScene {
    pub(super) fn bind_presenter(
        &mut self,
        presenter: Option<&SurfacePresenter>,
    ) -> Result<(), FrameError> {
        bind_presenter(&self.prepared, &mut self.slots, presenter)
    }
}

pub(super) fn bind_presenter(
    prepared: &PreparedRenderer,
    slots: &mut [FrameSlot],
    presenter: Option<&SurfacePresenter>,
) -> Result<(), FrameError> {
    let device = prepared.capabilities().device();
    for slot in slots {
        if let FrameTarget::TwoD(target) = &slot.target {
            slot.presentation = presenter
                .map(|presenter| {
                    presenter.bind_output(
                        device,
                        prepared.capabilities().generation(),
                        target.output(),
                    )
                })
                .transpose()
                .map_err(|error| FrameError::Gpu(format!("sprite surface binding: {error:?}")))?;
            continue;
        }
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
    retained_ray_meshes: Option<RetainedRayMeshes>,
) -> Result<PreparedScene, FrameError> {
    let mut prepared = PreparedSynthetic::prepare_with_retained_ray_meshes(
        capabilities,
        queue,
        scene,
        lighting,
        exposure,
        retained_ray_meshes,
    )
    .await
    .map_err(|error| FrameError::Gpu(format!("scene prepare: {error:?}")))?;
    prepared.set_contribution_passes(contribution_passes);
    prepared.set_effect_graph(effect_graph);
    let prepared = PreparedRenderer::ThreeD(Box::new(prepared));
    let slots = prepare_capture_slots(&prepared, queue, extent, surface, None, CaptureSource::Sdr)?;
    Ok(PreparedScene { prepared, slots })
}

pub(super) async fn prepare_sprites(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: crate::sprite::SpriteScene,
    extent: [u32; 2],
    profile: Option<SurfaceProfile>,
) -> Result<PreparedScene, FrameError> {
    let scene = PreparedSprites::prepare(capabilities, queue, scene)
        .await
        .map_err(|error| FrameError::Gpu(format!("sprite scene prepare: {error:?}")))?;
    let prepared = PreparedRenderer::TwoD(Box::new(scene));
    let slots = prepare_slots(&prepared, queue, extent, profile)?;
    Ok(PreparedScene { prepared, slots })
}

pub(super) fn prepare_slots(
    prepared: &PreparedRenderer,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    profile: Option<SurfaceProfile>,
) -> Result<Vec<FrameSlot>, FrameError> {
    prepare_capture_slots(
        prepared,
        queue,
        extent,
        profile.is_some(),
        profile,
        CaptureSource::Sdr,
    )
}

pub(super) fn prepare_capture_slots(
    prepared: &PreparedRenderer,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    surface: bool,
    profile: Option<SurfaceProfile>,
    source: CaptureSource,
) -> Result<Vec<FrameSlot>, FrameError> {
    TargetPreparation::new(prepared).prepare_slots(queue, extent, surface, profile, source, None)
}

impl TargetPreparation {
    pub(super) fn prepare_slots(
        &self,
        queue: &wgpu::Queue,
        extent: [u32; 2],
        surface: bool,
        profile: Option<SurfaceProfile>,
        source: CaptureSource,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<FrameSlot>, FrameError> {
        let capabilities = &self.capabilities;
        let mut slots: Vec<FrameSlot> = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
                return Err(FrameError::InvalidState("target preparation cancelled"));
            }
            let (target, timing_layout) = match &self.inputs {
                TargetInputs::ThreeD(prepared) => {
                    let mut target = OffscreenTarget::with_target_inputs(
                        capabilities.device(),
                        prepared,
                        extent[0],
                        extent[1],
                        source,
                    )
                    .map_err(|error| FrameError::Gpu(format!("scene target: {error:?}")))?;
                    if let Some(first) = slots.first() {
                        target.share_reconstruction_history(prepared, &first.target);
                    }
                    let timing_layout = self.timing.map(|layout| layout.with_stars(&target));
                    (FrameTarget::ThreeD(Box::new(target)), timing_layout)
                }
                TargetInputs::TwoD(prepared) => {
                    let target = SpriteTarget::prepare(prepared, extent, source, profile)
                        .map_err(|error| FrameError::Gpu(format!("sprite target: {error:?}")))?;
                    (FrameTarget::TwoD(Box::new(target)), None)
                }
            };
            slots.push(FrameSlot {
                target,
                presentation: None,
                timings: GpuTimings::new(capabilities, queue, self.passes + u32::from(surface))
                    .map_err(|error| FrameError::Gpu(format!("scene timings: {error:?}")))?,
                timing_layout,
                timing_readback: false,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        Ok(slots)
    }
}
