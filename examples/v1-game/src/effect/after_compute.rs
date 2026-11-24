//! External consumer of late trace, history resolve, composition and compute epilogue.
use super::{
    after_opaque::{half, host},
    qualification::{read_texture, read_texture_bytes, submitted_colour},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_render::binding;
use thrum_render::contribution::{
    AfterOpaqueKind as K, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
    EffectCadence as C, EffectGraph, EffectState, EffectStep, HistoryBinding as H,
    SceneContribution, SceneSurface,
};
use thrum_render::graph::{
    AccessKind as A, ResourceDescriptor as D, ResourceScope as S, TextureKind,
};

struct Witness {
    state: EffectState,
    textures: Vec<wgpu::Texture>,
}
type Handle = Arc<Mutex<Witness>>;
struct Effect {
    handle: Handle,
    gpu: Option<Gpu>,
    step: EffectStep,
    generation: u64,
}
struct Gpu {
    queue: wgpu::Queue,
    params: wgpu::Buffer,
    trace: wgpu::ComputePipeline,
    resolve: wgpu::ComputePipeline,
    epilogue: wgpu::ComputePipeline,
    render: wgpu::RenderPipeline,
    trace_group: wgpu::BindGroup,
    resolve_groups: [wgpu::BindGroup; 2],
    epilogue_groups: [wgpu::BindGroup; 2],
    render_groups: [wgpu::BindGroup; 2],
}
fn access(resource: usize, kind: A, history: Option<H>) -> EffectAccess {
    EffectAccess {
        resource,
        range: 0..if resource == 3 { 16 } else { 1 },
        kind,
        history,
        view: None,
    }
}
fn declaration(
    name: &str,
    kind: K,
    cadence: C,
    depth: bool,
    accesses: Vec<EffectAccess>,
) -> AfterOpaquePass {
    AfterOpaquePass {
        name: name.into(),
        kind,
        cadence,
        depth,
        write_depth: false,
        refraction: false,
        accesses,
    }
}
fn texture() -> D {
    D::Texture {
        dimension: TextureKind::D2,
        width: 4,
        height: 1,
        layers: 1,
        mips: 1,
        samples: 1,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
    }
}
impl SceneContribution for Effect {
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            view_volume_media: Vec::new(),
            sun_paths: Vec::new(),
            resources: vec![
                (texture(), S::Transient),
                (
                    texture(),
                    S::History {
                        owner: 0,
                        epoch: self.generation,
                        initialized: true,
                    },
                ),
                (
                    texture(),
                    S::Persistent {
                        owner: 1,
                        epoch: self.generation,
                        initialized: true,
                    },
                ),
                (
                    D::Buffer {
                        bytes: 16,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    },
                    S::Imported { initialized: true },
                ),
            ],
            passes: Vec::new(),
            opaque_reads: Vec::new(),
            after_opaque: vec![
                declaration(
                    "late depth trace",
                    K::Compute,
                    C::EveryFrame,
                    true,
                    vec![
                        access(0, A::StorageWrite, None),
                        access(3, A::UniformRead, None),
                    ],
                ),
                declaration(
                    "late history resolve",
                    K::Compute,
                    C::Skippable,
                    false,
                    vec![
                        access(0, A::SampledRead, None),
                        access(1, A::SampledRead, Some(H::Previous)),
                        access(1, A::StorageWrite, Some(H::Next)),
                        access(3, A::UniformRead, None),
                    ],
                ),
                declaration(
                    "late composition",
                    K::Render,
                    C::EveryFrame,
                    false,
                    vec![access(1, A::SampledRead, Some(H::Result))],
                ),
                declaration(
                    "late compute epilogue",
                    K::Compute,
                    C::EveryFrame,
                    false,
                    vec![
                        access(1, A::SampledRead, Some(H::Result)),
                        access(2, A::StorageWrite, None),
                    ],
                ),
            ],
        }
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        if self.generation == surface.generation {
            return Ok(());
        }
        self.generation = surface.generation;
        let gpu = Gpu::new(surface, &self.effect_graph(), &self.handle)
            .map_err(thrum_render::runtime::FrameError::Scene)?;
        self.handle
            .lock()
            .map_err(|_| {
                thrum_render::runtime::FrameError::InvalidState("compute effect state unavailable")
            })?
            .state
            .rebuild(surface.generation, 1);
        self.gpu = Some(gpu);
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.step = self
            .handle
            .lock()
            .expect("late witness")
            .state
            .begin(frame.time, 8, [0, 0]);
        if let Some(gpu) = &self.gpu {
            let mut bytes = [0; 16];
            bytes[..4].copy_from_slice(&u32::from(self.step == EffectStep::Reset).to_le_bytes());
            gpu.queue.write_buffer(&gpu.params, 0, &bytes);
        }
    }
    fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        index != 1 || self.step != EffectStep::Keep
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        if let Some(gpu) = &self.gpu {
            let mut witness = self.handle.lock().expect("late witness");
            match index {
                0 => {
                    pass.set_pipeline(&gpu.trace);
                    pass.set_bind_group(2, &gpu.trace_group, &[]);
                }
                1 => {
                    pass.set_pipeline(&gpu.resolve);
                    pass.set_bind_group(
                        2,
                        &gpu.resolve_groups[witness.state.side(0, H::Previous)],
                        &[],
                    );
                    witness.state.produced(0);
                }
                _ => {
                    pass.set_pipeline(&gpu.epilogue);
                    pass.set_bind_group(
                        2,
                        &gpu.epilogue_groups[witness.state.side(0, H::Result)],
                        &[],
                    );
                }
            }
            pass.dispatch_workgroups(1, 1, 1);
        }
    }
    fn draw_after_opaque(&self, _: usize, pass: &mut wgpu::RenderPass<'_>, _: &ContributionFrame) {
        if let Some(gpu) = &self.gpu {
            let witness = self.handle.lock().expect("late witness");
            pass.set_pipeline(&gpu.render);
            pass.set_bind_group(2, &gpu.render_groups[witness.state.side(0, H::Result)], &[]);
            pass.draw(0..3, 0..1);
        }
    }
    fn submitted(&mut self, advance: bool) {
        if advance {
            self.handle.lock().expect("late witness").state.submitted();
        }
    }
}
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn layout(
    surface: &SceneSurface<'_>,
    entries: &[(u32, bool)],
    uniform: Option<u32>,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    let mut entries = entries
        .iter()
        .map(|&(binding, storage)| {
            if storage {
                binding::storage_texture(
                    binding,
                    visibility,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::R32Uint,
                    wgpu::TextureViewDimension::D2,
                )
            } else {
                binding::texture(
                    binding,
                    visibility,
                    wgpu::TextureSampleType::Uint,
                    wgpu::TextureViewDimension::D2,
                )
            }
        })
        .collect::<Vec<_>>();
    if let Some(binding) = uniform {
        entries.push(binding::uniform(binding, visibility, 16));
    }
    surface
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("late witness owned"),
            entries: &entries,
        })
}
fn pipeline_layout(
    surface: &SceneSurface<'_>,
    pass: &AfterOpaquePass,
    own: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    surface
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&pass.name),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&pass.input_layout(surface.device)),
                Some(own),
            ],
            immediate_size: 0,
        })
}
fn compute(
    surface: &SceneSurface<'_>,
    name: &str,
    source: &str,
    layout: &wgpu::PipelineLayout,
) -> wgpu::ComputePipeline {
    let module = surface
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
    surface.device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some(name),
        layout: Some(layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}
impl Gpu {
    fn new(
        surface: &SceneSurface<'_>,
        graph: &EffectGraph,
        handle: &Handle,
    ) -> Result<Self, String> {
        let textures = (0..4)
            .map(|_| {
                surface.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("late witness"),
                    size: wgpu::Extent3d {
                        width: 4,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R32Uint,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
            })
            .collect::<Vec<_>>();
        let views = textures
            .iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<_>>();
        let params = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("late reset"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let trace_layout = layout(surface, &[(0, true)], None, wgpu::ShaderStages::COMPUTE);
        let resolve_layout = layout(
            surface,
            &[(0, false), (1, false), (2, true)],
            Some(3),
            wgpu::ShaderStages::COMPUTE,
        );
        let render_layout = layout(surface, &[(0, false)], None, wgpu::ShaderStages::FRAGMENT);
        let epilogue_layout = layout(
            surface,
            &[(0, false), (1, true)],
            None,
            wgpu::ShaderStages::COMPUTE,
        );
        let trace = compute(
            surface,
            "late depth trace",
            include_str!("after_compute/trace.wgsl"),
            &pipeline_layout(surface, &graph.after_opaque[0], &trace_layout),
        );
        let resolve = compute(
            surface,
            "late history resolve",
            include_str!("after_compute/resolve.wgsl"),
            &pipeline_layout(surface, &graph.after_opaque[1], &resolve_layout),
        );
        let epilogue = compute(
            surface,
            "late compute epilogue",
            include_str!("after_compute/epilogue.wgsl"),
            &pipeline_layout(surface, &graph.after_opaque[3], &epilogue_layout),
        );
        let module = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("late composition"),
                source: wgpu::ShaderSource::Wgsl(include_str!("after_compute/compose.wgsl").into()),
            });
        let render = surface.device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("late composition"),
            layout: Some(&pipeline_layout(
                surface,
                &graph.after_opaque[2],
                &render_layout,
            )),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &surface.color_formats.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        });
        let bind = |layout: &wgpu::BindGroupLayout, entries: &[wgpu::BindGroupEntry<'_>]| {
            surface
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("late witness"),
                    layout,
                    entries,
                })
        };
        let texture = |binding, index| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&views[index]),
        };
        let trace_group = bind(&trace_layout, &[texture(0, 0)]);
        let resolve_groups = std::array::from_fn(|side| {
            bind(
                &resolve_layout,
                &[
                    texture(0, 0),
                    texture(1, side + 1),
                    texture(2, 2 - side),
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params.as_entire_binding(),
                    },
                ],
            )
        });
        let epilogue_groups = std::array::from_fn(|side| {
            bind(&epilogue_layout, &[texture(0, side + 1), texture(1, 3)])
        });
        let render_groups =
            std::array::from_fn(|side| bind(&render_layout, &[texture(0, side + 1)]));
        handle.lock().map_err(error)?.textures = textures;
        Ok(Self {
            queue: surface.queue.clone(),
            params,
            trace,
            resolve,
            epilogue,
            render,
            trace_group,
            resolve_groups,
            epilogue_groups,
            render_groups,
        })
    }
}

