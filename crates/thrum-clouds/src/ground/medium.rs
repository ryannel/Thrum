use super::{
    GroundField,
    band::Band,
    lighting::{FIRST, LightTable},
};
use crate::{Error, SunPath, WeatherMap, layer};
use thrum_render::{
    atmosphere::{EnvironmentProducts, ViewVolumeMediumTarget},
    binding,
    contribution::SceneSurface,
};

pub(crate) const PASS: &str = "cloud medium coefficients";

pub(crate) struct Medium {
    pub(crate) target: ViewVolumeMediumTarget,
    pub(crate) lighting: layer::light::Lighting,
    environment: EnvironmentProducts,
    target_group: wgpu::BindGroup,
    input_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}

impl Medium {
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        sun: &SunPath,
        sun_mapping: &wgpu::Buffer,
        field: &GroundField,
        phases: &LightTable,
        band: &Band,
    ) -> Result<Self, Error> {
        let target = surface
            .view_volume_medium_target()
            .ok_or(Error::Unsupported(
                "Ground requires the scene's view volume medium",
            ))?;
        let environment = surface.environment_products().ok_or(Error::Unsupported(
            "Ground requires the physical environment",
        ))?;
        let lighting = layer::light::Lighting::new(surface, weather, &environment)?;
        let device = surface.device;
        let stages = wgpu::ShaderStages::COMPUTE;
        let mut layout = GroundField::layout(FIRST).to_vec();
        layout.extend(LightTable::layout());
        layout.extend([
            binding::uniform(8, stages, sun_mapping.size()),
            binding::storage(10, stages, true, lighting.ambient.size()),
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
        let input_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &layout,
        });
        let target_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &ViewVolumeMediumTarget::layout_entries(),
        });
        let products = sun.products().each_ref().map(|texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        });
        let mut entries = field.entries(FIRST).to_vec();
        entries.extend(phases.entries(band));
        entries.extend([
            wgpu::BindGroupEntry {
                binding: 8,
                resource: sun_mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: lighting.ambient.as_entire_binding(),
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
        let input_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &input_layout,
            entries: &entries,
        });
        let target_group = target.bind(device, &target_layout);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(format!(
                "{}\n{}\n{}\n{}\n{}",
                target.shader_source(), layer::sun_source(1), layer::SCATTERING, LightTable::source(1),
                concat!(
                    "const PI:f32=3.141592653589793;\n",
                    "@group(1) @binding(10) var<storage,read> cloud_ambient:array<vec4<f32>,2>;\n",
                    include_str!("medium.wgsl"),
                ),
            ).into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&target_layout), Some(&input_layout)],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("ground_medium"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(Self {
            target,
            lighting,
            environment,
            target_group,
            input_group,
            pipeline,
        })
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>, exposure_side: usize) {
        self.lighting.encode(pass, &self.environment, exposure_side);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.target_group, &[]);
        pass.set_bind_group(1, &self.input_group, &[]);
        let extent = self.target.coefficients.size();
        pass.dispatch_workgroups(
            extent.width.div_ceil(8),
            extent.height.div_ceil(8),
            extent.depth_or_array_layers,
        );
    }
}
