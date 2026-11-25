//! Prepared texture-to-surface pass. The platform owns the surface and
//! presentation; this module only declares and encodes one view-sized GPU pass.

use crate::binding;
use std::sync::Arc;

use crate::{
    capabilities::DeviceCapabilities,
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, GraphError, PassBody,
        PassScope, PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
    },
    pass_key::{PassKey, PipelineRole},
    synthetic::OffscreenTarget,
    timing::{GpuTimings, TimingError},
};

const REVISION: u64 = 1;
/// Name of the optional fifth timestamped V1 pass.
pub const PASS_NAME: &str = PassKey::Surface.constant_name();

/// One selected surface and its current nonzero pixel extent.
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
    /// The surface format is outside the supported profiles.
    UnsupportedFormat,
    /// A zero, oversized or stale surface extent was supplied.
    Extent,
    /// A target or binding belongs to another device generation or presenter.
    StaleTarget,
    /// The closed graph contains no prepared presentation recorder.
    GraphPassMismatch,
    /// Optional timestamp batch lacks this pass or belongs to another generation.
    Timing(TimingError),
    /// The GPU rejected the prepared shader or pipeline.
    Validation(wgpu::Error),
}

impl std::fmt::Display for PresentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat => {
                f.write_str("Surface format is outside the supported profiles")
            }
            Self::Extent => f.write_str("Surface extent is zero, oversized or stale"),
            Self::StaleTarget => {
                f.write_str("Surface binding belongs to another presenter or device generation")
            }
            Self::GraphPassMismatch => f.write_str("Presentation graph lacks a matching recorder"),
            Self::Timing(error) => write!(f, "Presentation timing: {error}"),
            Self::Validation(error) => write!(f, "GPU refused the presentation pipeline: {error}"),
        }
    }
}

impl std::error::Error for PresentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timing(error) => Some(error),
            Self::Validation(error) => Some(error),
            _ => None,
        }
    }
}

/// Bound source texture retained by one host frame slot. A new surface texture
/// does not change this binding; the source is generation-bound and reused.
pub(crate) struct PresentationBinding {
    source_extent: [u32; 2],
    group: wgpu::BindGroup,
    owner: Arc<()>,
}

/// Pipeline and bindings for the surface pass in the host graph.
pub(crate) struct SurfacePresenter {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    generation: u64,
    extent_limit: u32,
    source_extent: [u32; 2],
    profile: SurfaceProfile,
    owner: Arc<()>,
}

pub(crate) struct PresenterInputs {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    generation: u64,
    extent_limit: u32,
    format: wgpu::TextureFormat,
}

struct PresentContext<'a> {
    presenter: &'a SurfacePresenter,
    binding: &'a PresentationBinding,
    surface_view: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
    timings: Option<&'a mut GpuTimings>,
}

fn record_present_resource(
    frame: &mut PresentContext<'_>,
    compiled: &CompiledPass,
    surface_resource: &ResourceId,
) -> Result<(), PresentError> {
    let presenter = frame.presenter;
    let mut pass = compiled
        .begin_render(
            frame.encoder,
            [Some(GraphColorAttachment {
                resource: surface_resource,
                range: 0..1,
                view: frame.surface_view,
                clear: wgpu::Color::BLACK,
            })],
            None,
            presenter.generation,
            frame.timings.as_deref_mut(),
        )
        .map_err(PresentError::Timing)?
        .ok_or(PresentError::GraphPassMismatch)?;
    pass.set_pipeline(&presenter.pipeline);
    pass.set_bind_group(0, &frame.binding.group, &[]);
    presenter.set_viewport(&mut pass, frame.binding);
    pass.draw(0..3, 0..1);
    Ok(())
}

impl SurfacePresenter {
    pub(crate) fn profile(&self) -> SurfaceProfile {
        self.profile
    }

