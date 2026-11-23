use super::{Shapes, ShapesReport, shader, upload_changes};
use crate::{WeatherMap, WeatherProfile};
use thrum_render::{binding, runtime::DeviceSite};

pub(crate) const PASS: &str = "cloud placed weather map";

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MapParams {
    level: [[f32; 4]; 4],
}

pub(crate) struct ShapeMap {
    pub(crate) texture: wgpu::Texture,
    pub(crate) params: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    uploaded: MapParams,
    upload_bytes: usize,
    dirty: bool,
}

impl ShapeMap {
    pub(crate) fn new(device: &DeviceSite, weather: &WeatherProfile, shapes: &Shapes) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: weather.size,
                height: weather.size,
                depth_or_array_layers: 4,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("placed weather windows"),
            size: size_of::<MapParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let stage = wgpu::ShaderStages::COMPUTE;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                binding::uniform(0, stage, size_of::<MapParams>() as u64),
                binding::storage_texture(
                    1,
                    stage,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba32Float,
                    wgpu::TextureViewDimension::D2Array,
                ),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&layout), Some(&shapes.layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(
                format!("{}\n{}", shader(1), include_str!("map.wgsl")).into(),
            ),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("write_placed_map"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        Self {
            texture,
            params,
            pipeline,
            group,
            uploaded: MapParams {
                level: [[0.0; 4]; 4],
            },
            upload_bytes: 0,
            dirty: true,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "weather coordinates narrow relative to the innermost window"
    )]
    pub(crate) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        weather: &WeatherMap,
        instance_bytes: usize,
    ) {
        let origin = weather.origins()[0];
        let level: [_; 4] = std::array::from_fn(|i| {
            [
                (weather.origins()[i][0] - origin[0]) as f32,
                (weather.origins()[i][1] - origin[1]) as f32,
                weather.profile().texels[i] as f32,
                0.0,
            ]
        });
        self.upload_bytes =
            upload_changes(queue, &self.params, 0, &level, &mut self.uploaded.level);
        // Queue writes survive an abandoned recording, but the map dispatch
        // does not. Keep the map dirty until a submission has used the inputs.
        self.dirty |= self.upload_bytes != 0 || instance_bytes != 0;
    }

    pub(crate) fn runs(&self) -> bool {
        self.dirty
    }

    pub(crate) fn submitted(&mut self, report: &mut ShapesReport) {
        report.placement_upload_bytes += self.upload_bytes;
        report.placed_map_updated = std::mem::take(&mut self.dirty);
    }

    pub(crate) fn encode(&self, shapes: &Shapes, pass: &mut wgpu::ComputePass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_bind_group(1, &shapes.group, &[]);
        pass.dispatch_workgroups(
            self.texture.width().div_ceil(8),
            self.texture.height().div_ceil(8),
            4,
        );
    }
}
