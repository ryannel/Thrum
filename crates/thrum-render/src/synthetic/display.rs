//! Prepared SDR display transform, binding and recording ownership. Camera
//! exposure is resolved separately; output remains relative to display white.

use super::{OUTPUT_FORMAT, OffscreenTarget, RenderError, exposure};
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

pub(super) struct DisplayPrepared {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

pub(super) struct DisplayTarget {
    groups: [wgpu::BindGroup; 2],
}

impl DisplayPrepared {
    pub(super) fn new(device: &wgpu::Device) -> Self {
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic authored SDR display"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        Self { pipeline, layout }
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        scene: &wgpu::TextureView,
        samples: &[wgpu::Buffer; 2],
    ) -> DisplayTarget {
        DisplayTarget {
            groups: std::array::from_fn(|side| {
                prepare_group(device, &self.layout, scene, &samples[side])
            }),
        }
    }

    pub(super) fn append_graph(
        graph: &mut FrameGraph,
        key: PipelineKey,
        scene: &ResourceId,
        exposure: &ResourceId,
        width: u32,
        height: u32,
    ) -> Result<ResourceId, RenderError> {
        let output = graph.resource(
            ResourceDescriptor::Texture {
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
        graph.pass(Pass {
            name: "synthetic tone encode".into(),
            scope: PassScope::View(1),
            pipeline: Some(key),
            accesses: vec![
                Access {
                    resource: scene.clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    resource: exposure.clone(),
                    range: 0..exposure::SAMPLE_BYTES,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
                Access {
                    resource: output.clone(),
                    range: 0..1,
                    kind: AccessKind::AttachmentClear,
                    store: true,
                },
            ],
            external_side_effect: true,
        });
        Ok(output)
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        write_side: usize,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(GraphColorAttachment {
                    resource: &target.ids.output,
                    range: 0..1,
                    view: &target.view,
                    clear: wgpu::Color::BLACK,
                })],
                None,
                generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &target.display.groups[write_side], &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

fn prepare_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene: &wgpu::TextureView,
    exposure: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic display inputs"),
        layout,
        entries: &[
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