    pub(crate) fn validate_output(
        &self,
        binding: &PresentationBinding,
        view: &wgpu::TextureView,
        extent: [u32; 2],
    ) -> Result<(), PresentError> {
        if !Arc::ptr_eq(&self.owner, &binding.owner) {
            return Err(PresentError::StaleTarget);
        }
        let texture = view.texture();
        if extent != self.profile.extent || extent != [texture.width(), texture.height()] {
            return Err(PresentError::Extent);
        }
        Ok(())
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "validated GPU texture dimensions fit exactly in f32 viewport coordinates on supported devices"
    )]
    pub(crate) fn set_viewport(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        binding: &PresentationBinding,
    ) {
        let (_, [x, y, width, height]) = self.source_viewport(binding);
        pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
        pass.set_scissor_rect(x, y, width, height);
    }

    pub(crate) fn encode_compiled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        binding: &PresentationBinding,
        surface_view: &wgpu::TextureView,
        surface_extent: [u32; 2],
        timings: Option<&mut GpuTimings>,
        compiled: &CompiledPass,
        surface_resource: &ResourceId,
    ) -> Result<(), PresentError> {
        self.validate_output(binding, surface_view, surface_extent)?;
        let mut frame = PresentContext {
            presenter: self,
            binding,
            surface_view,
            encoder,
            timings,
        };
        record_present_resource(&mut frame, compiled, surface_resource)
    }

    /// Prepare the pipeline before the first surface draw.
    /// # Errors
    /// Refuses unsupported formats/extents or GPU validation.
    pub(crate) async fn prepare(
        capabilities: &DeviceCapabilities,
        source_extent: [u32; 2],
        profile: SurfaceProfile,
    ) -> Result<Self, PresentError> {
        let device = capabilities.device();
        let generation = capabilities.generation();
        if !matches!(
            profile.format,
            wgpu::TextureFormat::Rgba16Float
                | wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(PresentError::UnsupportedFormat);
        }
        let extent_limit = capabilities.limits().max_texture_dimension_2d;
        check_extent(extent_limit, source_extent)?;
        check_extent(extent_limit, profile.extent)?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("V1 surface source"),
            entries: &[binding::texture(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            )],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("V1 surface present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = crate::shaders::Shader::Present.compile(device, "V1 surface present");
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
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
                entry_point: Some(
                    if profile.format.is_srgb()
                        || profile.format == wgpu::TextureFormat::Rgba16Float
                    {
                        "srgb_surface"
                    } else {
                        "unorm_surface"
                    },
                ),
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
            return Err(PresentError::Validation(error));
        }
        Ok(Self {
            device: wgpu::Device::clone(device),
            pipeline,
            layout,
            generation,
            extent_limit,
            source_extent,
            profile,
            owner: Arc::new(()),
        })
    }

    /// Prepare a reusable binding to a target from this device generation.
    /// # Errors
    /// Refuses a stale target or a target with a different source extent.
    pub(crate) fn bind_target(
        &self,
        device: &wgpu::Device,
        target: &OffscreenTarget,
    ) -> Result<PresentationBinding, PresentError> {
        if !target.belongs_to_device(device) {
            return Err(PresentError::StaleTarget);
        }
        self.bind_source(device, target.device_generation(), target.output_view())
    }

    pub(crate) fn bind_output(
        &self,
        device: &wgpu::Device,
        generation: u64,
        output: &wgpu::TextureView,
    ) -> Result<PresentationBinding, PresentError> {
        let texture = output.texture();
        if [texture.width(), texture.height()] != self.source_extent {
            return Err(PresentError::StaleTarget);
        }
        self.bind_source(device, generation, output)
    }

    fn bind_source(
        &self,
        device: &wgpu::Device,
        generation: u64,
        output: &wgpu::TextureView,
    ) -> Result<PresentationBinding, PresentError> {
        if self.device != *device || generation != self.generation {
            return Err(PresentError::StaleTarget);
        }
        let texture = output.texture();
        let source_extent = [texture.width(), texture.height()];
        check_extent(self.extent_limit, source_extent)?;
        Ok(PresentationBinding {
            source_extent,
            group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("V1 surface source"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(output),
                }],
            }),
            owner: Arc::clone(&self.owner),
        })
    }

    /// Update the configured extent after validation.
    pub(crate) fn resize(
        &mut self,
        device: &wgpu::Device,
        extent: [u32; 2],
    ) -> Result<(), PresentError> {
        if self.device != *device {
            return Err(PresentError::StaleTarget);
        }
        check_extent(self.extent_limit, extent)?;
        self.profile.extent = extent;
        Ok(())
    }

    /// Prepare new source and destination extents, retaining the pipeline.
    pub(crate) fn refit(
        &self,
        source_extent: [u32; 2],
        extent: [u32; 2],
    ) -> Result<Self, PresentError> {
        self.inputs().prepare(source_extent, extent)
    }

    pub(crate) fn inputs(&self) -> PresenterInputs {
        PresenterInputs {
            device: self.device.clone(),
            pipeline: self.pipeline.clone(),
            layout: self.layout.clone(),
            generation: self.generation,
            extent_limit: self.extent_limit,
            format: self.profile.format,
        }
    }
}

