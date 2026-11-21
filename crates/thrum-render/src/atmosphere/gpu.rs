//! Generation-bound LUTs and prepared portable pipelines. Encoding belongs to
//! the caller's existing compiled graph; this owner never submits GPU work.

use crate::graph::{CompiledPass, PipelineKey};
use crate::pass_key::{AtmosphereStage, PipelineRole, REFLECTION_MIPS};
use crate::shaders::wgsl;
use crate::synthetic::{DEPTH_FORMAT, RenderError, SyntheticView};
use crate::timing::GpuTimings;

use super::{
    AtmosphereFrame,
    night::{self, NightProfile},
    reflection::ReflectionPrepared,
    spectrum::EARTH,
    stage::{
        AERIAL_VALID, Chart, Charts, DFG_VALID, DIFFUSE_VALID, GROUND_VALID, REFLECTION_VALID,
        SKY_VALID, TRANSPORT_VALID,
    },
};
mod uniform;
use uniform::AtmosphereUniform;
mod aerial_view;
pub(crate) use aerial_view::SecondaryAerial;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(in crate::atmosphere) mod tests;

pub(super) use crate::shader_constants::AERIAL_EXTENT;
use crate::shader_constants::{AERIAL_WORKGROUP_EDGE, LUT_WORKGROUP_EDGE};
const LUT_SHADER: &str = concat!(
    concat!(
        wgsl!("atmosphere/storage.wgsl"),
        "\n",
        concat!(
            wgsl!("atmosphere/geometry.wgsl"),
            "\n",
            wgsl!("atmosphere/common.wgsl")
        )
    ),
    "\n",
    wgsl!("atmosphere/transport.wgsl"),
    "\n",
    wgsl!("atmosphere/night.wgsl"),
    "\n",
    wgsl!("atmosphere/lut.wgsl"),
    "\n",
    wgsl!("atmosphere/ground.wgsl")
);
const UNIFORM_LEN: usize = size_of::<AtmosphereUniform>();
pub(super) const SKY_MOTION_BYTES: u64 = size_of::<crate::records::SkyMotion>() as u64;
pub(super) const UNIFORM_BYTES: u64 = UNIFORM_LEN as u64;
const _: () = assert!(super::reflection::MIP_COUNT == REFLECTION_MIPS);

#[derive(Clone)]
pub(crate) struct AtmospherePrepared {
    frame: AtmosphereFrame,
    environment: super::environment::EnvironmentState,
    pub(super) generation: u64,
    pub(super) epoch: u64,
    pub(super) textures: Charts<wgpu::Texture>,
    pub(super) night: NightProfile,
    pub(super) aerial_textures: [wgpu::Texture; 2],
    aerial_views: [wgpu::TextureView; 2],
    transmittance_view: wgpu::TextureView,
    aerial_sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    // The four chart stages, each under the chart it integrates.
    chart_stages: Charts<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    sky_motion: wgpu::Buffer,
    submitted_sky: Option<(SyntheticView, [f32; 3])>,
    pending_sky: Option<(SyntheticView, [f32; 3])>,
    sky_pipeline: wgpu::RenderPipeline,
    sky_groups: [wgpu::BindGroup; 2],
    diffuse_pipeline: wgpu::ComputePipeline,
    diffuse_group: wgpu::BindGroup,
    aerial_pipeline: wgpu::ComputePipeline,
    aerial_group: wgpu::BindGroup,
    pub(super) reflection: ReflectionPrepared,
    validity: u8,
    sky_dependencies: [u32; 4],
    diffuse_dependencies: [u32; 4],
    aerial_dependencies: [[u32; 4]; 5],
}

impl AtmospherePrepared {
    pub(crate) fn products(
        &self,
        device: &wgpu::Device,
        directional: &wgpu::Buffer,
        environment_light: &wgpu::Buffer,
        exposure: &[wgpu::Buffer; 2],
        far_aerial: bool,
    ) -> super::EnvironmentProducts {
        super::products::create(
            self,
            device,
            directional,
            environment_light,
            exposure,
            far_aerial,
        )
    }

