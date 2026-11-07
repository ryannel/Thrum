//! The GPU pyramid and occlusion test against the CPU reference: every mip
//! equal for odd, non-power-of-two and degenerate extents, and every test
//! decision equal on screen-aligned and perspective boxes.

use super::*;
use crate::visibility::occlusion_reference as reference;

struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(clippy::cast_precision_loss, reason = "24 random bits fit f32 exactly")]
        let unit = (self.0 >> 40) as f32 / (1u64 << 24) as f32;
        unit
    }
}

fn device() -> (wgpu::Device, wgpu::Queue) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for the pyramid regression");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("pyramid device")
    })
}

fn bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn buffer_with(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pyramid test"),
        size: data.len().max(16) as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

/// A depth target holding exactly `depth`, written through `frag_depth`
/// because depth formats cannot be copy destinations on WebGPU.
pub(in crate::visibility) fn depth_target(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    depth: &[f32],
    extent: [u32; 2],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pyramid test depth"),
        size: wgpu::Extent3d {
            width: extent[0],
            height: extent[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::synthetic::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pyramid test depth"),
        source: wgpu::ShaderSource::Wgsl(
            "@group(0) @binding(0) var<storage, read> values: array<f32>;
            @group(0) @binding(1) var<storage, read> width: array<u32>;
            @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
                let p = vec2(f32((i << 1u) & 2u), f32(i & 2u));
                return vec4(p * 2.0 - 1.0, 0.5, 1.0);
            }
            @fragment fn fs(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {
                let x = u32(p.x);
                let y = u32(p.y);
                return values[y * width[0] + x];
            }"
            .into(),
        ),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pyramid test depth"),
        entries: &[storage_entry(0, true), storage_entry(1, true)],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pyramid test depth"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pyramid test depth"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: crate::synthetic::DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let values = buffer_with(device, queue, &bytes(depth), wgpu::BufferUsages::STORAGE);
    let width = buffer_with(
        device,
        queue,
        &extent[0].to_le_bytes(),
        wgpu::BufferUsages::STORAGE,
    );
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pyramid test depth"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: values.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: width.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pyramid test depth"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
    view
}

pub(in crate::visibility) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    depth: &wgpu::TextureView,
    extent: [u32; 2],
) -> PyramidTarget {
    let prepared = PyramidPrepared::new(device);
    let target = prepared.target(device, depth, extent);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        prepared.encode(&mut pass, &target);
    }
    queue.submit([encoder.finish()]);
    target
}

fn read_mip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    level: u32,
) -> (Vec<f32>, [u32; 2]) {
    let size = texture
        .size()
        .mip_level_size(level, wgpu::TextureDimension::D2);
    let row = (size.width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pyramid readback"),
        size: u64::from(row * size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: level,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let mapped = staging
        .slice(..)
        .get_mapped_range()
        .expect("mapped pyramid");
    let mut texels = Vec::with_capacity((size.width * size.height) as usize);
    for y in 0..size.height {
        let start = (y * row) as usize;
        let line = &mapped[start..start + (size.width * 4) as usize];
        texels.extend(
            line.as_chunks::<4>()
                .0
                .iter()
                .map(|word| f32::from_le_bytes(*word)),
        );
    }
    (texels, [size.width, size.height])
}

/// Depth in [0, 1]: a near wall over part of the view, far noise elsewhere.
fn depth_field(extent: [u32; 2], random: &mut Lcg) -> Vec<f32> {
    // The wall covers 40-50% of the width, so the test has work to hide.
    let low = 0.5 * random.unit();
    let high = low + 0.4 + 0.1 * random.unit();
    (0..extent[0] * extent[1])
        .map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "small test extents")]
            let x = (index % extent[0]) as f32 / extent[0] as f32;
            if (low..=high).contains(&x) {
                0.5 + 0.1 * random.unit()
            } else {
                0.01 * random.unit()
            }
        })
        .collect()
}

#[test]
fn every_gpu_mip_equals_the_reference() {
    let (device, queue) = device();
    for extent in [[1, 1], [3, 5], [7, 1], [64, 48], [1281, 719], [1280, 720]] {
        let mut random = Lcg(u64::from(extent[0]) * 977 + u64::from(extent[1]));
        let depth = depth_field(extent, &mut random);
        let view = depth_target(&device, &queue, &depth, extent);
        let target = build(&device, &queue, &view, extent);
        let expected = reference::build(&depth, extent);
        assert_eq!(
            target.texture().mip_level_count() as usize,
            expected.len(),
            "{extent:?}"
        );
        for (level, mip) in expected.iter().enumerate() {
            let (texels, size) = read_mip(
                &device,
                &queue,
                target.texture(),
                u32::try_from(level).expect("few"),
            );
            assert_eq!(size, mip.extent, "{extent:?} level {level}");
            let differing = texels
                .iter()
                .zip(&mip.texels)
                .filter(|(a, b)| a.to_bits() != b.to_bits())
                .count();
            assert_eq!(
                differing, 0,
                "{extent:?} level {level}: {differing} texels differ"
            );
        }
    }
}

