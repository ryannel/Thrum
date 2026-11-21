use thrum_render::binding;
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SecondarySceneSurface,
};

const BEACON_Z: f32 = -16.9;
const BEACON_Y: f32 = super::bank_height(BEACON_Z) + 0.74;

#[derive(Default)]
pub(super) struct Mover {
    main: Option<wgpu::RenderPipeline>,
    views: Vec<wgpu::RenderPipeline>,
    group: Option<(wgpu::BindGroup, wgpu::Buffer)>,
    queue: Option<wgpu::Queue>,
}

impl SceneContribution for Mover {
    fn secondary_views_enabled(&self) -> bool {
        true
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("moving beacon"),
            entries: &[binding::uniform(0, wgpu::ShaderStages::VERTEX, 0)],
        });
        let marker = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("moving beacon"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("moving beacon"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: marker.as_entire_binding(),
            }],
        });
        let main = pipeline(surface, None, &layout);
        self.views = surface
            .secondary_surfaces
            .iter()
            .map(|view| pipeline(surface, Some(view), &layout))
            .collect();
        self.main = Some(main);
        self.group = Some((group, marker));
        self.queue = Some(surface.queue.clone());
        Ok(())
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        if let (Some(queue), Some((_, placement))) = (&self.queue, &self.group) {
            let seconds = (frame.time.tick as f32 - 1.0 + frame.time.display_phase).max(0.0)
                * frame.time.tick_duration.as_secs_f32();
            let x = 12.0 * (seconds * 0.67).sin();
            queue.write_buffer(
                placement,
                0,
                bytemuck::cast_slice(&[x, BEACON_Y, BEACON_Z, 0.0]),
            );
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.record(pass, self.main.as_ref());
    }

    fn draw_secondary(&self, index: usize, pass: &mut wgpu::RenderPass<'_>) {
        self.record(pass, self.views.get(index));
    }
}

impl Mover {
    fn record(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: Option<&wgpu::RenderPipeline>) {
        if let (Some(pipeline), Some((group, _))) = (pipeline, &self.group) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(2, group, &[]);
            pass.draw(0..36, 0..1);
        }
    }
}

fn pipeline(
    surface: &SceneSurface<'_>,
    view: Option<&SecondarySceneSurface<'_>>,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let source = view.map_or(surface.shader_source, |view| view.shader_source);
    let source = format!(
        "{source}\n{}\n{}",
        include_str!("mover.wgsl"),
        if view.is_some() {
            " @fragment fn beacon_fragment(input: BeaconVertex) -> SecondaryOpaqueOutput {
            if secondary_view.options.x > 0.0 && dot(secondary_view.clip_plane, vec4(input.world, 1.0)) < 0.0 { discard; }
            return SecondaryOpaqueOutput(beacon_colour(input), 0xffffffffu);
        }"
        } else {
            " @fragment fn beacon_fragment(input: BeaconVertex) -> FragmentOutput {
            return FragmentOutput(beacon_colour(input), vec2(65504.0), 1.0);
        }"
        }
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("moving beacon"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("moving beacon"),
        bind_group_layouts: &[
            Some(view.map_or(surface.view_layout, |view| view.view_layout)),
            Some(view.map_or(surface.light_layout, |view| view.light_layout)),
            Some(layout),
        ],
        immediate_size: 0,
    });
    let colors = view.map_or(surface.color_formats.as_slice(), |view| {
        std::slice::from_ref(&view.colour_format)
    });
    let targets = colors
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
        label: Some("moving beacon"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("beacon_vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
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
            entry_point: Some("beacon_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}