    pub(crate) fn observer_uniform(&self) -> &wgpu::Buffer {
        &self.uniform
    }

    pub(crate) fn transmittance_view(&self) -> &wgpu::TextureView {
        &self.transmittance_view
    }

    pub(crate) fn aerial_textures(&self) -> [&wgpu::Texture; 2] {
        self.aerial_textures.each_ref()
    }

    pub(crate) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        generation: u64,
        frame: AtmosphereFrame,
        view: &SyntheticView,
        environment_light: &wgpu::Buffer,
        exposure_samples: &[wgpu::Buffer; 2],
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere frame"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let textures = Charts::from_fn(|chart| {
            let [width, height] = chart.extent();
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atmosphere persistent LUT"),
                size: wgpu::Extent3d {
                    width,
                    height: height * chart.planes(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        });
        let views =
            textures.map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atmosphere LUT linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let aerial_textures = std::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atmosphere aerial volume"),
                size: wgpu::Extent3d {
                    width: AERIAL_EXTENT[0],
                    height: AERIAL_EXTENT[1],
                    depth_or_array_layers: AERIAL_EXTENT[2],
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        });
        let aerial_views = aerial_textures.each_ref().map(|texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        });
        let aerial_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atmosphere aerial bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let night = NightProfile::new(device, queue, frame.nightglow_active());
        let shader =
            crate::shaders::Shader::AtmosphereLuts.compile(device, "atmosphere LUT integration");
        let chart_stages = Charts::from_fn(|chart| {
            let stage = AtmosphereStage::ORDER
                .into_iter()
                .find(|stage| stage.chart_output() == Some(chart))
                .expect("every chart has its stage");
            prepare_compute(device, stage, &uniform, &views, &sampler, &shader, &night)
        });
        let sky_motion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky submitted motion"),
            size: SKY_MOTION_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (sky_pipeline, sky_groups) = prepare_sky(
            device,
            &uniform,
            &sky_motion,
            &views,
            &sampler,
            exposure_samples,
        );
        let (diffuse_pipeline, diffuse_group) =
            prepare_diffuse(device, &uniform, &views, &sampler, environment_light);
        let (aerial_pipeline, aerial_group) =
            prepare_aerial(device, &uniform, &views, &aerial_views, &sampler, &night);
        let reflection = ReflectionPrepared::new(
            device,
            &uniform,
            &chart_entries(AtmosphereStage::Reflection(0), &views),
            &sampler,
        );
        let mut result = Self {
            frame,
            environment: super::environment::EnvironmentState::new(frame),
            generation,
            epoch: frame.profile_epoch,
            textures,
            night,
            aerial_textures,
            aerial_views,
            transmittance_view: views.transmittance.clone(),
            aerial_sampler,
            uniform,
            chart_stages,
            sky_motion,
            submitted_sky: None,
            pending_sky: None,
            sky_pipeline,
            sky_groups,
            diffuse_pipeline,
            diffuse_group,
            aerial_pipeline,
            aerial_group,
            reflection,
            validity: 0,
            sky_dependencies: [0; 4],
            diffuse_dependencies: [0; 4],
            aerial_dependencies: [[0; 4]; 5],
        };
        result.update(queue, frame, view, [1, 1]);
        result
    }

    pub(crate) fn update(
        &mut self,
        queue: &wgpu::Queue,
        frame: AtmosphereFrame,
        view: &SyntheticView,
        extent: [u32; 2],
    ) {
        self.frame = frame;
        self.environment.update(frame);
        if frame.profile_epoch != self.epoch {
            self.validity &= DFG_VALID;
            self.epoch = frame.profile_epoch;
        }
        let dependencies = [
            frame.toward_sun[0].to_bits(),
            frame.toward_sun[1].to_bits(),
            frame.toward_sun[2].to_bits(),
            frame.camera_altitude_m.to_bits(),
        ];
        let night_changed = self.night.select(queue, frame.nightglow_active());
        if dependencies != self.sky_dependencies || night_changed {
            self.validity &= !(SKY_VALID | REFLECTION_VALID);
            self.sky_dependencies = dependencies;
        }
        let diffuse_dependencies = [
            frame.solar_illuminance_lux.to_bits(),
            frame.solar_colour[0].to_bits(),
            frame.solar_colour[1].to_bits(),
            frame.solar_colour[2].to_bits(),
        ];
        if self.validity & SKY_VALID == 0 || diffuse_dependencies != self.diffuse_dependencies {
            self.validity &= !DIFFUSE_VALID;
            self.diffuse_dependencies = diffuse_dependencies;
        }
        let (previous, sun) = self.submitted_sky.unwrap_or((*view, frame.toward_sun));
        let prior = glam::Mat4::from_cols_array_2d(&previous.view_proj_columns());
        let current = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
        let rotation = solar_rotation(frame.toward_sun, sun);
        let motion = crate::records::SkyMotion {
            camera_delta: (prior - current).to_cols_array_2d(),
            current: current.to_cols_array_2d(),
            extent: [
                extent[0] as f32,
                extent[1] as f32,
                f32::from(view.history_reset()),
                0.0,
            ],
            solar_delta: (prior * rotation - current).to_cols_array_2d(),
        };
        queue.write_buffer(&self.sky_motion, 0, bytemuck::bytes_of(&motion));
        self.pending_sky = Some((*view, frame.toward_sun));
        let record = AtmosphereUniform::new(frame, view, frame.camera_altitude_m);
        let [a, b, c, d] = record.inverse_view_projection;
        let aerial_dependencies =
            [a, b, c, d, record.camera_altitude].map(|lane| lane.map(f32::to_bits));
        if self.validity & SKY_VALID == 0 || aerial_dependencies != self.aerial_dependencies {
            self.validity &= !AERIAL_VALID;
            self.aerial_dependencies = aerial_dependencies;
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&record));
    }

    pub(crate) fn commit(&mut self) {
        self.submitted_sky = self.pending_sky.take();
        self.validity = TRANSPORT_VALID
            | SKY_VALID
            | DIFFUSE_VALID
            | AERIAL_VALID
            | GROUND_VALID
            | REFLECTION_VALID
            | DFG_VALID;
    }

    pub(crate) fn environment(&self) -> super::EnvironmentFrame {
        self.environment.sample()
    }

    pub(crate) fn reflection_sampled(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler) {
        self.reflection.sampled()
    }

    pub(crate) fn key(&self, stage: AtmosphereStage) -> PipelineKey {
        PipelineRole::Atmosphere(stage)
            .pipeline_key(self.generation, super::super::synthetic::SYNTHETIC_REVISION)
    }

    pub(crate) fn aerial_sampled(
        &self,
    ) -> ([&wgpu::TextureView; 2], &wgpu::Sampler, &wgpu::Buffer) {
        (
            [&self.aerial_views[0], &self.aerial_views[1]],
            &self.aerial_sampler,
            &self.uniform,
        )
    }

    pub(crate) fn encode(
        &self,
        stage: AtmosphereStage,
        encoder: &mut wgpu::CommandEncoder,
        pass: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        // Cached dependencies retain their graph identity without adding empty
        // GPU passes or timestamp work to an ordinary frame.
        if self.validity & stage.validity() != 0 {
            return Ok(());
        }
        let mut compute = pass
            .begin_compute(encoder, self.generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        let (pipeline, group, workgroups) = match stage {
            AtmosphereStage::Dfg | AtmosphereStage::Reflection(_) => {
                return self.reflection.encode(stage, &mut compute);
            }
            AtmosphereStage::Diffuse => (&self.diffuse_pipeline, &self.diffuse_group, [1, 1, 1]),
            AtmosphereStage::Aerial => (
                &self.aerial_pipeline,
                &self.aerial_group,
                [
                    AERIAL_EXTENT[0].div_ceil(AERIAL_WORKGROUP_EDGE),
                    AERIAL_EXTENT[1].div_ceil(AERIAL_WORKGROUP_EDGE),
                    1,
                ],
            ),
            AtmosphereStage::Ground => {
                let (pipeline, group) = &self.chart_stages.ground;
                (pipeline, group, [Chart::Ground.extent()[0], 1, 1])
            }
            AtmosphereStage::Transmittance
            | AtmosphereStage::MultipleScattering
            | AtmosphereStage::SkyView => {
                let chart = stage
                    .chart_output()
                    .expect("a chart stage writes its chart");
                let (pipeline, group) = self.chart_stages.get(chart);
                let [width, height] = chart.extent();
                (
                    pipeline,
                    group,
                    [
                        width.div_ceil(LUT_WORKGROUP_EDGE),
                        height.div_ceil(LUT_WORKGROUP_EDGE),
                        1,
                    ],
                )
            }
        };
        compute.set_pipeline(pipeline);
        compute.set_bind_group(0, group, &[]);
        compute.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
        Ok(())
    }

    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, exposure_side: usize) {
        pass.set_pipeline(&self.sky_pipeline);
        pass.set_bind_group(0, &self.sky_groups[exposure_side], &[]);
        pass.draw(0..3, 0..1);
    }
}

