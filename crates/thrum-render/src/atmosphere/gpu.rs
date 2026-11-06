//! Generation-bound LUTs and prepared portable pipelines. Encoding belongs to
//! the caller's existing compiled graph; this owner never submits GPU work.

use crate::graph::{CompiledPass, PipelineKey};
use crate::synthetic::{DEPTH_FORMAT, RenderError, SyntheticView};
use crate::timing::GpuTimings;

use super::AtmosphereFrame;

pub(super) const EXTENTS: [[u32; 2]; 3] = [[256, 64], [32, 32], [192, 108]];
const UNIFORM_BYTES: u64 = 128;

pub(crate) struct AtmospherePrepared {
    pub(super) generation: u64,
    pub(super) epoch: u64,
    pub(super) textures: [wgpu::Texture; 3],
    uniform: wgpu::Buffer,
    compute: [wgpu::ComputePipeline; 3],
    groups: [wgpu::BindGroup; 3],
    sky_pipeline: wgpu::RenderPipeline,
    sky_group: wgpu::BindGroup,
    diffuse_pipeline: wgpu::ComputePipeline,
    diffuse_group: wgpu::BindGroup,
    cache_valid: bool,
    sky_valid: bool,
    sky_dependencies: [u32; 4],
    diffuse_valid: bool,
    diffuse_dependencies: [u32; 4],
}

impl AtmospherePrepared {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        frame: AtmosphereFrame,
        view: &SyntheticView,
        directional: &wgpu::Buffer,
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere frame"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let textures = EXTENTS.map(|[width, height]| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("atmosphere persistent LUT"),
                size: wgpu::Extent3d {
                    width,
                    height,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("atmosphere LUT integration"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("common.wgsl"), "\n", include_str!("lut.wgsl")).into(),
            ),
        });
        let prepared: [(wgpu::ComputePipeline, wgpu::BindGroup); 3] =
            std::array::from_fn(|index| {
                prepare_compute(device, index, &uniform, &views, &sampler, &shader)
            });
        let [first, second, third] = prepared;
        let (sky_pipeline, sky_group) = prepare_sky(device, &uniform, &views, &sampler);
        let (diffuse_pipeline, diffuse_group) =
            prepare_diffuse(device, &uniform, &views[2], &sampler, directional);
        let mut result = Self {
            generation,
            epoch: frame.profile_epoch,
            textures,
            uniform,
            compute: [first.0, second.0, third.0],
            groups: [first.1, second.1, third.1],
            sky_pipeline,
            sky_group,
            diffuse_pipeline,
            diffuse_group,
            cache_valid: false,
            sky_valid: false,
            sky_dependencies: [0; 4],
            diffuse_valid: false,
            diffuse_dependencies: [0; 4],
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
            self.cache_valid = false;
            self.sky_valid = false;
            self.epoch = frame.profile_epoch;
        }
        let dependencies = [
            frame.toward_sun[0].to_bits(),
            frame.toward_sun[1].to_bits(),
            frame.toward_sun[2].to_bits(),
            frame.camera_altitude_m.to_bits(),
        ];
        if dependencies != self.sky_dependencies {
            self.sky_valid = false;
            self.sky_dependencies = dependencies;
        }
        let diffuse_dependencies = [
            frame.solar_illuminance_lux.to_bits(),
            frame.solar_colour[0].to_bits(),
            frame.solar_colour[1].to_bits(),
            frame.solar_colour[2].to_bits(),
        ];
        if !self.sky_valid || diffuse_dependencies != self.diffuse_dependencies {
            self.diffuse_valid = false;
            self.diffuse_dependencies = diffuse_dependencies;
        }
        let mut bytes = [0_u8; 128];
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
        bytes[112..128].copy_from_slice(&aux[64..80]);
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    pub(crate) fn commit(&mut self) {
        self.cache_valid = true;
        self.sky_valid = true;
        self.diffuse_valid = true;
    }

    pub(crate) fn key(&self, index: u32) -> PipelineKey {
        PipelineKey {
            device_generation: self.generation,
            revision: super::super::synthetic::SYNTHETIC_REVISION,
            role: 10 + index,
        }
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
        if (index < 2 && self.cache_valid)
            || (index == 2 && self.sky_valid)
            || (index == 3 && self.diffuse_valid)
        {
            return Ok(());
        }
        let mut compute = pass
            .begin_compute(encoder, self.generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        if index == 3 {
            compute.set_pipeline(&self.diffuse_pipeline);
            compute.set_bind_group(0, &self.diffuse_group, &[]);
            compute.dispatch_workgroups(1, 1, 1);
            return Ok(());
        }
        compute.set_pipeline(&self.compute[index]);
        compute.set_bind_group(0, &self.groups[index], &[]);
        let [width, height] = EXTENTS[index];
        compute.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        Ok(())
    }

    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.sky_pipeline);
        pass.set_bind_group(0, &self.sky_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn prepare_diffuse(
    device: &wgpu::Device,
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
                    min_binding_size: wgpu::BufferSize::new(64),
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
                include_str!("common.wgsl"),
                "\n",
                include_str!("diffuse.wgsl")
            )
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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

fn sampled_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
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
    device: &wgpu::Device,
    index: usize,
    uniform: &wgpu::Buffer,
    views: &[wgpu::TextureView; 3],
    sampler: &wgpu::Sampler,
    shader: &wgpu::ShaderModule,
) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::COMPUTE;
    let mut entries = vec![uniform_entry(visibility)];
    let mut bindings = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform.as_entire_binding(),
    }];
    for (input, view) in views.iter().enumerate().take(index) {
        let binding = u32::try_from(input + 1).expect("at most two LUT inputs");
        entries.push(sampled_entry(binding, visibility));
        bindings.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
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
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere LUT closure"),
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("atmosphere compute"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("atmosphere LUT"),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some(["transmittance_main", "multiple_main", "sky_main"][index]),
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

fn prepare_sky(
    device: &wgpu::Device,
    uniform: &wgpu::Buffer,
    views: &[wgpu::TextureView; 3],
    sampler: &wgpu::Sampler,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let visibility = wgpu::ShaderStages::FRAGMENT;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atmosphere sky resources"),
        entries: &[
            uniform_entry(visibility),
            sampled_entry(1, visibility),
            sampled_entry(2, visibility),
            sampler_entry(3, visibility),
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
            concat!(include_str!("common.wgsl"), "\n", include_str!("sky.wgsl")).into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("atmosphere sky inputs"),
        layout: &layout,
        entries: &[
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
        ],
    });
    (pipeline, group)
}
