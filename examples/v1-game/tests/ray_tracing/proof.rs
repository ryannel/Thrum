//! Shared Metal admission, cooked source fixture and same-submission observations.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use thrum_asset::{AssetStore, Profile};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedStats, Drawn, MeshPlacement, await_drawn, rows::CookedRows,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::runtime::{FrameConfig, FrameHost};
use thrum_render::{
    capabilities::{DeviceCapabilities, HardwareRequest},
    synthetic::{CullMode, DisplayOverlay, RayBuildProbe, RayStructureInstance},
};
use thrum_test_support::{Gpu, Restriction};
use v1_game::{GameHost, GameScene, GameSetup, GameWorld};

type Host = FrameHost<GameWorld, GameScene>;

mod casters;
mod contribution_shadows;
mod cost;
mod demand;
mod shadows;
pub(super) use casters::prove_casters;
pub(super) use contribution_shadows::prove_contributions;
pub(super) use cost::measure_valley;
pub(super) use demand::prove_demand;
pub(super) use shadows::{measure_shadows, prove_sun_out};

pub(super) const EXTENT: [u32; 2] = [320, 180];
pub(super) type Stats = Arc<Mutex<CookedStats>>;

pub(super) struct Metal {
    gpu: Gpu,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) caps: DeviceCapabilities,
}

impl Metal {
    pub(super) fn new(generation: u64) -> Option<Self> {
        let gpu = thrum_test_support::gpu("cooked ray geometry", Some(wgpu::Backend::Metal))?;
        Some(Self::open(gpu, generation))
    }

    pub(super) fn replacement(&self) -> Self {
        Self::open(self.gpu.fresh_adapter(), self.caps.generation() + 1)
    }

    fn open(gpu: Gpu, generation: u64) -> Self {
        let request = HardwareRequest::parse("ray-query").expect("required ray queries");
        let (device, queue, caps) = gpu
            .device_with_hardware(
                thrum_test_support::Profile::Portable,
                &Restriction::default(),
                generation,
                &request,
            )
            .expect("Metal generation with ray queries");
        let caps = caps.with_recipe_override("apple-tile").expect("Apple tile");
        Self {
            gpu,
            device,
            queue,
            caps,
        }
    }
}

pub(super) struct Fixture {
    cook: CookConfig,
    cooker: Cooker,
}

impl Fixture {
    pub(super) fn new(label: &str) -> Self {
        let root = scratch(label);
        let source = root.join("source");
        std::fs::create_dir_all(&source).expect("fixture directory");
        for name in ["placement", "entity", "stable", "third"] {
            let divisions = match name {
                "stable" => 4,
                "third" => 5,
                _ => 3,
            };
            plane(&source, name, false, divisions);
        }
        demo::write_checker_png(&source.join("colour.png"), 16, [70, 170, 30]).expect("texture");
        let cook = configuration(source, root.join("store"));
        let cooker = Cooker::open(cook.clone()).expect("fixture cooker");
        let mut fixture = Self { cook, cooker };
        fixture.publish();
        fixture
    }

    pub(super) fn generation(&self) -> u64 {
        generation(&self.cook.store)
    }

    pub(super) fn edit(&mut self, name: &str, new: bool, divisions: u32) -> u64 {
        plane(&self.cook.source, name, new, divisions);
        self.publish();
        self.generation()
    }

    pub(super) fn edit_both_with_oversized_row(&mut self) -> u64 {
        plane(&self.cook.source, "placement", false, 5);
        self.edit("entity", false, 96)
    }

    fn publish(&mut self) {
        assert_eq!(self.cooker.cook_all().expect("cook").failed, []);
    }

