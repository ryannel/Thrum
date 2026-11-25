//! One preparation body for initial and replacement scene resources.

use super::{DeferredFrameTiming, FrameError, FrameSlot, FrameTimingBatch, IN_FLIGHT, SlotState};
#[cfg(not(target_arch = "wasm32"))]
use crate::contribution::ContributionCandidate;
use crate::contribution::SceneContribution;
use crate::{
    capabilities::{DeviceCapabilities, RayStructureSelection},
    contribution::ContributionPasses,
    present::{SurfacePresenter, SurfaceProfile},
    sprite::{PreparedSprites, SpriteTarget},
    synthetic::{
        CaptureSource, FrameLighting, OffscreenTarget, PreparedSynthetic, RetainedRayMeshes,
        SceneTimingLayout, SyntheticScene, SyntheticTargetInputs,
    },
    timing::{GpuTimings, TimingRecords},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct PreparedScene<R = PreparedRenderer, S = Vec<FrameSlot>> {
    pub(super) prepared: R,
    pub(super) slots: S,
}

/// Rebuild the contribution against the supplied scene and its exact layouts.
/// A new slot set is needed only when the graph or secondary views changed.
pub(super) fn rebuild_3d_contribution(
    prepared: &mut PreparedRenderer,
    contribution: &mut dyn SceneContribution,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    extent: [u32; 2],
    presenter: Option<&SurfacePresenter>,
    attaching: bool,
) -> Result<Option<Vec<FrameSlot>>, FrameError> {
    use crate::contribution::SecondaryViewError;

    let had_views = prepared.secondary_view_count() != 0;
    let previous_views = prepared.take_secondary();
    let previous_graph = prepared.effect_graph().clone();
    let previous_passes = prepared.contribution_passes();
    let result = (|| {
        let declarations = contribution.secondary_view_slots();
        prepared
            .prepare_secondary_views(queue, &declarations, extent, &scene.local_lights)
            .map_err(|error| match error {
                SecondaryViewError::RecordLimit
                | SecondaryViewError::LightingLimit
                | SecondaryViewError::ShaderLimit => {
                    FrameError::gpu("secondary view preparation", error)
                }
                _ => FrameError::SecondaryView(error),
            })?;
        let surface =
            prepared.scene_surface_with_views(queue, contribution.secondary_views_enabled());
        if attaching {
            contribution.rebuild_for_attachment(&surface)?;
        } else {
            contribution.rebuild(&surface)?;
        }
        let graph = contribution.effect_graph();
        graph.validate().map_err(FrameError::EffectGraph)?;
        let mut passes = contribution.passes();
        passes.upload |= prepared.secondary_view_count() != 0;
        if !attaching
            && graph == previous_graph
            && passes == previous_passes
            && !had_views
            && prepared.secondary_view_count() == 0
        {
            return Ok(None);
        }
        prepared.set_effect_graph(graph);
        prepared.set_contribution_passes(passes);
        let mut slots = prepare_slots(
            prepared,
            queue,
            extent,
            presenter.map(SurfacePresenter::profile),
        )?;
        bind_presenter(prepared, &mut slots, presenter)?;
        Ok(Some(slots))
    })();
    if result.is_err() {
        prepared.set_effect_graph(previous_graph);
        prepared.set_contribution_passes(previous_passes);
        prepared.restore_secondary(previous_views);
    }
    result
}

pub(super) enum PreparedRenderer {
    ThreeD(Box<PreparedSynthetic>),
    TwoD(Box<PreparedSprites>),
}

#[cfg(not(target_arch = "wasm32"))]
impl PreparedRenderer {
    pub(super) fn traces(&self) -> bool {
        self.three_d()
            .is_some_and(|scene| scene.ray_structure_selection() == RayStructureSelection::Enabled)
    }
}

pub(super) enum TargetInputs {
    ThreeD(Box<SyntheticTargetInputs>),
    TwoD(Box<PreparedSprites>),
}

pub(super) struct TargetPreparation {
    inputs: TargetInputs,
    capabilities: DeviceCapabilities,
    timing: Option<SceneTimingLayout>,
    // Timed passes of a 2D slot. A 3D slot counts its target's compiled graphs.
    sprite_passes: u32,
}

impl TargetPreparation {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn resize_secondary(
        &mut self,
        extents: &[[u32; 2]],
        job: crate::contribution::SecondaryResizeJob,
    ) -> Result<
        (
            crate::synthetic::secondary::SecondarySet,
            crate::contribution::SecondaryResize,
        ),
        FrameError,
    > {
        let TargetInputs::ThreeD(inputs) = &mut self.inputs else {
            unreachable!("secondary views belong to 3D scenes");
        };
        let views = inputs.resize_secondary(extents);
        let replacement = job(views.handles())?;
        replacement
            .graph
            .validate()
            .map_err(FrameError::EffectGraph)?;
        inputs.effect_graph.clone_from(&replacement.graph);
        Ok((views, replacement))
    }

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
            sprite_passes: prepared.sprite_pass_count(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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
                                .map_err(|error| FrameError::gpu("merged surface graph", error))?;
                            presenter.bind_target(device, target)
                        }
                        _ => unreachable!("targets come from the same preparation"),
                    }
                    .map_err(|error| FrameError::gpu("replacement surface binding", error))?,
                );
        }
        Ok(())
    }
}

