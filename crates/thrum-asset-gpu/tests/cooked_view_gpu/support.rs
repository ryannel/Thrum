use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedDrawPath, MeshPlacement};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, SceneContribution, SceneContributions, SceneSurface,
        SecondaryView, SecondaryViewRequest, SecondaryViewSlot,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{
        FrameLighting, PhotometricEnvironment, SyntheticPointLight, SyntheticScene, SyntheticView,
    },
};

const PAGE: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 1,
};
const LIGHT: FrameLighting = FrameLighting::Photometric(PhotometricEnvironment {
    toward_sun: [0.0, 0.8, 0.6],
    sun_lux: 50_000.0,
    sun_colour: [1.0; 3],
    ambient_lux: [1500.0; 3],
    background_radiance: [0.1; 3],
});

struct World;
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        PAGE
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: 0,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: 0,
            },
            timeline: 1,
            page: PAGE,
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

struct Source(SyntheticView);
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |_| {})
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.0,
            lighting: LIGHT,
        })
    }
}

pub(super) fn camera(eye: [f32; 3], target: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], 0.92, 1.0, 0.1, 4000.0)
        .expect("camera")
}

pub(super) enum Witness {
    Main,
    Idle,
    View(Box<SecondaryViewRequest>),
}

pub(super) struct Fixture<'a> {
    pub(super) caps: &'a DeviceCapabilities,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

pub(super) struct Observation {
    pub(super) pixels: Vec<[f32; 8]>,
    pub(super) path: CookedDrawPath,
    pub(super) secondary: Vec<(usize, CookedDrawPath)>,
    pub(super) attachment_pipelines: Option<u64>,
}

impl Fixture<'_> {
    pub(super) fn draw(&self, view: &SyntheticView, side: u32, witness: Witness) -> Observation {
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = *view;
        scene.sun_shadows = true;
        scene.instances[0].ids[3] = 0;
        scene.local_lights.push(SyntheticPointLight {
            position: [-2.0, 1.0, 0.8],
            radius: 5.0,
            candela: [180.0, 22.0, 5.0],
        });
        let mut host = pollster::block_on(FrameHost::new(
            self.caps,
            self.queue,
            FrameConfig {
                render_extent: [side; 2],
                surface: None,
            },
            World,
            Source(*view),
            scene,
            1.0,
        ))
        .expect("host");
        let mut placements = vec![
            MeshPlacement::new("meshes/rock", "textures/white")
                .scale(1.3)
                .on_ground(),
        ];
        for x in [-3.8, 3.7, 0.85] {
            placements.push(
                MeshPlacement::new("meshes/rock", "textures/white")
                    .at([x, 0.0, -1.6])
                    .scale(0.75)
                    .on_ground(),
            );
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        let cooked = CookedAssets::new(CookedConfig::new(path, placements)).expect("cooked");
        let stats = cooked.stats();
        let output = Arc::new(Mutex::new(None));
        let before = host.pipeline_creation_count();
        host.set_scene_contribution(Box::new(SceneContributions::new(vec![
            Box::new(cooked),
            Box::new(Probe {
                size: if matches!(witness, Witness::View(_)) {
                    32
                } else {
                    side
                },
                witness,
                output: Arc::clone(&output),
                prepared: None,
            }),
        ])))
        .expect("cooked and probe");
        let attachment_pipelines = host
            .pipeline_creation_count()
            .zip(before)
            .map(|(count, before)| count - before);
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            host.capture_pixels().expect("frame");
            let stats = stats.lock().expect("stats");
            if stats.drawn.is_some() {
                break;
            }
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            assert!(Instant::now() < deadline, "cooked asset never drew");
            drop(stats);
            std::thread::sleep(Duration::from_millis(2));
        }
        host.capture_pixels().expect("resident frame");
        let buffer = output
            .lock()
            .expect("output")
            .clone()
            .expect("prepared output");
        let pixels = self.read(&buffer);
        let stats = stats.lock().expect("stats");
        Observation {
            pixels,
            path: stats.draw_path.expect("path"),
            secondary: stats.secondary_draw_paths.clone(),
            attachment_pipelines,
        }
    }

    fn read(&self, source: &wgpu::Buffer) -> Vec<[f32; 8]> {
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cooked comparison readback"),
            size: source.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(source, 0, &readback, 0, source.size());
        let submit = self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, .., move |result| {
            send.send(result).expect("map");
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submit),
                timeout: Some(Duration::from_secs(15)),
            })
            .expect("Metal");
        receive.recv().expect("map callback").expect("readback");
        bytemuck::cast_slice(&readback.get_mapped_range(..).expect("mapped")).to_vec()
    }
}

