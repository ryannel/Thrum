//! Capture the shipping coordinate/sampler path at fixed physical directions.

use wgpu::util::DeviceExt;

const SHADER: &str = concat!(
    concat!(
        include_str!("../../../../storage.wgsl"),
        "\n",
        concat!(
            include_str!("../../../../geometry.wgsl"),
            "\n",
            include_str!("../../../../common.wgsl")
        )
    ),
    r"
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var lookup: sampler;
@group(0) @binding(3) var<storage,read> directions: array<vec4<f32>>;
@group(0) @binding(4) var<storage,read_write> samples: array<vec4<f32>>;
@compute @workgroup_size(8)
fn sample_ray(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=arrayLength(&directions) { return; }
    let direction=normalize(directions[id.x].xyz);
    let size=vec2<f32>(textureDimensions(source));
    let uv=clamp(sky_uv(direction,atmosphere.sun_lux.xyz),0.5/size,1.0-0.5/size);
    samples[id.x]=textureSampleLevel(source,lookup,uv,0.0);
}
"
);

pub(super) struct Rays {
    pipeline: wgpu::ComputePipeline,
    directions: wgpu::Buffer,
    sampler: wgpu::Sampler,
    count: u32,
}

impl Rays {
    pub(super) fn new(device: &wgpu::Device, directions: &[[f32; 4]]) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shipping sky sampler observer"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("reference physical ray lookup"),
            layout: None,
            module: &shader,
            entry_point: Some("sample_ray"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let mut bytes = Vec::with_capacity(directions.len() * 16);
        for direction in directions {
            for value in direction {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        let count = u32::try_from(directions.len()).expect("small fixed ray grid");
        let directions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fixed reference ray directions"),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            directions,
            sampler,
            count,
        }
    }

    pub(super) fn observe(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uniform: &wgpu::Buffer,
        texture: &wgpu::Texture,
    ) -> wgpu::Buffer {
        let size = u64::from(self.count) * 16;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference sampled radiance"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference sampled readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reference shipping sampler inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.directions.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(self.count.div_ceil(8), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
        readback
    }
}
