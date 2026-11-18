//! An ordered scene attachment, retaining the existing single-contribution API.

use super::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SecondaryShadowCascade,
    ShadowBounds,
};

/// Contributions in game-selected order. Empty lists add no graph passes.
#[derive(Default)]
pub struct SceneContributions {
    entries: Vec<Box<dyn SceneContribution>>,
    pass_counts: Vec<usize>,
    after_counts: Vec<usize>,
}

impl SceneContributions {
    /// Construct an ordered attachment. Each member keeps its own GPU state.
    pub fn new(entries: Vec<Box<dyn SceneContribution>>) -> Self {
        Self {
            entries,
            pass_counts: Vec::new(),
            after_counts: Vec::new(),
        }
    }
}

impl SceneContribution for SceneContributions {
    fn ray_instances(&self, instances: &mut super::RayInstances<'_>) {
        for entry in &self.entries {
            entry.ray_instances(instances);
        }
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
        let mut graph = super::EffectGraph::default();
        for (member, entry) in self.entries.iter().enumerate() {
            let mut declaration = entry.effect_graph();
            let offset = graph.resources.len();
            for pass in &mut declaration.passes {
                pass.name = format!("effect {member}: {}", pass.name);
                for access in &mut pass.accesses {
                    access.resource += offset;
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
            graph.passes.extend(declaration.passes);
            for pass in &mut declaration.after_opaque {
                pass.name = format!("effect {member}: {}", pass.name);
            }
            graph.after_opaque.extend(declaration.after_opaque);
        }
        graph
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
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.pass_counts.clear();
        self.after_counts.clear();
        let mut first = 0;
        for entry in &mut self.entries {
            let end = first + entry.secondary_view_slots().len();
            let mut member = surface.clone();
            if !entry.secondary_views_enabled() {
                member.secondary_surfaces.clear();
            }
            member.secondary_views = surface
                .secondary_views
                .get(first..end)
                .ok_or_else(|| "secondary view declaration changed during rebuild".to_owned())?;
            entry.rebuild(&member)?;
            first = end;
            self.pass_counts.push(entry.effect_graph().passes.len());
            self.after_counts
                .push(entry.effect_graph().after_opaque.len());
        }
        Ok(())
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
