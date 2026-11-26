use thrum_test_support::Scratch;
use wgpu::hal::metal::capture_stage_translations;
use wgpu::util::DeviceExt as _;

use super::{
    metal::{cooked_module, dispatch_read, pipeline, storage},
    metallib_cook::cook,
};

fn bind(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty,
        count: None,
    }
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    bind(
        binding,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
    )
}

fn buffer(device: &wgpu::Device, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data,
        usage,
    })
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn pair(
    device: &wgpu::Device,
    bindings: &wgpu::BindGroupLayout,
    source: &str,
    entry_point: &str,
    sized: bool,
) -> [wgpu::ComputePipeline; 2] {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(bindings)],
        immediate_size: 0,
    });
    let (_, source) = thrum_render::composed_sources()
        .into_iter()
        .find(|(name, _)| *name == source)
        .expect("the route's own composed shader");
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry_point),
        source: wgpu::ShaderSource::Wgsl(source),
    });
    let (reference, stages) =
        capture_stage_translations(|| pipeline(device, &layout, &module, entry_point));
    let [stage] = stages.as_slice() else {
        panic!("expected one route stage for {entry_point}");
    };
    let record = &stage.record;
    assert!(!record.workgroup_memory_sizes.is_empty(), "{entry_point}");
    if sized {
        assert!(!record.sized_bindings.is_empty(), "{entry_point}");
    }
    let blob =
        cook(stage, &Scratch::new("metallib-route-stage")).expect("cook the real route stage");
    let module = cooked_module(device, &blob, &record.function, record.workgroup_size);
    let cooked = pipeline(device, &layout, &module, &record.function);
    [reference, cooked]
}

pub(super) fn visibility(device: &wgpu::Device, queue: &wgpu::Queue) {
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("visibility offsets layouts"),
        entries: &[
            storage(3, true),
            uniform(4),
            storage(5, false),
            storage(7, false),
        ],
    });
    let [reference, cooked] = pair(device, &bindings, "Visibility", "offsets", true);
    let mesh = buffer(
        device,
        bytemuck::cast_slice(&[12_u32, 42, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        wgpu::BufferUsages::STORAGE,
    );
    let params = buffer(
        device,
        bytemuck::cast_slice(&[0_u32, 3, 0, 0]),
        wgpu::BufferUsages::UNIFORM,
    );
    let dispatch = |pipeline| {
        let counts = buffer(
            device,
            bytemuck::cast_slice(&[2_u32, 3, 5]),
            wgpu::BufferUsages::STORAGE,
        );
        let args = buffer(
            device,
            &[0; 20],
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bindings,
            entries: &[
                entry(3, &mesh),
                entry(4, &params),
                entry(5, &counts),
                entry(7, &args),
            ],
        });
        dispatch_read(device, queue, &group, pipeline, &args)
    };
    let expected = dispatch(&reference);
    assert_eq!(expected, [42, 10, 12, 3, 0]);
    assert_eq!(dispatch(&cooked), expected);
}

pub(super) fn diffuse(device: &wgpu::Device, queue: &wgpu::Queue) {
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        entries: &[
            uniform(0),
            bind(
                1,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            bind(
                2,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ),
            storage(3, false),
        ],
    });
    let [reference, cooked] = pair(
        device,
        &bindings,
        "AtmosphereDiffuse",
        "diffuse_main",
        false,
    );
    let mut observer = [0.0_f32; 32];
    let half_pi = std::f32::consts::FRAC_PI_2;
    observer[20..].copy_from_slice(&[
        0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, half_pi, half_pi,
    ]);
    let observer = buffer(
        device,
        bytemuck::cast_slice(&observer),
        wgpu::BufferUsages::UNIFORM,
    );
    let sky = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("constant radiance sky"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        sky.as_image_copy(),
        &[64, 32, 16, 255],
        wgpu::TexelCopyBufferLayout::default(),
        sky.size(),
    );
    let sky = sky.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let dispatch = |pipeline| {
        let output = buffer(
            device,
            &[0; 160],
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bindings,
            entries: &[
                entry(0, &observer),
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sky),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                entry(3, &output),
            ],
        });
        dispatch_read(device, queue, &group, pipeline, &output)
    };
    let expected = dispatch(&reference);
    assert!(f32::from_bits(expected[0]) > f32::from_bits(expected[1]));
    assert!(f32::from_bits(expected[1]) > f32::from_bits(expected[2]));
    assert!(f32::from_bits(expected[2]) > 0.0);
    assert_eq!(dispatch(&cooked), expected);
}
