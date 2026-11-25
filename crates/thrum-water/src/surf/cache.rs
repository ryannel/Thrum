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
    contribution::{ContributionFrame, EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_sea_model::Spectrum;

pub(crate) const PASS: &str = "surf field cache";
const OUTPUT_BINDING: u32 = 13;
const COMPUTE_SURF_BINDING: u32 = 7;
const GROUP_WIDTH: u32 = 64;
const MAX_SHORE_GRID_SIDE: u32 = 384;

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
    /// Displacement xyz and 1 plus breaking-face weight (0 outside the bed), then previous
    /// displacement xyz and long-wave variance before the depth limit, then
    /// the current and previous short-wave geometry weights and two packed pairs
    /// of horizontal throw removed by the renderer at coarse spacing.
    /// Only the prefix with one sample per rest node is part of the mesh witness.
    pub samples: wgpu::Buffer,
    /// World x/z used to place the camera-relative sea nodes.
    pub anchor: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct CacheSettings {
    pub(super) anchor: [f32; 2],
    pub(super) count: u32,
    pub(super) components: u32,
    pub(super) heights: [f32; 4],
    pub(super) chop_dt: [f32; 2],
    pub(super) shore_size: [u32; 2],
    pub(super) camera_planes: [[f32; 4]; 4],
    pub(super) vertical_bounds: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CacheSample {
    current: [f32; 4],
    previous: [f32; 4],
    short_weights: [f32; 2],
    unresolved_throw: [u32; 2],
}

pub(crate) struct Cache {
    settings: wgpu::Buffer,
    components: wgpu::Buffer,
    samples: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    count: u32,
    dispatch_count: u32,
    shore_size: [u32; 2],
    recorded_anchor: [f32; 2],
}

impl Cache {
    pub(crate) fn new(
        device: &DeviceSite,
        map: &MapGpu,
        surf: &SurfGpu,
        mesh: &wgpu::Buffer,
        count: u32,
        shore_lattice: Option<[usize; 2]>,
    ) -> Result<Self, WaterError> {
        use wgpu::BufferUsages as Usage;
        let stride = size_of::<CacheSample>() as u64;
        let sample_limit = device.limits().max_storage_buffer_binding_size;
        let mesh_bytes = u64::from(count) * stride;
        if mesh_bytes.max(mesh.size()) > sample_limit {
            return Err(WaterError::UnsupportedMap(
                "surf field cache exceeds storage binding limits",
            ));
        }
        let shore_size = shore_lattice
            .map(|size| {
                size.map(|side| {
                    u32::try_from(side)
                        .unwrap_or(u32::MAX)
                        .min(MAX_SHORE_GRID_SIDE)
                })
            })
            .filter(|[w, h]| mesh_bytes + u64::from(w * h + 1) * stride <= sample_limit)
            .unwrap_or_default();
        let shore_count = shore_size[0] * shore_size[1];
        let dispatch_count = count + shore_count + u32::from(shore_count > 0);
        let bytes = u64::from(dispatch_count) * stride;
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
        let source = Self::compute_source();
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
            dispatch_count,
            shore_size,
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
        frame: &ContributionFrame,
        bounds: Option<[f32; 2]>,
        [time, dt, chop]: [f64; 3],
    ) {
        let anchor = [frame.camera[0], frame.camera[2]];
        self.recorded_anchor = anchor;
        let matrix = glam::Mat4::from_cols_array_2d(&frame.view_proj);
        let [x, y, _, w] = std::array::from_fn(|row| matrix.row(row));
        let camera_planes = [w + x, w - x, w + y, w - y].map(|plane| plane.to_array());
        let vertical_bounds = bounds.map_or([0.0; 4], |[low, high]| [low, high, 1.0, 0.0]);
        let mut active = 0;
        let mut components = [Component::zeroed(); COMPONENT_LIMIT];
        for component in spectrum.long.iter().filter(|wave| wave.amplitude > 0.0) {
            components[active] = Component::at(component, anchor, time);
            active += 1;
        }
        let count = u32::try_from(active).expect("validated spectrum");
        queue.write_buffer(
            &self.settings,
            0,
            bytemuck::bytes_of(&CacheSettings {
                anchor,
                count: self.count,
                components: count,
                heights,
                chop_dt: [chop as f32, dt as f32],
                shore_size: self.shore_size,
                camera_planes,
                vertical_bounds,
            }),
        );
        if active != 0 {
            queue.write_buffer(
                &self.components,
                0,
                bytemuck::cast_slice(&components[..active]),
            );
        }
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(self.dispatch_count.div_ceil(GROUP_WIDTH), 1, 1);
    }

    pub(super) fn compute_source() -> String {
        map_gpu::declarations_at(0)
            + include_str!("../column/records.wgsl")
            + &SurfGpu::source(0, COMPUTE_SURF_BINDING)
            + &Self::ring_source(true)
            + include_str!("cache.wgsl")
    }

    fn ring_source(enabled: bool) -> String {
        let radius = crate::prepare::SEA_RING_HALF_WIDTH;
        let fine_radius = crate::prepare::SURF_FINE_RING_HALF_WIDTH;
        let half_metre_radius = crate::prepare::SURF_HALF_METRE_RING_HALF_WIDTH;
        let metre_radius = crate::prepare::SURF_METRE_RING_HALF_WIDTH;
        let two_metre_radius = crate::prepare::SURF_TWO_METRE_RING_HALF_WIDTH;
        let four_metre_radius = crate::prepare::SURF_FOUR_METRE_RING_HALF_WIDTH;
        let eight_metre_radius = crate::prepare::SURF_EIGHT_METRE_RING_HALF_WIDTH;
        format!(
            "const SURF_CACHE_ENABLED: bool = {enabled};\nconst SURF_GRID_RADIUS = {radius}u;\n\
             fn sea_grid_radius(spacing: f32) -> u32 {{
                 if SURF_CACHE_ENABLED {{
                     if spacing <= 0.25 {{ return {fine_radius}u; }}
                     if spacing == 0.5 {{ return {half_metre_radius}u; }}
                     if spacing == 1.0 {{ return {metre_radius}u; }}
                     if spacing == 2.0 {{ return {two_metre_radius}u; }}
                     if spacing == 4.0 {{ return {four_metre_radius}u; }}
                     if spacing == 8.0 {{ return {eight_metre_radius}u; }}
                 }}
                 return SURF_GRID_RADIUS;
             }}\n"
        )
    }

    pub(crate) fn source(cache: Option<&Self>) -> String {
        let enabled = cache.is_some();
        let shore = cache.is_some_and(|cache| cache.shore_size[0] > 0);
        Self::ring_source(enabled)
            + &format!("const SURF_SHORE_GRID_ENABLED: bool = {shore};\n")
            + if enabled {
                include_str!("render.wgsl")
            } else {
                "fn surf_current(_index: u32) -> vec4<f32> { return vec4(0.0); }
                 fn surf_previous(_index: u32) -> vec4<f32> { return vec4(0.0); }
                 fn surf_short_weights(_index: u32) -> vec2<f32> { return vec2(1.0); }
                 fn surf_shore_face(_point: vec2<f32>) -> f32 { return 0.0; }"
            }
    }

    pub(crate) fn surface_binding(&self) -> wgpu::BindGroupLayoutEntry {
        binding::storage(
            OUTPUT_BINDING,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
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
