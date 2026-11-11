//! Generation-bound LUTs and prepared portable pipelines. Encoding belongs to
//! the caller's existing compiled graph; this owner never submits GPU work.

use crate::graph::{CompiledPass, PipelineKey};
use crate::synthetic::{DEPTH_FORMAT, RenderError, SyntheticView};
use crate::timing::GpuTimings;

use super::{
    AtmosphereFrame,
    night::{self, NightProfile},
    reflection::ReflectionPrepared,
    spectrum::EARTH,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub(super) const EXTENTS: [[u32; 2]; 4] = [[256, 64], [512, 32], [192, 108], [128, 1]];
const LUT_PLANES: [u32; 4] = [3, 2, 1, 2];
pub(super) const AERIAL_EXTENT: [u32; 3] = [32, 32, 33];
const LUT_SHADER: &str = concat!(
    concat!(
        include_str!("storage.wgsl"),
        "\n",
        concat!(
            include_str!("geometry.wgsl"),
            "\n",
            include_str!("common.wgsl")
        )
    ),
    "\n",
    include_str!("transport.wgsl"),
    "\n",
    include_str!("night.wgsl"),
    "\n",
    include_str!("lut.wgsl"),
    "\n",
    include_str!("ground.wgsl")
);
const UNIFORM_LEN: usize = 128;
pub(super) const UNIFORM_BYTES: u64 = UNIFORM_LEN as u64;
const TRANSPORT_VALID: u8 = 1;
const SKY_VALID: u8 = 2;
const DIFFUSE_VALID: u8 = 4;
const AERIAL_VALID: u8 = 8;
const GROUND_VALID: u8 = 16;
const REFLECTION_VALID: u8 = 32;
const DFG_VALID: u8 = 64;

pub(crate) struct AtmospherePrepared {
    pub(super) generation: u64,
    pub(super) epoch: u64,
    pub(super) textures: [wgpu::Texture; 4],
    pub(super) night: NightProfile,
    pub(super) aerial_textures: [wgpu::Texture; 2],
    aerial_views: [wgpu::TextureView; 2],
    transmittance_view: wgpu::TextureView,
    aerial_sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    compute: [wgpu::ComputePipeline; 4],
    groups: [wgpu::BindGroup; 4],
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
    aerial_dependencies: [u8; 80],
}

impl AtmospherePrepared {
    pub(crate) fn observer_uniform(&self) -> &wgpu::Buffer {
        &self.uniform
    }

    pub(crate) fn transmittance_view(&self) -> &wgpu::TextureView {
        &self.transmittance_view
    }

    pub(crate) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        generation: u64,
        frame: AtmosphereFrame,
        view: &SyntheticView,
        directional: &wgpu::Buffer,
        exposure_samples: &[wgpu::Buffer; 2],
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere frame"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let textures = std::array::from_fn(|index| {
            let [width, height] = EXTENTS[index];
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atmosphere persistent LUT"),
                size: wgpu::Extent3d {
                    width,
                    height: height * LUT_PLANES[index],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        });
        let views = textures
            .each_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
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
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("atmosphere LUT integration"),
            source: wgpu::ShaderSource::Wgsl(EARTH.shader(LUT_SHADER).into()),
        });
        let prepared: [(wgpu::ComputePipeline, wgpu::BindGroup); 4] =
            std::array::from_fn(|index| {
                prepare_compute(device, index, &uniform, &views, &sampler, &shader, &night)
            });
        let [first, second, third, ground] = prepared;
        let sky_motion = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky submitted motion"),
            size: 208,
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
            prepare_diffuse(device, &uniform, &views[2], &sampler, directional);
        let (aerial_pipeline, aerial_group) =
            prepare_aerial(device, &uniform, &views, &aerial_views, &sampler, &night);
        let reflection = ReflectionPrepared::new(device, &uniform, &views[2], &sampler);
        let mut result = Self {
            generation,
            epoch: frame.profile_epoch,
            textures,
            night,
            aerial_textures,
            aerial_views,
            transmittance_view: views[0].clone(),
            aerial_sampler,
            uniform,
            compute: [first.0, second.0, third.0, ground.0],
            groups: [first.1, second.1, third.1, ground.1],
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
            aerial_dependencies: [0; 80],
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
        let mut motion = [0_u8; 208];
        for (destination, value) in motion.as_chunks_mut::<4>().0.iter_mut().zip(
            (prior - current)
                .to_cols_array()
                .into_iter()
                .chain(current.to_cols_array())
                .chain([extent[0] as f32, extent[1] as f32, 0.0, 0.0])
                .chain((prior * rotation - current).to_cols_array()),
        ) {
            destination.copy_from_slice(&value.to_le_bytes());
        }
        queue.write_buffer(&self.sky_motion, 0, &motion);
        self.pending_sky = Some((*view, frame.toward_sun));
        let mut bytes = [0_u8; UNIFORM_LEN];
        let aux = view.pack_aux(extent[0], extent[1]);
        bytes[..64].copy_from_slice(&aux[..64]);
        let packed_view = view.pack_view(view, 1.0);
        bytes[64..76].copy_from_slice(&packed_view[128..140]);
        bytes[76..80].copy_from_slice(&frame.camera_altitude_m.to_le_bytes());
        for (index, value) in frame
            .toward_sun
            .into_iter()
            .chain([frame.solar_illuminance_lux])
            .chain(frame.solar_colour)
            .chain([super::super::synthetic::PRE_EXPOSURE])
            .enumerate()
        {
            bytes[80 + index * 4..84 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in super::model::sky_mapping(frame.camera_altitude_m)
            .into_iter()
            .enumerate()
        {
            bytes[112 + index * 4..116 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        let mut aerial_dependencies = [0_u8; 80];
        aerial_dependencies[..64].copy_from_slice(&bytes[..64]);
        aerial_dependencies[64..80].copy_from_slice(&bytes[64..80]);
        if self.validity & SKY_VALID == 0 || aerial_dependencies != self.aerial_dependencies {
            self.validity &= !AERIAL_VALID;
            self.aerial_dependencies = aerial_dependencies;
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
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

    pub(crate) fn pass_count(&self) -> u32 {
        7 + u32::try_from(self.reflection.mip_count())
            .expect("prepared reflection mip count fits u32")
    }

    pub(crate) fn stage_index(&self, role: u32) -> Option<usize> {
        let index = role.checked_sub(10)?;
        (index < self.pass_count()).then(|| usize::try_from(index).expect("bounded stage index"))
    }

    pub(crate) fn reflection_sampled(
        &self,
    ) -> (&wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler) {
        self.reflection.sampled()
    }

    pub(crate) fn key(&self, index: u32) -> PipelineKey {
        PipelineKey {
            device_generation: self.generation,
            revision: super::super::synthetic::SYNTHETIC_REVISION,
            role: 10 + index,
        }
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
        index: usize,
        encoder: &mut wgpu::CommandEncoder,
        pass: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        // Cached dependencies retain their graph identity without adding empty
        // GPU passes or timestamp work to an ordinary frame.
        if (index < 2 && self.validity & TRANSPORT_VALID != 0)
            || (index == 2 && self.validity & SKY_VALID != 0)
            || (index == 3 && self.validity & DIFFUSE_VALID != 0)
            || (index == 4 && self.validity & AERIAL_VALID != 0)
            || (index == 5 && self.validity & GROUND_VALID != 0)
            || (index == 6 && self.validity & DFG_VALID != 0)
            || (index > 6 && self.validity & REFLECTION_VALID != 0)
        {
            return Ok(());
        }
        let mut compute = pass
            .begin_compute(encoder, self.generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        if index >= 6 {
            return self.reflection.encode(index - 6, &mut compute);
        }
        if index == 3 {
            compute.set_pipeline(&self.diffuse_pipeline);
            compute.set_bind_group(0, &self.diffuse_group, &[]);
            compute.dispatch_workgroups(1, 1, 1);
            return Ok(());
        }
        if index == 4 {
            compute.set_pipeline(&self.aerial_pipeline);
            compute.set_bind_group(0, &self.aerial_group, &[]);
            compute.dispatch_workgroups(
                AERIAL_EXTENT[0].div_ceil(8),
                AERIAL_EXTENT[1].div_ceil(8),
                1,
            );
            return Ok(());
        }
        if index == 5 {
            compute.set_pipeline(&self.compute[3]);
            compute.set_bind_group(0, &self.groups[3], &[]);
            compute.dispatch_workgroups(EXTENTS[3][0], 1, 1);
            return Ok(());
        }
        compute.set_pipeline(&self.compute[index]);
        compute.set_bind_group(0, &self.groups[index], &[]);
        let [width, height] = EXTENTS[index];
        compute.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
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
    sky: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    directional: &wgpu::Buffer,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        entries: &[
            uniform_entry(visibility),
            sampled_entry(1, visibility),
            sampler_entry(2, visibility),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(crate::synthetic::LIGHT_BYTES as u64),
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere diffuse irradiance"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("atmosphere diffuse hemisphere integration"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                concat!(
                    include_str!("storage.wgsl"),
                    "\n",
                    concat!(
                        include_str!("geometry.wgsl"),
                        "\n",
                        include_str!("common.wgsl")
                    )
                ),
                "\n",
                include_str!("irradiance.wgsl"),
                "\n",
                include_str!("diffuse.wgsl")
            )
            .into(),
        ),
    });
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
                binding: 1,
                resource: wgpu::BindingResource::TextureView(sky),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: directional.as_entire_binding(),
            },
        ],
    });
    (pipeline, group)
}

