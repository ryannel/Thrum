//! Prepared SDR texture-to-surface pass. The platform owns the surface and
//! presentation; this module only declares and encodes one view-sized GPU pass.

use std::{collections::BTreeSet, sync::Arc};

use crate::{
    graph::{
        Access, AccessKind, CompiledGraph, FrameGraph, GraphColorAttachment, GraphError, Pass,
        PassScope, PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
    },
    synthetic::{OUTPUT_FORMAT, OffscreenTarget},
    timing::{GpuTimings, TimingError},
};

const ROLE: u32 = 0x5052;
const REVISION: u64 = 1;
/// Name of the optional fifth timestamped V1 pass.
pub const PASS_NAME: &str = "V1 surface present";

/// One selected 8-bit SDR surface and its current nonzero pixel extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceProfile {
    /// Pipeline target format, selected from the surface capabilities.
    pub format: wgpu::TextureFormat,
    /// Physical surface pixel extent, updated on resize.
    pub extent: [u32; 2],
}

/// A presentation preparation or encode boundary could not be used.
#[derive(Debug)]
pub enum PresentError {
    /// The surface format is outside the first SDR profile.
    UnsupportedFormat,
    /// A zero, oversized or stale surface extent was supplied.
    Extent,
    /// A target or binding belongs to another device generation or presenter.
    StaleTarget,
    /// The one-pass graph fails its source/target closure.
    Graph(GraphError),
    /// The closed graph contains no prepared presentation recorder.
    GraphPassMismatch,
    /// Optional timestamp batch lacks this pass or belongs to another generation.
    Timing(TimingError),
    /// The GPU rejected the prepared shader or pipeline.
    Validation(String),
}

/// Bound source texture retained by one host frame slot. A new surface texture
/// does not change this binding; the source is generation-bound and reused.
pub struct PresentationBinding {
    group: wgpu::BindGroup,
    owner: Arc<()>,
}

/// One prepared surface pipeline and compiled imported-resource pass plan.
/// Resize recompiles only the small graph; device loss prepares a new owner.
pub struct SurfacePresenter {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    plan: CompiledGraph,
    surface_resource: ResourceId,
    generation: u64,
    source_extent: [u32; 2],
    profile: SurfaceProfile,
    owner: Arc<()>,
}

impl SurfacePresenter {
    /// Prepare the pipeline and graph before the first surface draw.
    /// # Errors
    /// Refuses unsupported formats/extents, invalid graph closure or GPU validation.
    pub async fn prepare(
        device: &wgpu::Device,
        generation: u64,
        source_extent: [u32; 2],
        profile: SurfaceProfile,
    ) -> Result<Self, PresentError> {
        if !matches!(
            profile.format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(PresentError::UnsupportedFormat);
        }
        check_extent(device, source_extent)?;
        check_extent(device, profile.extent)?;
        let (plan, surface_resource) = compile_plan(generation, source_extent, profile)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("V1 surface source"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("V1 surface present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("V1 surface present"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("V1 surface present"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if profile.format.is_srgb() {
                    "srgb_surface"
                } else {
                    "unorm_surface"
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: profile.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = scope.pop().await {
            return Err(PresentError::Validation(error.to_string()));
        }
        Ok(Self {
            device: device.clone(),
            pipeline,
            layout,
            plan,
            surface_resource,
            generation,
            source_extent,
            profile,
            owner: Arc::new(()),
        })
    }

    /// Prepare a reusable binding to a target from this device generation.
    /// # Errors
    /// Refuses a stale target or a target with a different source extent.
    pub fn bind_target(
        &self,
        device: &wgpu::Device,
        target: &OffscreenTarget,
    ) -> Result<PresentationBinding, PresentError> {
        if self.device != *device
            || !target.belongs_to_device(device)
            || target.device_generation() != self.generation
            || target.extent() != self.source_extent
        {
            return Err(PresentError::StaleTarget);
        }
        Ok(PresentationBinding {
            group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("V1 surface source"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(target.output_view()),
                }],
            }),
            owner: Arc::clone(&self.owner),
        })
    }

    /// Recompile the extent-dependent graph after the platform configures a
    /// nonzero surface. Existing GPU submissions retain their prior resources.
    /// # Errors
    /// Refuses an invalid extent or graph closure without changing this presenter.
    pub fn resize(&mut self, device: &wgpu::Device, extent: [u32; 2]) -> Result<(), PresentError> {
        if self.device != *device {
            return Err(PresentError::StaleTarget);
        }
        check_extent(device, extent)?;
        if self.profile.extent == extent {
            return Ok(());
        }
        let profile = SurfaceProfile {
            format: self.profile.format,
            extent,
        };
        let (plan, surface_resource) = compile_plan(self.generation, self.source_extent, profile)?;
        self.plan = plan;
        self.surface_resource = surface_resource;
        self.profile = profile;
        Ok(())
    }

    /// Current configured surface extent.
    pub fn extent(&self) -> [u32; 2] {
        self.profile.extent
    }

    /// Encode one letterboxed pass into an acquired surface texture. The caller
    /// submits and presents on its own queue in the same frame transaction.
    /// # Errors
    /// Refuses a stale binding/extent or unavailable timestamp slot before a pass begins.
    #[expect(
        clippy::cast_precision_loss,
        reason = "validated GPU texture dimensions fit exactly in f32 viewport coordinates on supported devices"
    )]
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        binding: &PresentationBinding,
        surface_view: &wgpu::TextureView,
        surface_extent: [u32; 2],
        mut timings: Option<&mut GpuTimings>,
    ) -> Result<(), PresentError> {
        if self.plan.device_generation != self.generation
            || !Arc::ptr_eq(&self.owner, &binding.owner)
        {
            return Err(PresentError::StaleTarget);
        }
        if surface_extent != self.profile.extent {
            return Err(PresentError::Extent);
        }
        let expected = PipelineKey {
            device_generation: self.generation,
            revision: REVISION,
            role: ROLE,
        };
        self.plan.record(|compiled| {
            if compiled.pipeline != Some(expected) {
                return Err(PresentError::GraphPassMismatch);
            }
            let viewport = letterbox(self.source_extent, surface_extent);
            let mut pass = compiled
                .begin_render(
                    encoder,
                    Some(GraphColorAttachment {
                        resource: &self.surface_resource,
                        range: 0..1,
                        view: surface_view,
                        clear: wgpu::Color::BLACK,
                    }),
                    None,
                    self.generation,
                    timings.as_deref_mut(),
                )
                .map_err(PresentError::Timing)?
                .ok_or(PresentError::GraphPassMismatch)?;
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &binding.group, &[]);
            pass.set_viewport(
                viewport[0] as f32,
                viewport[1] as f32,
                viewport[2] as f32,
                viewport[3] as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(viewport[0], viewport[1], viewport[2], viewport[3]);
            pass.draw(0..3, 0..1);
            Ok(())
        })
    }
}

