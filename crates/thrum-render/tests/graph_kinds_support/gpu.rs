use super::{access, descriptor, device, pass};
use std::collections::BTreeSet;
use thrum_render::graph::{
    AccessKind as A, FrameGraph, GraphTextures, ResourceDescriptor, ResourceScope as S,
    TextureKind as K,
};

pub(crate) fn prove_kinds() {
    for generation in [41, 42] {
        let (device, queue, caps) = device(generation);
        for size in [4, 8] {
            for scope in [
                S::Transient,
                S::Persistent {
                    owner: 7,
                    epoch: generation,
                    initialized: false,
                },
                S::History {
                    owner: 8,
                    epoch: generation,
                    initialized: false,
                },
            ] {
                for kind in [K::D3 { depth: size }, K::Cube, K::D2] {
                    prove(&device, &queue, &caps, kind, size, scope);
                }
            }
        }
        device.destroy();
    }
}

fn prove(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &thrum_render::capabilities::DeviceCapabilities,
    kind: K,
    size: u32,
    scope: S,
) {
    let mut graph = FrameGraph::default();
    let desc = descriptor(kind, size);
    let ResourceDescriptor::Texture {
        layers,
        mips,
        format,
        usage,
        ..
    } = desc
    else {
        unreachable!()
    };
    let id = graph
        .resource(desc, scope)
        .expect("valid graph-kind fixture");
    for mip in 0..mips {
        let range = u64::from(mip * layers)..u64::from((mip + 1) * layers);
        let mut write = access(&id, range.clone(), A::StorageWrite);
        if kind == K::Cube {
            write.view = Some(wgpu::TextureViewDimension::D2Array);
        }
        graph.pass(pass("write every texel", vec![write]));
        graph.pass(pass(
            "read every texel",
            vec![access(&id, range, A::SampledRead)],
        ));
    }
    let compiled = graph
        .compile_with_capabilities(caps, &BTreeSet::new())
        .expect("valid graph-kind fixture");
    assert_eq!(compiled.device_generation, caps.generation());
    assert_eq!(compiled.pass_order, [0, 1, 2, 3, 4, 5]);
    let transients = GraphTextures::new(
        device,
        &compiled,
        caps.recipe().expect("valid graph-kind fixture"),
    );
    let retained;
    let texture = if scope == S::Transient {
        transients
            .texture(&id)
            .expect("real graph transient allocation")
    } else {
        assert!(
            compiled
                .lifetimes
                .iter()
                .all(|life| life.transient_slot.is_none())
        );
        retained = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("effect-owned persistent/history texture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: if matches!(kind, K::D3 { .. }) {
                    size
                } else {
                    layers
                },
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: if matches!(kind, K::D3 { .. }) {
                wgpu::TextureDimension::D3
            } else {
                wgpu::TextureDimension::D2
            },
            format,
            usage: usage - wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        &retained
    };
    for mip in 0..mips {
        prove_mip(device, queue, texture, kind, size, mip, layers);
    }
}

