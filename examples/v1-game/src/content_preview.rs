//! Opt-in valley reader that displays a lent texture and a slice of a lent volume.

use std::sync::{Arc, Mutex};

use thrum_asset_gpu::{CookedAssets, CookedStats, LeaseRequest, ProductLease};
use thrum_render::{
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    runtime::DeviceSite,
    synthetic::mesh_rows::MeshRows,
};

pub(crate) const VOLUME: &str = "preview-cloud";

#[derive(Clone)]
pub(crate) struct PreviewRecord {
    assets: Arc<Mutex<CookedStats>>,
    rows: MeshRows,
    lease: Arc<Mutex<Option<u64>>>,
    reconstruction: thrum_render::synthetic::Reconstruction,
}

impl PreviewRecord {
    pub(crate) fn frame(&self) -> Option<serde_json::Value> {
        let assets = self.assets.lock().ok()?;
        let lease = *self.lease.lock().ok()?;
        Some(serde_json::json!({
            "placement": assets.drawn,
            "row": self.rows.generation(),
            "lentTexture": lease,
            "lentVolume": lease,
            "staged": assets.staged,
            "submitted": assets.submitted_frames,
            "swaps": assets.swaps.len(),
            "copyBytes": assets.last_frame_copy_bytes,
            "stagingPeakBytes": assets.staging_peak_bytes,
            "recipe": format!("{:?}", assets.recipe),
            "reconstruction": format!("{:?}", self.reconstruction),
            "errors": assets.errors,
        }))
    }
}

pub(crate) fn requests() -> Vec<LeaseRequest> {
    vec![
        LeaseRequest {
            name: "ground".into(),
            dimension: wgpu::TextureDimension::D2,
        },
        LeaseRequest {
            name: VOLUME.into(),
            dimension: wgpu::TextureDimension::D3,
        },
    ]
}

pub(crate) fn attach(
    assets: CookedAssets,
    rows: MeshRows,
    reconstruction: thrum_render::synthetic::Reconstruction,
) -> (Box<dyn SceneContribution>, PreviewRecord) {
    let record = PreviewRecord {
        assets: assets.stats(),
        rows,
        lease: Arc::new(Mutex::new(None)),
        reconstruction,
    };
    let reader = PreviewReader {
        lease: assets.lease(),
        submitted: Arc::clone(&record.lease),
        prepared: None,
        recorded: None,
    };
    (
        Box::new(thrum_render::contribution::SceneContributions::new(vec![
            Box::new(assets),
            Box::new(reader),
        ])),
        record,
    )
}

struct PreviewReader {
    lease: ProductLease,
    submitted: Arc<Mutex<Option<u64>>>,
    prepared: Option<PreviewSurface>,
    recorded: Option<u64>,
}

struct PreviewSurface {
    device: DeviceSite,
    pipeline: wgpu::RenderPipeline,
    binding: Option<([u64; 2], wgpu::BindGroup)>,
    sampler: wgpu::Sampler,
}

impl SceneContribution for PreviewReader {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let device = surface.device.clone();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lent content preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("content_preview.wgsl").into()),
        });
        let targets = surface.color_formats.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let binding_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lent content preview"),
            entries: &[
                texture_binding(0, wgpu::TextureViewDimension::D2),
                texture_binding(1, wgpu::TextureViewDimension::D3),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lent content preview"),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(surface.light_layout),
                Some(&binding_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("lent content preview"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lent content preview"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        self.prepared = Some(PreviewSurface {
            device,
            pipeline,
            binding: None,
            sampler,
        });
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.recorded = None;
        let (Some(surface), Some(set)) = (&mut self.prepared, self.lease.offered()) else {
            return;
        };
        let [texture, volume] = set.products.as_slice() else {
            return;
        };
        if texture.device_generation != frame.generation
            || volume.device_generation != frame.generation
        {
            return;
        }
        let serials = [texture.serial, volume.serial];
        if surface
            .binding
            .as_ref()
            .is_none_or(|(old, _)| *old != serials)
        {
            let group = surface
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("lent content preview"),
                    layout: &surface.pipeline.get_bind_group_layout(2),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&volume.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&surface.sampler),
                        },
                    ],
                });
            surface.binding = Some((serials, group));
        }
        self.recorded = Some(set.generation);
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.recorded.is_some()
            && let Some(surface) = &self.prepared
            && let Some((_, group)) = &surface.binding
        {
            pass.set_pipeline(&surface.pipeline);
            pass.set_bind_group(2, group, &[]);
            pass.draw(0..6, 0..2);
        }
    }

    fn submitted(&mut self, _: bool) {
        if let Ok(mut submitted) = self.submitted.lock() {
            *submitted = self.recorded;
        }
    }
}

fn texture_binding(
    binding: u32,
    view_dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension,
            multisampled: false,
        },
        count: None,
    }
}
