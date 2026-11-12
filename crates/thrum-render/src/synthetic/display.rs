//! Prepared SDR display transform, binding and recording ownership. Camera
//! exposure is resolved separately; output remains relative to display white.

use super::{OUTPUT_FORMAT, RenderError, exposure};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, Pass, PassScope,
    PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::timing::GpuTimings;

/// Per-view SDR tone/gamut policy, independent of fixed or incident exposure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum SdrToneMap {
    /// Historical luminance Reinhard followed by maximum-channel normalization.
    #[default]
    Reinhard = 0,
    /// Smooth highlight compression/desaturation without a material shadow toe.
    /// This is not a reproduction of the standard Khronos PBR Neutral operator.
    NeutralHighlights = 1,
}

/// Per-view diagnostic drawn by the display pass in place of the tone-mapped scene.
/// It reads exposed scene luminance before the tone scale, so it is identical for every
/// [`SdrToneMap`], and it writes exact palette codes without dither.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum DisplayOverlay {
    /// The tone-mapped scene. Its pixels are byte-identical to a view with no overlay.
    #[default]
    Off = 0,
    /// Exposed luminance in one-stop false-colour bands, from -6 to +6 stops relative to
    /// an 18% grey Lambertian card lit at the metered illuminance with zero compensation.
    ExposureZones = 1,
    /// Motion in render pixels: red is right, green is down, blue is magnitude.
    /// Magenta marks invalid correspondence; olive is stationary.
    Motion = 2,
    /// Actual reactive/depth/motion inputs. Requires temporal inputs at preparation.
    TemporalInputs = 3,
}

/// Word 7 of the exposure policy and sample: the tone map in bits 0–7 and the
/// overlay in bits 8–15. With the overlay off the word equals the tone-map value.
pub(super) fn display_selectors(tone_map: SdrToneMap, overlay: DisplayOverlay) -> u32 {
    tone_map as u32 | (overlay as u32) << 8
}

pub(super) const SHADER: &str = concat!(
    include_str!("display/neutral_highlights.wgsl"),
    "\n",
    include_str!("../synthetic_display.wgsl"),
);

pub(crate) struct SdrToneStage {
    key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// Graph-bound colour, motion and matching exposure for the retained SDR stage.
/// Colour is pre-exposed scene-linear RGBA16F at `extent`, with the existing
/// source-pixel coordinate convention. Motion shares that grid. The exposure
/// sample carries its GPU-resolved submitted/sample identity (C07).
#[derive(Clone, Debug)]
pub(crate) struct PreToneInput {
    /// Scene-linear forward composition or a prepared reconstruction output.
    pub colour: ResourceId,
    /// Motion for the diagnostic overlay, in current source pixels.
    pub motion: ResourceId,
    /// Matching resolved exposure sample; never an independently derived gain.
    pub exposure: ResourceId,
    /// Source/output grid for this identity-preserving SDR stage.
    pub extent: [u32; 2],
}

/// Physical views for one admitted pre-tone choice, prepared before readiness.
/// The graph IDs in `PreToneInput` must name these exact resources. Both exposure
/// sides follow the forward/history write side; retained/skipped inputs must
/// retain their matching exposure rather than borrow a newer sample.
#[derive(Clone, Copy)]
pub(crate) struct PreToneViews<'a> {
    /// Colour on the declared source grid.
    pub colour: &'a wgpu::TextureView,
    /// Motion on that grid.
    pub motion: &'a wgpu::TextureView,
    /// Resolved exposure buffers, selected by the submitted frame's write side.
    pub exposure: &'a [wgpu::Buffer; 2],
}

/// Prepared input bindings retain the source resources for their GPU lifetime.
/// Replacing a binding requires a graph compiled with that binding's resources.
pub(crate) struct PreparedPreToneInput {
    groups: [wgpu::BindGroup; 2],
    input: PreToneInput,
    generation: u64,
}

/// Execute-time output of the SDR stage. Surface acquisition remains later.
#[derive(Clone, Copy)]
pub(crate) struct ToneOutput<'a> {
    /// Declared Rgba8Unorm intermediate, retained until presentation/capture.
    pub resource: &'a ResourceId,
    /// Physical view of that intermediate.
    pub view: &'a wgpu::TextureView,
    /// Current submitted forward/history write side (0 or 1).
    pub exposure_side: usize,
}

impl PreparedPreToneInput {
    /// Declared source identity for this binding; unchanged by surface resize.
    pub(crate) fn input(&self) -> &PreToneInput {
        &self.input
    }

    fn check_pass(&self, pass: &CompiledPass) -> Result<(), RenderError> {
        for (resource, kind) in [
            (&self.input.colour, AccessKind::SampledRead),
            (&self.input.motion, AccessKind::SampledRead),
            (&self.input.exposure, AccessKind::UniformRead),
        ] {
            if !pass
                .reads
                .iter()
                .any(|a| &a.resource == resource && a.kind == kind)
            {
                return Err(RenderError::GraphPassMismatch);
            }
        }
        Ok(())
    }
}

