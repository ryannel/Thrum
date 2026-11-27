//! The signed, reduced-resolution remainder of the underwater Sun transport.

use std::sync::{Arc, Mutex};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    binding,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, EffectAccess, EffectCadence, EffectGraph, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

const PASS: &str = "underwater Sun shafts";
const IMAGE: u32 = 27;
const OUTPUT: u32 = 28;
const FIELD: u32 = 29;
const SAMPLER: u32 = 30;
const MAPPING: u32 = 31;
const COLUMNS: u32 = 32;
const CAPACITY: [u32; 2] = [1280, 720];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Diagnostic access to the submitted signed shafts texture.
#[derive(Clone, Default)]
pub struct ShaftTap(pub(crate) Arc<Mutex<Option<wgpu::Texture>>>);

impl ShaftTap {
    /// The generation's reduced signed RGB radiance and path length in alpha.
    /// # Panics
    /// Panics if the diagnostic tap's lock was poisoned.
    #[must_use]
    pub fn texture(&self) -> Option<wgpu::Texture> {
        self.0.lock().expect("shaft tap").clone()
    }
}

pub(crate) struct Image {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Image {
    pub(crate) fn new(device: &thrum_render::runtime::DeviceSite) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: CAPACITY[0],
                height: CAPACITY[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }

    pub(crate) fn read_layout(mapping: &wgpu::Buffer) -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            binding::texture(
                IMAGE,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
            binding::uniform(MAPPING, wgpu::ShaderStages::FRAGMENT, mapping.size()),
        ]
    }

    pub(crate) fn read_entries<'a>(
        &'a self,
        mapping: &'a wgpu::Buffer,
    ) -> [wgpu::BindGroupEntry<'a>; 2] {
        [
            texture(IMAGE, &self.view),
            wgpu::BindGroupEntry {
                binding: MAPPING,
                resource: mapping.as_entire_binding(),
            },
        ]
    }
}

pub(crate) struct Shafts {
    image: Image,
    pipeline: wgpu::ComputePipeline,
    inputs: wgpu::BindGroup,
}

fn texture(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

impl Shafts {
    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.image.texture
    }

    pub(crate) fn bytes(&self) -> u64 {
        u64::from(self.image.texture.width()) * u64::from(self.image.texture.height()) * 8
    }

    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        shader: &wgpu::ShaderModule,
        image: Image,
        environment: &EnvironmentProducts,
        optics: &wgpu::Buffer,
        water: &wgpu::Buffer,
        field: &thrum_render::sun_path::SunPathTarget,
    ) -> Self {
        let device = surface.device;
        let stage = wgpu::ShaderStages::COMPUTE;
        let output = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                binding::uniform(0, stage, water.size()),
                crate::optics::uniform(stage),
                binding::storage_texture(
                    OUTPUT,
                    stage,
                    wgpu::StorageTextureAccess::WriteOnly,
                    FORMAT,
                    wgpu::TextureViewDimension::D2,
                ),
                binding::texture(
                    FIELD,
                    stage,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2Array,
                ),
                binding::sampler(SAMPLER, stage, wgpu::SamplerBindingType::Filtering),
                binding::uniform(MAPPING, stage, field.mapping.size()),
                binding::texture(
                    COLUMNS,
                    stage,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2,
                ),
            ],
        });
        let field_view = field.field.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let column_view = field
            .columns
            .create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(PASS),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let inputs = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &output,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: water.as_entire_binding(),
                },
                crate::optics::entry(optics),
                texture(OUTPUT, &image.view),
                texture(FIELD, &field_view),
                texture(COLUMNS, &column_view),
                wgpu::BindGroupEntry {
                    binding: SAMPLER,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: MAPPING,
                    resource: field.mapping.as_entire_binding(),
                },
            ],
        });
        let sampled = Self::declaration().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&sampled),
                Some(&environment.layout),
                Some(&output),
            ],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&layout),
            module: shader,
            entry_point: Some("march_shafts"),
            cache: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });
        Self {
            image,
            pipeline,
            inputs,
        }
    }

    fn declaration() -> AfterOpaquePass {
        AfterOpaquePass {
            name: PASS.into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::Skippable,
            accesses: Vec::new(),
            depth: false,
            write_depth: false,
            refraction: false,
            images: crate::optics::sampled(0),
        }
    }

    pub(crate) fn declare(
        &self,
        graph: &mut EffectGraph,
        field: [EffectAccess; 2],
    ) -> EffectAccess {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.image.texture),
            ResourceScope::Persistent {
                owner: 0x7368_6166,
                epoch: 0,
                initialized: false,
            },
        ));
        let read = EffectAccess {
            resource,
            range: 0..1,
            kind: AccessKind::SampledRead,
            view: Some(wgpu::TextureViewDimension::D2),
            history: None,
        };
        let mut pass = Self::declaration();
        pass.accesses.extend(field);
        pass.accesses.push(EffectAccess {
            kind: AccessKind::StorageWrite,
            ..read.clone()
        });
        graph.after_opaque.push(pass);
        read
    }

    pub(crate) fn encode(
        &self,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &thrum_render::contribution::ContributionFrame,
        environment: &wgpu::BindGroup,
        divisor: u32,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, environment, &[]);
        pass.set_bind_group(3, &self.inputs, &[]);
        let [x, y] = std::array::from_fn(|axis| {
            frame.render_extent[axis]
                .div_ceil(divisor)
                .min(CAPACITY[axis])
                .div_ceil(8)
        });
        pass.dispatch_workgroups(x, y, 1);
    }
}

pub(crate) fn shader(surface: &SceneSurface<'_>) -> String {
    include_str!("../underwater.wgsl")
        .replace(
            "return water_path(body, light, length_m, start_depth, end_depth);",
            "return water_path(body, light, length_m, start_depth, end_depth)
                + shaft_scattering(uv, eye.surface, ray, length_m, start_depth, end_depth);",
        )
        .replace(
            "max(0.0, eye.surface - view.camera_xyz_time.y), 0.0);",
            "max(0.0, eye.surface - view.camera_xyz_time.y), 0.0)
            + shaft_scattering(optics_project(world).xy, eye.surface, -v, length(world - view.camera_xyz_time.xyz),
                max(0.0, eye.surface - view.camera_xyz_time.y), 0.0);",
        )
        + include_str!("refraction.wgsl")
        + include_str!("sampling.wgsl")
        + include_str!("shaft_transport.wgsl")
        + include_str!("shafts.wgsl")
        + surface.sun_path_visibility_shader()
}
