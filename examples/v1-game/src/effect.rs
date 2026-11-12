//! Opt-in integer diagnostic for EFFECT-GRAPH. It is never installed by default.
pub mod qualification;
mod visual;

use std::sync::{Arc, Mutex};
use thrum_render::{
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence as C, EffectGraph,
        EffectPass, EffectState, EffectStep, HistoryBinding as H, SceneContribution, SceneSurface,
    },
    graph::{AccessKind as A, ResourceDescriptor as D, ResourceScope as S, TextureKind as K},
};

/// Field width and height in integer texels.
pub const SIZE: u32 = 32;
const BOUND: u64 = 8;

/// Runtime controls, sampled only at a frame boundary.
#[derive(Clone, Copy, Debug)]
pub struct EffectControls {
    /// Second history updates every this many world ticks (at least one).
    pub cadence: u64,
    /// Whether the moving source deposits new trail samples.
    pub source: bool,
    /// Integer world origin of the toroidal field.
    pub anchor: [i64; 2],
    /// Ground witness height in world metres.
    pub ground_height: f32,
}
impl Default for EffectControls {
    fn default() -> Self {
        Self {
            cadence: 3,
            source: true,
            anchor: [0; 2],
            ground_height: 0.0,
        }
    }
}

/// Submitted state and physical resources, lent read-only to qualification code.
pub struct EffectWitness {
    /// Runtime controls for the next frame.
    pub controls: EffectControls,
    /// Independently committed history clock and anchor.
    pub state: EffectState,
    /// Device generation of all published textures.
    pub generation: u64,
    /// Histories A and B, consumer, volume, array and cube, in declaration order.
    pub textures: Vec<Vec<wgpu::Texture>>,
}

/// Shared qualification handle; no device or submission authority is held here.
pub type EffectHandle = Arc<Mutex<EffectWitness>>;

/// A game-owned effect using only thrum-render's public contribution seam.
pub struct DiagnosticEffect {
    witness: EffectHandle,
    gpu: Option<Prepared>,
    step: EffectStep,
    runs: [bool; 7],
    scripted: bool,
}

struct Prepared {
    queue: wgpu::Queue,
    params: wgpu::Buffer,
    pipelines: Vec<wgpu::ComputePipeline>,
    groups: Vec<Vec<wgpu::BindGroup>>,
    visual: visual::Visual,
}

impl DiagnosticEffect {
    /// Enable the capture's world-clock stop and cadence transition, off by default.
    #[must_use]
    pub fn scripted(mut self) -> Self {
        self.scripted = true;
        self
    }

    /// Make an unattached diagnostic and its qualification handle.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "the browser uses the same witness API on its single WebGPU owner thread; native captures share it with the frame owner"
        )
    )]
    pub fn new() -> (Self, EffectHandle) {
        let witness = Arc::new(Mutex::new(EffectWitness {
            controls: EffectControls::default(),
            state: EffectState::new(2),
            generation: 0,
            textures: Vec::new(),
        }));
        (
            Self {
                witness: Arc::clone(&witness),
                gpu: None,
                step: EffectStep::Keep,
                runs: [false; 7],
                scripted: false,
            },
            witness,
        )
    }
}

fn descriptor(kind: K, size: u32, mips: u32) -> D {
    D::Texture {
        dimension: kind,
        width: size,
        height: size,
        layers: if kind == K::Cube {
            6
        } else if kind == K::D2 {
            2
        } else {
            1
        },
        mips,
        samples: 1,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
    }
}
fn access(resource: usize, kind: A, history: Option<H>) -> EffectAccess {
    EffectAccess {
        resource,
        range: 0..1,
        kind,
        history,
        view: Some(wgpu::TextureViewDimension::D2),
    }
}
fn pass(name: &str, cadence: C, accesses: Vec<EffectAccess>) -> EffectPass {
    EffectPass {
        name: name.into(),
        cadence,
        accesses,
    }
}