impl SdrToneStage {
    pub(super) fn new(device: &crate::runtime::DeviceSite, key: PipelineKey) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic display layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(exposure::SAMPLE_BYTES),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic display"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // A cold, explicit Mac qualification fixture uses this same final output,
        // tap and presentation binding. It changes only the authored diagnostic pixels.
        #[cfg(target_os = "macos")]
        let pattern = std::env::var("THRUM_MAC_PATTERN").as_deref() == Ok("1");
        #[cfg(not(target_os = "macos"))]
        let pattern = false;
        let source = if pattern {
            include_str!("display/capture_pattern.wgsl")
        } else {
            SHADER
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic authored SDR display"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("prepared synthetic tone encode role"),
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
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OUTPUT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            key,
            pipeline,
            layout,
        }
    }

    /// Prepare an admitted input without compiling shaders or changing arithmetic.
    /// # Errors
    /// Refuses incompatible extents, formats, usages or exposure capacity.
    pub(crate) fn prepare_input(
        &self,
        device: &wgpu::Device,
        input: PreToneInput,
        views: PreToneViews<'_>,
    ) -> Result<PreparedPreToneInput, RenderError> {
        for (view, format) in [
            (views.colour, super::SCENE_FORMAT),
            (views.motion, super::MOTION_FORMAT),
        ] {
            let texture = view.texture();
            if [texture.width(), texture.height()] != input.extent
                || texture.format() != format
                || texture.sample_count() != 1
                || !texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
            {
                return Err(RenderError::TargetExtent);
            }
        }
        if views.exposure.iter().any(|b| {
            b.size() < exposure::SAMPLE_BYTES || !b.usage().contains(wgpu::BufferUsages::UNIFORM)
        }) {
            return Err(RenderError::GraphPassMismatch);
        }
        Ok(PreparedPreToneInput {
            groups: std::array::from_fn(|side| {
                prepare_group(
                    device,
                    &self.layout,
                    views.colour,
                    views.motion,
                    &views.exposure[side],
                )
            }),
            input,
            generation: self.key.device_generation,
        })
    }

    /// Declare all tone inputs, including optional motion-overlay reads. A
    /// reconstruction producer writes `input.colour` before this consumer.
    /// # Errors
    /// Refuses foreign IDs or descriptors incompatible with the SDR source grid.
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        input: &PreToneInput,
    ) -> Result<(ResourceId, usize), RenderError> {
        let [width, height] = input.extent;
        for (id, format) in [
            (&input.colour, super::SCENE_FORMAT),
            (&input.motion, super::MOTION_FORMAT),
        ] {
            if !matches!(graph.descriptor(id), Some(ResourceDescriptor::Texture { dimension: crate::graph::TextureKind::D2, width: w, height: h, layers: 1, mips: 1, samples: 1, format: f, usage }) if *w == width && *h == height && *f == format && usage.contains(wgpu::TextureUsages::TEXTURE_BINDING))
            {
                return Err(RenderError::GraphPassMismatch);
            }
        }
        if !matches!(graph.descriptor(&input.exposure), Some(ResourceDescriptor::Buffer { bytes, usage }) if *bytes >= exposure::SAMPLE_BYTES && usage.contains(wgpu::BufferUsages::UNIFORM))
        {
            return Err(RenderError::GraphPassMismatch);
        }
        let output = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width,
                height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Transient,
        )?;
        let pass = graph.pass(Pass {
            name: "synthetic tone encode".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.key),
            accesses: vec![
                Access {
                    view: None,
                    resource: input.motion.clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    view: None,
                    resource: input.colour.clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    view: None,
                    resource: input.exposure.clone(),
                    range: 0..exposure::SAMPLE_BYTES,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
                Access {
                    view: None,
                    resource: output.clone(),
                    range: 0..1,
                    kind: AccessKind::AttachmentClear,
                    store: true,
                },
            ],
            external_side_effect: true,
        });
        Ok((output, pass))
    }

    /// Encode the retained SDR shader against one prepared graph-bound input.
    /// # Errors
    /// Refuses stale generations, wrong exposure sides or mismatched graph reads.
    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        input: &PreparedPreToneInput,
        compiled: &CompiledPass,
        output: ToneOutput<'_>,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        if generation != input.generation
            || generation != self.key.device_generation
            || output.exposure_side > 1
        {
            return Err(RenderError::StaleTarget);
        }
        if compiled.pipeline != Some(self.key) {
            return Err(RenderError::GraphPassMismatch);
        }
        input.check_pass(compiled)?;
        if [
            output.view.texture().width(),
            output.view.texture().height(),
        ] != input.input().extent
        {
            return Err(RenderError::TargetExtent);
        }
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(GraphColorAttachment {
                    resource: output.resource,
                    range: 0..1,
                    view: output.view,
                    clear: wgpu::Color::BLACK,
                })],
                None,
                generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &input.groups[output.exposure_side], &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

fn prepare_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene: &wgpu::TextureView,
    motion: &wgpu::TextureView,
    exposure: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic display inputs"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(motion),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(scene),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: exposure,
                    offset: 0,
                    size: wgpu::BufferSize::new(exposure::SAMPLE_BYTES),
                }),
            },
        ],
    })
}

#[cfg(test)]
mod tests;
