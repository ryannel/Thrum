//! Per-view camera policy and GPU-resolved exposure. Physical light and the
//! fixed scene storage scale remain independent of the camera.

use super::{PRE_EXPOSURE, RenderError, SdrToneMap, history_gpu, uniform_layout};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::timing::GpuTimings;

pub(super) const SAMPLE_BYTES: u64 = 32;
const POLICY_BYTES: u64 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ExposurePolicy {
    Fixed {
        inverse: f32,
    },
    IncidentHorizontal {
        compensation_ev: f32,
        min_ev100: f32,
        max_ev100: f32,
        /// Render dim scenes below mid-grey, as an incompletely adapted eye sees them.
        low_light_key: bool,
    },
}

impl ExposurePolicy {
    pub(super) fn incident(
        compensation_ev: f32,
        min_ev100: f32,
        max_ev100: f32,
    ) -> Result<Self, RenderError> {
        if !compensation_ev.is_finite()
            || !(-24.0..=24.0).contains(&compensation_ev)
            || !min_ev100.is_finite()
            || !max_ev100.is_finite()
            || min_ev100 > max_ev100
            || !(-24.0..=24.0).contains(&min_ev100)
            || !(-24.0..=24.0).contains(&max_ev100)
        {
            return Err(RenderError::InvalidExposure);
        }
        Ok(Self::IncidentHorizontal {
            compensation_ev,
            min_ev100,
            max_ev100,
            low_light_key: false,
        })
    }

    pub(super) fn pack(
        self,
        fallback_inverse: f32,
        atmosphere: bool,
        tone_map: SdrToneMap,
    ) -> [u8; 32] {
        let (inverse, compensation, min, max, mode) = match self {
            Self::Fixed { inverse } => (inverse, 0.0, 0.0, 0.0, 0),
            Self::IncidentHorizontal {
                compensation_ev,
                min_ev100,
                max_ev100,
                low_light_key,
            } => (
                fallback_inverse,
                compensation_ev,
                min_ev100,
                max_ev100,
                1 + u32::from(low_light_key),
            ),
        };
        let words = [
            inverse.to_bits(),
            compensation.to_bits(),
            min.to_bits(),
            max.to_bits(),
            mode,
            PRE_EXPOSURE.to_bits(),
            u32::from(atmosphere),
            tone_map as u32,
        ];
        std::array::from_fn(|byte| words[byte / 4].to_le_bytes()[byte % 4])
    }
}

pub(super) struct ExposureResolve {
    policy: wgpu::Buffer,
    pub(super) samples: [wgpu::Buffer; 2],
    groups: [wgpu::BindGroup; 2],
    pipeline: wgpu::ComputePipeline,
}

impl ExposureResolve {
    pub(super) fn new(
        device: &wgpu::Device,
        directional: &wgpu::Buffer,
        serials: &wgpu::Buffer,
    ) -> Self {
        let policy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera exposure policy"),
            size: POLICY_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Each separately bound record starts at offset zero, satisfying both
        // storage and uniform alignment without a packed dynamic-offset ABI.
        let samples = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("submitted camera exposure sample"),
                size: SAMPLE_BYTES,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera exposure resolve"),
            entries: &[
                uniform_layout(0, POLICY_BYTES, wgpu::ShaderStages::COMPUTE),
                uniform_layout(1, 64, wgpu::ShaderStages::COMPUTE),
                uniform_layout(2, history_gpu::SERIAL_BYTES, wgpu::ShaderStages::COMPUTE),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(SAMPLE_BYTES),
                    },
                    count: None,
                },
            ],
        });
        let groups = std::array::from_fn(|side| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("camera exposure resolve"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: policy.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: directional.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: serials.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: samples[side].as_entire_binding(),
                    },
                ],
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("incident camera exposure"),
            source: wgpu::ShaderSource::Wgsl(include_str!("exposure.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera exposure resolve"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("camera exposure resolve"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("resolve"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            policy,
            samples,
            groups,
            pipeline,
        }
    }

    pub(super) fn update(&self, queue: &wgpu::Queue, policy: [u8; 32]) {
        queue.write_buffer(&self.policy, 0, &policy);
    }

    pub(super) fn append_graph(
        graph: &mut FrameGraph,
        key: PipelineKey,
        directional: ResourceId,
    ) -> Result<ResourceId, GraphError> {
        let mut accesses = Vec::with_capacity(4);
        for bytes in [POLICY_BYTES, history_gpu::SERIAL_BYTES] {
            let resource = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource,
                range: 0..bytes,
                kind: AccessKind::UniformRead,
                store: true,
            });
        }
        accesses.push(Access {
            resource: directional,
            range: 0..64,
            kind: AccessKind::UniformRead,
            store: true,
        });
        let sample = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: SAMPLE_BYTES,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC,
            },
            ResourceScope::History {
                owner: 2,
                epoch: key.device_generation,
                initialized: false,
            },
        )?;
        accesses.push(Access {
            resource: sample.clone(),
            range: 0..SAMPLE_BYTES,
            kind: AccessKind::StorageWrite,
            store: true,
        });
        graph.pass(Pass {
            name: "synthetic exposure resolve".into(),
            scope: PassScope::View(1),
            pipeline: Some(key),
            accesses,
            external_side_effect: false,
        });
        Ok(sample)
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        pass: &CompiledPass,
        write_side: usize,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = pass
            .begin_compute(encoder, generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[write_side], &[]);
        pass.dispatch_workgroups(1, 1, 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
