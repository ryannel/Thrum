//! Exact attachment and forward tests. Scripted positions are independent of the renderer.
use super::StaticWorld;
use crate::{
    capabilities::{DeviceCapabilities, DeviceProfile, DeviceRestriction},
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectDrawTarget,
        EffectGraph, EffectPass, EffectState, SceneContribution, SceneContributions, SceneSurface,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    graph::{AccessKind, PassKind, ResourceDescriptor, ResourceScope},
    pass_key::PassKey,
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SurfaceOutput,
    },
    synthetic::{CaptureSource, FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
    timing::PassExecution,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_test_support::{readback, wait::until};

const SIZE: u32 = 8;
const EXTENT: [u32; 2] = [2 * SIZE, SIZE];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Rgba8Unorm,
    extent: EXTENT,
};
const PASS_NAMES: [&str; 2] = ["draw test field left", "draw test field right"];
const SHADER: &str = r"
    @vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
        return vec4f(vec2f(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0, 1.0, 1.0);
    }
    @fragment fn one() -> @location(0) vec4f { return vec4f(1.0); }
    @group(0) @binding(0) var field: texture_2d_array<f32>;
    struct Forward {
        @location(0) colour: vec4f,
        @location(1) motion: vec2f,
        @location(2) reactive: f32,
    }
    @fragment fn forward(@builtin(position) p: vec4f) -> Forward {
        let part = u32(p.x) / 8u;
        let layer = min(part, textureNumLayers(field) - 1u);
        let value = textureLoad(field, vec2i(i32(p.x) % 8, i32(p.y)), i32(layer), 0).r;
        return Forward(vec4f(value, 0.0, 0.0, 1.0), vec2f(0.0), 0.0);
    }
";

struct Field {
    first: usize,
    layers: usize,
    skippable: usize,
    state: EffectState,
    witness: Arc<Mutex<Option<wgpu::Texture>>>,
    gpu: Option<FieldGpu>,
}

struct FieldGpu {
    generation: u64,
    texture: wgpu::Texture,
    targets: Vec<EffectDrawTarget>,
    draw: wgpu::RenderPipeline,
    forward: wgpu::RenderPipeline,
    input: wgpu::BindGroup,
}

impl Field {
    fn new(first: usize, layers: usize, skippable: usize) -> Self {
        Self {
            first,
            layers,
            skippable,
            state: EffectState::new(0),
            witness: Arc::default(),
            gpu: None,
        }
    }
}

