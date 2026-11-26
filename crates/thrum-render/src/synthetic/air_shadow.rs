//! Sun shadow heights for terrain-shadowed aerial perspective. One compute pass
//! reduces the top-down caster height map toward the Sun's azimuth into the
//! height below which air is hidden from the Sun, sampled by the shadowed aerial
//! perspective. Prepared with the shadow recipe when atmosphere lighting is on.

use super::shadow::ShadowTarget;
use super::{DIRECTIONAL_BYTES, RenderError, shadow};
use crate::binding;
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::pass_key::PassKey;
use crate::shaders::wgsl;
use crate::timing::GpuTimings;

/// Timed name of the Sun air shadow heights compute pass.
pub(crate) const AIR_SHADOW_PASS: &str = PassKey::AirShadow.constant_name();
/// Output columns per side: two caster height texels each, the same footprint.
pub(super) use crate::shader_constants::AIR_SHADOW_SIZE as SIZE;
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Float;

/// The forward pass's binding of the heights (view group, binding 4).
pub(super) const VIEW_ENTRY: wgpu::BindGroupLayoutEntry = binding::texture(
    4,
    wgpu::ShaderStages::FRAGMENT,
    wgpu::TextureSampleType::Float { filterable: false },
    wgpu::TextureViewDimension::D2,
);

/// The forward shader's aerial perspective: terrain-shadowed with the heights, clear without
/// them, or none without atmosphere lighting.
pub(super) fn aerial_source(
    atmosphere: bool,
    shadowed: bool,
    lighting: crate::atmosphere::overlay::Lighting,
) -> std::borrow::Cow<'static, str> {
    if lighting == crate::atmosphere::overlay::Lighting::OverlayAndHaze {
        let aerial = crate::atmosphere::overlay::aerial_source(lighting);
        let end = if shadowed {
            concat!(
                wgsl!("synthetic_aerial_visibility.wgsl"),
                wgsl!("synthetic_aerial_camera_shadowed.wgsl")
            )
        } else {
            wgsl!("synthetic_aerial_clear.wgsl")
        };
        return format!("{aerial}\n{end}").into();
    }
    clear_aerial_source(atmosphere, shadowed).into()
}

fn clear_aerial_source(atmosphere: bool, shadowed: bool) -> &'static str {
    match (atmosphere, shadowed) {
        (true, true) => concat!(
            wgsl!("synthetic_aerial.wgsl"),
            "\n",
            wgsl!("synthetic_aerial_visibility.wgsl"),
            wgsl!("synthetic_aerial_camera_shadowed.wgsl")
        ),
        (true, false) => concat!(
            wgsl!("synthetic_aerial.wgsl"),
            "\n",
            wgsl!("synthetic_aerial_clear.wgsl")
        ),
        _ => wgsl!("synthetic_plain.wgsl"),
    }
}

/// A target's heights and the compute bindings that write them.
pub(super) struct AirShadowTarget {
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

impl AirShadowTarget {
    pub(super) fn view_entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: VIEW_ENTRY.binding,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}

/// Declare the heights and their compute pass, which reads the shadow samples, the caster maps
/// and the directional light; the forward pass (`accesses`) samples the heights.
pub(super) fn append_graph(
    graph: &mut FrameGraph,
    key: PipelineKey,
    [maps, directional]: [ResourceId; 2],
    accesses: &mut Vec<Access>,
    copy_heights: bool,
) -> Result<ResourceId, RenderError> {
    let sample = graph.resource(
        ResourceDescriptor::Buffer {
            bytes: shadow::SAMPLE_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM,
        },
        ResourceScope::Imported { initialized: true },
    )?;
    let heights = graph.resource(
        ResourceDescriptor::Texture {
            dimension: crate::graph::TextureKind::D2,
            width: SIZE,
            height: SIZE,
            layers: 1,
            mips: 1,
            samples: 1,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | if copy_heights {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
        },
        ResourceScope::Transient,
    )?;
    let read = |resource, range, kind| Access::new(resource, range, kind);
    graph.declare(
        PassKey::AirShadow,
        PassBody {
            scope: PassScope::View(1),
            pipeline: Some(key),
            accesses: vec![
                read(
                    sample,
                    0..shadow::SAMPLE_BYTES as u64,
                    AccessKind::UniformRead,
                ),
                read(
                    maps,
                    shadow::HEIGHT_LAYER as u64..u64::from(shadow::LAYERS),
                    AccessKind::SampledRead,
                ),
                read(directional, 0..DIRECTIONAL_BYTES, AccessKind::UniformRead),
                read(heights.clone(), 0..1, AccessKind::AttachmentClear),
            ],
            external_side_effect: false,
        },
    );
    accesses.push(read(heights.clone(), 0..1, AccessKind::SampledRead));
    Ok(heights)
}

#[derive(Clone)]
pub(super) struct AirShadowPrepared {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl AirShadowPrepared {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        sample_bytes: u64,
        light_bytes: u64,
    ) -> Self {
        let uniform =
            |binding, bytes| binding::uniform(binding, wgpu::ShaderStages::FRAGMENT, bytes);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic Sun air shadow heights"),
            entries: &[
                uniform(0, sample_bytes),
                binding::texture(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Depth,
                    wgpu::TextureViewDimension::D2Array,
                ),
                uniform(2, light_bytes),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic Sun air shadow heights"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader =
            crate::shaders::Shader::AirShadow.compile(device, "synthetic Sun air shadow heights");
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("synthetic Sun air shadow heights"),
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
                entry_point: Some("build"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            layout: layout.into_raw(),
        }
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        shadow: Option<(&wgpu::Buffer, &ShadowTarget)>,
        light: &wgpu::Buffer,
    ) -> Result<AirShadowTarget, RenderError> {
        let (samples, maps) = shadow.ok_or(RenderError::GraphPassMismatch)?;
        let maps = &maps.array_view;
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic Sun air shadow heights"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: samples.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(maps),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light.as_entire_binding(),
                },
            ],
        });
        Ok(AirShadowTarget { view, group })
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &AirShadowTarget,
        generation: u64,
        compiled: &CompiledPass,
        resource: &ResourceId,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(GraphColorAttachment {
                    resource,
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
        pass.set_bind_group(0, &target.group, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