impl PreparedRenderer {
    pub(super) fn three_d_mut(&mut self) -> Option<&mut PreparedSynthetic> {
        match self {
            Self::ThreeD(scene) => Some(scene),
            Self::TwoD(_) => None,
        }
    }

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

    // Timed passes of a 2D scene's slot; zero for a 3D scene, whose slots
    // count their targets' compiled graphs.
    fn sprite_pass_count(&self) -> u32 {
        match self {
            Self::ThreeD(_) => 0,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn stage_contribution(
        &mut self,
        contribution: &dyn SceneContribution,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
        extent: [u32; 2],
        presenter: Option<&SurfacePresenter>,
    ) -> Result<ContributionCandidate, FrameError> {
        use crate::contribution::SecondaryViewError;

        self.prepared
            .prepare_secondary_views(
                queue,
                &contribution.secondary_view_slots(),
                extent,
                &scene.local_lights,
            )
            .map_err(|error| match error {
                SecondaryViewError::RecordLimit
                | SecondaryViewError::LightingLimit
                | SecondaryViewError::ShaderLimit => {
                    FrameError::gpu("secondary view preparation", error)
                }
                _ => FrameError::SecondaryView(error),
            })?;
        let surface = self
            .prepared
            .scene_surface_with_views(queue, contribution.secondary_views_enabled());
        let mut candidate = contribution.prepare_candidate(&surface)?;
        candidate
            .graph
            .validate()
            .map_err(FrameError::EffectGraph)?;
        self.prepared.set_effect_graph(candidate.graph.clone());
        let mut passes = candidate.passes;
        passes.upload |= self.prepared.secondary_view_count() != 0;
        self.prepared.set_contribution_passes(passes);
        self.slots = prepare_slots(
            &self.prepared,
            queue,
            extent,
            presenter.map(SurfacePresenter::profile),
        )?;
        self.bind_presenter(presenter)?;
        if !candidate.initialization.is_empty() {
            queue.submit(std::mem::take(&mut candidate.initialization));
        }
        Ok(candidate)
    }

    pub(super) fn bind_presenter(
        &mut self,
        presenter: Option<&SurfacePresenter>,
    ) -> Result<(), FrameError> {
        bind_presenter(&self.prepared, &mut self.slots, presenter)
    }
}

pub(super) fn contribution_admission(
    selection: RayStructureSelection,
    supported: bool,
) -> RayStructureSelection {
    if selection == RayStructureSelection::Enabled && !supported {
        RayStructureSelection::UnsupportedContribution
    } else {
        selection
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn supports_candidates(contribution: Option<&dyn SceneContribution>) -> bool {
    contribution.is_none_or(SceneContribution::supports_candidate_preparation)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn is_candidate_transition(selection: RayStructureSelection) -> bool {
    matches!(
        selection,
        RayStructureSelection::Enabled | RayStructureSelection::PreparationFailed
    )
}

// The same retry decision encloses the initial scene and all render-owner
// transitions. Each attempt returns owned resources for installation.
pub(super) async fn prepare_3d_transition<T>(
    selection: RayStructureSelection,
    mut attempt: impl AsyncFnMut(RayStructureSelection) -> Result<T, FrameError>,
) -> Result<T, FrameError> {
    match attempt(selection).await {
        Err(_) if selection == RayStructureSelection::Enabled => {
            attempt(RayStructureSelection::PreparationFailed).await
        }
        result => result,
    }
}

pub(super) async fn validate_attempt<T>(
    capabilities: &DeviceCapabilities,
    selection: RayStructureSelection,
    attempt: impl AsyncFnOnce() -> Result<T, FrameError>,
) -> Result<T, FrameError> {
    #[cfg(not(target_arch = "wasm32"))]
    if is_candidate_transition(selection) {
        let device = capabilities.device();
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = attempt().await;
        let errors = [validation, memory, internal].map(wgpu::ErrorScopeGuard::pop);
        let [validation, memory, internal] = errors;
        if let Some(error) = validation.await.or(memory.await).or(internal.await) {
            return Err(FrameError::gpu("3D scene preparation", error));
        }
        return result;
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (capabilities, selection);
    attempt().await
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct SceneTransition<
    R = PreparedRenderer,
    S = Vec<FrameSlot>,
    V = crate::synthetic::SyntheticView,
> {
    pub(super) scene: PreparedScene<R, S>,
    view: V,
    pub(super) contribution: Option<ContributionCandidate>,
    inherit_rows: bool,
}

#[cfg(not(target_arch = "wasm32"))]
struct ScenePublicationTarget<'a, R, S, V> {
    prepared: &'a mut R,
    slots: &'a mut S,
    view: &'a mut V,
    contribution: Option<&'a mut dyn SceneContribution>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<R, S, V> SceneTransition<R, S, V> {
    fn commit(
        mut self,
        target: ScenePublicationTarget<'_, R, S, V>,
        current: impl FnOnce() -> bool,
        prepare_install: impl FnOnce(&mut R, &mut R, bool),
    ) -> bool {
        if !current() {
            return false;
        }
        if let Some(member) = self.contribution {
            member.commit(target.contribution.expect("prepared contribution owner"));
        }
        prepare_install(target.prepared, &mut self.scene.prepared, self.inherit_rows);
        *target.prepared = self.scene.prepared;
        *target.slots = self.scene.slots;
        *target.view = self.view;
        true
    }
}

pub(super) enum ScenePublication {
    Legacy(PreparedScene, Option<crate::synthetic::SyntheticView>),
    #[cfg(not(target_arch = "wasm32"))]
    Candidate(SceneTransition),
}

impl<W: super::FrameWorld, V: super::SceneSource<W::Page>> super::FrameHost<W, V> {
    /// Install a validated replacement. Returns whether its contribution was already prepared.
    pub(super) fn install_scene_publication(&mut self, publication: ScenePublication) -> bool {
        match publication {
            ScenePublication::Legacy(scene, view) => {
                self.install_prepared_for_display(scene.prepared);
                self.slots = scene.slots;
                if let Some(view) = view {
                    self.scene.view = view;
                }
                false
            }
            #[cfg(not(target_arch = "wasm32"))]
            ScenePublication::Candidate(staged) => self.commit_3d_transition(staged, || true),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
pub(super) struct TransitionInputs<'a> {
    pub(super) capabilities: &'a DeviceCapabilities,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) extent: [u32; 2],
    pub(super) presenter: Option<&'a SurfacePresenter>,
    pub(super) frame: &'a super::SceneFrame,
    pub(super) exposure: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl<W: super::FrameWorld, V: super::SceneSource<W::Page>> super::FrameHost<W, V> {
    pub(super) fn stage_attached_3d_transition(
        &mut self,
        selection: RayStructureSelection,
        detached: Option<PreparedScene>,
        inputs: TransitionInputs<'_>,
    ) -> Result<SceneTransition, FrameError> {
        let contribution = self.contribution.take();
        let result = self.stage_3d_transition(selection, detached, inputs, contribution.as_deref());
        self.contribution = contribution;
        result
    }

    pub(super) fn commit_3d_transition(
        &mut self,
        staged: SceneTransition,
        current: impl FnOnce() -> bool,
    ) -> bool {
        staged.commit(
            ScenePublicationTarget {
                prepared: &mut self.prepared,
                slots: &mut self.slots,
                view: &mut self.scene.view,
                contribution: self.contribution.as_deref_mut(),
            },
            current,
            |previous, prepared, inherit_rows| {
                if inherit_rows {
                    prepared.commit_mesh_rows(previous);
                }
                prepared.initialize_display(&self.queue, self.display_headroom);
            },
        )
    }

    pub(super) fn stage_3d_transition(
        &mut self,
        selection: RayStructureSelection,
        mut detached: Option<PreparedScene>,
        inputs: TransitionInputs<'_>,
        contribution: Option<&dyn SceneContribution>,
    ) -> Result<SceneTransition, FrameError> {
        pollster::block_on(prepare_3d_transition(selection, async |selection| {
            let mut scene = match detached.take() {
                Some(scene) if scene.prepared.ray_structure_selection() == selection => scene,
                _ => {
                    validate_attempt(inputs.capabilities, selection, async || {
                        let mut scene = self.scene.clone();
                        scene.view = inputs.frame.view;
                        prepare_scene(
                            inputs.capabilities,
                            inputs.queue,
                            &scene,
                            inputs.frame.lighting,
                            inputs.exposure,
                            inputs.extent,
                            inputs.presenter.is_some(),
                            ContributionPasses::default(),
                            crate::contribution::EffectGraph::default(),
                            if selection == RayStructureSelection::Enabled
                                && **inputs.capabilities.device()
                                    == **self.prepared.capabilities().device()
                            {
                                self.prepared.retained_ray_meshes()
                            } else {
                                None
                            },
                            selection,
                        )
                        .await
                    })
                    .await?
                }
            };
            let (view, contribution, inherit_rows) =
                validate_attempt(inputs.capabilities, selection, async || {
                    let member = if let Some(contribution) =
                        contribution.filter(|_| is_candidate_transition(selection))
                    {
                        Some(scene.stage_contribution(
                            contribution,
                            inputs.queue,
                            &self.scene,
                            inputs.extent,
                            inputs.presenter,
                        )?)
                    } else {
                        scene.bind_presenter(inputs.presenter)?;
                        None
                    };
                    let (view, inherit_rows) =
                        self.refresh_replacement(&mut scene, inputs.queue, inputs.frame)?;
                    Ok((view, member, inherit_rows))
                })
                .await?;
            Ok(SceneTransition {
                scene,
                view,
                contribution,
                inherit_rows,
            })
        }))
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
                .map_err(|error| FrameError::gpu("sprite surface binding", error))?;
            continue;
        }
        if let Some(presenter) = presenter {
            slot.target
                .bind_surface_graph(prepared, presenter.profile())
                .map_err(|error| FrameError::gpu("merged surface graph", error))?;
        }
        slot.presentation = presenter
            .map(|presenter| presenter.bind_target(device, &slot.target))
            .transpose()
            .map_err(|error| FrameError::gpu("replacement surface binding", error))?;
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
    selection: RayStructureSelection,
) -> Result<PreparedScene, FrameError> {
    let mut prepared = PreparedSynthetic::prepare_host_candidate(
        capabilities,
        queue,
        scene,
        lighting,
        exposure,
        retained_ray_meshes,
        selection,
    )
    .await
    .map_err(|error| FrameError::gpu("scene prepare", error))?;
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
        .map_err(|error| FrameError::gpu("sprite scene prepare", error))?;
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
            let (target, timing_layout, passes) = match &self.inputs {
                TargetInputs::ThreeD(prepared) => {
                    let mut target = OffscreenTarget::with_target_inputs(
                        capabilities.device(),
                        prepared,
                        extent[0],
                        extent[1],
                        source,
                    )
                    .map_err(|error| FrameError::gpu("scene target", error))?;
                    if let Some(first) = slots.first() {
                        target.share_view_history(prepared, &first.target);
                    }
                    let timing_layout = self.timing.map(|layout| layout.with_stars(&target));
                    let passes = target.timed_pass_count();
                    (FrameTarget::ThreeD(Box::new(target)), timing_layout, passes)
                }
                TargetInputs::TwoD(prepared) => {
                    let target = SpriteTarget::prepare(prepared, extent, source, profile)
                        .map_err(|error| FrameError::gpu("sprite target", error))?;
                    (
                        FrameTarget::TwoD(Box::new(target)),
                        None,
                        self.sprite_passes,
                    )
                }
            };
            let timings = GpuTimings::new(capabilities, queue, passes + u32::from(surface))
                .map_err(|error| FrameError::gpu("scene timings", error))?;
            // Native 3D can release the render target when its GPU work
            // completes, while a second prepared batch maps its timestamps.
            let deferred_timing = if cfg!(not(target_arch = "wasm32"))
                && timings.is_enabled()
                && matches!(&target, FrameTarget::ThreeD(_))
            {
                let queries = GpuTimings::new(capabilities, queue, timings.capacity())
                    .map_err(|error| FrameError::gpu("deferred scene timings", error))?;
                Some(DeferredFrameTiming {
                    batch: FrameTimingBatch {
                        queries,
                        layout: None,
                        readback: false,
                        counter_floor: 0,
                    },
                    sample: None,
                })
            } else {
                None
            };
            let timing_records = if timings.is_enabled() {
                let graph_passes = match &target {
                    FrameTarget::ThreeD(target) => target.timing_metadata_capacity(),
                    FrameTarget::TwoD(target) => target.timing_metadata_capacity(),
                };
                // The host can queue IN_FLIGHT samples while callers hold earlier reports.
                TimingRecords::new(2 * IN_FLIGHT + 1, timings.capacity() as usize, graph_passes)
            } else {
                TimingRecords::new(0, 0, 0)
            };
            slots.push(FrameSlot {
                target,
                presentation: None,
                timing: FrameTimingBatch {
                    queries: timings,
                    layout: timing_layout,
                    readback: false,
                    counter_floor: 0,
                },
                deferred_timing,
                timing_records,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        Ok(slots)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod contribution_preparation;