impl SceneContribution for DiagnosticEffect {
    fn effect_graph(&self) -> EffectGraph {
        let mut field = descriptor(K::D2, SIZE, 1);
        if let D::Texture { layers, .. } = &mut field {
            *layers = 1;
        }
        let resources = vec![
            (
                field.clone(),
                S::History {
                    owner: 0,
                    epoch: 1,
                    initialized: true,
                },
            ),
            (
                field.clone(),
                S::History {
                    owner: 1,
                    epoch: 1,
                    initialized: true,
                },
            ),
            (
                field,
                S::Persistent {
                    owner: 2,
                    epoch: 1,
                    initialized: true,
                },
            ),
            (
                descriptor(K::D3 { depth: 4 }, 8, 1),
                S::Persistent {
                    owner: 3,
                    epoch: 1,
                    initialized: true,
                },
            ),
            (
                descriptor(K::D2, 8, 3),
                S::Persistent {
                    owner: 4,
                    epoch: 1,
                    initialized: true,
                },
            ),
            (
                descriptor(K::Cube, 8, 1),
                S::Persistent {
                    owner: 5,
                    epoch: 1,
                    initialized: true,
                },
            ),
        ];
        let mut passes = vec![
            pass(
                "trail",
                C::Skippable,
                vec![
                    access(0, A::SampledRead, Some(H::Previous)),
                    access(0, A::StorageWrite, Some(H::Next)),
                ],
            ),
            pass(
                "derived",
                C::Skippable,
                vec![
                    access(0, A::SampledRead, Some(H::Result)),
                    access(1, A::StorageWrite, Some(H::Next)),
                ],
            ),
            pass(
                "consumer",
                C::EveryFrame,
                vec![
                    access(0, A::SampledRead, Some(H::Result)),
                    access(1, A::SampledRead, Some(H::Result)),
                    access(2, A::StorageWrite, None),
                ],
            ),
        ];
        for (resource, range, view, name, cadence) in [
            (
                3,
                0..1,
                wgpu::TextureViewDimension::D3,
                "volume",
                C::TimeSliced,
            ),
            (
                4,
                0..2,
                wgpu::TextureViewDimension::D2Array,
                "array mip 0",
                C::Skippable,
            ),
            (
                4,
                2..4,
                wgpu::TextureViewDimension::D2Array,
                "array mip 1",
                C::Skippable,
            ),
            (
                4,
                4..6,
                wgpu::TextureViewDimension::D2Array,
                "array mip 2",
                C::Skippable,
            ),
            (
                5,
                0..6,
                wgpu::TextureViewDimension::D2Array,
                "cube",
                C::Skippable,
            ),
        ] {
            passes.push(pass(
                name,
                cadence,
                vec![EffectAccess {
                    resource,
                    range,
                    kind: A::StorageWrite,
                    view: Some(view),
                    history: None,
                }],
            ));
        }
        EffectGraph { resources, passes }
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let mut witness = self.witness.lock().map_err(|error| error.to_string())?;
        if witness.generation == surface.generation {
            return Ok(());
        }
        let graph = self.effect_graph();
        let textures = graph
            .resources
            .iter()
            .map(|(descriptor, scope)| {
                let D::Texture {
                    dimension,
                    width,
                    height,
                    layers,
                    mips,
                    samples,
                    format,
                    usage,
                } = descriptor
                else {
                    unreachable!("diagnostic is textures only")
                };
                let create = || {
                    surface.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("effect diagnostic"),
                        size: wgpu::Extent3d {
                            width: *width,
                            height: *height,
                            depth_or_array_layers: if let K::D3 { depth } = dimension {
                                *depth
                            } else {
                                *layers
                            },
                        },
                        mip_level_count: *mips,
                        sample_count: *samples,
                        dimension: if matches!(dimension, K::D3 { .. }) {
                            wgpu::TextureDimension::D3
                        } else {
                            wgpu::TextureDimension::D2
                        },
                        format: *format,
                        usage: *usage,
                        view_formats: &[],
                    })
                };
                if matches!(scope, S::History { .. }) {
                    vec![create(), create()]
                } else {
                    vec![create()]
                }
            })
            .collect::<Vec<_>>();
        let params = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect parameters"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut pipelines = Vec::new();
        let mut groups = Vec::new();
        for index in 0..8 {
            let source = shader(index);
            let module = surface
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("effect diagnostic"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let pipeline = surface.device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(&graph.passes[index].name),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            let layout = pipeline.get_bind_group_layout(0);
            let mut variants = Vec::new();
            for a in 0..2 {
                for b in 0..2 {
                    let mut views = Vec::new();
                    let view = |resource: usize, side: usize, dimension, mip| {
                        textures[resource][side].create_view(&wgpu::TextureViewDescriptor {
                            dimension: Some(dimension),
                            base_mip_level: mip,
                            mip_level_count: Some(1),
                            ..Default::default()
                        })
                    };
                    match index {
                        0 => {
                            views.push(view(0, a, wgpu::TextureViewDimension::D2, 0));
                            views.push(view(0, 1 - a, wgpu::TextureViewDimension::D2, 0));
                        }
                        1 => {
                            views.push(view(0, a, wgpu::TextureViewDimension::D2, 0));
                            views.push(view(1, 1 - b, wgpu::TextureViewDimension::D2, 0));
                        }
                        2 => {
                            views.push(view(0, a, wgpu::TextureViewDimension::D2, 0));
                            views.push(view(1, b, wgpu::TextureViewDimension::D2, 0));
                            views.push(view(2, 0, wgpu::TextureViewDimension::D2, 0));
                        }
                        3 => views.push(view(3, 0, wgpu::TextureViewDimension::D3, 0)),
                        4..=6 => views.push(view(
                            4,
                            0,
                            wgpu::TextureViewDimension::D2Array,
                            u32::try_from(index - 4).map_err(|error| error.to_string())?,
                        )),
                        _ => views.push(view(5, 0, wgpu::TextureViewDimension::D2Array, 0)),
                    }
                    let mut entries = vec![wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params.as_entire_binding(),
                    }];
                    entries.extend(views.iter().enumerate().map(|(binding, view)| {
                        wgpu::BindGroupEntry {
                            binding: u32::try_from(binding + 1).expect("at most three textures"),
                            resource: wgpu::BindingResource::TextureView(view),
                        }
                    }));
                    variants.push(
                        surface
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("effect prepared sides"),
                                layout: &layout,
                                entries: &entries,
                            }),
                    );
                }
            }
            pipelines.push(pipeline);
            groups.push(variants);
        }
        let visual = visual::Visual::new(surface, &params, &textures[0]);
        witness.generation = surface.generation;
        witness.textures = textures;
        witness.state.rebuild(surface.generation, 1);
        self.gpu = Some(Prepared {
            queue: surface.queue.clone(),
            params,
            pipelines,
            groups,
            visual,
        });
        Ok(())
    }
    fn encode_upload(&mut self, _encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let mut witness = self.witness.lock().expect("diagnostic witness");
        if self.scripted {
            witness.controls.source = frame.time.tick < 270;
            witness.controls.cadence = if frame.time.tick < 300 { 2 } else { 4 };
            witness.controls.anchor = [snap_anchor(frame.camera[0]), snap_anchor(frame.camera[2])];
        }
        let controls = witness.controls;
        self.step = witness.state.begin(frame.time, BOUND, controls.anchor);
        let full = self.step == EffectStep::Reset;
        let advance = matches!(self.step, EffectStep::Advance { .. });
        self.runs = [
            full || advance,
            full || (advance && frame.time.tick.is_multiple_of(controls.cadence.max(1))),
            true,
            !frame.time.capture,
            full || advance,
            full || advance,
            full || advance,
        ];
        let ticks = match self.step {
            EffectStep::Advance { ticks, .. } => ticks,
            _ => 0,
        };
        let values = [
            u32::try_from(frame.time.tick % u64::from(u32::MAX)).unwrap_or(0),
            u32::try_from(ticks).unwrap_or(0),
            u32::from(full),
            u32::from(controls.source),
            u32::from(full),
            u32::try_from(frame.time.tick % 4).unwrap_or(0),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        let mut bytes = [0_u8; 128];
        for (index, value) in values.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        let origin = [
            controls.anchor[0] as f32,
            controls.ground_height,
            controls.anchor[1] as f32,
            0.0,
        ];
        for (index, value) in origin.into_iter().enumerate() {
            bytes[32 + index * 4..36 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in frame.view_proj.iter().flatten().enumerate() {
            bytes[64 + index * 4..68 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        if let Some(gpu) = &self.gpu {
            gpu.queue.write_buffer(&gpu.params, 0, &bytes);
        }
    }
    fn effect_runs(&self, index: usize, frame: &ContributionFrame) -> bool {
        if index == 7 {
            !frame.time.capture && self.runs[0]
        } else {
            self.runs[index]
        }
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
        let Some(gpu) = &self.gpu else {
            return;
        };
        let mut witness = self.witness.lock().expect("diagnostic witness");
        let a = witness
            .state
            .side(0, if index == 0 { H::Previous } else { H::Result });
        let b = witness
            .state
            .side(1, if index == 1 { H::Previous } else { H::Result });
        pass.set_pipeline(&gpu.pipelines[index]);
        pass.set_bind_group(0, &gpu.groups[index][a * 2 + b], &[]);
        let (size, depth) = match index {
            0..=2 => (SIZE, 1),
            3 => (8, 4),
            4..=6 => (8 >> (index - 4), 2),
            _ => (8, 6),
        };
        pass.dispatch_workgroups(size.div_ceil(8), size.div_ceil(8), depth);
        if index < 2 {
            witness.state.produced(index);
        }
    }
    fn computes(&self, _: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(gpu) = &self.gpu {
            let witness = self.witness.lock().expect("diagnostic witness");
            gpu.visual.draw(pass, witness.state.side(0, H::Result));
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            self.witness
                .lock()
                .expect("diagnostic witness")
                .state
                .submitted();
        }
    }
}

fn shader(index: usize) -> String {
    let header = "struct Params { tick:u32, ticks:u32, reset:u32, source:u32, full:u32, slice:u32, pad0:u32, pad1:u32 }; @group(0) @binding(0) var<uniform> p:Params;";
    let body = match index {
        0 => {
            r"@group(0) @binding(1) var previous:texture_2d<u32>; @group(0) @binding(2) var next:texture_storage_2d<r32uint,write>;
        @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) { let q=vec2<i32>(id.xy); if(any(id.xy>=vec2<u32>(32))){return;} var v=0u;
        if(p.reset==0u){let n=i32(p.ticks); for(var y=-n;y<=n;y++){for(var x=-n;x<=n;x++){if(abs(x)+abs(y)<=n){let old=textureLoad(previous,(q+vec2<i32>(x,y)+vec2<i32>(64))%vec2<i32>(32),0).x; v=max(v,old-min(old,p.ticks*2u));}}}
        if(p.source!=0u){for(var age=0u;age<p.ticks;age++){let sx=i32((p.tick-age)%32u); let dx=min(abs(q.x-sx),32-abs(q.x-sx)); let dy=min(abs(q.y-16),32-abs(q.y-16)); if(u32(dx+dy)<=age){v=max(v,255u-age*2u);}}}}
        textureStore(next,q,vec4<u32>(v)); }"
        }
        1 => {
            r"@group(0) @binding(1) var trail:texture_2d<u32>; @group(0) @binding(2) var next:texture_storage_2d<r32uint,write>;
        @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {if(any(id.xy>=vec2<u32>(32))){return;} let q=vec2<i32>(id.xy); textureStore(next,q,vec4<u32>(textureLoad(trail,q,0).x*3u));}"
        }
        2 => {
            r"@group(0) @binding(1) var trail:texture_2d<u32>; @group(0) @binding(2) var derived:texture_2d<u32>; @group(0) @binding(3) var result:texture_storage_2d<r32uint,write>;
        @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {if(any(id.xy>=vec2<u32>(32))){return;} let q=vec2<i32>(id.xy); textureStore(result,q,vec4<u32>(textureLoad(trail,q,0).x+1024u*textureLoad(derived,q,0).x));}"
        }
        3 => {
            r"@group(0) @binding(1) var result:texture_storage_3d<r32uint,write>;
        @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {if(p.full==0u && (p.ticks==0u || id.z!=p.slice)){return;} textureStore(result,vec3<i32>(id),vec4<u32>(select(p.tick,0u,p.reset!=0u)+id.x+16u*id.y+256u*id.z));}"
        }
        _ => {
            r"@group(0) @binding(1) var result:texture_storage_2d_array<r32uint,write>;
        @compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {let size=textureDimensions(result); if(any(id.xy>=size)){return;} textureStore(result,vec2<i32>(id.xy),i32(id.z),vec4<u32>(select(p.tick,0u,p.reset!=0u)+id.x+16u*id.y+256u*id.z));}"
        }
    };
    format!("{header}\n{body}")
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the diagnostic deliberately snaps finite camera metres to integer toroidal cells"
)]
fn snap_anchor(position: f32) -> i64 {
    ((position / 32.0).floor() as i64).saturating_mul(32)
}
