//! The scene contribution's place in the prepared scene: its graph passes, the surface it draws
//! into and the camera facts it is given.

use std::collections::BTreeSet;

use super::{
    ContributionFrame, ContributionPasses, DEPTH_FORMAT, MOTION_FORMAT, PreparedSynthetic,
    RenderError, SCENE_FORMAT, SceneSurface,
};
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceScope,
};

/// Timed graph pass that runs the scene contribution's compute work.
pub(crate) const CONTRIBUTION_COMPUTE: &str = "contribution compute";
/// Graph name of the contribution's staged-copy pass.
pub(super) const CONTRIBUTION_UPLOAD: &str = "contribution upload";
/// Extent of each contribution token resource. Tokens only order the contribution passes before
/// forward for resources the contribution owns; they are not those resources' extent, memory or
/// hazard evidence.
const CONTRIBUTION_TOKEN_BYTES: u64 = 4;

impl PreparedSynthetic {
    pub(crate) fn set_effect_graph(&mut self, graph: crate::contribution::EffectGraph) {
        self.effect_graph = graph;
    }
    pub(crate) fn effect_graph(&self) -> &crate::contribution::EffectGraph {
        &self.effect_graph
    }
    pub(crate) fn set_effect_time(&mut self, time: crate::contribution::EffectTime) {
        self.effect_time = time;
    }

    pub(super) fn contribution_key(&self) -> PipelineKey {
        PipelineKey {
            role: 1000,
            ..self.key
        }
    }

    /// Declare the graph passes of the scene contribution drawn into this scene's targets.
    /// Targets created afterwards include them; existing targets are unchanged.
    pub(crate) fn set_contribution_passes(&mut self, passes: ContributionPasses) {
        self.contribution_passes = passes;
    }

    /// The contribution passes targets of this scene include.
    pub(crate) fn contribution_passes(&self) -> ContributionPasses {
        self.contribution_passes
    }

    /// Declare the contribution's upload and compute passes before forward; forward reads what
    /// they write, so the graph orders them, and the compute pass is timed like any other.
    pub(super) fn append_contribution(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<(), RenderError> {
        let passes = self.contribution_passes;
        let token = |graph: &mut FrameGraph, usage| {
            graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: CONTRIBUTION_TOKEN_BYTES,
                    usage,
                },
                ResourceScope::Imported { initialized: true },
            )
        };
        let products = if passes.upload {
            let products = token(
                graph,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            )?;
            graph.pass(Pass {
                name: CONTRIBUTION_UPLOAD.into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    view: None,
                    resource: products.clone(),
                    range: 0..CONTRIBUTION_TOKEN_BYTES,
                    kind: AccessKind::CopyWrite,
                    store: true,
                }],
                external_side_effect: false,
            });
            accesses.push(Access {
                view: None,
                resource: products.clone(),
                range: 0..CONTRIBUTION_TOKEN_BYTES,
                kind: AccessKind::StorageRead,
                store: true,
            });
            Some(products)
        } else {
            None
        };
        if passes.compute {
            let draws = token(
                graph,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            )?;
            let mut compute = vec![Access {
                view: None,
                resource: draws.clone(),
                range: 0..CONTRIBUTION_TOKEN_BYTES,
                kind: AccessKind::StorageWrite,
                store: true,
            }];
            if let Some(products) = products {
                compute.push(Access {
                    view: None,
                    resource: products,
                    range: 0..CONTRIBUTION_TOKEN_BYTES,
                    kind: AccessKind::StorageRead,
                    store: true,
                });
            }
            prepared_keys.insert(self.contribution_key());
            graph.pass(Pass {
                name: CONTRIBUTION_COMPUTE.into(),
                scope: PassScope::View(1),
                pipeline: Some(self.contribution_key()),
                accesses: compute,
                external_side_effect: false,
            });
            accesses.push(Access {
                view: None,
                resource: draws,
                range: 0..CONTRIBUTION_TOKEN_BYTES,
                kind: AccessKind::IndirectRead,
                store: true,
            });
        }
        accesses.extend(self.effect_graph.append(graph, self.key, prepared_keys)?);
        Ok(())
    }

    /// The forward pass a [`SceneContribution`] draws into.
    pub(crate) fn scene_surface<'a>(&'a self, queue: &'a wgpu::Queue) -> SceneSurface<'a> {
        SceneSurface {
            sun_optical_depth: self.cloud.is_some(),
            environment: self.atmosphere.as_ref(),
            directional: &self.directional_buffer,
            exposure: &self.exposure.samples,
            device: self.capabilities.device(),
            queue,
            generation: self.key.device_generation,
            features: self.capabilities.features(),
            downlevel: self.capabilities.downlevel(),
            limits: self.capabilities.limits().clone(),
            shader_source: &self.surface.shader_source,
            pipeline_layout: &self.surface.pipeline_layout,
            view_layout: &self.view_layout,
            light_layout: &self.surface.light_layout,
            material_layout: &self.surface.material_layout,
            fragment_entry: self.surface.fragment_entry,
            color_formats: [SCENE_FORMAT, MOTION_FORMAT, super::REACTIVE_FORMAT],
            depth_format: DEPTH_FORMAT,
            shadow: self
                .shadow
                .as_ref()
                .map(super::shadow::ShadowPrepared::surface),
        }
    }

    /// Camera facts of the view the next encode draws.
    pub(crate) fn contribution_frame(&self) -> ContributionFrame {
        let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        ContributionFrame {
            time: self.effect_time,
            exposure_side: self.history.write_side(),
            environment: self
                .atmosphere
                .as_ref()
                .map(crate::atmosphere::AtmospherePrepared::environment),
            view_proj: view.raster_projection().to_cols_array_2d(),
            camera: view.camera_position(),
            generation: self.key.device_generation,
            history_reset: view.history_reset(),
        }
    }
}

impl super::SceneContext<'_> {
    pub(super) fn effect(
        &mut self,
        index: usize,
        compiled: &super::CompiledPass,
    ) -> Result<(), RenderError> {
        if let Some(contribution) = self.contribution.as_deref_mut()
            && contribution.effect_runs(index, &self.contribution_frame)
        {
            let mut pass = compiled
                .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
                .ok_or(RenderError::GraphPassMismatch)?;
            contribution.encode_effect(index, &mut pass, &self.contribution_frame);
        }
        Ok(())
    }
}