impl PresenterInputs {
    pub(crate) fn prepare(
        self,
        source_extent: [u32; 2],
        extent: [u32; 2],
    ) -> Result<SurfacePresenter, PresentError> {
        let format = self.format;
        self.prepare_format(source_extent, extent, format)
    }

    pub(crate) fn prepare_format(
        self,
        source_extent: [u32; 2],
        extent: [u32; 2],
        format: wgpu::TextureFormat,
    ) -> Result<SurfacePresenter, PresentError> {
        check_extent(self.extent_limit, source_extent)?;
        check_extent(self.extent_limit, extent)?;
        let profile = SurfaceProfile { format, extent };
        Ok(SurfacePresenter {
            device: self.device,
            pipeline: self.pipeline,
            layout: self.layout,
            generation: self.generation,
            extent_limit: self.extent_limit,
            source_extent,
            profile,
            owner: Arc::new(()),
        })
    }
}

impl SurfacePresenter {
    /// Source grid and viewport used by the presentation pass.
    pub(crate) fn source_viewport(&self, binding: &PresentationBinding) -> ([u32; 2], [u32; 4]) {
        let source = binding.source_extent;
        (source, letterbox(source, self.profile.extent))
    }

    /// Current configured surface extent.
    pub(crate) fn extent(&self) -> [u32; 2] {
        self.profile.extent
    }
}

fn check_extent(limit: u32, extent: [u32; 2]) -> Result<(), PresentError> {
    if extent.iter().any(|&value| value == 0 || value > limit) {
        Err(PresentError::Extent)
    } else {
        Ok(())
    }
}

pub(crate) fn pipeline_key(generation: u64) -> PipelineKey {
    PipelineRole::Surface.pipeline_key(generation, REVISION)
}

/// Declare the surface consumer in the host graph. The surface view is bound at execution.
pub(crate) fn surface_pass(
    graph: &mut FrameGraph,
    source: &ResourceId,
    profile: SurfaceProfile,
    generation: u64,
) -> Result<(ResourceId, PassBody), GraphError> {
    let surface = graph.resource(
        ResourceDescriptor::Texture {
            dimension: crate::graph::TextureKind::D2,
            width: profile.extent[0],
            height: profile.extent[1],
            layers: 1,
            mips: 1,
            samples: 1,
            format: profile.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        },
        ResourceScope::Imported { initialized: false },
    )?;
    let pass = PassBody {
        scope: PassScope::View(1),
        pipeline: Some(pipeline_key(generation)),
        accesses: vec![
            Access::new(source.clone(), 0..1, AccessKind::SampledRead),
            Access::new(surface.clone(), 0..1, AccessKind::AttachmentClear),
        ],
        external_side_effect: true,
    };
    Ok((surface, pass))
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
