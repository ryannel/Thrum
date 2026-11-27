use crate::AmbientError;
use crate::{Ambient, CacheLevel, LEVELS, Profile, TRANSFORM_SIZE, spec};
use spec::BufferKey;
use std::collections::BTreeMap;
use thrum_render::binding;
use thrum_render::runtime::DeviceSite;

pub(super) struct PreparedPass {
    pub pipeline: wgpu::ComputePipeline,
    pub groups: BTreeMap<spec::GroupKey, wgpu::BindGroup>,
}

pub(super) fn sizes() -> Vec<u64> {
    spec::resources().iter().map(|r| r.size).collect()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "round unit roots to the GPU's f32 format"
)]
fn transform_twiddles() -> [[f32; 2]; TRANSFORM_SIZE as usize / 2] {
    std::array::from_fn(|index| {
        let angle = std::f64::consts::TAU * index as f64 / f64::from(TRANSFORM_SIZE);
        let (sin, cos) = angle.sin_cos();
        [cos as f32, sin as f32]
    })
}

fn prepare_buffers(
    device: &DeviceSite,
    resources: Vec<spec::Resource>,
) -> Result<BTreeMap<BufferKey, wgpu::Buffer>, AmbientError> {
    let mut buffers = BTreeMap::new();
    for resource in resources {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&resource.name),
            size: resource.size,
            usage: resource.usage,
            mapped_at_creation: resource.key == BufferKey::Twiddles,
        });
        if resource.key == BufferKey::Twiddles {
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .map_err(AmbientError::Mapping)?
                .copy_from_slice(bytemuck::cast_slice(&transform_twiddles()));
            buffer.unmap();
        }
        buffers.insert(resource.key, buffer);
    }
    Ok(buffers)
}