fn prove_mip(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    kind: K,
    size: u32,
    mip: u32,
    layers: u32,
) {
    let n = (size >> mip).max(1);
    let depth = if matches!(kind, K::D3 { .. }) {
        n
    } else {
        layers
    };
    let count = n * n * depth;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("consumer output"),
        size: u64::from(count) * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact readback"),
        size: output.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let array = !matches!(kind, K::D3 { .. });
    let (write, read) = shaders(kind, n, depth, mip);
    let write_pipeline = pipeline(device, &write, None);
    let mut layout_entries = vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: match kind {
                    K::Cube => wgpu::TextureViewDimension::Cube,
                    K::D2 => wgpu::TextureViewDimension::D2Array,
                    K::D3 { .. } => wgpu::TextureViewDimension::D3,
                },
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ];
    if kind == K::Cube {
        layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        });
    }
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &layout_entries,
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bindings)],
        immediate_size: 0,
    });
    let read_pipeline = pipeline(device, &read, Some(&layout));
    let view = |dimension| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(dimension),
            base_mip_level: mip,
            mip_level_count: Some(1),
            ..wgpu::TextureViewDescriptor::default()
        })
    };
    let write_view = view(if array {
        wgpu::TextureViewDimension::D2Array
    } else {
        wgpu::TextureViewDimension::D3
    });
    let read_view = view(match kind {
        K::Cube => wgpu::TextureViewDimension::Cube,
        K::D2 => wgpu::TextureViewDimension::D2Array,
        K::D3 { .. } => wgpu::TextureViewDimension::D3,
    });
    let writer = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &write_pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&write_view),
        }],
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let mut entries = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&read_view),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: output.as_entire_binding(),
        },
    ];
    if kind == K::Cube {
        entries.push(wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });
    }
    let reader = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &read_pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (pipeline, group) in [(&write_pipeline, &writer), (&read_pipeline, &reader)] {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.dispatch_workgroups(n, n, depth);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, output.size());
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("readback receiver");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("valid graph-kind fixture");
    receive
        .recv()
        .expect("valid graph-kind fixture")
        .expect("valid graph-kind fixture");
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("valid graph-kind fixture");
    for z in 0..depth {
        for y in 0..n {
            for x in 0..n {
                for channel in 0..4 {
                    let offset = usize::try_from((x + n * (y + n * z)) * 16 + channel * 4)
                        .expect("valid graph-kind fixture");
                    let actual = u32::from_le_bytes(
                        mapped[offset..offset + 4]
                            .try_into()
                            .expect("valid graph-kind fixture"),
                    );
                    let expected = x + 17 * y + 257 * z + 4096 * mip + 1 + channel;
                    assert_eq!(
                        actual, expected,
                        "{kind:?} mip {mip} ({x},{y},{z}) channel {channel}"
                    );
                }
            }
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    source: &str,
    layout: Option<&wgpu::PipelineLayout>,
) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("graph kind witness"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("graph kind witness"),
        layout,
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn shaders(kind: K, n: u32, depth: u32, mip: u32) -> (String, String) {
    let array = !matches!(kind, K::D3 { .. });
    let storage_type = if array {
        "texture_storage_2d_array<rgba32float, write>"
    } else {
        "texture_storage_3d<rgba32float, write>"
    };
    let store = if array {
        "textureStore(image_out, vec2<i32>(p.xy), i32(p.z), value);"
    } else {
        "textureStore(image_out, vec3<i32>(p), value);"
    };
    let sampled_type = match kind {
        K::Cube => "texture_cube<f32>",
        K::D2 => "texture_2d_array<f32>",
        K::D3 { .. } => "texture_3d<f32>",
    };
    let load = match kind {
        K::D3 { .. } => "textureLoad(source, vec3<i32>(p), 0)",
        K::D2 => "textureLoad(source, vec2<i32>(p.xy), i32(p.z), 0)",
        K::Cube => "textureSampleLevel(source, nearest, cube_direction(p), 0.0)",
    };
    let prelude = format!("const N: u32 = {n}u; const D: u32 = {depth}u; const M: u32 = {mip}u;\n");
    let write = format!(
        "{prelude}
        @group(0) @binding(0) var image_out: {storage_type};
        @compute @workgroup_size(1) fn main(@builtin(global_invocation_id) p: vec3<u32>) {{
            let v = p.x + 17u*p.y + 257u*p.z + 4096u*M + 1u;
            let value = vec4<f32>(f32(v), f32(v+1u), f32(v+2u), f32(v+3u)); {store}
        }}"
    );
    let cube = if kind == K::Cube {
        "@group(0) @binding(2) var nearest: sampler;
        fn cube_direction(p: vec3<u32>) -> vec3<f32> {
            let uv = (vec2<f32>(p.xy) + vec2<f32>(0.5)) / f32(N) * 2.0 - 1.0;
            switch p.z {
                case 0u: { return vec3<f32>(1.0, -uv.y, -uv.x); }
                case 1u: { return vec3<f32>(-1.0, -uv.y, uv.x); }
                case 2u: { return vec3<f32>(uv.x, 1.0, uv.y); }
                case 3u: { return vec3<f32>(uv.x, -1.0, -uv.y); }
                case 4u: { return vec3<f32>(uv.x, -uv.y, 1.0); }
                default: { return vec3<f32>(-uv.x, -uv.y, -1.0); }
            }
        }"
    } else {
        ""
    };
    let read = format!("{prelude}
        @group(0) @binding(0) var source: {sampled_type};
        @group(0) @binding(1) var<storage, read_write> result: array<vec4<u32>>;
        {cube}
        @compute @workgroup_size(1) fn main(@builtin(global_invocation_id) p: vec3<u32>) {{ result[p.x + N*(p.y + N*p.z)] = vec4<u32>({load}); }}");
    (write, read)
}