fn prepare_diffuse(
    device: &crate::runtime::DeviceSite,
    uniform: &wgpu::Buffer,
    views: &Charts<wgpu::TextureView>,
    sampler: &wgpu::Sampler,
    environment_light: &wgpu::Buffer,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        entries: &[
            uniform_entry(visibility),
            sampler_entry(2, visibility),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(crate::light::ENVIRONMENT_BYTES),
                },
                count: None,
            },
        ]
        .into_iter()
        .chain(chart_layout(AtmosphereStage::Diffuse, visibility))
        .collect::<Vec<_>>(),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader = crate::shaders::Shader::AtmosphereDiffuse
        .compile(device, "atmosphere diffuse hemisphere integration");
    let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("diffuse_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere diffuse resources"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: environment_light.as_entire_binding(),
            },
        ]
        .into_iter()
        .chain(chart_entries(AtmosphereStage::Diffuse, views))
        .collect::<Vec<_>>(),
    });
    (pipeline, group)
}

fn prepare_aerial(
    device: &crate::runtime::DeviceSite,
    uniform: &wgpu::Buffer,
    views: &Charts<wgpu::TextureView>,
    aerial: &[wgpu::TextureView; 2],
    sampler: &wgpu::Sampler,
    night: &NightProfile,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let output = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: wgpu::TextureFormat::Rgba16Float,
            view_dimension: wgpu::TextureViewDimension::D2Array,
        },
        count: None,
    };
    let entries = [
        uniform_entry(visibility),
        output(3),
        output(4),
        sampler_entry(5, visibility),
    ];
    let entries = entries
        .into_iter()
        .chain(chart_layout(AtmosphereStage::Aerial, visibility))
        .chain(night::layout_entries())
        .collect::<Vec<_>>();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere aerial integration"),
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere aerial"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader =
        crate::shaders::Shader::AtmosphereAerial.compile(device, "atmosphere aerial integration");
    let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("atmosphere aerial integration"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("aerial_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bindings = [
        wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::TextureView(&aerial[0]),
        },
        wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::TextureView(&aerial[1]),
        },
        wgpu::BindGroupEntry {
            binding: 5,
            resource: wgpu::BindingResource::Sampler(sampler),
        },
    ];
    let bindings = bindings
        .into_iter()
        .chain(chart_entries(AtmosphereStage::Aerial, views))
        .chain(night.bindings())
        .collect::<Vec<_>>();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere aerial resources"),
        layout: &layout,
        entries: &bindings,
    });
    (pipeline, group)
}