pub(super) fn new(
    device: &DeviceSite,
    profile: Profile,
    rain: bool,
    flow: Option<(&wgpu::Queue, &thrum_water_model::FlowBands)>,
) -> Result<Ambient, AmbientError> {
    if profile.transform_size != TRANSFORM_SIZE {
        return Err(AmbientError::UnsupportedProfile(
            crate::ProfileField::TransformSize,
        ));
    }
    if profile.cascades != spec::CASCADES {
        return Err(AmbientError::UnsupportedProfile(
            crate::ProfileField::Cascades,
        ));
    }
    if profile.cache_levels != LEVELS {
        return Err(AmbientError::UnsupportedProfile(
            crate::ProfileField::CacheLevels,
        ));
    }
    if !profile.choppiness.is_finite() || !(0.0..=4.0).contains(&profile.choppiness) {
        return Err(AmbientError::UnsupportedProfile(
            crate::ProfileField::Choppiness,
        ));
    }
    let limits = device.limits();
    if (rain && limits.max_texture_dimension_2d < crate::rain::SIZE)
        || ((rain || flow.is_some())
            && limits.max_texture_array_layers
                < crate::flow::first_layer(rain)
                    + if flow.is_some() {
                        crate::flow::LAYERS
                    } else {
                        0
                    })
    {
        return Err(AmbientError::DeviceLimit {
            resource: "shared wave layer array".into(),
            limit: "cascade texture extent or array layers",
        });
    }
    let resources = spec::resources();
    for resource in &resources {
        if resource.size > limits.max_buffer_size {
            return Err(AmbientError::DeviceLimit {
                resource: resource.name.clone(),
                limit: "max_buffer_size",
            });
        }
        if resource.usage.contains(wgpu::BufferUsages::STORAGE)
            && resource.size > limits.max_storage_buffer_binding_size
        {
            return Err(AmbientError::DeviceLimit {
                resource: resource.name.clone(),
                limit: "max_storage_buffer_binding_size",
            });
        }
    }
    // Keep SEA's admitted portable envelope, though each pass now binds only what it uses.
    if limits.max_storage_buffers_per_shader_stage < 8 {
        return Err(AmbientError::DeviceLimit {
            resource: "FFT".into(),
            limit: "max_storage_buffers_per_shader_stage",
        });
    }
    if limits.max_compute_workgroup_storage_size < 2048 {
        return Err(AmbientError::DeviceLimit {
            resource: "FFT".into(),
            limit: "max_compute_workgroup_storage_size",
        });
    }
    if limits.max_compute_invocations_per_workgroup < TRANSFORM_SIZE
        || limits.max_compute_workgroup_size_x < TRANSFORM_SIZE
    {
        return Err(AmbientError::DeviceLimit {
            resource: "FFT".into(),
            limit: "max_compute_workgroup_size",
        });
    }
    let buffers = prepare_buffers(device, resources)?;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sea ambient"),
        source: wgpu::ShaderSource::Wgsl(crate::SHADER.into()),
    });
    let specs = spec::passes();
    let mut passes = BTreeMap::new();
    let mut pipelines = BTreeMap::<spec::PipelineKey, wgpu::ComputePipeline>::new();
    for pass in &specs {
        let mut layouts = Vec::new();
        let mut groups = BTreeMap::new();
        for group in spec::GroupKey::ALL
            .into_iter()
            .filter(|group| pass.bindings.iter().any(|b| b.group == *group))
        {
            let bindings: Vec<_> = pass.bindings.iter().filter(|b| b.group == group).collect();
            let entries: Vec<_> = bindings
                .iter()
                .map(|b| {
                    binding::buffer(
                        b.slot,
                        wgpu::ShaderStages::COMPUTE,
                        if b.access == thrum_render::graph::AccessKind::UniformRead {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            // WGSL declares bindings 4..8 read_write, even in entries which only read.
                            wgpu::BufferBindingType::Storage {
                                read_only: b.slot < 4 || b.buffer == BufferKey::Twiddles,
                            }
                        },
                        0,
                    )
                })
                .collect();
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&pass.name),
                entries: &entries,
            });
            let entries: Vec<_> = bindings
                .iter()
                .map(|b| wgpu::BindGroupEntry {
                    binding: b.slot,
                    resource: buffers[&b.buffer].as_entire_binding(),
                })
                .collect();
            groups.insert(
                group,
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&pass.name),
                    layout: &layout,
                    entries: &entries,
                }),
            );
            layouts.push(layout);
        }
        let layout_refs: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().map(|layout| Some(&**layout)).collect();
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&pass.name),
            bind_group_layouts: &layout_refs,
            immediate_size: 0,
        });
        let pipeline = pipelines
            .entry(pass.pipeline())
            .or_insert_with(|| {
                device.compute(&wgpu::ComputePipelineDescriptor {
                    label: Some(&pass.name),
                    layout: Some(&layout),
                    module: &shader,
                    entry_point: Some(pass.entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                })
            })
            .clone();
        passes.insert(pass.key, PreparedPass { pipeline, groups });
    }
    let dynamic_size =
        usize::try_from(buffers[&BufferKey::UploadStaging].size()).expect("bounded upload");
    let filtered = crate::filtered::Filtered::new(
        device,
        &buffers[&BufferKey::Cascades],
        &buffers[&BufferKey::LongField],
        rain,
        flow.is_some(),
    );
    let flow = flow.map(|(queue, bands)| {
        crate::flow::Flow::new(
            device,
            queue,
            bands,
            rain,
            &filtered.textures[0],
            &buffers,
            &passes,
            &specs,
        )
    });
    Ok(Ambient {
        filtered,
        flow,
        profile,
        buffers,
        passes,
        specs,
        upload: Vec::with_capacity(dynamic_size),
        modes: vec![bytemuck::Zeroable::zeroed(); (spec::CASCADES * crate::PIXELS) as usize],
        cached_cascades: std::array::from_fn(|_| Vec::new()),
        cached_lengths: [0.0; spec::CASCADES as usize],
        modes_initialized: false,
        last_upload: crate::UploadBytes::default(),
        levels: [CacheLevel {
            origin: [0.0; 2],
            texel: 1.0,
            max_wavenumber: std::f64::consts::PI,
        }; LEVELS as usize],
    })
}