fn check_extent(device: &wgpu::Device, extent: [u32; 2]) -> Result<(), PresentError> {
    let limit = device.limits().max_texture_dimension_2d;
    if extent.iter().any(|&value| value == 0 || value > limit) {
        Err(PresentError::Extent)
    } else {
        Ok(())
    }
}

fn compile_plan(
    generation: u64,
    source_extent: [u32; 2],
    profile: SurfaceProfile,
) -> Result<(CompiledGraph, ResourceId), PresentError> {
    let key = PipelineKey {
        device_generation: generation,
        revision: REVISION,
        role: ROLE,
    };
    let mut graph = FrameGraph::default();
    let source = graph
        .resource(
            ResourceDescriptor::Texture {
                width: source_extent[0],
                height: source_extent[1],
                layers: 1,
                mips: 1,
                samples: 1,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: true },
        )
        .map_err(PresentError::Graph)?;
    let surface = graph
        .resource(
            ResourceDescriptor::Texture {
                width: profile.extent[0],
                height: profile.extent[1],
                layers: 1,
                mips: 1,
                samples: 1,
                format: profile.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            ResourceScope::Imported { initialized: false },
        )
        .map_err(PresentError::Graph)?;
    graph.pass(Pass {
        name: PASS_NAME.into(),
        scope: PassScope::View(1),
        pipeline: Some(key),
        accesses: vec![
            Access {
                resource: source,
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            },
            Access {
                resource: surface.clone(),
                range: 0..1,
                kind: AccessKind::AttachmentClear,
                store: true,
            },
        ],
        external_side_effect: true,
    });
    Ok((
        graph
            .compile(generation, &BTreeSet::from([key]))
            .map_err(PresentError::Graph)?,
        surface,
    ))
}

/// Integer viewport preserving the source aspect without a per-frame upload.
fn letterbox(source: [u32; 2], surface: [u32; 2]) -> [u32; 4] {
    let [sw, sh] = source;
    let [dw, dh] = surface;
    let (width, height) = if u64::from(dw) * u64::from(sh) > u64::from(dh) * u64::from(sw) {
        (
            u32::try_from(u64::from(dh) * u64::from(sw) / u64::from(sh)).unwrap_or(dw),
            dh,
        )
    } else {
        (
            dw,
            u32::try_from(u64::from(dw) * u64::from(sh) / u64::from(sw)).unwrap_or(dh),
        )
    };
    let width = width.clamp(1, dw);
    let height = height.clamp(1, dh);
    [(dw - width) / 2, (dh - height) / 2, width, height]
}

#[cfg(test)]
mod tests {
    use super::letterbox;

    #[test]
    fn letterbox_preserves_aspect_and_nonzero_viewport() {
        assert_eq!(letterbox([640, 360], [1280, 720]), [0, 0, 1280, 720]);
        assert_eq!(letterbox([640, 360], [1000, 1000]), [0, 219, 1000, 562]);
        assert_eq!(letterbox([640, 360], [1, 1000]), [0, 499, 1, 1]);
        assert_eq!(letterbox([640, 360], [1000, 1]), [499, 0, 1, 1]);
    }
}
