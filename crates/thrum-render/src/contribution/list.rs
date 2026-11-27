//! An ordered scene attachment, retaining the existing single-contribution API.

use super::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SecondaryShadowCascade,
    ShadowBounds,
};
use crate::runtime::FrameError;

/// Contributions in game-selected order. Empty lists add no graph passes.
#[derive(Default)]
pub struct SceneContributions {
    entries: Vec<Box<dyn SceneContribution>>,
    pass_counts: Vec<usize>,
    tick_counts: Vec<usize>,
    after_counts: Vec<usize>,
}

impl SceneContributions {
    /// Construct an ordered attachment. Each member keeps its own GPU state.
    pub fn new(entries: Vec<Box<dyn SceneContribution>>) -> Self {
        Self {
            entries,
            pass_counts: Vec::new(),
            tick_counts: Vec::new(),
            after_counts: Vec::new(),
        }
    }

    fn rebuild_members(
        &mut self,
        surface: &SceneSurface<'_>,
        attaching: bool,
    ) -> Result<(), FrameError> {
        let mut first = 0;
        self.rebuild_entries(|entry| {
            let end = first + entry.secondary_view_slots().len();
            let mut member = surface.clone();
            if !entry.secondary_views_enabled() {
                member.secondary_surfaces.clear();
            }
            member.secondary_views =
                surface
                    .secondary_views
                    .get(first..end)
                    .ok_or(FrameError::InvalidState(
                        "secondary view declaration changed during rebuild",
                    ))?;
            if attaching {
                entry.rebuild_for_attachment(&member)?;
            } else {
                entry.rebuild(&member)?;
            }
            first = end;
            Ok(())
        })
    }

