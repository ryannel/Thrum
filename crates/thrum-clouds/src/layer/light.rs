//! Preparation-time droplet tables and frame-local cloud-free hemisphere light.
use crate::{Error, WeatherMap};
use thrum_cloud_model::{Phase, PhaseTable};
use thrum_render::binding;
use thrum_render::{atmosphere::EnvironmentProducts, contribution::SceneSurface};

pub(crate) const PASS: &str = "cloud hemisphere light";
const ENTRIES: u32 = 1025;

pub(crate) struct Lighting {
    pub(crate) phase: wgpu::Buffer,
    pub(crate) ambient: wgpu::Buffer,
    phases: Vec<Phase>,
    radius: Option<f32>,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
}

impl Lighting {
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        environment: &EnvironmentProducts,
        ground: bool,
    ) -> Result<Self, Error> {
        let phases: Vec<_> = weather
            .types()
            .iter()
            .map(|t| Phase::fit(t.droplet_radius * 2e6))
            .collect::<Result<_, _>>()
            .map_err(Error::Model)?;
        let bytes = u64::try_from(phases.len())
            .map_err(|_| Error::Unsupported("phase row count"))?
            * u64::from(ENTRIES)
            * 4;
        if bytes > surface.limits.max_storage_buffer_binding_size {
            return Err(Error::DeviceLimit("phase table storage limit"));
        }
        let phase = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud droplet phase tables"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ambient = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud hemisphere radiance"),
            size: 16 * if ground { 2 + 32 } else { 2 },
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let layout = surface
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(PASS),
                entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 0)],
            });
        let group = surface
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ambient.as_entire_binding(),
                }],
            });
        let pipeline_layout =
            surface
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(PASS),
                    bind_group_layouts: &[Some(&environment.layout), Some(&layout)],
                    immediate_size: 0,
                });
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(PASS),
                source: wgpu::ShaderSource::Wgsl(
                    format!(
                        "{}\n{}",
                        environment.shader_source,
                        include_str!("hemisphere.wgsl")
                    )
                    .into(),
                ),
            });
        let pipeline = surface.device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("hemispheres"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            phase,
            ambient,
            phases,
            radius: None,
            pipeline,
            group,
        })
    }

    pub(crate) fn upload(&mut self, queue: &wgpu::Queue, radius: f32) -> Result<(), Error> {
        if self.radius == Some(radius) {
            return Ok(());
        }
        for (row, &phase) in self.phases.iter().enumerate() {
            let table = PhaseTable::new(phase, f64::from(radius), ENTRIES).map_err(Error::Model)?;
            queue.write_buffer(
                &self.phase,
                row as u64 * u64::from(ENTRIES) * 4,
                bytemuck::cast_slice(table.values()),
            );
        }
        self.radius = Some(radius);
        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "validated type axis selects bounded phase rows and shader floats"
    )]
    pub(crate) fn type_parameters(&self, kind: f64, plane_turn: f64) -> ([f32; 4], [f32; 4]) {
        let axis = kind * (self.phases.len() - 1) as f64;
        let low = (axis.floor() as usize).min(self.phases.len() - 2);
        let weight = axis - low as f64;
        let blend = |parameter: fn(Phase) -> f64| {
            (parameter(self.phases[low]) * (1.0 - weight)
                + parameter(self.phases[low + 1]) * weight) as f32
        };
        (
            [
                low as f32,
                weight as f32,
                blend(Phase::mean_cosine),
                ENTRIES as f32,
            ],
            [
                blend(Phase::forward_share),
                blend(|phase| phase.parameters()[3]),
                blend(Phase::broad_mean_cosine),
                plane_turn as f32,
            ],
        )
    }

    pub(crate) fn encode(
        &self,
        pass: &mut wgpu::ComputePass<'_>,
        environment: &EnvironmentProducts,
        side: usize,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, environment.group(side), &[]);
        pass.set_bind_group(1, &self.group, &[]);
        pass.dispatch_workgroups(2 + u32::from(self.ambient.size() > 32), 1, 1);
    }
}
