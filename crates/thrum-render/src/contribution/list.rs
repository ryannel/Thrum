//! An ordered scene attachment, retaining the existing single-contribution API.

use super::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, ShadowBounds};

/// Contributions in game-selected order. Empty lists add no graph passes.
#[derive(Default)]
pub struct SceneContributions {
    entries: Vec<Box<dyn SceneContribution>>,
    pass_counts: Vec<usize>,
}

impl SceneContributions {
    /// Construct an ordered attachment. Each member keeps its own GPU state.
    pub fn new(entries: Vec<Box<dyn SceneContribution>>) -> Self {
        Self {
            entries,
            pass_counts: Vec::new(),
        }
    }
}

impl SceneContribution for SceneContributions {
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
            graph.passes.extend(declaration.passes);
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
        for entry in &mut self.entries {
            entry.rebuild(surface)?;
            self.pass_counts.push(entry.effect_graph().passes.len());
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
