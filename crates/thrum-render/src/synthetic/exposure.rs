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
macro_rules! exposure_source {
    ($body:expr) => {
        concat!(
            wgsl!("directional_light.wgsl"),
            "\n",
            wgsl!("environment_light.wgsl"),
            "\n",
            $body,
        )
    };
}
pub(crate) const SHADER: &str = concat!(
    exposure_source!(wgsl!("synthetic/exposure.wgsl")),
    "\nfn incident_sun_factor() -> f32 { return 1.0; }"
);

pub(crate) fn ground_band_source(variant: crate::shaders::CloudVariant) -> String {
    incident_sun_source(exposure_source!(wgsl!("synthetic/exposure.wgsl")), variant)
}

fn incident_sun_source(body: &str, variant: crate::shaders::CloudVariant) -> String {
    let source = format!(
        "{}\n{}\n{}",
        body,
        wgsl!("records/observer.wgsl"),
        variant.sampling_source()
    );
    format!(
        "{source}\n@group(1) @binding(3) var<uniform> aerial_frame: Atmosphere;\n@group(1) @binding(4) var aerial_sampler: sampler;\nfn incident_sun_factor() -> f32 {{ return cloud_factor(aerial_frame.camera_altitude.xyz); }}"
    )
}

pub(crate) fn world_light_source(variant: crate::shaders::CloudVariant) -> String {
    let body = exposure_source!(wgsl!(@exposure_body
        "                let adapted = adapt_world_light_ev(raw_ev, measured, parameters.zw);\n                ev = adapted.x;\n                inverse = CAMERA_EXPOSURE_NORMALIZATION * exp2(ev);\n                flags = select(0u, WORLD_LIGHT_ADAPTING, adapted.y > 0.0);\n"));
    let source = if variant.present() {
        incident_sun_source(body, variant)
    } else {
        format!("{body}\nfn incident_sun_factor() -> f32 {{ return 1.0; }}")
    };
    format!("{source}\n{}", wgsl!("atmosphere/overlay_exposure.wgsl"))
}

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
        forced_storage_scale: Option<f32>,
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
            // A negative value is reserved for the labelled storage-scale proof.
            pre_exposure: forced_storage_scale.map_or(PRE_EXPOSURE, |scale| -scale),
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
    sun_path: Option<wgpu::BindGroup>,
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
        ground: Option<(
            &crate::atmosphere::CloudPrepared,
            &crate::atmosphere::AtmospherePrepared,
        )>,
    ) -> Self {
        let (cloud, atmosphere) = ground.unzip();
        Self::prepare(
            device,
            directional,
            environment_light,
            serials,
            samples,
            (cloud, atmosphere),
        )
    }

    pub(super) fn prepare(
        device: &crate::runtime::DeviceSite,
        directional: &wgpu::Buffer,
        environment_light: &wgpu::Buffer,
        serials: &wgpu::Buffer,
        samples: [wgpu::Buffer; 2],
        (cloud, atmosphere): (
            Option<&crate::atmosphere::CloudPrepared>,
            Option<&crate::atmosphere::AtmospherePrepared>,
        ),
    ) -> Self {
        let policy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera exposure policy"),
            size: POLICY_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let visibility = wgpu::ShaderStages::COMPUTE;
        let adapt = atmosphere.is_some_and(crate::atmosphere::AtmospherePrepared::uses_haze_scale);
        let entries = [
            binding::uniform(0, visibility, POLICY_BYTES),
            binding::uniform(1, visibility, DIRECTIONAL_BYTES),
            binding::uniform(4, visibility, ENVIRONMENT_BYTES),
            binding::uniform(2, visibility, history_gpu::SERIAL_BYTES),
            binding::storage(3, visibility, false, SAMPLE_BYTES),
        ];
        let mut entries = Vec::from(entries);
        if adapt {
            entries.push(binding::uniform(5, visibility, SAMPLE_BYTES));
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera exposure resolve"),
            entries: &entries,
        });
        let groups = std::array::from_fn(|side| {
            let mut entries = vec![
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
            ];
            if adapt {
                entries.push(wgpu::BindGroupEntry {
                    binding: 5,
                    resource: samples[side ^ 1].as_entire_binding(),
                });
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("camera exposure resolve"),
                layout: &layout,
                entries: &entries,
            })
        });
        let sun_path = cloud.zip(atmosphere).and_then(|(cloud, air)| {
            let band = cloud.band.as_ref();
            if band.is_none() && !air.has_overlay() {
                return None;
            }
            let (_, sampler, observer) = air.aerial_sampled();
            let stages = wgpu::ShaderStages::COMPUTE;
            let mut entries = vec![
                binding::uniform(3, stages, observer.size()),
                binding::sampler(4, stages, wgpu::SamplerBindingType::Filtering),
                binding::texture(
                    13,
                    stages,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2,
                ),
                binding::uniform(14, stages, cloud.uniform.size()),
            ];
            if band.is_some() {
                entries.extend(crate::atmosphere::ground_band::GroundBandPrepared::layout(
                    stages,
                ));
            }
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("incident ground band"),
                entries: &entries,
            });
            let mut entries = vec![
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: observer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: wgpu::BindingResource::TextureView(&cloud.view),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: cloud.uniform.as_entire_binding(),
                },
            ];
            if let Some(band) = band {
                entries.extend(band.entries());
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("incident ground band"),
                layout: &layout,
                entries: &entries,
            });
            Some((layout, group, cloud.variant()))
        });
        let shader = if adapt {
            let variant = sun_path
                .as_ref()
                .map_or(crate::shaders::CloudVariant::Absent, |(_, _, variant)| {
                    *variant
                });
            crate::shaders::Shader::WorldLightExposure(variant)
        } else if let Some((_, _, variant)) = &sun_path {
            crate::shaders::Shader::GroundBandExposure(*variant)
        } else {
            crate::shaders::Shader::Exposure
        }
        .compile(device, "incident camera exposure");
        let layouts = [
            Some(&*layout),
            sun_path.as_ref().map(|(layout, ..)| &**layout),
        ];
        let layouts = &layouts[..=usize::from(sun_path.is_some())];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera exposure resolve"),
            bind_group_layouts: layouts,
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
            sun_path: sun_path.map(|(_, group, _)| group),
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
        light: [ResourceId; 2],
        sun_path: Vec<Access>,
    ) -> Result<ResourceId, GraphError> {
        Self::append_with_adaptation(graph, key, light, sun_path, None)
    }

    pub(super) fn history_sample(
        graph: &mut FrameGraph,
        generation: u64,
        initialized: bool,
    ) -> Result<ResourceId, GraphError> {
        graph.resource(
            ResourceDescriptor::Buffer {
                bytes: SAMPLE_BYTES,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_SRC,
            },
            ResourceScope::History {
                owner: 2,
                epoch: generation,
                initialized,
            },
        )
    }

    pub(super) fn append_with_adaptation(
        graph: &mut FrameGraph,
        key: PipelineKey,
        [directional, environment_light]: [ResourceId; 2],
        sun_path: Vec<Access>,
        previous: Option<&ResourceId>,
    ) -> Result<ResourceId, GraphError> {
        let mut accesses = sun_path;
        if let Some(previous) = previous {
            accesses.push(Access::new(
                previous.clone(),
                0..SAMPLE_BYTES,
                AccessKind::UniformRead,
            ));
        }
        for bytes in [POLICY_BYTES, history_gpu::SERIAL_BYTES] {
            let resource = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access::new(resource, 0..bytes, AccessKind::UniformRead));
        }
        for (resource, bytes) in [
            (directional, DIRECTIONAL_BYTES),
            (environment_light, ENVIRONMENT_BYTES),
        ] {
            accesses.push(Access::new(resource, 0..bytes, AccessKind::UniformRead));
        }
        let sample = Self::history_sample(graph, key.device_generation, false)?;
        accesses.push(Access::new(
            sample.clone(),
            0..SAMPLE_BYTES,
            AccessKind::StorageWrite,
        ));
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
        if let Some(group) = &self.sun_path {
            pass.set_bind_group(1, group, &[]);
        }
        pass.dispatch_workgroups(1, 1, 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
