//! Prepare and record the float present pass with the display headroom uniform.
use crate::{
    binding,
    graph::{CompiledPass, GraphColorAttachment},
    present::SurfacePresenter,
    runtime::DeviceSite,
    synthetic::RenderError,
    timing::GpuTimings,
};
use wgpu::util::DeviceExt as _;

/// Prepared with the SDR stage so a display mode change needs no compilation.
#[derive(Clone)]
pub(crate) struct EdrTone {
    pipeline: wgpu::RenderPipeline,
    settings: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl EdrTone {
    pub(super) fn prepare(
        device: &DeviceSite,
        shader: &wgpu::ShaderModule,
        inputs: &wgpu::BindGroupLayout,
        vertex: &str,
        fragment: &str,
    ) -> Self {
        let label = Some("extended dynamic range present");
        let settings = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&[1.0_f32; 2]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let settings_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[binding::uniform(0, wgpu::ShaderStages::FRAGMENT, 8)],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: &settings_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: settings.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[Some(inputs), Some(&settings_layout)],
            immediate_size: 0,
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(vertex),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba16Float.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            settings,
            group,
        }
    }

    pub(crate) fn write_headroom(&self, queue: &wgpu::Queue, headroom: f32) {
        let nearest = half::f16::from_f32(headroom);
        let limit = if nearest.to_f32() > headroom {
            half::f16::from_bits(nearest.to_bits() - 1)
        } else {
            nearest
        };
        queue.write_buffer(
            &self.settings,
            0,
            bytemuck::cast_slice(&[headroom, limit.to_f32()]),
        );
    }

    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        compiled: &CompiledPass,
        input: (u64, &wgpu::BindGroup),
        output: GraphColorAttachment<'_>,
        timings: Option<&mut GpuTimings>,
        presenter: &SurfacePresenter,
    ) -> Result<(), RenderError> {
        if output.view.texture().format() != wgpu::TextureFormat::Rgba16Float {
            return Err(RenderError::TargetExtent);
        }
        let (generation, group) = input;
        let mut pass = compiled
            .begin_render(encoder, [Some(output)], None, generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.set_bind_group(1, &self.group, &[]);
        presenter.set_viewport(&mut pass);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
