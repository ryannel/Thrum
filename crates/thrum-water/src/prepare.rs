use super::{Profile, SURFACE_PASS, records::Parameters};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{AfterOpaqueKind, AfterOpaquePass, EffectCadence, EffectGraph, SceneSurface},
};
use wgpu::util::DeviceExt;

pub(super) struct Prepared {
    pub(super) ambient: thrum_sea::Ambient,
    pub(super) environment: EnvironmentProducts,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) parameters: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pub(super) mesh: wgpu::Buffer,
    pub(super) vertices: u32,
    pub(super) queue: wgpu::Queue,
}
fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: SURFACE_PASS.into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: true,
        refraction: false,
    }
}
impl Prepared {
    pub(super) fn graph(&self) -> EffectGraph {
        let mut graph = self.ambient.graph();
        let mut surface = declaration();
        surface.accesses = self
            .ambient
            .surface_accesses(&[4, 8, 0])
            .expect("prepared water products");
        graph.opaque_reads.clear();
        graph.after_opaque.push(surface);
        graph
    }
}
pub(super) fn new(surface: &SceneSurface<'_>, profile: Profile) -> Result<Prepared, String> {
    let device = surface.device;
    let ambient = thrum_sea::Ambient::new(device, profile.waves)?;
    let environment = surface
        .environment_products()
        .ok_or("water needs the scene's physical environment")?;
    let parameters = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("water parameters"),
        size: size_of::<Parameters>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let entries: Vec<_> = (0..6)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 0 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage { read_only: true }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        })
        .collect();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("water field"),
        entries: &entries,
    });
    let buffers = [
        &parameters,
        ambient.cascade_buffer(),
        ambient.long_buffer(),
        ambient.moments_buffer(4).ok_or("water moments")?,
        ambient.moments_buffer(8).ok_or("water moments")?,
        ambient.moments_buffer(0).ok_or("water moments")?,
    ];
    let bindings: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(i, b)| {
            Ok(wgpu::BindGroupEntry {
                binding: u32::try_from(i).map_err(|e| e.to_string())?,
                resource: b.as_entire_binding(),
            })
        })
        .collect::<Result<_, String>>()?;
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("water field"),
        layout: &layout,
        entries: &bindings,
    });
    let inputs = declaration().input_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(SURFACE_PASS),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(&inputs),
            Some(&environment.layout),
            Some(&layout),
        ],
        immediate_size: 0,
    });
    let source = format!(
        "{}\n{}",
        environment.shader_source.replace("@group(0)", "@group(2)"),
        include_str!("surface.wgsl")
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(SURFACE_PASS),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let targets: Vec<_> = surface
        .color_formats
        .iter()
        .map(|f| {
            Some(wgpu::ColorTargetState {
                format: *f,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        })
        .collect();
    let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(SURFACE_PASS),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("water_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
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
            module: &shader,
            entry_point: Some("water_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    });
    let vertices = mesh();
    let count = u32::try_from(vertices.len()).map_err(|e| e.to_string())?;
    let mesh = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("water rings and horizon skirt"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Ok(Prepared {
        ambient,
        environment,
        pipeline,
        parameters,
        group,
        mesh,
        vertices: count,
        queue: surface.queue.clone(),
    })
}
fn mesh() -> Vec<[f32; 4]> {
    let mut vertices = Vec::new();
    // Nested world-snapped square rings. Every outer boundary has the next ring's
    // even vertices; boundary heights are interpolated at the shared coarse edge.
    for level in 0_i32..17 {
        let spacing = 0.25 * 2.0_f32.powi(level);
        for z in -32_i16..32 {
            for x in -32_i16..32 {
                if level > 0 && (-16..16).contains(&x) && (-16..16).contains(&z) {
                    continue;
                }
                for [dx, dz] in [[0, 0], [0, 1], [1, 0], [1, 0], [0, 1], [1, 1]] {
                    let px = f32::from(x + dx);
                    let pz = f32::from(z + dz);
                    vertices.push([px * spacing, pz * spacing, spacing, 0.0]);
                }
            }
        }
    }
    vertices
}
