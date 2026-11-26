//! One independent meshlet argument stream per prepared secondary view.

use thrum_render::binding;
use thrum_render::contribution::{
    SceneSurface, SecondaryShadowCascade, SecondaryView, SecondaryViewFrame, SecondaryViewRecord,
};

use super::{Bound, CULL_BYTES, DRAW_ARGS_BYTES, MeshPipelines, Pipelines};
use crate::{CookedDrawPath, load::ResidentSet};

pub(super) struct SecondaryCull {
    pipeline: wgpu::ComputePipeline,
    pub(super) layout: wgpu::BindGroupLayout,
    pub(super) records: Vec<wgpu::Buffer>,
    limits: wgpu::Limits,
}

impl SecondaryCull {
    pub(super) fn new(surface: &SceneSurface<'_>, count: usize) -> Option<Self> {
        let record_bytes = size_of::<SecondaryViewRecord>() as u64;
        let limits = &surface.limits;
        // Unlike the ordinary cull, this layout needs two uniform bindings. Each
        // slot has its own buffers, so slot count never multiplies a binding size.
        if limits.max_uniform_buffers_per_shader_stage < 2
            || limits.max_uniform_buffer_binding_size < record_bytes.max(CULL_BYTES as u64)
            || limits.max_buffer_size < record_bytes
            || limits.max_bindings_per_bind_group <= 5
        {
            return None;
        }
        let device = surface.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cooked secondary cull"),
            entries: &[
                binding::uniform(0, wgpu::ShaderStages::COMPUTE, 0),
                binding::storage(1, wgpu::ShaderStages::COMPUTE, true, 0),
                binding::storage(2, wgpu::ShaderStages::COMPUTE, true, 0),
                binding::storage(3, wgpu::ShaderStages::COMPUTE, false, 0),
                binding::uniform(5, wgpu::ShaderStages::COMPUTE, 0),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cooked secondary cull"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let source = format!(
            "{}\n{}\n{}",
            SecondaryView::record_wgsl(),
            crate::records::CULL_SHADER,
            include_str!("secondary.wgsl"),
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked secondary cull"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("cooked secondary cull"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cull_secondary"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let records = (0..count)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("cooked secondary cull view"),
                    size: record_bytes,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Some(Self {
            pipeline,
            layout: layout.into_raw(),
            records,
            limits: limits.clone(),
        })
    }

    pub(super) fn admits(&self, set: &ResidentSet) -> bool {
        let max_bytes = self
            .limits
            .max_buffer_size
            .min(self.limits.max_storage_buffer_binding_size);
        set.meshes.iter().all(|(mesh, _)| {
            mesh.ranges.is_some()
                && u64::from(mesh.meshlet_count.max(1)) * DRAW_ARGS_BYTES <= max_bytes
                && mesh.meshlet_count.div_ceil(64)
                    <= self.limits.max_compute_workgroups_per_dimension
        })
    }
}

impl Pipelines {
    pub(crate) fn stage_views(&self, views: &[SecondaryViewFrame]) {
        if let Some(cull) = &self.secondary_cull {
            for view in views {
                self.queue.write_buffer(
                    &cull.records[view.index],
                    0,
                    bytemuck::bytes_of(&view.record),
                );
            }
        }
    }

    pub(crate) fn cull_views(
        &self,
        bound: &Bound,
        pass: &mut wgpu::ComputePass<'_>,
        views: &[SecondaryViewFrame],
    ) {
        let Some(cull) = &self.secondary_cull else {
            return;
        };
        if !bound.culls_views(views) {
            return;
        }
        pass.set_pipeline(&cull.pipeline);
        for view in views {
            let [x, y, z, _] = view.record.camera;
            for mesh in &bound.meshes {
                self.cull_mesh(
                    pass,
                    mesh,
                    &mesh.secondary[view.index],
                    view.record.view_proj,
                    [x, y, z],
                );
            }
        }
    }

    pub(crate) fn draw_view(
        &self,
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        index: usize,
        meshes: Option<&MeshPipelines>,
    ) {
        if bound.has_meshes() {
            pass.set_pipeline(&meshes.unwrap_or(&self.mesh).secondary[index]);
            for mesh in &bound.meshes {
                mesh.bind_draw(pass);
                mesh.draw(pass, mesh.secondary.get(index));
            }
        }
    }

    pub(crate) fn draw_view_shadow(
        &self,
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        index: usize,
        cascade: SecondaryShadowCascade,
    ) {
        Self::draw_casters(
            bound,
            pass,
            self.secondary_shadows[index].as_ref(),
            None,
            Some(cascade),
        );
    }
}

impl Bound {
    pub(super) fn culls_views(&self, views: &[SecondaryViewFrame]) -> bool {
        !views.is_empty() && self.view_path() == CookedDrawPath::Indirect
    }

    pub(crate) fn view_path(&self) -> CookedDrawPath {
        if self
            .meshes
            .first()
            .is_some_and(|mesh| !mesh.secondary.is_empty())
        {
            CookedDrawPath::Indirect
        } else {
            CookedDrawPath::Direct
        }
    }
}
