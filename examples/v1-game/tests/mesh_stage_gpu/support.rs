use std::{
    io::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thrum_asset::Profile;
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedDrawPath, CookedStats, MaterialReload, MeshPlacement,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    capabilities::{DeviceCapabilities, HardwareRequest},
    contribution::{
        ContributionFrame, ContributionPasses, SceneContribution, SceneContributions, SceneSurface,
        SecondaryView, SecondaryViewRequest, SecondaryViewSlot,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, FrameConfig, FrameHost, FrameSample, FrameWorld, SceneFrame, SceneSource,
        SurfaceOutput,
    },
    synthetic::{
        CaptureSource, FrameLighting, PhotometricEnvironment, SyntheticScene, SyntheticView,
    },
};

pub(super) const SIDE: u32 = 128;
pub(super) const CULL_OUTCOMES: usize = 8;
pub(super) const CULL_FAR: f32 = 8.0;
const PAGE: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 1,
};
const LIGHT: FrameLighting = FrameLighting::Photometric(PhotometricEnvironment {
    toward_sun: [0.0, 0.8, 0.6],
    sun_lux: 45_000.0,
    sun_colour: [1.0; 3],
    ambient_lux: [1800.0; 3],
    background_radiance: [0.1, 0.2, 0.35],
});

pub(super) struct World;
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

pub(super) struct Source(Arc<Mutex<SyntheticView>>, FrameLighting);
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
            view: *self.0.lock().expect("view"),
            lighting: self.1,
        })
    }
}

pub(super) fn camera(eye: [f32; 3], target: [f32; 3]) -> SyntheticView {
    camera_with_far(eye, target, 4000.0)
}

pub(super) fn camera_with_far(eye: [f32; 3], target: [f32; 3], far: f32) -> SyntheticView {
    SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], 0.92, 1.0, 0.1, far)
        .expect("camera")
        .with_sun_shadow_profile(2, 16.0)
        .expect("metre-scale fixture shadow reach")
        .with_temporal_inputs(true)
}

pub(super) struct Fixture {
    root: PathBuf,
    pub(super) placements: Vec<MeshPlacement>,
    pub(super) count_cull: bool,
    parity_depth: Option<f32>,
    pub(super) secondary_request: Option<Arc<Mutex<Option<SecondaryViewRequest>>>>,
    pub(super) shadows: bool,
}