fn uniform_entry(visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTES),
        },
        count: None,
    }
}

pub(super) fn sampled_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

/// Layout entries for the charts `stage` samples, from its one table.
fn chart_layout(
    stage: AtmosphereStage,
    visibility: wgpu::ShaderStages,
) -> impl Iterator<Item = wgpu::BindGroupLayoutEntry> {
    stage
        .chart_reads()
        .iter()
        .map(move |read| sampled_entry(read.binding, visibility))
}

/// Group entries for the charts `stage` samples, from the same table.
pub(super) fn chart_entries(
    stage: AtmosphereStage,
    views: &Charts<wgpu::TextureView>,
) -> Vec<wgpu::BindGroupEntry<'_>> {
    stage
        .chart_reads()
        .iter()
        .map(|read| wgpu::BindGroupEntry {
            binding: read.binding,
            resource: wgpu::BindingResource::TextureView(views.get(read.chart)),
        })
        .collect()
}

fn prepare_compute(
    device: &crate::runtime::DeviceSite,
    stage: AtmosphereStage,
    uniform: &wgpu::Buffer,
    views: &Charts<wgpu::TextureView>,
    sampler: &wgpu::Sampler,
    shader: &wgpu::ShaderModule,
    night: &NightProfile,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let output = stage
        .chart_output()
        .expect("a chart stage writes its chart");
    let mut entries = vec![uniform_entry(visibility)];
    entries.extend(chart_layout(stage, visibility));
    let mut bindings = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform.as_entire_binding(),
    }];
    bindings.extend(chart_entries(stage, views));
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 3,
        visibility,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: wgpu::TextureFormat::Rgba16Float,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    });
    bindings.push(wgpu::BindGroupEntry {
        binding: 3,
        resource: wgpu::BindingResource::TextureView(views.get(output)),
    });
    if !stage.chart_reads().is_empty() {
        entries.push(sampler_entry(4, visibility));
        bindings.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }
    if stage == AtmosphereStage::SkyView {
        entries.extend(night::layout_entries());
        bindings.extend(night.bindings());
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere LUT closure"),
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere compute"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("atmosphere LUT"),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some(match output {
            Chart::Transmittance => "transmittance_main",
            Chart::MultipleScattering => "multiple_main",
            Chart::SkyView => "sky_main",
            Chart::Ground => "ground_main",
        }),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere LUT resources"),
        layout: &layout,
        entries: &bindings,
    });
    (pipeline, group)
}

