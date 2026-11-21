//! Test-only stage observer; never selected in a shipping build.
use super::Layer;
use thrum_render::binding;

pub(crate) struct Probe {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    pub(crate) output: wgpu::Buffer,
}

impl Probe {
    pub(crate) fn new(layer: &Layer, device: &thrum_render::runtime::DeviceSite) -> Self {
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud lighting stage observation"),
            size: (6152 + 16384) * 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 0)],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud lighting stage observer"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    super::prepare::trace_shader(&layer.environment),
                    include_str!("diagnostic.wgsl")
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(&layer.environment.layout),
                Some(&layer.trace.get_bind_group_layout(1)),
                Some(&layer.trace.get_bind_group_layout(2)),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("cloud lighting stage observer"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("observe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            pipeline,
            group,
            output,
        }
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(3, &self.group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
