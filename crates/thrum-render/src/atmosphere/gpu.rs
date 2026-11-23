//! Generation-bound LUTs and prepared portable pipelines. Encoding belongs to
//! the caller's existing compiled graph; this owner never submits GPU work.

use crate::binding;
use crate::error::RenderError;
use crate::graph::{CompiledPass, PipelineKey};
use crate::pass_key::{AtmosphereStage, PipelineRole, REFLECTION_MIPS};
use crate::shaders::wgsl;
use crate::timing::GpuTimings;
use crate::{render_config::DEPTH_FORMAT, synthetic::SyntheticView};

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
pub(super) mod aerosol;
pub(crate) use aerial_view::SecondaryAerial;
pub(crate) mod view_volume;

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
    pub(super) aerosol: Option<wgpu::Buffer>,
    // The four chart stages, each under the chart it integrates.
    chart_stages: Charts<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    aerosol_chart_stages: Option<Box<Charts<(wgpu::ComputePipeline, wgpu::BindGroup)>>>,
    sky_motion: wgpu::Buffer,
    submitted_sky: Option<(SyntheticView, [f32; 3])>,
    pending_sky: Option<(SyntheticView, [f32; 3])>,
    sky_pipeline: wgpu::RenderPipeline,
    sky_groups: [wgpu::BindGroup; 2],
    diffuse_pipeline: wgpu::ComputePipeline,
    diffuse_group: wgpu::BindGroup,
    aerial_pipeline: wgpu::ComputePipeline,
    aerial_group: wgpu::BindGroup,
    pub(crate) view_volume: Option<Box<view_volume::ViewVolume>>,
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
        let extent = if frame.view_volume() {
            view_volume::EXTENT
        } else {
            AERIAL_EXTENT
        };
        let aerial_textures = std::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atmosphere aerial volume"),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: extent[2],
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
        let aerosol = frame.aerosol.map(|sample| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("atmosphere aerosol"),
                size: aerosol::BYTES,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            aerosol::write(queue, &buffer, sample);
            buffer
        });
        let make_charts = |aerosol| {
            let shader = aerosol::compile(
                device,
                crate::shaders::Shader::AtmosphereLuts,
                "atmosphere LUT integration",
                aerosol,
            );
            Charts::from_fn(|chart| {
                let stage = AtmosphereStage::ORDER
                    .into_iter()
                    .find(|stage| stage.chart_output() == Some(chart))
                    .expect("every chart has its stage");
                prepare_compute(
                    device,
                    stage,
                    &uniform,
                    &views,
                    &sampler,
                    &shader,
                    (&night, aerosol),
                )
            })
        };
        let chart_stages = make_charts(None);
        let aerosol_chart_stages = aerosol
            .as_ref()
            .map(|buffer| Box::new(make_charts(Some(buffer))));
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
            (
                exposure_samples,
                frame.view_volume().then_some(&aerial_views[0]),
            ),
        );
        let (diffuse_pipeline, diffuse_group) =
            prepare_diffuse(device, &uniform, &views, &sampler, environment_light);
        let (aerial_pipeline, aerial_group) = prepare_aerial(
            device,
            &uniform,
            &views,
            &aerial_views,
            &sampler,
            &night,
            aerosol.as_ref(),
        );
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
            aerosol,
            chart_stages,
            aerosol_chart_stages,
            sky_motion,
            submitted_sky: None,
            pending_sky: None,
            sky_pipeline,
            sky_groups,
            diffuse_pipeline,
            diffuse_group,
            aerial_pipeline,
            aerial_group,
            view_volume: None,
            reflection,
            validity: 0,
            sky_dependencies: [0; 4],
            diffuse_dependencies: [0; 4],
            aerial_dependencies: [[0; 4]; 5],
        };
        result.update(queue, frame, view, [1, 1]);
        result
    }

    pub(crate) fn prepare_view_volume(
        &mut self,
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        view: &SyntheticView,
        cascades: bool,
        cloud: Option<(&super::SunOpticalDepth, &super::CloudPrepared)>,
    ) {
        if self.frame.view_volume() {
            let mut volume = view_volume::ViewVolume::new(device, self, cascades, cloud);
            volume.update(queue, self.frame, view);
            self.view_volume = Some(Box::new(volume));
        }
    }

    pub(crate) fn update(
        &mut self,
        queue: &wgpu::Queue,
        frame: AtmosphereFrame,
        view: &SyntheticView,
        extent: [u32; 2],
    ) {
        if let Some(aerosol) = &self.aerosol
            && frame.aerosol.unwrap_or_default() != self.frame.aerosol.unwrap_or_default()
        {
            self.validity &= DFG_VALID;
            aerosol::write(queue, aerosol, frame.aerosol.unwrap_or_default());
        }
        self.frame = frame;
        if let Some(volume) = &mut self.view_volume {
            volume.update(queue, frame, view);
        }
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

    /// Skip the geometry lighting products in the sky-only exposure fixture.
    /// Call after updating the atmosphere, with no active geometry or effects.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn omit_geometry_products_for_sky_test(&mut self) {
        self.validity |= AERIAL_VALID | REFLECTION_VALID | DFG_VALID;
    }

    pub(crate) fn commit(&mut self) {
        if let Some(volume) = &mut self.view_volume {
            volume.submitted();
        }
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

    pub(crate) fn accepts(&self, frame: AtmosphereFrame) -> bool {
        self.view_volume.is_some() == frame.view_volume()
            && (!frame.variable_aerosol() || self.aerosol.is_some())
    }

    #[cfg(feature = "creation-counters")]
    pub(crate) fn chart_witnesses(&self) -> [&wgpu::Texture; 4] {
        let charts = &self.textures;
        [
            &charts.transmittance,
            &charts.multiple_scattering,
            &charts.sky_view,
            &charts.ground,
        ]
    }

    pub(crate) fn reflection_sampled(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler) {
        self.reflection.sampled()
    }

    pub(crate) fn key(&self, stage: AtmosphereStage) -> PipelineKey {
        PipelineRole::Atmosphere(stage)
            .pipeline_key(self.generation, crate::render_config::SYNTHETIC_REVISION)
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

    pub(crate) fn prepare_volume_scene_group(
        &self,
        device: &wgpu::Device,
        shadow: Option<(&wgpu::Buffer, &wgpu::TextureView)>,
        cloud: Option<&super::CloudPrepared>,
    ) -> Option<wgpu::BindGroup> {
        self.view_volume.as_ref().map(|volume| {
            volume.prepare_scene_group(device, shadow, cloud, (&self.uniform, &self.aerial_sampler))
        })
    }

    pub(crate) fn volume_storage(&self) -> Option<super::ViewVolumeStorage> {
        self.view_volume.as_ref().map(|volume| volume.storage(self))
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
        let charts = self
            .aerosol_chart_stages
            .as_deref()
            .filter(|_| self.frame.aerosol.unwrap_or_default() != super::Aerosol::default())
            .unwrap_or(&self.chart_stages);
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
                let (pipeline, group) = &charts.ground;
                (pipeline, group, [Chart::Ground.extent()[0], 1, 1])
            }
            AtmosphereStage::Transmittance
            | AtmosphereStage::MultipleScattering
            | AtmosphereStage::SkyView => {
                let chart = stage
                    .chart_output()
                    .expect("a chart stage writes its chart");
                let (pipeline, group) = charts.get(chart);
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
            binding::uniform(0, visibility, UNIFORM_BYTES),
            binding::sampler(2, visibility, wgpu::SamplerBindingType::Filtering),
            binding::storage(3, visibility, false, crate::light::ENVIRONMENT_BYTES),
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
    aerosol: Option<&wgpu::Buffer>,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let output = |binding| {
        binding::storage_texture(
            binding,
            visibility,
            wgpu::StorageTextureAccess::WriteOnly,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureViewDimension::D2Array,
        )
    };
    let entries = [
        binding::uniform(0, visibility, UNIFORM_BYTES),
        output(3),
        output(4),
        binding::sampler(5, visibility, wgpu::SamplerBindingType::Filtering),
    ];
    let entries = entries
        .into_iter()
        .chain(chart_layout(AtmosphereStage::Aerial, visibility))
        .chain(night::layout_entries())
        .chain(aerosol.map(|_| aerosol::layout()))
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
    let shader = aerosol::compile(
        device,
        crate::shaders::Shader::AtmosphereAerial,
        "atmosphere aerial integration",
        aerosol,
    );
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
        .chain(aerosol.map(aerosol::entry))
        .collect::<Vec<_>>();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere aerial resources"),
        layout: &layout,
        entries: &bindings,
    });
    (pipeline, group)
}