struct Probe {
    witness: Witness,
    size: u32,
    output: Arc<Mutex<Option<wgpu::Buffer>>>,
    prepared: Option<(
        wgpu::ComputePipeline,
        wgpu::BindGroup,
        Option<SecondaryView>,
    )>,
}
impl Probe {
    fn bytes(&self) -> u64 {
        u64::from(self.size).pow(2) * 32
    }
    fn declaration(&self) -> AfterOpaquePass {
        let main = !matches!(self.witness, Witness::View(_));
        AfterOpaquePass {
            name: "cooked comparison".into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::EveryFrame,
            accesses: vec![EffectAccess {
                resource: 0,
                range: 0..self.bytes(),
                kind: AccessKind::StorageWrite,
                view: None,
                history: None,
            }],
            depth: main,
            write_depth: false,
            refraction: main,
        }
    }
}

impl SceneContribution for Probe {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        match self.witness {
            Witness::Main => Vec::new(),
            _ => vec![SecondaryViewSlot::new(1.0).expect("slot")],
        }
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: matches!(self.witness, Witness::View(_)),
            compute: false,
        }
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: self.bytes(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: false,
                },
            )],
            after_opaque: vec![self.declaration()],
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.prepare(surface);
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        if let Witness::View(request) = &self.witness {
            self.prepared
                .as_ref()
                .expect("probe")
                .2
                .as_ref()
                .expect("slot")
                .request(request)
                .expect("request");
        }
    }
    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group, _) = self.prepared.as_ref().expect("probe");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(self.size / 8, self.size / 8, 1);
    }
}

impl Probe {
    fn prepare(&mut self, surface: &SceneSurface<'_>) {
        let device = surface.device;
        let slot = surface
            .secondary_views
            .first()
            .cloned()
            .filter(|_| matches!(self.witness, Witness::View(_)));
        let input = self.declaration().input_layout(device);
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        if slot.is_some() {
            entries.extend(
                [
                    wgpu::TextureSampleType::Depth,
                    wgpu::TextureSampleType::Float { filterable: false },
                ]
                .into_iter()
                .enumerate()
                .map(|(i, sample_type)| wgpu::BindGroupLayoutEntry {
                    binding: u32::try_from(i + 1).expect("two textures"),
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }),
            );
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(surface.view_layout), Some(&input), Some(&layout)],
            immediate_size: 0,
        });
        let group = if slot.is_some() {
            "@group(2) @binding(1)"
        } else {
            "@group(1) @binding(0)"
        };
        let colour = if slot.is_some() {
            "@group(2) @binding(2)"
        } else {
            "@group(1) @binding(1)"
        };
        let source = format!(
            r"
{group} var depth: texture_depth_2d;
{colour} var colour: texture_2d<f32>;
struct Pixel {{ rgba: vec4<f32>, depth: vec4<f32> }}
@group(2) @binding(0) var<storage, read_write> pixels: array<Pixel>;
@compute @workgroup_size(8, 8) fn observe(@builtin(global_invocation_id) id: vec3<u32>) {{
    let at = vec2<i32>(id.xy);
    pixels[id.y * {}u + id.x] = Pixel(textureLoad(colour, at, 0), vec4(textureLoad(depth, at, 0)));
}}",
            self.size
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("observe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: self.bytes(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }];
        if let Some(slot) = &slot {
            entries.extend([
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(slot.depth()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(slot.colour()),
                },
            ]);
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        *self.output.lock().expect("output") = Some(buffer);
        self.prepared = Some((pipeline, group, slot));
    }
}