    pub(super) fn host(
        &self,
        metal: &Metal,
        observe: bool,
    ) -> (Host, Stats, Option<RayBuildProbe>) {
        let rows = CookedRows::from_store(
            &AssetStore::new(&self.cook.store),
            ["entity", "stable", "third"].map(str::to_owned).to_vec(),
            metal.device.limits().max_storage_buffer_binding_size,
        )
        .expect("cooked entity rows");
        drop(rows.updates().with_binding_limit(64 << 10));
        let caps = if observe {
            metal.caps.clone().with_ray_preparation_observation()
        } else {
            metal.caps.clone()
        };
        let mut game = pollster::block_on(
            GameSetup {
                field_meshes: true,
                camera_culling: CullMode::Occlusion,
                display_overlay: DisplayOverlay::TracedGeometry,
                ..GameSetup::default()
            }
            .prepare_with_rows(
                &metal.device,
                &metal.queue,
                &caps,
                frame_config(),
                Some(rows.clone()),
            ),
        )
        .expect("game host");
        let probe = observe.then(|| game.observe_ray_mesh_builds().expect("ray build observer"));
        game.set_display_overlay(DisplayOverlay::Off);
        let mut config = CookedConfig::new(
            self.cook.store.clone(),
            vec![
                MeshPlacement::new("placement", "colour").at([-0.55, -0.25, 0.28]),
                MeshPlacement::new("stable", "colour").at([12.0, 4.0, 0.28]),
                // Sharing a product must need only one build and retain both placements.
                MeshPlacement::new("stable", "colour").at([-12.0, 4.0, 0.28]),
            ],
        );
        config.rows = Some(rows);
        config.poll_interval = Duration::from_millis(1);
        let cooked = CookedAssets::new(config).expect("cooked contribution");
        let stats = cooked.stats();
        game.set_game_contribution(Box::new(cooked))
            .expect("attach");
        game.world_mut().pause().expect("freeze authority");
        (game, stats, probe)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(root) = self.cook.source.parent() {
            std::fs::remove_dir_all(root).ok();
        }
    }
}

fn plane(directory: &Path, name: &str, new: bool, divisions: u32) {
    demo::write_grid_gltf(directory, name, divisions, 0.0).expect("write grid");
    let source = directory.join(format!("{name}.gltf"));
    let mut gltf: Value =
        serde_json::from_slice(&std::fs::read(&source).expect("read grid")).expect("grid JSON");
    let s = std::f32::consts::FRAC_1_SQRT_2;
    // A front-facing patch in the XY plane. The translated old and new
    // ranges are disjoint. A ray at its patch centre stays away from the edge.
    gltf["nodes"][0]["rotation"] = json!([s, 0.0, 0.0, s]);
    gltf["nodes"][0]["translation"] = json!([if new { 0.46 } else { -0.46 }, 0.0, 0.0]);
    gltf["nodes"][0]["scale"] = json!([0.22, 0.22, 0.22]);
    std::fs::write(source, serde_json::to_vec(&gltf).expect("grid JSON")).expect("write node");
}

fn configuration(source: PathBuf, store: PathBuf) -> CookConfig {
    CookConfig {
        source,
        store,
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

pub(super) fn scratch(label: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tmp/ray-tracing-m2")
        .join(format!("{label}-{}", std::process::id()))
}

fn generation(store: &Path) -> u64 {
    AssetStore::new(store)
        .read_head()
        .expect("HEAD")
        .expect("published")
        .generation
}

fn frame_config() -> FrameConfig {
    FrameConfig {
        render_extent: EXTENT,
        surface: Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: EXTENT,
        }),
    }
}

/// Submit one frame of the committed endpoint without a readback and wait for it.
pub(super) fn render_frame<W, V>(host: &mut FrameHost<W, V>)
where
    W: thrum_render::runtime::FrameWorld,
    V: thrum_render::runtime::SceneSource<W::Page>,
{
    let submitted = host.render_once().expect("endpoint frame");
    host.device_capabilities()
        .site()
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("endpoint frame completes");
}

/// Draw until `generation` is drawn, then read that frame back. One frame comes first: a
/// published device reaches the contribution only in a frame, so the drawn generation read
/// before it may belong to the old device.
pub(super) fn settle<W, V>(game: &mut FrameHost<W, V>, stats: &Stats, generation: u64) -> Vec<u8>
where
    W: thrum_render::runtime::FrameWorld,
    V: thrum_render::runtime::SceneSource<W::Page>,
{
    render_frame(game);
    await_drawn(game, stats, Drawn::Generation(generation))
        .unwrap_or_else(|error| panic!("generation {generation} never drew: {error}"));
    let pixels = game.capture_pixels().expect("settling frame");
    let stats = stats.lock().expect("stats");
    assert!(stats.errors.is_empty(), "{:?}", stats.errors);
    assert_eq!(stats.drawn, Some(generation));
    pixels
}

pub(super) struct Ray {
    origin: [f32; 4],
    direction: [f32; 4],
    pub(super) identity: u32,
}

