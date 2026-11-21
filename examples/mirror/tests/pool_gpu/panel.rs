use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SecondarySceneSurface,
};
use wgpu::util::DeviceExt;

/// An opaque, perfectly nonreflective swatch with a known radiance. Its
/// ordinary draw uses the main aerial decoder with no scene-local lighting.
pub(super) struct PanelDraw {
    panel: super::Panel,
    main: Option<wgpu::RenderPipeline>,
    view: Option<wgpu::RenderPipeline>,
    group: Option<wgpu::BindGroup>,
}

impl PanelDraw {
    pub(super) fn new(panel: super::Panel) -> Self {
        Self {
            panel,
            main: None,
            view: None,
            group: None,
        }
    }

    fn record(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: Option<&wgpu::RenderPipeline>) {
        pass.set_pipeline(pipeline.expect("panel pipeline"));
        pass.set_bind_group(2, self.group.as_ref().expect("panel group"), &[]);
        pass.draw(0..6, 0..1);
    }
}

impl SceneContribution for PanelDraw {
    fn secondary_views_enabled(&self) -> bool {
        true
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiance swatch"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let [x, y, z] = self.panel.center.to_array();
        let [a, b, c] = self.panel.half.to_array();
        let record = [x, y, z, self.panel.radiance, a, b, c, 0.0];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiance swatch"),
            contents: bytemuck::cast_slice(&record),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        self.group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiance swatch"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        }));
        self.main = Some(make_pipeline(surface, None, &layout));
        self.view = surface
            .secondary_surfaces
            .first()
            .map(|view| make_pipeline(surface, Some(view), &layout));
        Ok(())
    }
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.record(pass, self.main.as_ref());
    }
    fn draw_secondary(&self, _: usize, pass: &mut wgpu::RenderPass<'_>) {
        self.record(pass, self.view.as_ref());
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

fn make_pipeline(
    surface: &SceneSurface<'_>,
    view: Option<&SecondarySceneSurface<'_>>,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let source = view.map_or(surface.shader_source, |view| view.shader_source);
    let source = format!(
        "{source}\n{}\n{}",
        include_str!("panel.wgsl"),
        if view.is_some() {
            "@fragment fn panel_fragment(input: PanelVertex) -> SecondaryOpaqueOutput {
            if dot(secondary_view.clip_plane, vec4(input.world, 1.0)) < 0.0 { discard; }
            return SecondaryOpaqueOutput(panel_colour(input), 0xffffffffu);
        }"
        } else {
            "@fragment fn panel_fragment(input: PanelVertex) -> FragmentOutput {
            return FragmentOutput(panel_colour(input), vec2(65504.0), 0.0);
        }"
        }
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("radiance swatch"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("radiance swatch"),
        bind_group_layouts: &[
            Some(view.map_or(surface.view_layout, |view| view.view_layout)),
            Some(view.map_or(surface.light_layout, |view| view.light_layout)),
            Some(layout),
        ],
        immediate_size: 0,
    });
    let targets = view
        .map_or(surface.color_formats.as_slice(), |view| {
            std::slice::from_ref(&view.colour_format)
        })
        .iter()
        .map(|&format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        })
        .collect::<Vec<_>>();
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some("radiance swatch"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("panel_vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: surface.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("panel_fragment"),
            targets: &targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
