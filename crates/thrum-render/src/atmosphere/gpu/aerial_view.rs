//! An observer's aerial product over the world's transport LUTs. This reuses
//! the prepared main pass's pipeline and sampling contract, including all rows
//! of the column volume. There is no second transport integrator.

use super::{AERIAL_EXTENT, AtmospherePrepared, AtmosphereUniform, UNIFORM_BYTES};
use crate::error::RenderError;
use crate::graph::PassBody;
use crate::pass_key::{AtmosphereStage, PassKey, SecondaryStage};
use crate::{
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, PassScope, PipelineKey, ResourceDescriptor,
        ResourceScope,
    },
    shader_constants::AERIAL_WORKGROUP_EDGE,
    synthetic::SyntheticView,
};
use std::collections::BTreeSet;

pub(crate) const PASS: &str = "secondary view aerial perspective";

#[derive(Clone)]
pub(crate) struct SecondaryAerial {
    uniform: wgpu::Buffer,
    sampling_uniform: wgpu::Buffer,
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    sampler: wgpu::Sampler,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    night: crate::SharedGpu<crate::atmosphere::night::NightProfile>,
}

impl AtmospherePrepared {
    pub(crate) fn secondary_aerial(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> SecondaryAerial {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(PASS),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampling_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("secondary aerial sampling frame"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let textures = self.aerial_textures().map(|texture| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(PASS),
                size: texture.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: texture.format(),
                usage: texture.usage(),
                view_formats: &[],
            })
        });
        let views = textures.each_ref().map(|texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..wgpu::TextureViewDescriptor::default()
            })
        });
        let charts = self
            .textures
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let night = self
            .night
            .observer(device, queue, self.frame.nightglow_active());
        let entries = [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&self.aerial_sampler),
            },
        ]
        .into_iter()
        // The same shader and chart reads as the camera's aerial stage.
        .chain(super::chart_entries(AtmosphereStage::Aerial, &charts))
        .chain(night.bindings())
        .collect::<Vec<_>>();
        let pipeline = self.aerial_pipeline.clone();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        SecondaryAerial {
            uniform,
            sampling_uniform,
            textures,
            views,
            sampler: self.aerial_sampler.clone(),
            pipeline,
            group,
            night: crate::SharedGpu::new(night),
        }
    }
}

impl SecondaryAerial {
    pub(crate) fn textures(&self) -> [&wgpu::Texture; 2] {
        self.textures.each_ref()
    }
    pub(crate) fn sampled(&self) -> ([&wgpu::TextureView; 2], &wgpu::Sampler, &wgpu::Buffer) {
        (
            [&self.views[0], &self.views[1]],
            &self.sampler,
            &self.sampling_uniform,
        )
    }

    pub(crate) fn update(
        &self,
        queue: &wgpu::Queue,
        world: &AtmospherePrepared,
        view: &SyntheticView,
        observer_world_height: f32,
    ) {
        let camera_height = world
            .pending_sky
            .as_ref()
            .map_or(world.frame.camera_altitude_m, |(camera, _)| {
                camera.camera_position()[1]
            });
        let datum = world.frame.camera_altitude_m - camera_height;
        let mut frame = world.frame;
        frame.camera_altitude_m = (observer_world_height + datum).clamp(0.0, 100_000.0);
        self.night.write().select(queue, frame.nightglow_active());
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&AtmosphereUniform::new(
                frame,
                view,
                frame.camera_altitude_m,
            )),
        );
        queue.write_buffer(
            &self.sampling_uniform,
            0,
            bytemuck::bytes_of(&AtmosphereUniform::new(
                frame,
                view,
                view.camera_position()[1] + datum,
            )),
        );
    }

    pub(crate) fn append(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        key: PipelineKey,
        index: usize,
        shared: &[Access],
        opaque: &mut Vec<Access>,
    ) -> Result<(), RenderError> {
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: UNIFORM_BYTES,
                usage: self.uniform.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        // The shared view inputs carry the forward pass's chart reads; the
        // aerial stage reads the charts its table lists instead.
        let mut writes =
            crate::atmosphere::graph::stage_reads(graph, AtmosphereStage::Aerial, shared)?;
        writes.extend(self.night.write().graph_reads(graph)?);
        let read = Access::new(uniform, 0..UNIFORM_BYTES, AccessKind::UniformRead);
        writes.push(read.clone());
        let sampling = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: UNIFORM_BYTES,
                usage: self.sampling_uniform.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        opaque.push(Access::new(
            sampling,
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        ));
        for texture in self.textures() {
            let id = graph.resource(
                ResourceDescriptor::of_texture(texture),
                ResourceScope::Imported { initialized: true },
            )?;
            let layers = u64::from(texture.depth_or_array_layers());
            writes.push(Access::new(id.clone(), 0..layers, AccessKind::StorageWrite));
            opaque.push(Access::new(id, 0..layers, AccessKind::SampledRead));
        }
        keys.insert(key);
        graph.declare(
            PassKey::Secondary {
                view: crate::pass_key::pass_index(index),
                stage: SecondaryStage::Aerial,
            },
            PassBody {
                scope: PassScope::View(index as u64 + 2),
                pipeline: Some(key),
                accesses: writes,
                external_side_effect: false,
            },
        );
        Ok(())
    }

    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut crate::timing::GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_compute(encoder, generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(
            AERIAL_EXTENT[0].div_ceil(AERIAL_WORKGROUP_EDGE),
            AERIAL_EXTENT[1].div_ceil(AERIAL_WORKGROUP_EDGE),
            1,
        );
        Ok(())
    }
}