#[cfg_attr(
    target_arch = "wasm32",
    expect(
        clippy::arc_with_non_send_sync,
        reason = "the shared native/WebGPU witness stays on the browser's single device thread"
    )
)]
pub(super) async fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &thrum_render::capabilities::DeviceCapabilities,
) -> Result<String, String> {
    let mut host = host(caps, queue, true, true).await?;
    let handle = Arc::new(Mutex::new(Witness {
        state: EffectState::new(1),
        textures: Vec::new(),
    }));
    host.set_scene_contribution(Box::new(Effect {
        handle: Arc::clone(&handle),
        gpu: None,
        step: EffectStep::Keep,
        generation: 0,
    }))?;
    let graph = host.scene_graph().map_err(error)?;
    let order = graph
        .pass_names()
        .filter(|name| name.starts_with("effect after "))
        .collect::<Vec<_>>();
    if order
        != [
            "effect after 0: late depth trace",
            "effect after 1: late history resolve",
            "effect after 2: late composition",
            "effect after 3: late compute epilogue",
        ]
    {
        return Err(format!("late compute stage order: {order:?}"));
    }
    let names = graph.pass_names().collect::<Vec<_>>();
    let epilogue = names
        .iter()
        .position(|name| *name == "effect after 3: late compute epilogue");
    let resolve = names.iter().position(|name| *name == "portable TAAU");
    if !matches!((epilogue, resolve), (Some(epilogue), Some(resolve)) if epilogue < resolve) {
        return Err("reconstruction did not follow the final late compute pass".into());
    }
    host.enable_submitted_capture_from(
        41,
        thrum_render::runtime::CaptureLimits::default(),
        thrum_render::synthetic::CaptureSource::SceneLinear,
    )
    .map_err(error)?;
    let mut expected = [0u32; 4];
    for (ordinal, (tick, discard)) in [
        (10, false),
        (11, false),
        (11, false),
        (12, true),
        (11, false),
        (12, false),
    ]
    .into_iter()
    .enumerate()
    {
        host.world_mut().0.tick = tick;
        let frame = host
            .advance(Duration::from_secs(ordinal as u64))
            .map_err(error)?
            .encode_scene()
            .map_err(error)?;
        if discard {
            drop(frame);
        } else {
            frame.submit(None).map_err(error)?;
            let depth = host
                .submitted_temporal_witness()
                .ok_or("late compute temporal witness")?
                .clone();
            let bytes = read_texture_bytes(device, queue, &depth, 0, 8).await?;
            if ordinal == 0 || ordinal == 1 || ordinal == 5 {
                for (i, x) in [0, 25, 39, 63].into_iter().enumerate() {
                    expected[i] +=
                        u32::from(half(&bytes[(32 * 64 + x) * 8 + 2..(32 * 64 + x) * 8 + 4]) > 0.);
                }
            }
            if ordinal == 0 && expected.iter().all(|value| *value == 0) {
                return Err("late depth trace witness was empty".into());
            }
            let (result, epilogue) = {
                let witness = handle.lock().map_err(error)?;
                (
                    witness.textures[witness.state.side(0, H::Previous) + 1].clone(),
                    witness.textures[3].clone(),
                )
            };
            if read_texture(device, queue, &result, 0).await? != expected {
                return Err(format!("late history run/skip/discard at {ordinal}"));
            }
            if read_texture(device, queue, &epilogue, 0).await? != expected.map(|v| v + 100) {
                return Err("compute after final render did not execute".into());
            }
            let pixels = submitted_colour(&mut host, device, queue, &result, tick).await?;
            for i in 0..3 {
                if (half(&pixels[i * 2..i * 2 + 2])
                    - f32::from(u16::try_from(expected[i]).map_err(error)?) / 16.)
                    .abs()
                    > f32::EPSILON
                {
                    return Err("render did not read latest late-history result".into());
                }
            }
        }
    }
    Ok("late compute trace -> history resolve -> render -> compute epilogue -> portable TAAU: exact submitted depth masks, run/skip/discard/retry and composition pixels".into())
}

#[cfg(test)]
mod tests;