impl Fixture {
    pub(super) fn new(case: &str, mixed: bool) -> Self {
        let scratch = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/mesh-shading-M1");
        let root = scratch.join(format!("{case}-{}", std::process::id()));
        let source = root.join("source");
        for directory in ["meshes", "textures"] {
            std::fs::create_dir_all(source.join(directory)).expect("source tree");
        }
        demo::write_grid_gltf(&source.join("meshes"), "small", 8, 0.16).expect("small");
        if case == "cull-equal" {
            let tangent = (0.92_f32 / 2.0).tan();
            let mut nodes = Vec::new();
            let front = [0.707_106_77, 0.0, 0.0, 0.707_106_77];
            let back = [-0.707_106_77, 0.0, 0.0, 0.707_106_77];
            let node = |translation: [f32; 3], scale: f32, rotation: [f32; 4]| serde_json::json!({"mesh":0, "translation":translation, "scale":([scale; 3]), "rotation":rotation});
            nodes.push(node([0.0, 0.0, -3.0], 0.55, front));
            nodes.push(node([-0.6, 0.6, -2.75], 0.22, back));
            for (x, y) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                nodes.push(node([x * 2.2, y * 2.2, -3.0], 0.18, front));
                nodes.push(node(
                    [x * 3.0 * tangent, y * 3.0 * tangent, -3.0],
                    0.22,
                    front,
                ));
            }
            // A small tile entirely in front of the near plane, and a tilted tile crossing it.
            nodes.push(node([0.0, 0.0, -0.04], 0.006, front));
            nodes.push(node(
                [0.0, 0.0, -0.10],
                0.04,
                [0.382_683_43, 0.0, 0.0, 0.923_879_5],
            ));
            // A front-facing tile wholly beyond the far plane, and a visible tilted tile
            // with vertices on each side of it.
            nodes.push(node([0.0, 0.0, -CULL_FAR - 1.5], 0.25, front));
            nodes.push(node(
                [2.2, 0.9, -CULL_FAR],
                0.9,
                [0.382_683_43, 0.0, 0.0, 0.923_879_5],
            ));
            patch_nodes(&source.join("meshes/small.gltf"), nodes);
        }
        let mut placements = vec![MeshPlacement::new("meshes/small", "textures/checker")];
        if mixed {
            let divisions = match case {
                "residency" => 160,
                "edit" => 8,
                _ => 128,
            };
            demo::write_grid_gltf(&source.join("meshes"), "large", divisions, 0.18).expect("large");
            placements[0] = placements[0].clone().at([-1.15, 0.0, 0.0]);
            placements
                .push(MeshPlacement::new("meshes/large", "textures/checker").at([1.15, 0.0, 0.0]));
        }
        demo::write_checker_png(&source.join("textures/checker.png"), 32, [200, 85, 40])
            .expect("checker");
        let mut cooker = Cooker::open(CookConfig {
            source,
            store: root.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker");
        let report = cooker.cook_all().expect("cook");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        Self {
            root,
            placements,
            count_cull: case == "cull-equal",
            parity_depth: match case {
                "parity" => Some(2.0),
                "parity-clear" => Some(0.0),
                _ => None,
            },
            secondary_request: None,
            shadows: true,
        }
    }

    pub(super) fn store(&self) -> PathBuf {
        self.root.join("store")
    }

    pub(super) fn publish_edit(&self, divisions: Option<u32>, height: f32) -> u64 {
        let source = self.root.join("source");
        let meshes = source.join("meshes");
        if let Some(divisions) = divisions {
            demo::write_grid_gltf(&meshes, "large", divisions, height).expect("edited grid");
        } else {
            std::fs::remove_file(meshes.join("large.gltf")).expect("remove placed product");
        }
        let mut cooker = Cooker::open(CookConfig {
            source,
            store: self.store(),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("edit cooker");
        let report = cooker.cook_all().expect("cook edit");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        report.published.expect("published edit").generation
    }

    pub(super) fn mesh(&self, name: &str) -> thrum_asset::MeshProduct {
        let store = thrum_asset::AssetStore::new(self.store());
        let head = store.read_head().expect("HEAD").expect("publication");
        let manifest = store.load_manifest(&head).expect("manifest");
        let hash = manifest
            .resolve(name, &[Profile::Portable])
            .expect("mesh product")
            .product;
        thrum_asset::MeshProduct::open(store.map(&hash).expect("product bytes"))
            .expect("cooked geometry")
    }
}

fn patch_nodes(path: &Path, nodes: Vec<serde_json::Value>) {
    let mut gltf: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("fixture glTF")).expect("parse glTF");
    gltf["scenes"][0]["nodes"] = serde_json::json!((0..nodes.len()).collect::<Vec<_>>());
    gltf["nodes"] = serde_json::Value::Array(nodes);
    std::fs::write(path, serde_json::to_vec(&gltf).expect("write nodes")).expect("fixture glTF");
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.root));
    }
}

pub(super) struct Gpu {
    gate: thrum_test_support::Gpu,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
}

impl Gpu {
    pub(super) fn new(
        gpu: &thrum_test_support::Gpu,
        stages: bool,
        recipe: &str,
        restrict: impl Fn(&mut wgpu::Limits),
    ) -> Self {
        Self::generation(gpu.fresh_adapter(), stages, recipe, 1, restrict)
    }

    pub(super) fn replacement(&self, restrict: impl Fn(&mut wgpu::Limits)) -> Self {
        Self::generation(self.gate.fresh_adapter(), true, "apple-tile", 2, restrict)
    }