/// The clip magnitude the probe gives box `index`, so the bound's terms are
/// exercised on both sides.
#[expect(clippy::cast_precision_loss, reason = "remainders below 5")]
fn magnitude(index: usize) -> [f32; 2] {
    [(index % 3) as f32 * 0.05, (index % 5) as f32 * 2.0]
}

/// Run the shared WGSL test on `boxes` against `target`, with `magnitude(index)`.
fn gpu_occluded(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &PyramidTarget,
    boxes: &[[[f32; 4]; 8]],
) -> Vec<bool> {
    let source = format!(
        "@group(0) @binding(0) var pyramid: texture_2d<f32>;
        @group(0) @binding(1) var<storage, read> boxes: array<array<vec4<f32>, 8>>;
        @group(0) @binding(2) var<storage, read_write> results: array<u32>;
        @compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {{
            if id.x >= arrayLength(&results) {{ return; }}
            let magnitude = vec2(f32(id.x % 3u) * 0.05, f32(id.x % 5u) * 2.0);
            results[id.x] = u32(occluded_by_pyramid(boxes[id.x], magnitude));
        }}
        {OCCLUSION_WGSL}"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("occlusion probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("occlusion probe"),
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let flat: Vec<f32> = boxes.iter().flatten().flatten().copied().collect();
    let input = buffer_with(device, queue, &bytes(&flat), wgpu::BufferUsages::STORAGE);
    let result_bytes = (boxes.len() * 4) as u64;
    let results = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("occlusion probe"),
        size: result_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let pyramid = target
        .texture()
        .create_view(&wgpu::TextureViewDescriptor::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("occlusion probe"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&pyramid),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: results.as_entire_binding(),
            },
        ],
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("occlusion probe readback"),
        size: result_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            u32::try_from(boxes.len()).expect("few boxes").div_ceil(64),
            1,
            1,
        );
    }
    encoder.copy_buffer_to_buffer(&results, 0, &staging, 0, result_bytes);
    queue.submit([encoder.finish()]);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let mapped = staging.slice(..).get_mapped_range().expect("mapped probe");
    mapped
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word) != 0)
        .collect()
}

#[test]
fn gpu_test_decisions_equal_the_reference() {
    let (device, queue) = device();
    for extent in [[7, 1], [64, 48], [1281, 719]] {
        let mut random = Lcg(u64::from(extent[0]) << 16 | u64::from(extent[1]));
        let depth = depth_field(extent, &mut random);
        let view = depth_target(&device, &queue, &depth, extent);
        let target = build(&device, &queue, &view, extent);
        let pyramid = reference::build(&depth, extent);
        // Screen-aligned boxes at w = 1, exact on both sides, plus perspective
        // corners with differing w; a few touch or cross the near plane.
        let mut boxes = Vec::new();
        for index in 0..6_000 {
            let x0 = random.unit() * 2.4 - 1.2;
            let y0 = random.unit() * 2.4 - 1.2;
            let (dx, dy) = (random.unit() * 0.6, random.unit() * 0.6);
            let z = random.unit() * 0.7;
            let mut corners = [[0.0; 4]; 8];
            for (corner_index, corner) in corners.iter_mut().enumerate() {
                let x = if corner_index & 1 == 0 { x0 } else { x0 + dx };
                let y = if corner_index & 2 == 0 { y0 } else { y0 + dy };
                let w = if index % 2 == 0 {
                    1.0
                } else {
                    0.5 + 4.0 * random.unit()
                };
                let near = index % 97 == 0 && corner_index == 5;
                *corner = [x * w, y * w, if near { w } else { z * w }, w];
            }
            boxes.push(corners);
        }
        let gpu = gpu_occluded(&device, &queue, &target, &boxes);
        let mut hidden = 0;
        let mut differing = Vec::new();
        for (index, corners) in boxes.iter().enumerate() {
            let cpu = reference::occluded(corners, &pyramid, magnitude(index));
            hidden += usize::from(cpu);
            if cpu != gpu[index] {
                differing.push(index);
            }
        }
        assert!(
            differing.is_empty(),
            "{extent:?}: decisions differ for boxes {differing:?}"
        );
        assert!(
            hidden > 400,
            "{extent:?}: the test must also hide work: {hidden}"
        );
    }
}