fn pipeline(
    surface: &SceneSurface<'_>,
    shader: &wgpu::ShaderModule,
    entry: &'static str,
    targets: &[Option<wgpu::ColorTargetState>],
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    surface.device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl SceneContribution for Field {
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
        self.state.rebuild(surface.generation, 0);
        let texture = surface.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test drawn field"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: u32::try_from(self.layers).unwrap(),
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
        let targets = (0..self.layers)
            .map(|index| {
                EffectDrawTarget::new(
                    PASS_NAMES[self.first + index],
                    &texture,
                    0,
                    u32::try_from(index).unwrap(),
                    wgpu::Color::TRANSPARENT,
                    true,
                    surface,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(FrameError::EffectGraph)?;
        let shader = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("simulate draw test"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let draw = pipeline(
            surface,
            &shader,
            "one",
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
        let forward = pipeline(
            surface,
            &shader,
            "forward",
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
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let input = surface
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sample test drawn field"),
                layout: &forward.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
        *self.witness.lock().unwrap() = Some(texture.clone());
        self.gpu = Some(FieldGpu {
            generation: surface.generation,
            texture,
            targets,
            draw,
            forward,
            input,
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
            passes: (0..self.layers)
                .map(|layer| EffectPass {
                    name: PASS_NAMES[self.first + layer].into(),
                    cadence: if self.first + layer == self.skippable {
                        EffectCadence::Skippable
                    } else {
                        EffectCadence::EveryFrame
                    },
                    async_eligible: false,
                    accesses: vec![access(
                        AccessKind::AttachmentClear,
                        layer as u64..layer as u64 + 1,
                    )],
                })
                .collect(),
            opaque_reads: vec![access(AccessKind::SampledRead, 0..self.layers as u64)],
            ..Default::default()
        }
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn effect_runs(&self, _: usize, frame: &ContributionFrame) -> bool {
        frame.time.tick.is_multiple_of(2)
            || self.state.committed().is_none_or(|time| {
                time.epoch != frame.time.epoch || time.timeline != frame.time.timeline
            })
    }

    fn effect_draw_target(&self, index: usize) -> Option<&EffectDrawTarget> {
        self.gpu.as_ref()?.targets.get(index)
    }

    fn draw_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        let gpu = self.gpu.as_ref().unwrap();
        self.state.begin(frame.time, 1, [0; 2]);
        let part = u32::try_from(self.first + index).unwrap();
        let x = ((frame.time.tick + u64::from(2 * part)) % 5) as u32;
        let (rectangles, height) = if part == 0 {
            ([(x, 1), (x + 1, 1)], 2)
        } else {
            ([(x, 4), (x, 5)], 3)
        };
        pass.set_pipeline(&gpu.draw);
        for (left, top) in rectangles {
            pass.set_scissor_rect(left, top, 2, height);
            pass.draw(0..3, 0..1);
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let gpu = self.gpu.as_ref().unwrap();
        pass.set_pipeline(&gpu.forward);
        pass.set_bind_group(0, &gpu.input, &[]);
        pass.set_scissor_rect(
            u32::try_from(self.first).unwrap() * SIZE,
            0,
            u32::try_from(self.layers).unwrap() * SIZE,
            SIZE,
        );
        pass.draw(0..3, 0..1);
        pass.set_scissor_rect(0, 0, EXTENT[0], EXTENT[1]);
    }

    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            self.state.submitted();
        }
    }
}

struct SunScene(SyntheticView);

impl SceneSource<SourceTick> for SunScene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 1, [0.0; 3], 0);
        })
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.0,
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}

type Host = FrameHost<StaticWorld, SunScene>;

async fn host(gpu: &crate::test_device::Opened) -> Host {
    let mut scene = SyntheticScene::grid(1).unwrap();
    scene.sun_shadows = true;
    scene.view = SyntheticView::perspective(
        [0.0, 2.0, 5.0],
        [0.0; 3],
        [0.0, 1.0, 0.0],
        1.0,
        2.0,
        0.1,
        100.0,
    )
    .unwrap();
    let source = SourceTick {
        epoch: 7,
        timeline: 3,
        tick: 0,
    };
    let policy = SunScene(scene.view);
    let mut host = FrameHost::new(
        &gpu.capabilities,
        &gpu.queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(PROFILE),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        policy,
        scene,
        1.0,
    )
    .await
    .unwrap();
    host.set_timing_requested(true);
    host
}

fn enable_capture(host: &mut Host) {
    host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
        .expect("prepare submitted scene-colour tap");
}

fn output(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("simulate draw test output"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PROFILE.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn set_tick(host: &mut Host, tick: u64) {
    host.world_mut().authority.tick = tick;
    host.world_mut().publication.tick = tick;
}

fn submit(
    host: &mut Host,
    output: &wgpu::TextureView,
    tick: u64,
) -> (super::super::SubmittedImage, super::super::FrameSample) {
    set_tick(host, tick);
    let omissions = host.submitted_capture_omissions();
    let report = host
        .advance(Duration::from_millis(16 * tick))
        .unwrap()
        .encode_scene()
        .unwrap()
        .submit(Some(&SurfaceOutput {
            view: output,
            extent: EXTENT,
        }))
        .unwrap();
    assert!(report.submitted);
    assert_eq!(
        host.submitted_capture_omissions(),
        omissions,
        "capture reservation at tick {tick}"
    );
    let (mut image, mut sample) = (None, None);
    let (image, sample) = until(
        "submitted simulate draw colour and scene timing",
        Duration::from_secs(3),
        || {
            let completed = host.poll_completed().unwrap();
            sample = sample.take().or(completed);
            if image.is_none() {
                image = host.try_submitted_image().unwrap();
            }
            (image.is_some() && sample.is_some())
                .then(|| (image.take().unwrap(), sample.take().unwrap()))
        },
    );
    assert_eq!(image.sample.tick, tick);
    assert_eq!(image.pixel_source, CaptureSource::SceneLinear);
    assert_eq!(sample.tick, tick);
    assert_eq!(sample.submission_order, image.sample.submission_order);
    (image, sample)
}

// One-dimensional scan pattern written by two overlapping rectangles. The literal
// origins are supplied per script row. No attachment or engine state sets expectations.
fn expected(origin: u32, part: u32) -> [u16; (SIZE * SIZE) as usize] {
    const ONE: u16 = 0x3c00;
    const TWO: u16 = 0x4000;
    std::array::from_fn(|index| {
        let index = u32::try_from(index).unwrap();
        let (x, y) = (index % SIZE, index / SIZE);
        if part == 0 && (1..3).contains(&y) {
            match x.checked_sub(origin) {
                Some(0 | 2) => ONE,
                Some(1) => TWO,
                _ => 0,
            }
        } else if part == 1 && (origin..origin + 2).contains(&x) {
            match y {
                4 | 7 => ONE,
                5 | 6 => TWO,
                _ => 0,
            }
        } else {
            0
        }
    })
}

fn assert_pixels(
    bytes: &[u8],
    width: u32,
    stride: u32,
    part: u32,
    expected: &[u16; (SIZE * SIZE) as usize],
    label: &str,
) {
    assert_eq!(bytes.len(), (width * SIZE * stride) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let offset = (stride * (y * width + part * SIZE + x)) as usize;
            let red = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            assert_eq!(
                red,
                expected[(y * SIZE + x) as usize],
                "{label}: part {part} ({x},{y})"
            );
        }
    }
}

fn assert_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &super::super::SubmittedImage,
    textures: [(&Arc<Mutex<Option<wgpu::Texture>>>, u32); 2],
    origins: [u32; 2],
) {
    for (part, ((witness, layer), origin)) in textures.into_iter().zip(origins).enumerate() {
        let part = u32::try_from(part).unwrap();
        let texture = witness.lock().unwrap().clone().unwrap();
        let bytes = readback::texture(device, queue, &texture, readback::Mip::layer(0, layer), ..);
        let expected = expected(origin, part);
        assert_pixels(&bytes, SIZE, 2, 0, &expected, "field attachment");
        assert_pixels(
            &image.rgba,
            EXTENT[0],
            8,
            part,
            &expected,
            "submitted scene colour",
        );
    }
}

#[test]
fn simulate_draw_gpu_orders_clears_blends_and_retains_each_layer() {
    let Some(gpu) = crate::test_device::open(
        "simulate_draw_gpu",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &DeviceRestriction::default(),
    ) else {
        return;
    };
    pollster::block_on(async {
        let mut host = Box::pin(host(&gpu)).await;
        let bare = host.scene_graph().unwrap();
        let field = Field::new(0, 2, 1);
        let witness = Arc::clone(&field.witness);
        host.set_scene_contribution(Box::new(field)).unwrap();
        let graph = host.scene_graph().unwrap();
        assert_eq!(graph.lifetimes.len(), bare.lifetimes.len() + 1);
        assert_eq!(graph.transient_slots, bare.transient_slots);
        assert_eq!(
            graph
                .pass_names()
                .filter(|name| !name.contains("draw test field"))
                .collect::<Vec<_>>(),
            bare.pass_names().collect::<Vec<_>>(),
        );
        let mut draws = Vec::new();
        let (mut sun, mut forward) = (false, false);
        for (_, record) in graph.pass_bindings() {
            match record.key {
                Some(PassKey::Effect(index)) => {
                    assert_eq!(record.kind, Some(PassKind::Render));
                    draws.push(PASS_NAMES[index as usize]);
                }
                Some(PassKey::ShadowNear | PassKey::ShadowFar | PassKey::HorizonHeights) => {
                    assert_eq!(draws, PASS_NAMES, "draws precede {}", record.name);
                    sun = true;
                }
                Some(PassKey::Opaque) => {
                    assert_eq!(draws, PASS_NAMES, "draws precede forward");
                    forward = true;
                }
                _ => {}
            }
        }
        assert!(sun && forward, "graph includes Sun passes and forward");

        enable_capture(&mut host);
        let drawable = output(&gpu.device);
        let textures = [(&witness, 0), (&witness, 1)];
        for (tick, origins) in [(0, [0, 2]), (2, [2, 4]), (3, [3, 4])] {
            let (image, sample) = submit(&mut host, &drawable, tick);
            assert_frame(&gpu.device, &gpu.queue, &image, textures, origins);
            let timing = sample
                .gpu_pass_ns
                .as_ref()
                .expect("requested Metal timestamps");
            let right = timing.execution("effect 1: draw test field right");
            if tick == 3 {
                assert_eq!(right, Some(PassExecution::NotRun));
            } else {
                assert!(matches!(right, Some(PassExecution::Timed(_))), "{right:?}");
            }
            println!(
                "tick {tick}: exact layers and submitted scene colour at {origins:?}, right pass {right:?}"
            );
            host.recycle_submitted_image(image);
        }
        set_tick(&mut host, 4);
        let discarded = host
            .advance(Duration::from_millis(64))
            .unwrap()
            .encode_scene()
            .unwrap();
        assert!(discarded.can_acquire_surface());
        drop(discarded);
        let (image, _) = submit(&mut host, &drawable, 5);
        assert_frame(&gpu.device, &gpu.queue, &image, textures, [0, 4]);
        host.recycle_submitted_image(image);
        println!("discard tick 4, submit tick 5: right layer stays at x=4");
        host.disable_submitted_capture();

        let (device, queue) = crate::test_device::portable(&gpu.gpu, None).await;
        let caps = DeviceCapabilities::capture(&gpu.gpu.adapter, &device, 2);
        let mut replacement = host
            .device_prepare_inputs()
            .unwrap()
            .prepare(&caps, &queue, Some(PROFILE))
            .await
            .unwrap();
        gpu.device.destroy();
        host.publish_device(&mut replacement).unwrap();
        enable_capture(&mut host);
        let (image, sample) = submit(&mut host, &output(&device), 9);
        assert_frame(&device, &queue, &image, textures, [4, 1]);
        assert!(matches!(
            sample
                .gpu_pass_ns
                .as_ref()
                .unwrap()
                .execution("effect 1: draw test field right"),
            Some(PassExecution::Timed(_))
        ));
        host.recycle_submitted_image(image);
        println!("replacement generation first frame at odd tick 9 drew both layers");
    });
}

#[test]
fn simulate_draw_list_gpu_routes_each_members_view_draw_and_cadence() {
    let Some(gpu) = crate::test_device::open(
        "simulate_draw_list_gpu",
        Some(wgpu::Backend::Metal),
        DeviceProfile::Portable,
        &DeviceRestriction::default(),
    ) else {
        return;
    };
    pollster::block_on(async {
        let mut host = Box::pin(host(&gpu)).await;
        let left = Field::new(0, 1, 0);
        let right = Field::new(1, 1, 0);
        let witnesses = [Arc::clone(&left.witness), Arc::clone(&right.witness)];
        host.set_scene_contribution(Box::new(SceneContributions::new(vec![
            Box::new(left),
            Box::new(right),
        ])))
        .unwrap();
        enable_capture(&mut host);
        let textures = [(&witnesses[0], 0), (&witnesses[1], 0)];
        let drawable = output(&gpu.device);
        for (tick, origins) in [(0, [0, 2]), (2, [2, 4]), (3, [2, 0])] {
            let (image, _) = submit(&mut host, &drawable, tick);
            assert_frame(&gpu.device, &gpu.queue, &image, textures, origins);
            host.recycle_submitted_image(image);
            println!(
                "tick {tick}: exact member-owned attachments and submitted scene colour at {origins:?}"
            );
        }
    });
}