    fn generation(
        gate: thrum_test_support::Gpu,
        stages: bool,
        recipe: &str,
        generation: u64,
        restrict: impl Fn(&mut wgpu::Limits),
    ) -> Self {
        let hardware =
            HardwareRequest::parse(if stages { "mesh-stages" } else { "" }).expect("hardware");
        let (device, queue, capabilities) = gate
            .device_with_hardware_adjusted(
                thrum_test_support::Profile::Portable,
                &thrum_test_support::Restriction::default(),
                generation,
                &hardware,
                |descriptor| restrict(&mut descriptor.required_limits),
            )
            .expect("explicit Metal launch");
        assert_eq!(device.adapter_info().backend, wgpu::Backend::Metal);
        let capabilities = capabilities
            .with_recipe_override(recipe)
            .expect("native recipe");
        Self {
            gate,
            device,
            queue,
            capabilities,
        }
    }

    pub(super) fn scene(
        &self,
        fixture: &Fixture,
        path: CookedDrawPath,
        cull: bool,
        fail_pipeline: bool,
    ) -> Scene<'_> {
        let initial = camera([0.0, 3.1, 4.6], [0.0, 0.0, 0.0]);
        let view = Arc::new(Mutex::new(initial));
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = initial;
        scene.sun_shadows = fixture.shadows;
        scene.instances[0].deactivate();
        let lighting = if let Some(tau) = fixture.parity_depth {
            // A small field of direct-Sun optical depth puts the cooked checker in light and
            // shade. The clear control has the same field and pipeline with zero attenuation.
            let depths: [f32; 64] = std::array::from_fn(|i| if i % 8 < 4 { tau } else { 0.0 });
            scene.sun_optical_depth = Some(
                SunOpticalDepth::new([8, 8], &depths, 10.0, [-4.0, 4.0, 4.0, 12.0], 0.1, 1)
                    .expect("fixture Sun optical depth"),
            );
            FrameLighting::Atmosphere(
                AtmosphereFrame::new([0.0, 0.8, 0.6], 100_000.0, [1.0; 3], 0.0, 1)
                    .expect("fixture Sun"),
            )
        } else {
            LIGHT
        };
        let mut host = pollster::block_on(FrameHost::new(
            &self.capabilities,
            &self.queue,
            FrameConfig {
                render_extent: [SIDE; 2],
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: [SIDE; 2],
                }),
            },
            World,
            Source(Arc::clone(&view), lighting),
            scene,
            1.0,
        ))
        .expect("prepared scene");
        let mut config = CookedConfig::new(fixture.store(), fixture.placements.clone());
        config.draw_path = path;
        config.mesh_stage_cull = cull;
        config.fail_mesh_pipeline = fail_pipeline;
        let rejections = Arc::new(Mutex::new(None));
        if fixture.count_cull {
            config.mesh_stage_rejections = Some(Arc::clone(&rejections));
        }
        config.upload_budget = thrum_asset_gpu::COPY_UNIT;
        let cooked = CookedAssets::new(config).expect("cooked placements");
        let stats = cooked.stats();
        let reload = cooked.material();
        let contribution: Box<dyn SceneContribution> =
            if let Some(request) = &fixture.secondary_request {
                Box::new(SceneContributions::new(vec![
                    Box::new(cooked),
                    Box::new(SelectionView {
                        request: Arc::clone(request),
                        slot: None,
                    }),
                ]))
            } else {
                Box::new(cooked)
            };
        host.set_scene_contribution(contribution)
            .expect("cooked contribution");
        let initially_prepared_mesh_pipelines = host
            .creation_report()
            .pipelines
            .iter()
            .filter(|row| row.label == "contribution")
            .map(|row| row.mesh)
            .sum();
        host.enable_submitted_capture_from(
            1,
            CaptureLimits {
                staging_slots: 2,
                cpu_buffers: 2,
                byte_allowance: 1 << 20,
            },
            CaptureSource::SceneLinear,
        )
        .expect("scene colour tap");
        let output = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("proof drawable"),
                size: wgpu::Extent3d {
                    width: SIDE,
                    height: SIDE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut scene = Scene {
            gpu: self,
            view,
            host,
            stats,
            reload,
            clock: Duration::ZERO,
            output,
            rejections,
            initially_prepared_mesh_pipelines,
        };
        let timeout = Instant::now() + Duration::from_secs(25);
        while scene.stats.lock().expect("stats").drawn.is_none() {
            scene.frame();
            let stats = scene.stats.lock().expect("stats");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            assert!(Instant::now() < timeout, "cooked residency timed out");
        }
        scene
    }
}

