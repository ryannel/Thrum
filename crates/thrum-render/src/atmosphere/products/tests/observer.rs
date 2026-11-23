//! Independent consumer pipeline. Only the lent API appears in its entry point.
use crate::atmosphere::EnvironmentProducts;
use wgpu::util::DeviceExt;

pub(super) fn sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    products: &EnvironmentProducts,
    expression: &str,
    queries: &[[f32; 4]],
    side: usize,
) -> Vec<[f32; 4]> {
    Probe::new(device, products, "", [expression])
        .sample(device, queue, products, queries, side)
        .into_iter()
        .map(|[record]| record)
        .collect()
}

pub(super) struct Probe<const N: usize> {
    pipeline: wgpu::ComputePipeline,
    query_layout: wgpu::BindGroupLayout,
}

impl<const N: usize> Probe<N> {
    pub(super) fn new(
        device: &wgpu::Device,
        products: &EnvironmentProducts,
        setup: &str,
        expressions: [&str; N],
    ) -> Self {
        use std::fmt::Write as _;

        let mut body = setup.to_owned();
        for (i, expression) in expressions.iter().enumerate() {
            write!(body, "\n    results[id.x * {N}u + {i}u] = {expression};").unwrap();
        }
        let source = format!(
            "{}
@group(1) @binding(0) var<storage,read> queries: array<vec4<f32>>;
@group(1) @binding(1) var<storage,read_write> results: array<vec4<f32>>;
@compute @workgroup_size(64)
fn query(@builtin(global_invocation_id) id: vec3<u32>) {{
    if id.x >= arrayLength(&queries) {{ return; }}
    let q = queries[id.x];
    {body}
}}",
            products.shader_source,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("independent lent environment observer"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let query_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[entry(0, true), entry(1, false)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&products.layout), Some(&query_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("query"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            pipeline,
            query_layout,
        }
    }

    pub(super) fn sample(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        products: &EnvironmentProducts,
        queries: &[[f32; 4]],
        side: usize,
    ) -> Vec<[[f32; 4]; N]> {
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(queries),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: input.size() * N as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.query_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                products.group(side).expect("selected exposure side"),
                &[],
            );
            pass.set_bind_group(1, &group, &[]);
            pass.dispatch_workgroups(
                u32::try_from(queries.len())
                    .expect("bounded query count")
                    .div_ceil(64),
                1,
                1,
            );
        }
        let readback =
            thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, ..);
        let submission = queue.submit([encoder.finish()]);
        readback
            .wait(device, submission)
            .chunks_exact(N * 16)
            .map(|record| {
                std::array::from_fn(|i| {
                    std::array::from_fn(|c| {
                        let offset = i * 16 + c * 4;
                        f32::from_le_bytes(
                            record[offset..offset + 4].try_into().expect("one f32 word"),
                        )
                    })
                })
            })
            .collect()
    }
}

fn entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(super) fn texels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Vec<[f64; 4]> {
    let mapped = thrum_test_support::readback::texture(
        device,
        queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    decode_texels(&mapped).collect()
}

pub(super) fn decode_texels(mapped: &[u8]) -> impl Iterator<Item = [f64; 4]> + '_ {
    mapped.as_chunks::<8>().0.iter().map(|pixel| {
        std::array::from_fn(|channel| {
            f64::from(
                half::f16::from_bits(u16::from_le_bytes([
                    pixel[channel * 2],
                    pixel[channel * 2 + 1],
                ]))
                .to_f32(),
            )
        })
    })
}

pub(super) fn write_field(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    value: impl Fn(u32, u32, u32) -> [f32; 4],
) {
    let mut bytes = Vec::new();
    for layer in 0..texture.depth_or_array_layers() {
        for row in 0..texture.height() {
            for column in 0..texture.width() {
                for component in value(column, row, layer) {
                    bytes
                        .extend_from_slice(&half::f16::from_f32(component).to_bits().to_le_bytes());
                }
            }
        }
    }
    queue.write_texture(
        texture.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width() * 8),
            rows_per_image: Some(texture.height()),
        },
        texture.size(),
    );
}
