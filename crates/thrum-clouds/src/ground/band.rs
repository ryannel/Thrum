use super::{GroundConfig, GroundField};
use crate::{Error, SunPath, layer};
use thrum_render::{
    atmosphere::{GPU_CLOUD_PLANE_SHADER_SOURCE, GroundBand, GroundBandMapping},
    binding,
    contribution::SceneSurface,
    error::RenderError,
};

pub(crate) const PASS: &str = "cloud ground band";
const SPACING_M: f32 = 80.0;
const WIDTH_M: f32 = 5120.0;
const HEIGHT_STEP_M: f32 = 12.0;
const MIN_INTERVALS: f32 = 8.0;
const PORTABLE_MAX_LAYERS: u32 = 256;

fn top_m([terrain_min, fog_top, terrain_max, _]: [f32; 4]) -> f32 {
    // A ray from any top node must start above the highest fog it could
    // cross. Leave room for the surface heights stored as half floats.
    let padding = HEIGHT_STEP_M.max(terrain_min.abs().max(terrain_max.abs()) / 2048.0);
    let intervals = ((fog_top - terrain_min + padding) / HEIGHT_STEP_M).ceil();
    intervals.max(MIN_INTERVALS) * HEIGHT_STEP_M
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the slice count is rounded and bounded by the portable texture limit"
)]
pub(crate) fn provider(config: &GroundConfig) -> Result<GroundBand, RenderError> {
    let top = top_m(config.heights());
    let layers = (top / HEIGHT_STEP_M + 1.0).min(PORTABLE_MAX_LAYERS as f32) as u32;
    GroundBand::gpu_target([65, 65, layers], mapping([0.0; 2], top))
}

fn mapping(origin: [f32; 2], top_m: f32) -> GroundBandMapping {
    GroundBandMapping {
        region: [
            origin[0],
            origin[1],
            origin[0] + WIDTH_M,
            origin[1] + WIDTH_M,
        ],
        top_m,
        border_m: 4.0 * SPACING_M,
        padding: [0.0; 2],
    }
}

pub(crate) struct Band {
    pub(crate) texture: wgpu::Texture,
    pub(crate) mapping: wgpu::Buffer,
    pub(crate) view: wgpu::TextureView,
    top_m: f32,
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
        let (upper_plane, _) = surface.sun_path_target().ok_or(Error::Unsupported(
            "Ground requires a scene Sun-path target",
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
            binding::texture(
                9,
                stages,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
            ),
            binding::uniform(10, stages, sun_mapping.size()),
            binding::sampler(12, stages, wgpu::SamplerBindingType::Filtering),
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
        let upper_plane = upper_plane.create_view(&wgpu::TextureViewDescriptor::default());
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
                binding: 9,
                resource: wgpu::BindingResource::TextureView(&upper_plane),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: sun_mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 12,
                resource: wgpu::BindingResource::Sampler(&sun.shape_noise.sampler),
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
                    "{}\n{}\n{}\n{}\n{}",
                    GroundField::source(0, 0),
                    layer::sun_source(0),
                    include_str!("window.wgsl"),
                    GPU_CLOUD_PLANE_SHADER_SOURCE,
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
            top_m: top_m(field.heights()),
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
        let window = mapping(origin, self.top_m);
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
