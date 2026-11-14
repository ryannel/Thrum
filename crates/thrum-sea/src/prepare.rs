use crate::{Ambient, CACHE_SIZE, CHANNELS, CacheLevel, LEVELS, MAX_QUERIES, PIXELS, Profile};
use thrum_render::runtime::DeviceSite;

pub(super) const NAMES: [&str; 17] = [
    "sea parameters",
    "sea spectrum / final transform",
    "sea long components",
    "sea query points",
    "sea transform A",
    "sea transform B",
    "sea cascades",
    "sea moments base",
    "sea long field",
    "sea moments mip 1",
    "sea moments mip 2",
    "sea moments mip 3",
    "sea moments mip 4",
    "sea moments mip 5",
    "sea moments mip 6",
    "sea moments mip 7",
    "sea moments mip 8",
];

pub(super) fn upload_sizes() -> [u64; 4] {
    [
        176,
        3 * u64::from(PIXELS) * 32,
        192 * 32,
        u64::from(MAX_QUERIES) * 8,
    ]
}

pub(super) fn sizes() -> Vec<u64> {
    let work = 3 * CHANNELS * u64::from(PIXELS) * 8;
    let mut sizes = vec![
        176,
        work,
        192 * 32,
        u64::from(MAX_QUERIES) * 8,
        work,
        work,
        3 * u64::from(PIXELS) * 48,
        3 * u64::from(PIXELS) * 32,
        u64::from(MAX_QUERIES + LEVELS * CACHE_SIZE * CACHE_SIZE) * 48,
    ];
    for level in 1..=8 {
        sizes.push(3 * u64::from(PIXELS >> (2 * level)) * 32);
    }
    sizes
}

pub(super) fn usage(index: usize) -> wgpu::BufferUsages {
    if index == 0 {
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
    } else {
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
    }
}

pub(super) fn new(device: &DeviceSite, profile: Profile) -> Result<Ambient, String> {
    if profile.transform_size != 256 {
        return Err("sea transform_size: supported value is 256".into());
    }
    if profile.cascades != 3 {
        return Err("sea cascades: supported value is 3".into());
    }
    if profile.cache_levels != LEVELS {
        return Err("sea cache_levels: supported value is 8".into());
    }
    if !profile.choppiness.is_finite() || !(0.0..=1.0).contains(&profile.choppiness) {
        return Err("sea choppiness: expected finite 0..=1".into());
    }
    let limits = device.limits();
    for (size, name) in sizes().into_iter().zip(NAMES) {
        if size > limits.max_buffer_size {
            return Err(format!("{name}: max_buffer_size"));
        }
        if size > limits.max_storage_buffer_binding_size {
            return Err(format!("{name}: max_storage_buffer_binding_size"));
        }
    }
    if limits.max_storage_buffers_per_shader_stage < 8 {
        return Err("sea: max_storage_buffers_per_shader_stage requires 8".into());
    }
    if limits.max_compute_workgroup_storage_size < 2048 {
        return Err("sea FFT: max_compute_workgroup_storage_size requires 2048".into());
    }
    if limits.max_compute_invocations_per_workgroup < 256
        || limits.max_compute_workgroup_size_x < 256
    {
        return Err("sea FFT: max_compute_invocations_per_workgroup and max_compute_workgroup_size_x require 256".into());
    }
    let mut entries = Vec::new();
    for binding in 0..9 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 0 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage {
                        read_only: binding < 4,
                    }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sea"),
        entries: &entries,
    });
    let mip_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sea mip"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sea"),
        bind_group_layouts: &[Some(&layout), Some(&mip_layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sea ambient"),
        source: wgpu::ShaderSource::Wgsl(include_str!("ambient.wgsl").into()),
    });
    let pipelines = [
        "long_waves",
        "evolve",
        "rows",
        "columns",
        "unpack",
        "reduce",
    ]
    .into_iter()
    .map(|entry| {
        device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    })
    .collect();
    let buffers: Vec<_> = sizes()
        .into_iter()
        .zip(NAMES)
        .enumerate()
        .map(|(i, (size, name))| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(name),
                size,
                usage: usage(i),
                mapped_at_creation: false,
            })
        })
        .collect();
    let mut groups = Vec::new();
    // The uploaded mode allocation is reused for the final transform after evolve.
    // Each moment mip has one producer and its own physical allocation.
    for variant in 0..10 {
        let indices: [usize; 9] = if variant == 0 {
            [0, 1, 2, 3, 4, 5, 6, 7, 8]
        } else if variant == 1 {
            [0, 2, 2, 3, 1, 5, 6, 7, 8]
        } else {
            let child = variant + 7;
            let parent = if variant == 2 { 7 } else { child - 1 };
            [0, 1, 2, 3, 4, parent, 6, child, 8]
        };
        let entries: Vec<_> = indices
            .into_iter()
            .enumerate()
            .map(|(binding, index)| wgpu::BindGroupEntry {
                binding: u32::try_from(binding).expect("nine bindings"),
                resource: buffers[index].as_entire_binding(),
            })
            .collect();
        groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea"),
            layout: &layout,
            entries: &entries,
        }));
    }
    let mut mip_uniforms = Vec::new();
    let mut mip_groups = Vec::new();
    let mut parent = 0_u32;
    let mut child = PIXELS;
    let mut width = 128_u32;
    for _ in 0..8 {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sea mip parameters"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        let bytes: Vec<_> = [parent, child, width, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        buffer
            .slice(..)
            .get_mapped_range_mut()
            .map_err(|e| format!("sea mip mapping: {e}"))?
            .copy_from_slice(&bytes);
        buffer.unmap();
        mip_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea mip"),
            layout: &mip_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        }));
        mip_uniforms.push(buffer);
        parent = child;
        child += width * width;
        width /= 2;
    }
    let staging_size = upload_sizes().iter().sum();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sea upload staging"),
        size: staging_size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    Ok(Ambient {
        profile,
        buffers,
        pipelines,
        groups,
        mip_groups,
        mip_uniforms,
        staging,
        upload: vec![0; usize::try_from(staging_size).expect("bounded upload")],
        levels: [CacheLevel {
            origin: [0.0; 2],
            texel: 1.0,
            max_wavenumber: std::f64::consts::PI,
        }; 8],
    })
}