struct SelectionView {
    request: Arc<Mutex<Option<SecondaryViewRequest>>>,
    slot: Option<SecondaryView>,
}

impl SceneContribution for SelectionView {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("selection slot")]
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.slot = Some(surface.secondary_views[0].clone());
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        if let Some(request) = *self.request.lock().expect("selection request") {
            self.slot
                .as_ref()
                .expect("prepared slot")
                .request(&request)
                .expect("secondary view accepted");
        }
    }
}

pub(super) struct Scene<'a> {
    gpu: &'a Gpu,
    pub(super) host: FrameHost<World, Source>,
    pub(super) stats: Arc<Mutex<CookedStats>>,
    pub(super) reload: MaterialReload,
    pub(super) view: Arc<Mutex<SyntheticView>>,
    pub(super) clock: Duration,
    output: wgpu::TextureView,
    rejections: Arc<Mutex<Option<wgpu::Buffer>>>,
    pub(super) initially_prepared_mesh_pipelines: u64,
}

pub(super) struct Image {
    pub(super) colour: Vec<u8>,
    pub(super) temporal: Vec<u8>,
    pub(super) sample: FrameSample,
}

impl Image {
    pub(super) fn equal(&self, other: &Self) {
        for (name, actual, expected) in [
            ("colour", &self.colour, &other.colour),
            ("temporal inputs", &self.temporal, &other.temporal),
        ] {
            assert_eq!(actual.len(), expected.len(), "{name} extent");
            let first = actual.iter().zip(expected).position(|(a, b)| a != b);
            assert!(first.is_none(), "{name} first differing byte {first:?}");
        }
    }

    pub(super) fn save(&self, case: &str, label: &str) {
        let Some(root) = std::env::var_os("THRUM_MESH_STAGE_PROOF_DIR") else {
            return;
        };
        for (name, bytes) in [("colour", &self.colour), ("temporal", &self.temporal)] {
            let path = Path::new(&root).join(format!("{case}-{label}-{name}.raw"));
            std::fs::write(path, bytes).expect("submitted frame bytes");
        }
    }
}