    pub(crate) fn rebuild_entries<E>(
        &mut self,
        mut rebuild: impl FnMut(&mut dyn SceneContribution) -> Result<(), E>,
    ) -> Result<(), E> {
        self.pass_counts.clear();
        self.tick_counts.clear();
        self.after_counts.clear();
        for entry in &mut self.entries {
            rebuild(entry.as_mut())?;
            let graph = entry.effect_graph();
            self.pass_counts.push(graph.passes.len());
            self.tick_counts.push(graph.tick_passes.len());
            self.after_counts.push(graph.after_opaque.len());
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resize_job(
        &self,
        extent: Option<[u32; 2]>,
        resize_secondary: bool,
    ) -> Result<Option<super::ExtentResizeJob>, FrameError> {
        let mut first = 0;
        let mut members = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let end = first + entry.secondary_view_slots().len();
            let job = if let Some(extent) =
                extent.filter(|&extent| entry.needs_render_capacity(extent))
            {
                entry.extent_resize_job(extent, resize_secondary)?
            } else if resize_secondary {
                entry.secondary_resize_job()?.map(|job| {
                    let job: super::ExtentResizeJob =
                        Box::new(move |slots| job(slots.expect("replacement secondary views")));
                    job
                })
            } else {
                None
            };
            members.push((first..end, job, entry.effect_graph()));
            first = end;
        }
        let needs_job = match extent {
            None => first != 0,
            Some(_) => {
                (resize_secondary && first != 0) || members.iter().any(|(_, job, _)| job.is_some())
            }
        };
        if !needs_job {
            return Ok(None);
        }
        Ok(Some(Box::new(move |slots| {
            let mut replacements = Vec::with_capacity(members.len());
            let mut graphs = Vec::with_capacity(members.len());
            for (range, job, graph) in members {
                let member_slots = slots
                    .map(|views| {
                        views.get(range).ok_or(FrameError::InvalidState(
                            "secondary view declaration changed during preparation",
                        ))
                    })
                    .transpose()?;
                let replacement = job.map(|job| job(member_slots)).transpose()?;
                graphs.push(
                    replacement
                        .as_ref()
                        .map_or(graph, |next| next.graph.clone()),
                );
                replacements.push(replacement);
            }
            Ok(super::SecondaryResize::new(
                Self::combined_graph(graphs),
                move |owner: &mut Self| {
                    for (index, replacement) in replacements.into_iter().enumerate() {
                        if let Some(next) = replacement {
                            owner.pass_counts[index] = next.graph.passes.len();
                            owner.tick_counts[index] = next.graph.tick_passes.len();
                            owner.after_counts[index] = next.graph.after_opaque.len();
                            next.publish(owner.entries[index].as_mut());
                        }
                    }
                },
            ))
        })))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_members(
        &self,
        mut prepare: impl FnMut(
            &dyn SceneContribution,
        ) -> Result<super::ContributionCandidate, FrameError>,
    ) -> Result<super::ContributionCandidate, FrameError> {
        let mut candidates = self
            .entries
            .iter()
            .map(|member| prepare(member.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let passes = candidates
            .iter()
            .fold(ContributionPasses::default(), |mut passes, member| {
                passes.upload |= member.passes.upload;
                passes.compute |= member.passes.compute;
                passes
            });
        let graph = Self::combined_graph(candidates.iter().map(|member| member.graph.clone()));
        let initialization = candidates
            .iter_mut()
            .flat_map(|candidate| std::mem::take(&mut candidate.initialization))
            .collect();
        Ok(
            super::ContributionCandidate::new(passes, graph, move |owner: &mut Self| {
                owner.pass_counts.clear();
                owner.tick_counts.clear();
                owner.after_counts.clear();
                for (entry, candidate) in owner.entries.iter_mut().zip(candidates) {
                    owner.pass_counts.push(candidate.graph.passes.len());
                    owner.tick_counts.push(candidate.graph.tick_passes.len());
                    owner.after_counts.push(candidate.graph.after_opaque.len());
                    candidate.commit(entry.as_mut());
                }
            })
            .with_initialization(initialization),
        )
    }

    fn tick_member(&self, mut index: usize) -> Option<(usize, usize)> {
        for (member, &count) in self.tick_counts.iter().enumerate() {
            if index < count {
                return Some((member, index));
            }
            index -= count;
        }
        None
    }

    fn combined_graph(
        declarations: impl IntoIterator<Item = super::EffectGraph>,
    ) -> super::EffectGraph {
        let mut graph = super::EffectGraph::default();
        for (member, mut declaration) in declarations.into_iter().enumerate() {
            let offset = graph.resources.len();
            for pass in &mut declaration.passes {
                pass.name = format!("effect {member}: {}", pass.name);
                for access in &mut pass.accesses {
                    access.resource += offset;
                }
            }
            for pass in &mut declaration.tick_passes {
                pass.name = format!("effect {member}: {}", pass.name);
                for read in &mut pass.reads {
                    // Preserve an invalid member-local index for preparation to
                    // refuse, even if it would name another member's resource.
                    read.resource = if read.resource < declaration.resources.len() {
                        offset + read.resource
                    } else {
                        usize::MAX
                    };
                }
            }
            for access in declaration.opaque_reads.iter_mut().chain(
                declaration
                    .after_opaque
                    .iter_mut()
                    .flat_map(|pass| pass.accesses.iter_mut()),
            ) {
                access.resource += offset;
            }
            let mut owners = std::collections::BTreeMap::new();
            for (_, scope) in &mut declaration.resources {
                if let crate::graph::ResourceScope::Persistent { owner, .. }
                | crate::graph::ResourceScope::History { owner, .. } = scope
                {
                    let next = (offset + owners.len()) as u64;
                    *owner = *owners.entry(*owner).or_insert(next);
                }
            }
            graph.resources.extend(declaration.resources);
            graph.opaque_reads.extend(declaration.opaque_reads);
            graph
                .sun_paths
                .extend(
                    declaration
                        .sun_paths
                        .into_iter()
                        .map(|path| super::SunPathResources {
                            optical_depth: path.optical_depth + offset,
                            mapping: path.mapping + offset,
                        }),
                );
            graph
                .view_volume_media
                .extend(declaration.view_volume_media.into_iter().map(|input| {
                    super::ViewVolumeResources {
                        coefficients: input.coefficients + offset,
                        ground_band: input.ground_band.map(|index| index + offset),
                    }
                }));
            graph.sky_overlays.extend(
                declaration
                    .sky_overlays
                    .into_iter()
                    .map(|index| index + offset),
            );
            graph.passes.extend(declaration.passes);
            graph.tick_passes.extend(declaration.tick_passes);
            let after_count = declaration.after_opaque.len();
            for pass in &mut declaration.after_opaque {
                pass.name = format!("effect {member}: {}", pass.name);
                if let super::AfterOpaqueImages::Sample { producer, .. } = &mut pass.images {
                    *producer = if *producer < after_count {
                        graph.after_opaque.len() + *producer
                    } else {
                        usize::MAX
                    };
                }
            }
            graph.after_opaque.extend(declaration.after_opaque);
        }
        graph
    }
}

impl SceneContribution for SceneContributions {
    #[cfg(not(target_arch = "wasm32"))]
    fn supports_candidate_preparation(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.supports_candidate_preparation())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_candidate(
        &self,
        surface: &SceneSurface<'_>,
    ) -> Result<super::ContributionCandidate, FrameError> {
        let mut first = 0;
        self.prepare_members(|entry| {
            let end = first + entry.secondary_view_slots().len();
            let mut member = surface.clone();
            if !entry.secondary_views_enabled() {
                member.secondary_surfaces.clear();
            }
            member.secondary_views =
                surface
                    .secondary_views
                    .get(first..end)
                    .ok_or(FrameError::InvalidState(
                        "secondary view declaration changed during preparation",
                    ))?;
            first = end;
            entry.prepare_candidate(&member)
        })
    }

    fn ray_instances(&self, instances: &mut super::RayInstances<'_>) {
        for entry in &self.entries {
            entry.ray_instances(instances);
        }
    }

    fn needs_render_capacity(&self, extent: [u32; 2]) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.needs_render_capacity(extent))
    }

    fn secondary_views_enabled(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.secondary_views_enabled())
    }
    fn encode_secondary_upload(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &ContributionFrame,
        views: &[super::SecondaryViewFrame],
    ) {
        for entry in &mut self.entries {
            if entry.secondary_views_enabled() {
                entry.encode_secondary_upload(encoder, frame, views);
            }
        }
    }
    fn draw_secondary(&self, index: usize, pass: &mut wgpu::RenderPass<'_>) {
        for entry in &self.entries {
            if entry.secondary_views_enabled() {
                entry.draw_secondary(index, pass);
            }
        }
    }
    fn draw_secondary_shadow(
        &self,
        index: usize,
        cascade: SecondaryShadowCascade,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        for entry in &self.entries {
            if entry.secondary_views_enabled() {
                entry.draw_secondary_shadow(index, cascade, pass);
            }
        }
    }
    fn secondary_view_slots(&self) -> Vec<super::SecondaryViewSlot> {
        self.entries
            .iter()
            .flat_map(|entry| entry.secondary_view_slots())
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn secondary_resize_job(&self) -> Result<Option<super::SecondaryResizeJob>, FrameError> {
        Ok(self.resize_job(None, true)?.map(|job| {
            let job: super::SecondaryResizeJob = Box::new(move |slots| job(Some(slots)));
            job
        }))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn extent_resize_job(
        &self,
        extent: [u32; 2],
        resize_secondary: bool,
    ) -> Result<Option<super::ExtentResizeJob>, FrameError> {
        self.resize_job(Some(extent), resize_secondary)
    }

    fn quality_knobs(&self) -> Vec<super::QualityKnob> {
        self.entries
            .iter()
            .enumerate()
            .flat_map(|(member, entry)| {
                entry.quality_knobs().into_iter().map(move |mut knob| {
                    knob.name = format!("{member}/{}", knob.name);
                    knob
                })
            })
            .collect()
    }
    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), super::QualityError> {
        let unknown = || super::QualityError::UnknownKnob { name: name.into() };
        let (member, local) = name.split_once('/').ok_or_else(unknown)?;
        let member: usize = member.parse().map_err(|_| unknown())?;
        self.entries
            .get_mut(member)
            .ok_or_else(unknown)?
            .set_quality_knob(local, value)
    }
    fn commit_quality_knobs(&mut self) -> super::QualityInvalidation {
        self.entries
            .iter_mut()
            .map(|entry| entry.commit_quality_knobs())
            .max()
            .unwrap_or_default()
    }
    fn effect_graph(&self) -> super::EffectGraph {
        Self::combined_graph(self.entries.iter().map(|entry| entry.effect_graph()))
    }
    fn tick_has_work(&self, index: usize) -> bool {
        self.tick_member(index)
            .is_some_and(|(member, local)| self.entries[member].tick_has_work(local))
    }

    fn encode_tick_upload(
        &mut self,
        index: usize,
        encoder: &mut wgpu::CommandEncoder,
        tick_duration: std::time::Duration,
    ) {
        if let Some((member, local)) = self.tick_member(index) {
            self.entries[member].encode_tick_upload(local, encoder, tick_duration);
        }
    }

    fn encode_tick(&mut self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        if let Some((member, local)) = self.tick_member(index) {
            self.entries[member].encode_tick(local, pass);
        }
    }

    fn encode_tick_readback(&mut self, index: usize, encoder: &mut wgpu::CommandEncoder) {
        if let Some((member, local)) = self.tick_member(index) {
            self.entries[member].encode_tick_readback(local, encoder);
        }
    }

    fn tick_submitted(&mut self, mut passes: &[usize]) {
        let mut first = 0;
        let mut local = Vec::new();
        for (entry, count) in self.entries.iter_mut().zip(&self.tick_counts) {
            let end = first + count;
            let split = passes.partition_point(|&index| index < end);
            if split != 0 {
                local.extend(passes[..split].iter().map(|index| index - first));
                entry.tick_submitted(&local);
                local.clear();
                passes = &passes[split..];
            }
            first = end;
        }
    }

    fn effect_runs(&self, mut index: usize, frame: &ContributionFrame) -> bool {
        for (entry, count) in self.entries.iter().zip(self.pass_counts.iter().copied()) {
            if index < count {
                return entry.effect_runs(index, frame);
            }
            index -= count;
        }
        false
    }
    fn draw_after_opaque(
        &self,
        mut index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        for (entry, count) in self.entries.iter().zip(self.after_counts.iter().copied()) {
            if index < count {
                entry.draw_after_opaque(index, pass, frame);
                return;
            }
            index -= count;
        }
    }
    fn after_opaque_runs(&self, mut index: usize, frame: &ContributionFrame) -> bool {
        for (entry, count) in self.entries.iter().zip(self.after_counts.iter().copied()) {
            if index < count {
                return entry.after_opaque_runs(index, frame);
            }
            index -= count;
        }
        false
    }
    fn encode_after_opaque(
        &mut self,
        mut index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        for (entry, count) in self
            .entries
            .iter_mut()
            .zip(self.after_counts.iter().copied())
        {
            if index < count {
                entry.encode_after_opaque(index, pass, frame);
                return;
            }
            index -= count;
        }
    }
    fn encode_effect(
        &mut self,
        mut index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        for (entry, count) in self
            .entries
            .iter_mut()
            .zip(self.pass_counts.iter().copied())
        {
            if index < count {
                entry.encode_effect(index, pass, frame);
                return;
            }
            index -= count;
        }
    }

    fn encode_effect_readback(&mut self, mut index: usize, encoder: &mut wgpu::CommandEncoder) {
        for (entry, count) in self
            .entries
            .iter_mut()
            .zip(self.pass_counts.iter().copied())
        {
            if index < count {
                entry.encode_effect_readback(index, encoder);
                return;
            }
            index -= count;
        }
    }

    fn passes(&self) -> ContributionPasses {
        self.entries
            .iter()
            .fold(ContributionPasses::default(), |mut passes, entry| {
                let member = entry.passes();
                passes.upload |= member.upload;
                passes.compute |= member.compute;
                passes
            })
    }
    fn draw_stages(&self) -> Vec<super::SceneDrawStage> {
        self.entries
            .iter()
            .flat_map(|entry| entry.draw_stages())
            .collect()
    }

    fn sprite_stage_refusal(&self) -> Option<String> {
        self.entries
            .iter()
            .find_map(|entry| entry.sprite_stage_refusal())
    }

    fn rebuild_2d(
        &mut self,
        surface: &super::SpriteContributionSurface<'_>,
    ) -> Result<(), FrameError> {
        self.pass_counts.clear();
        self.tick_counts.clear();
        self.after_counts.clear();
        for entry in &mut self.entries {
            entry.rebuild_2d(surface)?;
            self.pass_counts.push(entry.effect_graph().passes.len());
            self.after_counts.push(0);
        }
        Ok(())
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.rebuild_members(surface, false)
    }
    fn rebuild_for_attachment(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.rebuild_members(surface, true)
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        for entry in &mut self.entries {
            entry.encode_upload(encoder, frame);
        }
    }
    fn computes(&self, frame: &ContributionFrame) -> bool {
        self.entries.iter().any(|entry| entry.computes(frame))
    }
    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame) {
        for entry in &mut self.entries {
            if entry.computes(frame) {
                entry.encode_compute(pass, frame);
            }
        }
    }
    fn encode_after_compute(&mut self, encoder: &mut wgpu::CommandEncoder) {
        for entry in &mut self.entries {
            entry.encode_after_compute(encoder);
        }
    }
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        for entry in &self.entries {
            entry.draw(pass);
        }
    }
    fn draw_ambient_receivers(&self, pass: &mut wgpu::RenderPass<'_>) {
        for entry in &self.entries {
            entry.draw_ambient_receivers(pass);
        }
    }
    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        self.entries
            .iter()
            .filter_map(|entry| entry.shadow_bounds())
            .reduce(ShadowBounds::union)
    }
    fn draw_shadow(&self, pass: &mut wgpu::RenderPass<'_>) {
        for entry in &self.entries {
            entry.draw_shadow(pass);
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        for entry in &mut self.entries {
            entry.submitted(history_advanced);
        }
    }
}

