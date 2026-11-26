//! Display a primary-ray inspection of the same frame's opaque geometry.

use super::super::{OUTPUT_FORMAT, RenderError, SceneContext};
use crate::binding;
use crate::graph::{CompiledPass, GraphColorAttachment};
use crate::runtime::DeviceSite;

pub(in crate::synthetic) struct TracedView {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl TracedView {
    pub(in crate::synthetic) fn new(device: &DeviceSite, view: &wgpu::BindGroupLayout) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("traced view structure layout"),
            entries: &[binding::acceleration_structure(
                0,
                wgpu::ShaderStages::FRAGMENT,
            )],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("traced view pipeline layout"),
            bind_group_layouts: &[Some(view), Some(&layout)],
            immediate_size: 0,
        });
        let shader = crate::shaders::Shader::RayView.compile(device, "traced geometry");
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("traced geometry"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OUTPUT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            layout: layout.into_raw(),
            pipeline,
        }
    }

    pub(in crate::synthetic) fn encode(
        &self,
        frame: &mut SceneContext<'_>,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        let prepared = frame.prepared;
        let mut state = prepared
            .ray_structure
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .state
            .lock()
            .map_err(|_| RenderError::GraphPassMismatch)?;
        let super::SceneInstances {
            tlas, traced_group, ..
        } = &mut *state;
        let group = traced_group.get_or_insert_with(|| {
            prepared
                .capabilities
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("traced view structure"),
                    layout: &self.layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: tlas.as_binding(),
                    }],
                })
        });
        let target = frame.target;
        let mut pass = compiled
            .begin_render(
                frame.encoder,
                [Some(GraphColorAttachment {
                    resource: &target.ids.output,
                    range: 0..1,
                    view: &target.view,
                    clear: wgpu::Color::BLACK,
                })],
                None,
                frame.generation,
                frame.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        let offset = prepared.view_stride * u32::from(target.frame_revision & 1 != 0);
        let offsets = [offset, 0];
        let count = 1 + usize::from(prepared.bucket_route.is_some());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &target.view_bind_group, &offsets[..count]);
        pass.set_bind_group(1, &*group, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
