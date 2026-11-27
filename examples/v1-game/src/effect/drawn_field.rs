//! Opt-in drawn-field diagnostic for the browser qualification.
use super::qualification::{Scene, World, error, read_texture_bytes, submitted_colour};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectDrawTarget,
        EffectGraph, EffectPass, SceneContribution, SceneSurface,
    },
    extract::SourceTick,
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, FrameError, FrameHost},
    synthetic::{CaptureSource, SyntheticScene},
};

const SIZE: u32 = 8;
const EXTENT: [u32; 2] = [2 * SIZE, SIZE];
const DRAW_FIELD: [&str; 2] = ["draw field left", "draw field right"];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

#[derive(Default)]
struct DrawnField {
    gpu: Option<Prepared>,
    witness: Arc<Mutex<Option<wgpu::Texture>>>,
}

struct Prepared {
    generation: u64,
    texture: wgpu::Texture,
    targets: [EffectDrawTarget; 2],
    additive: wgpu::RenderPipeline,
    opaque: wgpu::RenderPipeline,
    field: wgpu::BindGroup,
}

impl SceneContribution for DrawnField {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        if self
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.generation == surface.generation)
        {
            return Ok(());
        }
        let texture = surface.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("drawn field"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 2,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = |layer: u32| {
            EffectDrawTarget::new(
                DRAW_FIELD[layer as usize],
                &texture,
                0,
                layer,
                wgpu::Color::TRANSPARENT,
                true,
                surface,
            )
            .map_err(FrameError::EffectGraph)
        };
        let targets = [target(0)?, target(1)?];
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("drawn field diagnostic"),
                source: wgpu::ShaderSource::Wgsl(include_str!("drawn_field.wgsl").into()),
            });
        let make_pipeline =
            |entry_point, targets: &[Option<wgpu::ColorTargetState>], depth_stencil| {
                surface.device.render(&wgpu::RenderPipelineDescriptor {
                    label: Some(entry_point),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("triangle"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some(entry_point),
                        compilation_options: Default::default(),
                        targets,
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            };
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let additive = make_pipeline(
            "deposit",
            &[Some(wgpu::ColorTargetState {
                format: FORMAT,
                blend: Some(wgpu::BlendState {
                    color: add,
                    alpha: add,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
        );
        let opaque = make_pipeline(
            "show_field",
            &surface.color_formats.map(|format| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            }),
            Some(wgpu::DepthStencilState {
                format: surface.depth_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let field = surface
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("show both drawn field layers"),
                layout: &opaque.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                }],
            });
        *self.witness.lock().expect("drawn field witness") = Some(texture.clone());
        self.gpu = Some(Prepared {
            generation: surface.generation,
            texture,
            targets,
            additive,
            opaque,
            field,
        });
        Ok(())
    }

    fn effect_graph(&self) -> EffectGraph {
        let Some(gpu) = &self.gpu else {
            return EffectGraph::default();
        };
        let access = |kind, range| EffectAccess {
            resource: 0,
            range,
            kind,
            view: None,
            history: None,
        };
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::of_texture(&gpu.texture),
                ResourceScope::Persistent {
                    owner: 0,
                    epoch: 0,
                    initialized: false,
                },
            )],
            passes: DRAW_FIELD
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    let layer = index as u64;
                    EffectPass {
                        name: name.into(),
                        cadence: EffectCadence::EveryFrame,
                        async_eligible: false,
                        accesses: vec![access(AccessKind::AttachmentClear, layer..layer + 1)],
                    }
                })
                .collect(),
            opaque_reads: vec![access(AccessKind::SampledRead, 0..2)],
            ..Default::default()
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn effect_draw_target(&self, index: usize) -> Option<&EffectDrawTarget> {
        self.gpu.as_ref()?.targets.get(index)
    }

    fn draw_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        let gpu = self.gpu.as_ref().expect("prepared drawn field");
        let layer = index as u32;
        let x = ((frame.time.tick + u64::from(layer * 2)) % 5) as u32;
        let y = 1 + layer * 3;
        pass.set_pipeline(&gpu.additive);
        for left in [x, x + 1] {
            pass.set_scissor_rect(left, y, 2, 2);
            pass.draw(0..3, 0..1);
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let gpu = self.gpu.as_ref().expect("prepared drawn field");
        pass.set_pipeline(&gpu.opaque);
        pass.set_bind_group(0, &gpu.field, &[]);
        pass.draw(0..3, 0..1);
    }
}

async fn two_frames(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
) -> Result<String, String> {
    let effect = DrawnField::default();
    let witness = Arc::clone(&effect.witness);
    let mut host = FrameHost::new(
        capabilities,
        queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: EXTENT,
            }),
        },
        World(SourceTick {
            epoch: 7,
            timeline: 3,
            tick: 0,
        }),
        Scene,
        SyntheticScene::grid(1).map_err(error)?,
        1.0,
    )
    .await
    .map_err(error)?;
    // Select only the drawn field for this qualification page. The ordinary
    // trail and slab qualifications select their original diagnostics.
    host.set_scene_contribution(Box::new(effect))
        .map_err(error)?;
    host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
        .map_err(error)?;
    let texture = witness
        .lock()
        .map_err(error)?
        .clone()
        .ok_or("drawn field was not prepared")?;
    let mut report = Vec::new();
    // These are the oracle's literal coordinates, independent of the drawing code.
    for (tick, lefts) in [(0, [0, 2]), (2, [2, 4])] {
        host.world_mut().0.tick = tick;
        if !host
            .advance(Duration::from_millis(tick * 16))
            .map_err(error)?
            .encode_scene()
            .map_err(error)?
            .submit(None)
            .map_err(error)?
            .submitted
        {
            return Err(format!("tick {tick}: drawn field did not submit"));
        }
        let bytes = read_texture_bytes(device, queue, &texture, 0, 2).await?;
        let scene = submitted_colour(&mut host, device, queue, &texture, tick).await?;
        if bytes.len() != 2 * (SIZE * SIZE * 2) as usize
            || scene.len() != (EXTENT[0] * EXTENT[1] * 8) as usize
        {
            return Err("drawn field or submitted colour dimensions differ".into());
        }
        for (layer, (left, top)) in lefts.into_iter().zip([1, 4]).enumerate() {
            let layer = layer as u32;
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let expected = if (top..top + 2).contains(&y) {
                        match x.checked_sub(left) {
                            Some(0 | 2) => 0x3c00,
                            Some(1) => 0x4000,
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    let read_red = |bytes: &[u8], pixel: u32, stride: u32| {
                        let i = (pixel * stride) as usize;
                        u16::from_le_bytes([bytes[i], bytes[i + 1]])
                    };
                    let texel = read_red(&bytes, (layer * SIZE + y) * SIZE + x, 2);
                    let shaded = read_red(&scene, y * EXTENT[0] + layer * SIZE + x, 8);
                    if (texel, shaded) != (expected, expected) {
                        return Err(format!(
                            "tick {tick}, layer {layer} ({x}, {y}): attachment {texel:#06x}, scene {shaded:#06x}, expected {expected:#06x}"
                        ));
                    }
                }
            }
        }
        report.push(format!("tick {tick}: exact 2/1/0 in both layers and both parts of submitted colour, origins {lefts:?}"));
    }
    Ok(report.join("\n"))
}

pub(crate) async fn qualify(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
) -> Result<String, String> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = Box::pin(two_frames(device, queue, capabilities)).await;
    if let Some(error) = validation.pop().await {
        return Err(format!("drawn field WebGPU validation: {error}"));
    }
    result.map(|report| format!("{report}\nWebGPU validation errors: 0"))
}