fn prepare_aerial(
    device: &crate::runtime::DeviceSite,
    uniform: &wgpu::Buffer,
    views: &[wgpu::TextureView; 4],
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
        sampled_entry(1, visibility),
        sampled_entry(2, visibility),
        output(3),
        output(4),
        sampler_entry(5, visibility),
    ];
    let entries = entries
        .into_iter()
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
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("atmosphere aerial integration"),
        source: wgpu::ShaderSource::Wgsl(
            EARTH
                .shader(concat!(
                    include_str!("storage.wgsl"),
                    "\n",
                    include_str!("geometry.wgsl"),
                    "\n",
                    include_str!("common.wgsl"),
                    "\n",
                    include_str!("transport.wgsl"),
                    "\n",
                    include_str!("night.wgsl"),
                    "\n",
                    include_str!("aerial.wgsl")
                ))
                .into(),
        ),
    });
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
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&views[0]),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::TextureView(&views[1]),
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

fn prepare_compute(
    device: &crate::runtime::DeviceSite,
    index: usize,
    uniform: &wgpu::Buffer,
    views: &[wgpu::TextureView; 4],
    sampler: &wgpu::Sampler,
    shader: &wgpu::ShaderModule,
    night: &NightProfile,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let mut entries = vec![uniform_entry(visibility)];
    let mut bindings = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform.as_entire_binding(),
    }];
    let inputs: &[usize] = match index {
        0 => &[],
        1 => &[0],
        2 => &[0, 1, 3],
        _ => &[0, 1],
    };
    for &input in inputs {
        let binding = if input == 3 {
            5
        } else {
            u32::try_from(input + 1).expect("two transport inputs")
        };
        entries.push(sampled_entry(binding, visibility));
        bindings.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&views[input]),
        });
    }
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
        resource: wgpu::BindingResource::TextureView(&views[index]),
    });
    if index > 0 {
        entries.push(sampler_entry(4, visibility));
        bindings.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }
    if index == 2 {
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
        entry_point: Some(
            [
                "transmittance_main",
                "multiple_main",
                "sky_main",
                "ground_main",
            ][index],
        ),
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
    views: &[wgpu::TextureView; 4],
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
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(208),
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
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("atmosphere sky"),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                concat!(
                    include_str!("storage.wgsl"),
                    "\n",
                    concat!(
                        include_str!("geometry.wgsl"),
                        "\n",
                        include_str!("common.wgsl")
                    )
                ),
                "\n",
                include_str!("sky.wgsl")
            )
            .into(),
        ),
    });
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
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
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
