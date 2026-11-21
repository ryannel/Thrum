use std::sync::{Arc, Mutex};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, SceneContribution, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

pub(super) const BYTES: u64 = 256;
pub(super) type Readback = Arc<Mutex<Option<wgpu::Buffer>>>;

pub(super) struct Air {
    low_layer: u32,
    output: Readback,
    prepared: Option<(EnvironmentProducts, wgpu::ComputePipeline, wgpu::BindGroup)>,
}

impl Air {
    pub(super) fn new(low_layer: u32, output: Readback) -> Self {
        Self {
            low_layer,
            output,
            prepared: None,
        }
    }
}

fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: "stored aerial rounding".into(),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::EveryFrame,
        accesses: vec![EffectAccess {
            resource: 0,
            range: 0..BYTES,
            kind: AccessKind::StorageWrite,
            history: None,
            view: None,
        }],
        depth: false,
        write_depth: false,
        refraction: false,
    }
}

impl SceneContribution for Air {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: BYTES,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: false,
                },
            )],
            after_opaque: vec![declaration()],
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device;
        let environment = surface.environment_products().expect("atmosphere");
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aerial rounding witness"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let capture = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aerial rounding witness"),
            size: BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aerial rounding witness"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: capture.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aerial rounding witness"),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&declaration().input_layout(device)),
                Some(&environment.layout),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let source = format!(
            r"
{}
@group(3) @binding(0) var<storage, read_write> values: array<vec4<f32>>;
@compute @workgroup_size(8) fn read_air(@builtin(local_invocation_index) index: u32) {{
    let size = vec2<i32>(textureDimensions(aerial_scatter));
    let texel = size / 2 - vec2(1) + vec2<i32>(i32(index & 1u), i32((index >> 1u) & 1u));
    let layer = {} + i32(index >> 2u);
    values[index * 2u] = textureLoad(aerial_scatter, texel, layer, 0);
    values[index * 2u + 1u] = textureLoad(aerial_transmittance, texel, layer, 0);
}}",
            environment.shader_source.replace("@group(0)", "@group(2)"),
            self.low_layer
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aerial rounding witness"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("aerial rounding witness"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("read_air"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        self.prepared = Some((environment, pipeline, output));
        *self.output.lock().expect("witness") = Some(capture);
        Ok(())
    }
    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        let (environment, pipeline, output) = self.prepared.as_ref().expect("witness");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(
            2,
            environment
                .group(frame.exposure_side)
                .expect("exposure side"),
            &[],
        );
        pass.set_bind_group(3, output, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

pub(super) async fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
) -> [[f32; 4]; 2] {
    let data = super::retire::copy_buffer(device, queue, source).await;
    let mut max = [[0.0_f32; 4]; 2];
    for (i, value) in bytemuck::cast_slice::<_, [f32; 4]>(&data)
        .iter()
        .enumerate()
    {
        for c in 0..4 {
            max[i % 2][c] = max[i % 2][c].max(super::step(value[c]));
        }
    }
    max
}
