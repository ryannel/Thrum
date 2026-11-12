//! Ground-plane witness, drawn in the ordinary opaque contribution slot.
use thrum_render::contribution::SceneSurface;

pub(super) struct Visual {
    pipeline: wgpu::RenderPipeline,
    groups: [wgpu::BindGroup; 2],
}
impl Visual {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        params: &wgpu::Buffer,
        trail: &[wgpu::Texture],
    ) -> Self {
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("effect ground trail"),
                source: wgpu::ShaderSource::Wgsl(include_str!("visual.wgsl").into()),
            });
        let pipeline = surface.device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("effect ground trail"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &surface.color_formats.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let groups = std::array::from_fn(|side| {
            surface
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("effect ground side"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: params.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &trail[side].create_view(&wgpu::TextureViewDescriptor::default()),
                            ),
                        },
                    ],
                })
        });
        Self { pipeline, groups }
    }
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, side: usize) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[side], &[]);
        pass.draw(0..6, 0..1);
    }
}
