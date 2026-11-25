//! The scene contribution's place in the prepared scene: its graph passes, the surface it draws
//! into and the camera facts it is given.

use crate::graph::PassBody;
use crate::pass_key::{PassKey, PipelineRole};
use std::collections::BTreeSet;

use super::{
    ContributionFrame, ContributionPasses, DEPTH_FORMAT, MOTION_FORMAT, PreparedSynthetic,
    RenderError, SCENE_FORMAT, SceneSurface,
};
use crate::graph::ResourceId;
use crate::graph::{
    Access, AccessKind, FrameGraph, PassScope, PipelineKey, ResourceDescriptor, ResourceScope,
};

/// Timed graph pass that runs the scene contribution's compute work.
pub(crate) const CONTRIBUTION_COMPUTE: &str = PassKey::ContributionCompute.constant_name();
/// Graph name of the contribution's staged-copy pass.
/// Extent of each contribution token resource. Tokens only order the contribution passes before
/// forward for resources the contribution owns; they are not those resources' extent, memory or
/// hazard evidence.
pub(super) const CONTRIBUTION_TOKEN_BYTES: u64 = 4;

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

    /// Declare the graph passes of the scene contribution drawn into this scene's targets.
    /// Targets created afterwards include them; existing targets are unchanged.
    pub(crate) fn set_contribution_passes(&mut self, passes: ContributionPasses) {
        self.contribution_passes = passes;
    }

    /// The contribution passes targets of this scene include.
    pub(crate) fn contribution_passes(&self) -> ContributionPasses {
        self.contribution_passes
    }
}

impl super::SyntheticTargetInputs {
    pub(super) fn contribution_key(&self) -> PipelineKey {
        PipelineRole::ContributionCompute.key(self.key)
    }

    /// Declare the contribution's upload and compute passes before forward; forward reads what
    /// they write, so the graph orders them, and the compute pass is timed like any other.
    pub(super) fn append_contribution(
        &self,
        graph: &mut FrameGraph,
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        mesh: &[Access; 2],
    ) -> Result<(crate::contribution::EffectResources, Option<ResourceId>), RenderError> {
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
                wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::STORAGE
                    | if self.ray_demand {
                        wgpu::BufferUsages::BLAS_INPUT
                    } else {
                        wgpu::BufferUsages::empty()
                    },
            )?;
            graph.declare(
                PassKey::ContributionUpload,
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: None,
                    accesses: vec![Access::new(
                        products.clone(),
                        0..CONTRIBUTION_TOKEN_BYTES,
                        AccessKind::CopyWrite,
                    )],
                    external_side_effect: false,
                },
            );
            accesses.push(Access::new(
                products.clone(),
                0..CONTRIBUTION_TOKEN_BYTES,
                AccessKind::StorageRead,
            ));
            Some(products)
        } else {
            None
        };
        let ray_structure = self.append_ray_structure(graph, mesh, products.as_ref())?;
        if passes.compute {
            let draws = token(
                graph,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            )?;
            let mut compute = vec![Access::new(
                draws.clone(),
                0..CONTRIBUTION_TOKEN_BYTES,
                AccessKind::StorageWrite,
            )];
            if let Some(products) = products {
                compute.push(Access::new(
                    products,
                    0..CONTRIBUTION_TOKEN_BYTES,
                    AccessKind::StorageRead,
                ));
            }
            prepared_keys.insert(self.contribution_key());
            graph.declare(
                PassKey::ContributionCompute,
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(self.contribution_key()),
                    accesses: compute,
                    external_side_effect: false,
                },
            );
            accesses.push(Access::new(
                draws,
                0..CONTRIBUTION_TOKEN_BYTES,
                AccessKind::IndirectRead,
            ));
        }
        let resources = self.effect_graph.append(graph, self.key, prepared_keys)?;
        accesses.extend_from_slice(&resources.opaque_reads);
        Ok((resources, ray_structure))
    }
}

impl PreparedSynthetic {
    /// The forward pass a [`SceneContribution`] draws into.
    pub(crate) fn scene_surface<'a>(&'a self, queue: &'a wgpu::Queue) -> SceneSurface<'a> {
        SceneSurface {
            ray_demand: self.mesh_arena.ray_demand(),
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            ray_build_probe: self.ray_build_probe(),
            secondary_surfaces: Vec::new(),
            secondary_views: &self.secondary_handles,
            sun_optical_depth: self.cloud.is_some(),
            cloud: self.cloud.as_ref(),
            environment: self.atmosphere.as_ref(),
            directional: &self.directional_buffer,
            environment_light: &self.environment_light,
            exposure: &self.exposure.samples,
            device: self.capabilities.device(),
            queue,
            generation: self.key.device_generation,
            features: self.capabilities.features(),
            downlevel: self.capabilities.downlevel(),
            limits: self.capabilities.limits().clone(),
            recipe: self
                .capabilities
                .recipe()
                .expect("scene preparation admitted this generation's recipe"),
            shader_source: &self.surface.shader_source,
            pipeline_layout: &self.surface.pipeline_layout,
            view_layout: &self.view_layout,
            mesh_view_uniform: wgpu::BufferBinding {
                buffer: &self.view_buffer,
                offset: 0,
                size: wgpu::BufferSize::new(super::VIEW_BYTES as u64),
            },
            light_layout: &self.surface.light_layout,
            material_layout: &self.surface.material_layout,
            fragment_entry: self.surface.fragment_entry,
            color_formats: [SCENE_FORMAT, MOTION_FORMAT, super::REACTIVE_FORMAT],
            ambient_receiver: self.ambient.as_ref().map(|_| {
                crate::contribution::AmbientReceiverSurface {
                    fragment_entry: super::raytrace::ambient::RECEIVER_ENTRY,
                    color_format: super::raytrace::ambient::RECEIVER_FORMAT,
                    depth_format: DEPTH_FORMAT,
                }
            }),
            depth_format: DEPTH_FORMAT,
            shadow: self
                .shadow
                .as_ref()
                .map(super::shadow::ShadowPrepared::surface),
        }
    }

    pub(crate) fn scene_surface_with_views<'a>(
        &'a self,
        queue: &'a wgpu::Queue,
        enabled: bool,
    ) -> SceneSurface<'a> {
        let mut surface = self.scene_surface(queue);
        if enabled {
            surface.secondary_surfaces = self
                .secondary
                .iter()
                .enumerate()
                .map(|(index, slot)| slot.surface(index, self))
                .collect();
        }
        surface
    }

    /// Camera facts of the view the next encode draws.
    pub(crate) fn contribution_frame(&self, render_extent: [u32; 2]) -> ContributionFrame {
        let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        ContributionFrame {
            time: self.effect_time,
            exposure_side: self.history.write_side(),
            environment: self
                .atmosphere
                .as_ref()
                .map(crate::atmosphere::AtmospherePrepared::environment),
            view_proj: view.raster_projection().to_cols_array_2d(),
            render_extent,
            view_uniform_offset: self.view_stride * u32::from(self.frame_revision & 1 != 0),
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
            drop(pass);
            contribution.encode_effect_readback(index, self.encoder);
        }
        Ok(())
    }
}