// The direction pair is checked by AtmosphereFrame. Resolve the arc angle from
// its cross product in f64: f32 dot-threshold arc helpers discard subpixel solar
// steps. The existing f32 quaternion/matrix still owns the GPU representation.
#[expect(
    clippy::cast_possible_truncation,
    reason = "unit axis and bounded angle enter the f32 GPU record"
)]
fn solar_rotation(from: [f32; 3], to: [f32; 3]) -> glam::Mat4 {
    let a = from.map(f64::from);
    let b = to.map(f64::from);
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let sine = cross.iter().map(|v| v * v).sum::<f64>().sqrt();
    let rotation = if sine > 0.0 {
        let axis = glam::Vec3::from_array(cross.map(|v| (v / sine) as f32));
        let cosine = a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        glam::Quat::from_axis_angle(axis, sine.atan2(cosine) as f32)
    } else {
        glam::Quat::from_rotation_arc(glam::Vec3::from_array(from), glam::Vec3::from_array(to))
    };
    glam::Mat4::from_quat(rotation)
}

fn prepare_sky(
    device: &crate::runtime::DeviceSite,
    uniform: &wgpu::Buffer,
    motion: &wgpu::Buffer,
    views: &Charts<wgpu::TextureView>,
    sampler: &wgpu::Sampler,
    exposure_samples: &[wgpu::Buffer; 2],
) -> (wgpu::RenderPipeline, [wgpu::BindGroup; 2]) {
    let visibility = wgpu::ShaderStages::FRAGMENT;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere sky resources"),
        entries: &[
            uniform_entry(visibility),
            sampled_entry(1, visibility),
            sampled_entry(2, visibility),
            sampler_entry(3, visibility),
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size_of::<[f32; 4]>() as u64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(SKY_MOTION_BYTES),
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere sky"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader = crate::shaders::Shader::AtmosphereSky.compile(device, "atmosphere sky");
    let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
        label: Some("atmosphere background in forward pass"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        multiview_mask: None,
        cache: None,
    });
    let groups = exposure_samples.each_ref().map(|sample| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere sky inputs"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: motion.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views.transmittance),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views.sky_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sample.as_entire_binding(),
                },
            ],
        })
    });
    (pipeline, groups)
}

pub(crate) fn lut_source() -> String {
    EARTH.shader(LUT_SHADER)
}