impl Ray {
    fn crossing(instance: &RayStructureInstance, x: f32) -> Self {
        let p = glam::Vec4::new(x, 0.037, 0.0, 1.0);
        let [a, b, c] =
            std::array::from_fn(|row| glam::Vec4::from_slice(&instance.transform[row * 4..]));
        let point = glam::Vec3::new(a.dot(p), b.dot(p), c.dot(p));
        let direction = glam::Vec3::new(a.z, b.z, c.z).normalize();
        let origin = point - direction * 0.0002;
        Self {
            origin: origin.extend(1.0).to_array(),
            direction: direction.extend(0.0004).to_array(),
            identity: instance.slot + 1,
        }
    }
}

pub(super) fn edit_rays(game: &Host) -> [Ray; 4] {
    let list = game.ray_structure_instances().expect("scene structure");
    let placement = list
        .iter()
        .find(|item| item.mesh_row == u32::MAX)
        .expect("first placement");
    let entity = list
        .iter()
        .filter(|item| item.mesh_row == 2)
        .min_by(|a, b| {
            let distance = |item: &RayStructureInstance| {
                glam::Vec2::new(item.transform[3], item.transform[7]).length_squared()
            };
            distance(a).total_cmp(&distance(b))
        })
        .expect("entity using the first cooked row");
    [
        Ray::crossing(placement, -0.46),
        Ray::crossing(placement, 0.46),
        Ray::crossing(entity, -0.46),
        Ray::crossing(entity, 0.46),
    ]
}

pub(super) fn cast(
    metal: &Metal,
    group: impl FnOnce(&wgpu::BindGroupLayout) -> Option<wgpu::BindGroup>,
    rays: &[Ray],
) -> Vec<u32> {
    let Metal { device, queue, .. } = metal;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("publication ray answers"),
        source: wgpu::ShaderSource::Wgsl(include_str!("probe.wgsl").into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("publication ray answers"),
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let structure = group(&pipeline.get_bind_group_layout(0)).expect("bind that frame's structure");
    let payload: Vec<_> = rays
        .iter()
        .flat_map(|ray| [ray.origin, ray.direction])
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect();
    let output_bytes = rays.len() as u64 * 4;
    let buffer = |size, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("publication probe records"),
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let inputs = buffer(
        payload.len() as u64,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    let answers = buffer(
        output_bytes,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let readback = buffer(
        output_bytes,
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    );
    queue.write_buffer(&inputs, 0, &payload);
    let records = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("publication probe records"),
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: inputs.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: answers.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &structure, &[]);
        pass.set_bind_group(1, &records, &[]);
        pass.dispatch_workgroups(
            u32::try_from(rays.len()).expect("ray count").div_ceil(64),
            1,
            1,
        );
    }
    encoder.copy_buffer_to_buffer(&answers, 0, &readback, 0, Some(output_bytes));
    queue.submit([encoder.finish()]);
    read(device, &readback)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&word| u32::from_le_bytes(word))
        .collect()
}

fn wait(device: &wgpu::Device) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("Metal completed");
}

fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).expect("mapping channel");
        });
    wait(device);
    rx.recv().expect("map callback").expect("readback mapped");
    let bytes = buffer
        .slice(..)
        .get_mapped_range()
        .expect("mapped range")
        .to_vec();
    buffer.unmap();
    bytes
}

#[derive(PartialEq)]
pub(super) struct Snapshot {
    pub(super) pixels: Vec<u8>,
    pub(super) hits: Vec<u32>,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("raster_digest", &digest(&self.pixels))
            .field("hits", &self.hits)
            .finish()
    }
}

impl Snapshot {
    pub(super) fn capture(game: &mut Host, metal: &Metal, rays: &[Ray]) -> Self {
        let pixels = game.capture_pixels().expect("raster readback");
        // The query is ordered after precisely that submission. No scene frame
        // records between the raster readback and this read of its structure.
        let hits = cast(metal, |layout| game.ray_structure_probe_group(layout), rays);
        Self { pixels, hits }
    }

    pub(super) fn record(&self, label: &str, stats: &Stats) -> Value {
        let stats = stats.lock().expect("stats");
        json!({
            "label": label, "recording": stats.recorded_frames, "submission": stats.submitted_frames,
            "publication": stats.drawn, "device_generation": stats.drawn_device_generation,
            "raster_digest": digest(&self.pixels), "ray_answers": self.hits,
            "submitted_cooked_mesh_builds": stats.submitted_ray_builds,
        })
    }
}

fn digest(bytes: &[u8]) -> String {
    let mut hash = DefaultHasher::new();
    bytes.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

pub(super) fn report(value: &Value) {
    writeln!(std::io::stdout().lock(), "ray proof {value}").expect("report");
}
