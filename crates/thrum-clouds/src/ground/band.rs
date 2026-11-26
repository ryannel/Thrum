use super::GroundField;
use crate::{Error, SunPath, layer};
use thrum_render::{
    atmosphere::{GroundBand, GroundBandMapping},
    binding,
    contribution::SceneSurface,
    error::RenderError,
};

pub(crate) const PASS: &str = "cloud ground band";
const SIZE: [u32; 3] = [65, 65, 9];
const SPACING_M: f32 = 80.0;
const WIDTH_M: f32 = 5120.0;
const TOP_M: f32 = 96.0;

pub(crate) fn provider() -> Result<GroundBand, RenderError> {
    GroundBand::gpu_target(SIZE, mapping([0.0; 2]))
}

fn mapping(origin: [f32; 2]) -> GroundBandMapping {
    GroundBandMapping {
        region: [
            origin[0],
            origin[1],
            origin[0] + WIDTH_M,
            origin[1] + WIDTH_M,
        ],
        top_m: TOP_M,
        border_m: 4.0 * SPACING_M,
        padding: [0.0; 2],
    }
}

pub(crate) struct Band {
    pub(crate) texture: wgpu::Texture,
    pub(crate) mapping: wgpu::Buffer,
    pub(crate) view: wgpu::TextureView,
    group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}

impl Band {
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        field: &GroundField,
        sun: &SunPath,
        sun_mapping: &wgpu::Buffer,
    ) -> Result<Self, Error> {
        let (texture, mapping) = surface.ground_band_target().ok_or(Error::Unsupported(
            "Ground requires a scene ground-band target",
        ))?;
        let device = surface.device;
        let stages = wgpu::ShaderStages::COMPUTE;
        let mut layout = GroundField::layout(0).to_vec();
        layout.extend([
            binding::storage_texture(
                3,
                stages,
                wgpu::StorageTextureAccess::WriteOnly,
                texture.format(),
                wgpu::TextureViewDimension::D3,
            ),
            binding::uniform(4, stages, mapping.size()),
            binding::uniform(8, stages, sun_mapping.size()),
            binding::sampler(13, stages, wgpu::SamplerBindingType::Filtering),
        ]);
        layout.extend((5..8).map(|slot| {
            binding::texture(
                slot,
                stages,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2Array,
            )
        }));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &layout,
        });
        let output = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let products = sun.products().each_ref().map(|texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        });
        let mut entries = field.entries(0).to_vec();
        entries.extend([
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&output),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: sun_mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 13,
                resource: wgpu::BindingResource::Sampler(&sun.shape_noise.sampler),
            },
        ]);
        entries.extend(
            (5..8)
                .zip(&products)
                .map(|(binding, view)| wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::TextureView(view),
                }),
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}",
                    GroundField::source(0, 0),
                    layer::sun_source(0),
                    include_str!("window.wgsl"),
                    include_str!("band.wgsl"),
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("fill_ground_band"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            texture,
            mapping,
            view: output,
            group,
            pipeline,
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "compute display-relative mapping in f64 first"
    )]
    pub(crate) fn upload(&self, queue: &wgpu::Queue, camera: [f64; 2], travel: [f64; 2]) {
        let snap = f64::from(SPACING_M);
        let origin = std::array::from_fn(|i| {
            (((camera[i] - travel[i]) / snap).round() * snap - 0.5 * f64::from(WIDTH_M) + travel[i])
                as f32
        });
        let window = mapping(origin);
        queue.write_buffer(&self.mapping, 0, bytemuck::bytes_of(&window));
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        let extent = self.texture.size();
        pass.dispatch_workgroups(
            extent.width.div_ceil(8),
            extent.height.div_ceil(8),
            extent.depth_or_array_layers,
        );
    }
}
