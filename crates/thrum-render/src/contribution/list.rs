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

    fn rebuild_members(
        &mut self,
        surface: &SceneSurface<'_>,
        mut rebuild: impl FnMut(&mut dyn SceneContribution, &SceneSurface<'_>) -> Result<(), FrameError>,
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
            rebuild(entry, &member)?;
            first = end;
            Ok(())
        })
    }

    pub(crate) fn rebuild_entries<E>(
        &mut self,
        mut rebuild: impl FnMut(&mut dyn SceneContribution) -> Result<(), E>,
    ) -> Result<(), E> {
        self.pass_counts.clear();
        self.after_counts.clear();
        for entry in &mut self.entries {
            rebuild(entry.as_mut())?;
            let graph = entry.effect_graph();
            self.pass_counts.push(graph.passes.len());
            self.after_counts.push(graph.after_opaque.len());
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_members(
        &self,
        mut prepare: impl FnMut(
            &dyn SceneContribution,
        ) -> Result<super::ContributionCandidate, FrameError>,
    ) -> Result<super::ContributionCandidate, FrameError> {
        let candidates = self
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
        Ok(super::ContributionCandidate::new(
            passes,
            graph,
            move |owner: &mut Self| {
                owner.pass_counts.clear();
                owner.after_counts.clear();
                for (entry, candidate) in owner.entries.iter_mut().zip(candidates) {
                    owner.pass_counts.push(candidate.graph.passes.len());
                    owner.after_counts.push(candidate.graph.after_opaque.len());
                    candidate.commit(entry.as_mut());
                }
            },
        ))
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
        let mut first = 0;
        let mut members = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let end = first + entry.secondary_view_slots().len();
            members.push((
                first..end,
                entry.secondary_resize_job()?,
                entry.effect_graph(),
            ));
            first = end;
        }
        if first == 0 {
            return Ok(None);
        }
        Ok(Some(Box::new(move |slots| {
            let mut replacements = Vec::with_capacity(members.len());
            let mut graphs = Vec::with_capacity(members.len());
            for (range, job, graph) in members {
                let member_slots = slots.get(range).ok_or(FrameError::InvalidState(
                    "secondary view declaration changed during preparation",
                ))?;
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
                            owner.after_counts[index] = next.graph.after_opaque.len();
                            next.publish(owner.entries[index].as_mut());
                        }
                    }
                },
            ))
        })))
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

    fn rebuild_2d(
        &mut self,
        surface: &super::SpriteContributionSurface<'_>,
    ) -> Result<(), FrameError> {
        self.pass_counts.clear();
        self.after_counts.clear();
        for entry in &mut self.entries {
            entry.rebuild_2d(surface)?;
            self.pass_counts.push(entry.effect_graph().passes.len());
            self.after_counts.push(0);
        }
        Ok(())
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.rebuild_members(surface, |entry, member| entry.rebuild(member))
    }
    fn rebuild_for_attachment(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.rebuild_members(surface, |entry, member| {
            entry.rebuild_for_attachment(member)
        })
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

#[cfg(test)]
mod tests {
    //! The list stands in front of its members, so a hook it leaves to the trait's default
    //! never reaches them. A probe behind it sees each of the eight hooks arrive.
    use super::*;
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<&'static str>>>;

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
