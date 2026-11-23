use super::SurfGpu;
use crate::{
    WaterError,
    column::{COMPONENT_LIMIT, Component},
    map_gpu::{self, MapGpu},
};
use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_sea_model::Spectrum;

pub(crate) const PASS: &str = "surf field cache";
const OUTPUT_BINDING: u32 = 13;
const COMPUTE_SURF_BINDING: u32 = 7;
const GROUP_WIDTH: u32 = 64;

/// Opt-in readback handle for the most recent submitted surf field.
#[derive(Clone, Default)]
pub struct SurfTap(pub(crate) Arc<Mutex<Option<SurfWitness>>>);

impl SurfTap {
    /// Observe the submitted products without waiting for the GPU.
    /// # Panics
    /// Panics if the witness lock was poisoned.
    pub fn witness(&self) -> Option<SurfWitness> {
        self.0.lock().expect("surf witness").clone()
    }
}

/// The mesh inputs and surf cache consumed by the same submitted surface draw.
#[derive(Clone)]
pub struct SurfWitness {
    /// Rest nodes as x, z, spacing and body slot.
    pub nodes: wgpu::Buffer,
    /// Displacement xyz and presence, then previous displacement xyz and index.
    pub samples: wgpu::Buffer,
    /// World x/z used to place the camera-relative sea nodes.
    pub anchor: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CacheSettings {
    anchor: [f32; 2],
    count: u32,
    components: u32,
    heights: [f32; 4],
    chop_dt: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CacheSample {
    current: [f32; 4],
    previous: [f32; 4],
}

pub(crate) struct Cache {
    settings: wgpu::Buffer,
    components: wgpu::Buffer,
    samples: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    count: u32,
    recorded_anchor: [f32; 2],
}

impl Cache {
    pub(crate) fn new(
        device: &DeviceSite,
        map: &MapGpu,
        surf: &SurfGpu,
        mesh: &wgpu::Buffer,
        count: u32,
    ) -> Result<Self, WaterError> {
        use wgpu::BufferUsages as Usage;
        let bytes = u64::from(count) * size_of::<CacheSample>() as u64;
        if bytes.max(mesh.size()) > device.limits().max_storage_buffer_binding_size {
            return Err(WaterError::UnsupportedMap(
                "surf field cache exceeds storage binding limits",
            ));
        }
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let settings = buffer(
            "surf field settings",
            size_of::<CacheSettings>() as u64,
            Usage::UNIFORM | Usage::COPY_DST,
        );
        let components = buffer(
            "surf field components",
            (COMPONENT_LIMIT * size_of::<Component>()) as u64,
            Usage::STORAGE | Usage::COPY_DST,
        );
        let samples = buffer(PASS, bytes, Usage::STORAGE | Usage::COPY_SRC);
        let compute = wgpu::ShaderStages::COMPUTE;
        let mut layout_entries = vec![
            binding::uniform(0, compute, settings.size()),
            binding::storage(1, compute, true, mesh.size()),
            binding::storage(2, compute, true, components.size()),
            binding::storage(3, compute, false, samples.size()),
        ];
        layout_entries.extend(map_gpu::bindings(compute));
        layout_entries.extend(SurfGpu::layout(COMPUTE_SURF_BINDING, compute));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &layout_entries,
        });
        let view = map.view();
        let mut entries: Vec<_> = [&settings, mesh, &components, &samples]
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: u32::try_from(binding).expect("four bindings"),
                resource: buffer.as_entire_binding(),
            })
            .collect();
        entries.extend(map.entries(&view));
        entries.extend(surf.entries(COMPUTE_SURF_BINDING));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let source = map_gpu::declarations_at(0)
            + include_str!("../column/records.wgsl")
            + &SurfGpu::source(0, COMPUTE_SURF_BINDING)
            + include_str!("cache.wgsl");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("fill_surf_cache"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            settings,
            components,
            samples,
            group,
            pipeline,
            count,
            recorded_anchor: [0.0; 2],
        })
    }

    pub(crate) fn declare(&self, graph: &mut EffectGraph) -> EffectAccess {
        let read = EffectAccess {
            resource: graph.resources.len(),
            range: 0..self.samples.size(),
            kind: AccessKind::StorageRead,
            view: None,
            history: None,
        };
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.samples.size(),
                usage: self.samples.usage(),
            },
            ResourceScope::Transient,
        ));
        graph.passes.push(EffectPass {
            name: PASS.into(),
            cadence: EffectCadence::EveryFrame,
            async_eligible: false,
            accesses: vec![EffectAccess {
                kind: AccessKind::StorageWrite,
                ..read.clone()
            }],
        });
        read
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded frame interval and choppiness use GPU f32"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        spectrum: &Spectrum,
        heights: [f32; 4],
        anchor: [f32; 2],
        time: f64,
        dt: f64,
        chop: f64,
    ) {
        self.recorded_anchor = anchor;
        let count = u32::try_from(spectrum.long.len()).expect("validated spectrum");
        let mut components = [Component::zeroed(); COMPONENT_LIMIT];
        for (out, component) in components.iter_mut().zip(&spectrum.long) {
            *out = Component::at(component, anchor, time);
        }
        queue.write_buffer(
            &self.settings,
            0,
            bytemuck::bytes_of(&CacheSettings {
                anchor,
                count: self.count,
                components: count,
                heights,
                chop_dt: [chop as f32, dt as f32, 0.0, 0.0],
            }),
        );
        if !spectrum.long.is_empty() {
            queue.write_buffer(
                &self.components,
                0,
                bytemuck::cast_slice(&components[..spectrum.long.len()]),
            );
        }
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.count.div_ceil(GROUP_WIDTH), 1, 1);
    }

    pub(crate) fn source(enabled: bool) -> String {
        let radius = crate::prepare::SEA_RING_HALF_WIDTH;
        format!(
            "const SURF_CACHE_ENABLED: bool = {enabled};\nconst SURF_GRID_RADIUS = {radius}u;\n"
        ) + if enabled {
            include_str!("render.wgsl")
        } else {
            "fn surf_current(_index: u32) -> vec4<f32> { return vec4(0.0); }
                 fn surf_previous(_index: u32) -> vec3<f32> { return vec3(0.0); }"
        }
    }

    pub(crate) fn surface_binding(&self) -> wgpu::BindGroupLayoutEntry {
        binding::storage(
            OUTPUT_BINDING,
            wgpu::ShaderStages::VERTEX,
            true,
            self.samples.size(),
        )
    }

    pub(crate) fn surface_entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: OUTPUT_BINDING,
            resource: self.samples.as_entire_binding(),
        }
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.samples.size() + self.settings.size() + self.components.size()
    }

    pub(crate) fn witness(&self, mesh: &wgpu::Buffer) -> SurfWitness {
        SurfWitness {
            nodes: mesh.clone(),
            samples: self.samples.clone(),
            anchor: self.recorded_anchor,
        }
    }
}