/// Layout entries for the charts `stage` samples, from its one table.
fn chart_layout(
    stage: AtmosphereStage,
    visibility: wgpu::ShaderStages,
) -> impl Iterator<Item = wgpu::BindGroupLayoutEntry> {
    stage.chart_reads().iter().map(move |read| {
        binding::texture(
            read.binding,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
        )
    })
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
    optics: (&NightProfile, Option<&wgpu::Buffer>),
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let (night, aerosol) = optics;
    let visibility = wgpu::ShaderStages::COMPUTE;
    let output = stage
        .chart_output()
        .expect("a chart stage writes its chart");
    let mut entries = vec![binding::uniform(0, visibility, UNIFORM_BYTES)];
    entries.extend(chart_layout(stage, visibility));
    let mut bindings = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform.as_entire_binding(),
    }];
    bindings.extend(chart_entries(stage, views));
    entries.push(binding::storage_texture(
        3,
        visibility,
        wgpu::StorageTextureAccess::WriteOnly,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureViewDimension::D2,
    ));
    bindings.push(wgpu::BindGroupEntry {
        binding: 3,
        resource: wgpu::BindingResource::TextureView(views.get(output)),
    });
    if !stage.chart_reads().is_empty() {
        entries.push(binding::sampler(
            4,
            visibility,
            wgpu::SamplerBindingType::Filtering,
        ));
        bindings.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }
    if stage == AtmosphereStage::SkyView {
        entries.extend(night::layout_entries());
        bindings.extend(night.bindings());
    }
    if let Some(aerosol) = aerosol {
        entries.push(aerosol::layout());
        bindings.push(aerosol::entry(aerosol));
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
    sky_output: (&[wgpu::Buffer; 2], Option<&wgpu::TextureView>),
) -> (wgpu::RenderPipeline, [wgpu::BindGroup; 2]) {
    let (exposure_samples, volume) = sky_output;
    let visibility = wgpu::ShaderStages::FRAGMENT;
    let mut entries = vec![
        binding::uniform(0, visibility, UNIFORM_BYTES),
        binding::texture(
            1,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
        ),
        binding::texture(
            2,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
        ),
        binding::sampler(3, visibility, wgpu::SamplerBindingType::Filtering),
        binding::uniform(4, visibility, size_of::<[f32; 4]>() as u64),
        binding::uniform(5, visibility, SKY_MOTION_BYTES),
    ];
    if volume.is_some() {
        entries.push(binding::texture(
            6,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2Array,
        ));
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere sky resources"),
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere sky"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let base = crate::shaders::Shader::AtmosphereSky;
    let shader = if volume.is_some() {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("atmosphere sky with view volume"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    wgsl!("atmosphere/storage.wgsl"),
                    wgsl!("atmosphere/geometry.wgsl"),
                    wgsl!("atmosphere/common.wgsl"),
                    wgsl!("atmosphere/volume_sky.wgsl")
                )
                .into(),
            ),
        })
    } else {
        base.compile(device, "atmosphere sky")
    };
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
        let mut entries = vec![
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
        ];
        if let Some(view) = volume {
            entries.push(wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere sky inputs"),
            layout: &layout,
            entries: &entries,
        })
    });
    (pipeline, groups)
}

pub(crate) fn lut_source() -> String {
    EARTH.shader(LUT_SHADER)
}