#[cfg(test)]
mod tests {
    //! The list stands in front of its members, so a hook it leaves to the trait's default
    //! never reaches them. A probe behind it sees each of the eight hooks arrive.
    use super::*;
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<&'static str>>>;

    #[test]
    fn a_tick_pass_can_only_reference_its_own_members_resources() {
        use super::super::{EffectAccess, EffectGraph, TickPass};
        use crate::graph::{AccessKind, ResourceDescriptor, ResourceScope};

        let resource = (
            ResourceDescriptor::Buffer {
                bytes: 4,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Persistent {
                owner: 0,
                epoch: 0,
                initialized: true,
            },
        );
        let member = |index| EffectGraph {
            resources: vec![resource.clone()],
            tick_passes: vec![TickPass {
                name: "local query".into(),
                reads: vec![EffectAccess {
                    resource: index,
                    range: 0..4,
                    kind: AccessKind::StorageRead,
                    view: None,
                    history: None,
                }],
            }],
            ..EffectGraph::default()
        };
        let nested = SceneContributions::combined_graph([member(0), member(0)]);
        let valid = SceneContributions::combined_graph([member(0), nested.clone()]);
        assert_eq!(valid.tick_passes[2].reads[0].resource, 2);
        assert_eq!(valid.validate(), Ok(()));
        let invalid = SceneContributions::combined_graph([member(1), nested]);
        assert_eq!(
            invalid.validate(),
            Err(crate::error::EffectGraphError::UnknownEffectResource)
        );
    }

    #[test]
    fn a_nested_list_offsets_sun_path_resources_with_their_opaque_reads() {
        use crate::{
            contribution::{EffectAccess, EffectGraph, SunPathResources},
            graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
        };
        let imported = ResourceScope::Imported { initialized: true };
        let buffer = |bytes| {
            (
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                imported,
            )
        };
        let mapping_bytes = size_of::<crate::atmosphere::GpuSunMapping>() as u64;
        let read = |resource, kind, range| EffectAccess {
            resource,
            kind,
            range,
            view: None,
            history: None,
        };
        let prefix = EffectGraph {
            resources: vec![buffer(4)],
            ..EffectGraph::default()
        };
        let provider = EffectGraph {
            resources: vec![
                (
                    ResourceDescriptor::Texture {
                        dimension: TextureKind::D2,
                        width: 2,
                        height: 2,
                        layers: 1,
                        mips: 1,
                        samples: 1,
                        format: wgpu::TextureFormat::Rgba16Float,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    },
                    imported,
                ),
                buffer(mapping_bytes),
            ],
            opaque_reads: vec![
                read(0, AccessKind::SampledRead, 0..1),
                read(1, AccessKind::UniformRead, 0..mapping_bytes),
            ],
            view_volume_media: Vec::new(),
            sky_overlays: Vec::new(),
            sun_paths: vec![SunPathResources {
                optical_depth: 0,
                mapping: 1,
            }],
            ..EffectGraph::default()
        };
        let nested = SceneContributions::combined_graph([prefix.clone(), provider.clone()]);
        let graph = SceneContributions::combined_graph([prefix, nested]);
        assert_eq!(
            graph.sun_paths,
            [SunPathResources {
                optical_depth: 2,
                mapping: 3
            }]
        );
        assert_eq!(graph.opaque_reads[0].resource, 2);
        assert_eq!(graph.opaque_reads[1].resource, 3);
        graph.validate().expect("combined Sun path");
        let duplicate = SceneContributions::combined_graph([provider, graph]);
        assert_eq!(
            duplicate.validate(),
            Err(crate::error::EffectGraphError::DuplicateSunPath)
        );
    }

    struct Probe(Seen);

    impl Probe {
        fn saw(&self, hook: &'static str) {
            self.0.lock().expect("probe record").push(hook);
        }
    }

    impl SceneContribution for Probe {
        fn passes(&self) -> ContributionPasses {
            self.saw("passes");
            ContributionPasses {
                upload: true,
                compute: true,
            }
        }
        fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), FrameError> {
            self.saw("rebuild");
            Ok(())
        }
        fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
            self.saw("encode_upload");
        }
        fn computes(&self, _: &ContributionFrame) -> bool {
            self.saw("computes");
            true
        }
        fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
            self.saw("encode_compute");
        }
        fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {
            self.saw("encode_after_compute");
        }
        fn draw(&self, _: &mut wgpu::RenderPass<'_>) {
            self.saw("draw");
        }
        fn submitted(&mut self, _: bool) {
            self.saw("submitted");
        }
    }

    /// Run `call` on the list and require that it reached the probe's `hook`.
    fn reaches<T>(seen: &Seen, hook: &'static str, call: impl FnOnce() -> T) -> T {
        seen.lock().expect("probe record").clear();
        let result = call();
        assert!(
            seen.lock().expect("probe record").contains(&hook),
            "SceneContributions did not forward `{hook}` to its member"
        );
        result
    }

    #[test]
    fn every_hook_reaches_a_member_behind_the_list() {
        let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
            "every_hook_reaches_a_member_behind_the_list",
            None,
            crate::capabilities::DeviceProfile::Portable,
            &crate::capabilities::DeviceRestriction {
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            },
        ) else {
            return;
        };
        pollster::block_on(async {
            let scene = crate::synthetic::SyntheticScene::grid(1).expect("scene");
            let prepared = crate::synthetic::PreparedSynthetic::prepare(&device, &queue, 1, &scene)
                .await
                .expect("prepared");
            let seen = Seen::default();
            let mut list = SceneContributions::new(vec![Box::new(Probe(Arc::clone(&seen)))]);
            let frame = prepared.contribution_frame([1, 1]);

            let passes = reaches(&seen, "passes", || list.passes());
            assert!(passes.upload && passes.compute);
            reaches(&seen, "rebuild", || {
                list.rebuild(&prepared.scene_surface(&queue))
            })
            .expect("rebuild");

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            reaches(&seen, "encode_upload", || {
                list.encode_upload(&mut encoder, &frame);
            });
            // The five hooks with defaults: a missing forward here falls silently to the default.
            assert!(reaches(&seen, "computes", || list.computes(&frame)));
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                reaches(&seen, "encode_compute", || {
                    list.encode_compute(&mut pass, &frame);
                });
            }
            reaches(&seen, "encode_after_compute", || {
                list.encode_after_compute(&mut encoder);
            });
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("list probe target"),
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("list probe draw"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations::default(),
                    })],
                    ..wgpu::RenderPassDescriptor::default()
                });
                reaches(&seen, "draw", || list.draw(&mut pass));
            }
            queue.submit([encoder.finish()]);
            reaches(&seen, "submitted", || list.submitted(true));
        });
    }
}
