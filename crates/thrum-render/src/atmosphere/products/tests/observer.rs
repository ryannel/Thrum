//! Independent consumer pipeline. Only the lent API appears in its entry point.
use crate::atmosphere::EnvironmentProducts;
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

pub(super) fn sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    products: &EnvironmentProducts,
    expression: &str,
    queries: &[[f32; 4]],
    side: usize,
) -> Vec<[f32; 4]> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("independent lent environment observer"),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n@group(1) @binding(0) var<storage,read> queries: array<vec4<f32>>;\n@group(1) @binding(1) var<storage,read_write> results: array<vec4<f32>>;\n@compute @workgroup_size(64)\nfn query(@builtin(global_invocation_id) id: vec3<u32>) {{ if id.x>=arrayLength(&queries) {{ return; }} let q=queries[id.x]; results[id.x]={expression}; }}", products.shader_source).into()),
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
    let bytes: Vec<_> = queries
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let length = u64::try_from(bytes.len()).expect("bounded query bytes");
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: length,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: length,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &query_layout,
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
        pass.set_pipeline(&pipeline);
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
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, length);
    collect(device, queue, encoder, &readback)
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

pub(super) fn collect(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    readback: &wgpu::Buffer,
) -> Vec<[f32; 4]> {
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("probe receiver remains alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("probe GPU work completes");
    receive.recv().expect("map callback").expect("probe mapped");
    readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped probe output")
        .as_chunks::<16>()
        .0
        .iter()
        .map(|record| {
            std::array::from_fn(|i| {
                f32::from_le_bytes(record[i * 4..i * 4 + 4].try_into().expect("one f32 word"))
            })
        })
        .collect()
}

pub(super) fn texels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Vec<[f64; 4]> {
    let width = texture.width();
    let height = texture.height();
    let layers = texture.depth_or_array_layers();
    let stride = (width * 8).div_ceil(256) * 256;
    let length = u64::from(stride) * u64::from(height) * u64::from(layers);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("independent stored-texel readback"),
        size: length,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("texel receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })
        .expect("stored texel copy completes");
    receive
        .recv()
        .expect("texel map callback")
        .expect("texels mapped");
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped stored texels");
    let mut result = Vec::with_capacity((width * height * layers) as usize);
    for row in mapped.chunks_exact(stride as usize) {
        for pixel in row[..width as usize * 8].as_chunks::<8>().0 {
            result.push(std::array::from_fn(|channel| {
                f64::from(
                    half::f16::from_bits(u16::from_le_bytes([
                        pixel[channel * 2],
                        pixel[channel * 2 + 1],
                    ]))
                    .to_f32(),
                )
            }));
        }
    }
    result
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
