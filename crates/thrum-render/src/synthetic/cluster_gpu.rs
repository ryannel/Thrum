//! Selective portable GPU cluster construction for the V1 four-source path.

use super::RenderError;
use super::cluster::{X, Y, Z};
use crate::binding;
use crate::graph::CompiledPass;
use crate::records::{Bounds, ClusterHead};
use crate::timing::GpuTimings;

pub(super) struct ClusterCompute {
    pub(super) bounds: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
}

impl ClusterCompute {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        bounds: wgpu::Buffer,
        heads: &wgpu::Buffer,
        indices: &wgpu::Buffer,
    ) -> Self {
        let entries = [
            binding::storage(
                0,
                wgpu::ShaderStages::COMPUTE,
                true,
                size_of::<Bounds>() as u64,
            ),
            binding::storage(
                1,
                wgpu::ShaderStages::COMPUTE,
                false,
                size_of::<ClusterHead>() as u64,
            ),
            binding::storage(
                2,
                wgpu::ShaderStages::COMPUTE,
                false,
                size_of::<u32>() as u64,
            ),
        ];
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic cluster compute"),
            entries: &entries,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic cluster compute"),
            layout: &group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bounds.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: heads.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indices.as_entire_binding(),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic cluster compute"),
            bind_group_layouts: &[Some(&group_layout)],
            immediate_size: 0,
        });
        let shader = crate::shaders::Shader::Cluster.compile(device, "synthetic cluster compute");
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("synthetic cluster compute"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("build"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            bounds,
            pipeline,
            group,
        }
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_compute(encoder, generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        let cells = u32::try_from(X * Y * Z).expect("bounded cluster grid fits u32");
        pass.dispatch_workgroups(
            cells.div_ceil(crate::shader_constants::CLUSTER_WORKGROUP),
            1,
            1,
        );
        Ok(())
    }
}
