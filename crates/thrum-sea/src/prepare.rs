use crate::{Ambient, CacheLevel, LEVELS, Profile, records, spec};
use std::collections::BTreeMap;
use thrum_render::runtime::DeviceSite;

pub(super) struct PreparedPass {
    pub pipeline: wgpu::ComputePipeline,
    pub groups: Vec<(u32, wgpu::BindGroup)>,
}

pub(super) fn sizes() -> Vec<u64> {
    spec::resources().iter().map(|r| r.size).collect()
}

pub(super) fn new(device: &DeviceSite, profile: Profile) -> Result<Ambient, String> {
    if profile.transform_size != crate::TRANSFORM_SIZE {
        return Err("sea transform_size: supported value is 256".into());
    }
    if profile.cascades != spec::CASCADES {
        return Err("sea cascades: supported value is 3".into());
    }
    if profile.cache_levels != LEVELS {
        return Err("sea cache_levels: supported value is 8".into());
    }
    if !profile.choppiness.is_finite() || !(0.0..=1.0).contains(&profile.choppiness) {
        return Err("sea choppiness: expected finite 0..=1".into());
    }
    let limits = device.limits();
    let resources = spec::resources();
    for resource in &resources {
        if resource.size > limits.max_buffer_size {
            return Err(format!("{}: max_buffer_size", resource.name));
        }
        if resource.usage.contains(wgpu::BufferUsages::STORAGE)
            && resource.size > limits.max_storage_buffer_binding_size
        {
            return Err(format!(
                "{}: max_storage_buffer_binding_size",
                resource.name
            ));
        }
    }
    // Keep SEA's admitted portable envelope, though each pass now binds only what it uses.
    if limits.max_storage_buffers_per_shader_stage < 8 {
        return Err("sea: max_storage_buffers_per_shader_stage requires 8".into());
    }
    if limits.max_compute_workgroup_storage_size < 2048 {
        return Err("sea FFT: max_compute_workgroup_storage_size requires 2048".into());
    }
    if limits.max_compute_invocations_per_workgroup < crate::TRANSFORM_SIZE
        || limits.max_compute_workgroup_size_x < crate::TRANSFORM_SIZE
    {
        return Err("sea FFT: max_compute_invocations_per_workgroup and max_compute_workgroup_size_x require 256".into());
    }
    let mut buffers = BTreeMap::new();
    for resource in resources {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&resource.name),
            size: resource.size,
            usage: resource.usage,
            mapped_at_creation: matches!(resource.key, spec::BufferKey::MipParams(_)),
        });
        if let spec::BufferKey::MipParams(level) = resource.key {
            let record = records::Mip {
                address: [0, 0, crate::TRANSFORM_SIZE >> level, 0],
            };
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .map_err(|e| format!("sea mip mapping: {e}"))?
                .copy_from_slice(bytemuck::bytes_of(&record));
            buffer.unmap();
        }
        buffers.insert(resource.key, buffer);
    }
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sea ambient"),
        source: wgpu::ShaderSource::Wgsl(crate::SHADER.into()),
    });
    let specs = spec::passes();
    let mut passes = BTreeMap::new();
    let mut pipelines = BTreeMap::<spec::PipelineKey, wgpu::ComputePipeline>::new();
    for pass in &specs {
        let mut layouts = Vec::new();
        let mut groups = Vec::new();
        let max_group = pass
            .bindings
            .iter()
            .map(|b| b.group)
            .max()
            .expect("nonempty pass");
        for group in 0..=max_group {
            let bindings: Vec<_> = pass.bindings.iter().filter(|b| b.group == group).collect();
            let entries: Vec<_> = bindings
                .iter()
                .map(|b| wgpu::BindGroupLayoutEntry {
                    binding: b.slot,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if b.access == thrum_render::graph::AccessKind::UniformRead {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            // WGSL declares bindings 4..8 read_write, even in entries which only read.
                            wgpu::BufferBindingType::Storage {
                                read_only: b.slot < 4,
                            }
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
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
            groups.push((
                group,
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&pass.name),
                    layout: &layout,
                    entries: &entries,
                }),
            ));
            layouts.push(layout);
        }
        let layout_refs: Vec<_> = layouts.iter().map(Some).collect();
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
        usize::try_from(buffers[&spec::BufferKey::UploadStaging].size()).expect("bounded upload");
    Ok(Ambient {
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
