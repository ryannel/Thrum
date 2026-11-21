//! Per-view camera policy and GPU-resolved exposure. Physical light and the
//! scene storage scale stay separate from display mapping.

use super::{
    DisplayOverlay, PRE_EXPOSURE, RenderError, SdrToneMap, display::display_selectors, history_gpu,
};
use crate::binding;
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::light::{DIRECTIONAL_BYTES, ENVIRONMENT_BYTES};
use crate::pass_key::PassKey;
use crate::shader_constants::{EXPOSURE_INCIDENT, EXPOSURE_LOW_LIGHT};
use crate::shaders::wgsl;
use crate::timing::GpuTimings;

pub(super) const SAMPLE_BYTES: u64 = size_of::<crate::records::ExposureSample>() as u64;
const POLICY_BYTES: u64 = size_of::<crate::records::Policy>() as u64;
pub(crate) const SHADER: &str = concat!(
    wgsl!("directional_light.wgsl"),
    "\n",
    wgsl!("environment_light.wgsl"),
    "\n",
    wgsl!("synthetic/exposure.wgsl"),
);

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
        overlay: DisplayOverlay,
    ) -> [u8; size_of::<crate::records::Policy>()] {
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
                if low_light_key {
                    EXPOSURE_LOW_LIGHT
                } else {
                    EXPOSURE_INCIDENT
                },
            ),
        };
        bytemuck::cast(crate::records::Policy {
            inverse_compensation_min_max: [inverse, compensation, min, max],
            mode,
            pre_exposure: PRE_EXPOSURE,
            atmosphere: u32::from(atmosphere),
            display_selectors: display_selectors(tone_map, overlay),
        })
    }
}

pub(super) struct ExposureResolve {
    policy: wgpu::Buffer,
    pub(super) samples: [wgpu::Buffer; 2],
    groups: [wgpu::BindGroup; 2],
    pipeline: wgpu::ComputePipeline,
}

impl ExposureResolve {
    pub(super) fn allocate_samples(device: &wgpu::Device) -> [wgpu::Buffer; 2] {
        // Each separately bound record starts at offset zero, satisfying both
        // storage and uniform alignment without a packed dynamic-offset ABI.
        std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("submitted camera exposure sample"),
                size: SAMPLE_BYTES,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        })
    }

    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        directional: &wgpu::Buffer,
        environment_light: &wgpu::Buffer,
        serials: &wgpu::Buffer,
        samples: [wgpu::Buffer; 2],
    ) -> Self {
        let policy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera exposure policy"),
            size: POLICY_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera exposure resolve"),
            entries: &[
                binding::uniform(0, wgpu::ShaderStages::COMPUTE, POLICY_BYTES),
                binding::uniform(1, wgpu::ShaderStages::COMPUTE, DIRECTIONAL_BYTES),
                binding::uniform(4, wgpu::ShaderStages::COMPUTE, ENVIRONMENT_BYTES),
                binding::uniform(2, wgpu::ShaderStages::COMPUTE, history_gpu::SERIAL_BYTES),
                binding::storage(3, wgpu::ShaderStages::COMPUTE, false, SAMPLE_BYTES),
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: environment_light.as_entire_binding(),
                    },
                ],
            })
        });
        let shader = crate::shaders::Shader::Exposure.compile(device, "incident camera exposure");
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera exposure resolve"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
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

    pub(super) fn update(
        &self,
        queue: &wgpu::Queue,
        policy: [u8; size_of::<crate::records::Policy>()],
    ) {
        queue.write_buffer(&self.policy, 0, &policy);
    }

    pub(super) fn append_graph(
        graph: &mut FrameGraph,
        key: PipelineKey,
        [directional, environment_light]: [ResourceId; 2],
    ) -> Result<ResourceId, GraphError> {
        let mut accesses = Vec::with_capacity(5);
        for bytes in [POLICY_BYTES, history_gpu::SERIAL_BYTES] {
            let resource = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                view: None,
                resource,
                range: 0..bytes,
                kind: AccessKind::UniformRead,
                store: true,
            });
        }
        for (resource, bytes) in [
            (directional, DIRECTIONAL_BYTES),
            (environment_light, ENVIRONMENT_BYTES),
        ] {
            accesses.push(Access::new(resource, 0..bytes, AccessKind::UniformRead));
        }
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
            view: None,
            resource: sample.clone(),
            range: 0..SAMPLE_BYTES,
            kind: AccessKind::StorageWrite,
            store: true,
        });
        graph.declare(
            PassKey::Exposure,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses,
                external_side_effect: false,
            },
        );
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