impl Scene<'_> {
    pub(super) fn report(&self, label: &str) {
        let s = self.stats.lock().expect("stats");
        writeln!(std::io::stdout(),
            "{label}: recipe {:?} requested {:?} resolved {:?} fallback {:?}; mesh {} indirect {} direct {} compute {}; exclusions {:?}; mesh buffers {} including stage {}, storage vertex streams {}; upload {} bytes over {} frames, maximum frame copy {}",
            s.recipe, s.requested_draw_path, s.draw_path, s.draw_fallback, s.mesh_stage_draws,
            s.indexed_indirect_draws, s.indexed_direct_draws, s.meshlet_compute_cull,
            s.mesh_stage_exclusions, s.mesh_buffer_bytes, s.mesh_stage_bytes, s.mesh_stage_vertex_streams,
            s.swaps[0].uploaded_bytes, s.swaps[0].upload_frames, s.max_frame_copy_bytes,
        ).expect("Metal proof report");
    }

    pub(super) fn camera(&mut self, view: &SyntheticView, cut: bool) {
        *self.view.lock().expect("camera") = *view;
        if cut {
            self.host.invalidate_history();
        }
    }

    pub(super) fn discard(&mut self) {
        self.clock += Duration::from_millis(17);
        let encoded = self
            .host
            .advance(self.clock)
            .expect("advance")
            .encode_scene()
            .expect("record");
        assert!(encoded.skip().dropped_view);
    }

    pub(super) fn clear_rejections(&self) {
        let source = self.rejections.lock().expect("witness");
        self.gpu.queue.write_buffer(
            source.as_ref().expect("cull witness"),
            0,
            &[0; size_of::<[u32; CULL_OUTCOMES]>()],
        );
    }

    pub(super) fn rejections(&self) -> [u32; CULL_OUTCOMES] {
        let source = self.rejections.lock().expect("witness");
        let source = source.as_ref().expect("cull witness");
        assert_eq!(source.size(), size_of::<[u32; CULL_OUTCOMES]>() as u64);
        let buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("object-stage cull count readback"),
            size: source.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, source.size());
        let bytes = read_buffer(self.gpu, encoder, &buffer);
        std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[i * 4..(i + 1) * 4].try_into().expect("counter word"))
        })
    }

    pub(super) fn frame(&mut self) -> Image {
        self.clock += Duration::from_millis(17);
        let output = SurfaceOutput {
            view: &self.output,
            extent: [SIDE; 2],
        };
        assert!(
            self.host
                .drive(self.clock, Some(&output))
                .expect("frame")
                .submitted
        );
        let temporal = read(
            self.gpu,
            self.host
                .submitted_temporal_witness()
                .expect("frame contract witness"),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut sample = None;
        let image = loop {
            if let Some(image) = self.host.try_submitted_image().expect("colour tap") {
                break image;
            }
            sample = self.host.poll_completed().expect("poll frame").or(sample);
            assert!(
                Instant::now() < deadline,
                "timed out waiting for submitted colour, omissions {}",
                self.host.submitted_capture_omissions()
            );
            std::thread::yield_now();
        };
        let colour = image.rgba.clone();
        self.host.recycle_submitted_image(image);
        let sample = loop {
            if let Some(sample) = sample {
                break sample;
            }
            sample = self.host.poll_completed().expect("poll frame");
            assert!(Instant::now() < deadline, "timed out retiring proof frame");
        };
        assert_eq!(sample.observation, self.clock);
        Image {
            colour,
            temporal,
            sample,
        }
    }
}

impl<'a> Scene<'a> {
    pub(super) fn replace_device(&mut self, gpu: &'a Gpu) {
        self.host.disable_submitted_capture();
        let inputs = self.host.device_prepare_inputs().expect("retained inputs");
        let mut replacement = pollster::block_on(inputs.prepare(
            &gpu.capabilities,
            &gpu.queue,
            Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: [SIDE; 2],
            }),
        ))
        .expect("prepared replacement");
        self.gpu.device.destroy();
        self.host
            .publish_device(&mut replacement)
            .expect("publish replacement");
        self.host
            .enable_submitted_capture_from(
                2,
                CaptureLimits {
                    staging_slots: 2,
                    cpu_buffers: 2,
                    byte_allowance: 1 << 20,
                },
                CaptureSource::SceneLinear,
            )
            .expect("replacement capture");
        self.gpu = gpu;
        self.output = gpu
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("replacement proof drawable"),
                size: wgpu::Extent3d {
                    width: SIDE,
                    height: SIDE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
    }
}

fn read(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let stride = SIDE * 8;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mesh proof temporal input readback"),
        size: u64::from(stride * SIDE),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(SIDE),
            },
        },
        texture.size(),
    );
    read_buffer(gpu, encoder, &buffer)
}

pub(super) fn read_buffer(
    gpu: &Gpu,
    encoder: wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
) -> Vec<u8> {
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("map");
    });
    let submission = gpu.queue.submit([encoder.finish()]);
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(15)),
        })
        .expect("Metal completion");
    receive
        .recv_timeout(Duration::from_secs(5))
        .expect("read callback")
        .expect("read map");
    let bytes = buffer.get_mapped_range(..).expect("proof bytes").to_vec();
    buffer.unmap();
    bytes
}
