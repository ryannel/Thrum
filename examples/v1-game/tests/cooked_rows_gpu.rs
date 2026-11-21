//! Cooked entity rows exercised through the V1 game's ordinary frame host.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thrum_asset::{AssetStore, MeshProduct, Profile};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedStats, Drawn, MeshPlacement, await_drawn, rows::CookedRows,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::runtime::{FrameConfig, FrameHost};
use thrum_render::synthetic::mesh_rows::{MeshRowError, MeshRowGeometry};
use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{GameHost, GameScene, GameSetup, GameWorld};

const EXTENT: [u32; 2] = [320, 180];
/// Under `cargo test` every case of this file shares one process, and a dozen game hosts at once
/// exhaust the device, so the cases this file splits by mode take turns here. `./dev check` runs
/// each test in a process of its own, where nothing else takes this lock.
static CASES: Mutex<()> = Mutex::new(());
type Stats = Arc<Mutex<CookedStats>>;
type Host = FrameHost<GameWorld, GameScene>;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(label: &str) -> Self {
        Self(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tmp/content-loop-m2")
                .join(format!("{}-{label}", std::process::id())),
        )
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn config(root: &std::path::Path) -> CookConfig {
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("source");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 8, 0.25).expect("mesh");
    for name in ["second", "third"] {
        demo::write_grid_gltf(&source.join("meshes"), name, 4, 0.1).expect("unchanged mesh");
    }
    demo::write_checker_png(&source.join("colour.png"), 16, [170, 70, 30]).expect("texture");
    CookConfig {
        source,
        store: root.join("store"),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

fn rows(cook: &CookConfig, device: &wgpu::Device) -> CookedRows {
    CookedRows::from_store(
        &AssetStore::new(&cook.store),
        ["meshes/terrain", "meshes/second", "meshes/third"]
            .map(str::to_owned)
            .to_vec(),
        device.limits().max_storage_buffer_binding_size,
    )
    .expect("row conversion")
}

async fn host(
    cook: &CookConfig,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    mode: CullMode,
    coupled: bool,
    binding: Option<u64>,
) -> (Host, Stats) {
    let rows = rows(cook, device);
    if let Some(binding) = binding {
        drop(rows.updates().with_binding_limit(binding));
    }
    let mut host = GameSetup {
        field_meshes: true,
        camera_culling: mode,
        ..GameSetup::default()
    }
    .prepare_with_rows(
        device,
        queue,
        caps,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(thrum_render::present::SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: EXTENT,
            }),
        },
        Some(rows.clone()),
    )
    .await
    .expect("game host");
    let mut config = CookedConfig::new(
        cook.store.clone(),
        if coupled {
            vec![MeshPlacement::new("meshes/terrain", "colour")]
        } else {
            vec![]
        },
    );
    config.poll_interval = Duration::from_millis(1);
    config.rows = Some(rows);
    let assets = CookedAssets::new(config).expect("residency");
    let stats = assets.stats();
    host.set_game_contribution(Box::new(assets))
        .expect("attach");
    host.world_mut().pause().expect("pause at same tick");
    (host, stats)
}

fn settle(host: &mut Host, stats: &Stats, generation: u64) -> Vec<u8> {
    // One frame first: a published device or surface reaches the contribution only in a frame,
    // which then restages, so the drawn generation read before it may be stale.
    render_frame(host);
    await_drawn(host, stats, Drawn::Generation(generation))
        .unwrap_or_else(|error| panic!("generation {generation}: {error}"));
    let image = host.capture_pixels().expect("game capture");
    let stats = stats.lock().expect("stats");
    assert!(stats.errors.is_empty(), "{:?}", stats.errors);
    assert_eq!(stats.drawn, Some(generation));
    image
}

/// Submit one frame of the committed endpoint without a readback and wait for it,
/// so row writes and staged copies advance as a capture would advance them.
fn render_frame(host: &mut Host) {
    let submitted = host.render_once().expect("endpoint frame");
    host.device_capabilities()
        .site()
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("endpoint frame completes");
}

fn generation(cook: &CookConfig) -> u64 {
    AssetStore::new(&cook.store)
        .read_head()
        .expect("HEAD")
        .expect("published")
        .generation
}

fn edit(cook: &CookConfig, cooker: &mut Cooker, size: u32, amplitude: f32) -> u64 {
    demo::write_grid_gltf(&cook.source.join("meshes"), "terrain", size, amplitude).expect("edit");
    assert_eq!(cooker.cook_all().expect("recook").failed, []);
    generation(cook)
}

fn staged(stats: &Stats, generation: u64) {
    let deadline = Instant::now() + Duration::from_secs(90);
    while stats.lock().expect("stats").staged != Some(generation) {
        assert!(
            Instant::now() < deadline,
            "loader never prepared generation {generation}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn only_old_or_new(live: &mut Host, stats: &Stats, generation: u64, old: &[u8], new: &[u8]) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let image = live.capture_pixels().expect("publication frame");
        assert!(
            image == old || image == new,
            "a submitted frame mixed or drew uncommitted geometry"
        );
        let stats = stats.lock().expect("stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(generation) {
            assert_eq!(image, new);
            break;
        }
        assert!(Instant::now() < deadline, "publication never switched");
    }
}

#[test]
fn a_recreated_store_with_the_same_generation_replaces_its_rows() {
    let Some(gpu) = thrum_test_support::gpu("row store epoch", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new("store-epoch");
        let cook = config(root.path());
        Cooker::open(cook.clone())
            .expect("cooker")
            .cook_all()
            .expect("cook");
        let first = AssetStore::new(&cook.store)
            .read_head()
            .expect("HEAD")
            .expect("first");
        let (mut live, stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let old = settle(&mut live, &stats, first.generation);
        demo::write_grid_gltf(&cook.source.join("meshes"), "terrain", 64, 1.2).expect("edit");
        // Resident GPU resources and immutable mappings survive unlinking the old store.
        std::fs::remove_dir_all(&cook.store).expect("replace owned fixture store");
        Cooker::open(cook.clone())
            .expect("new cooker")
            .cook_all()
            .expect("new store");
        let next = AssetStore::new(&cook.store)
            .read_head()
            .expect("HEAD")
            .expect("new epoch");
        assert_eq!(first.generation, next.generation);
        assert_ne!(first.epoch, next.epoch);
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let new = settle(&mut fresh, &fresh_stats, next.generation);
        assert_ne!(old, new);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let image = live.capture_pixels().expect("epoch frame");
            assert!(
                image == old || image == new,
                "store epoch produced a mixed frame"
            );
            if stats.lock().expect("stats").swaps.len() == 2 {
                assert_eq!(image, new);
                break;
            }
            assert!(Instant::now() < deadline, "new store epoch never switched");
        }
    });
}

#[test]
fn repeated_products_keep_each_initial_row_identity_without_overlap_space() {
    let Some(gpu) = thrum_test_support::gpu("repeated row products", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new("repeated-products");
        let cook = config(root.path());
        Cooker::open(cook.clone())
            .expect("cooker")
            .cook_all()
            .expect("cook");
        // These rows fit, but there is no room to redundantly copy either terrain row.
        let rows = CookedRows::from_store(
            &AssetStore::new(&cook.store),
            ["meshes/terrain", "meshes/terrain", "meshes/third"]
                .map(str::to_owned)
                .to_vec(),
            12 << 10,
        )
        .expect("compact rows");
        let updates = rows.updates();
        let mut live = GameSetup {
            field_meshes: true,
            camera_culling: CullMode::Occlusion,
            ..GameSetup::default()
        }
        .prepare_with_rows(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: EXTENT,
                surface: None,
            },
            Some(rows.clone()),
        )
        .await
        .expect("compact scene");
        let mut config = CookedConfig::new(cook.store.clone(), Vec::new());
        config.rows = Some(rows);
        let assets = CookedAssets::new(config).expect("residency");
        let stats = assets.stats();
        live.set_game_contribution(Box::new(assets))
            .expect("attach");
        live.world_mut().pause().expect("pause");
        let first = generation(&cook);
        let reference = settle(&mut live, &stats, first);
        assert_eq!(
            updates.report().submitted_bytes,
            0,
            "initial rows were redundantly uploaded"
        );
        let replacement_gpu = gpu.fresh_adapter();
        let (replacement, replacement_queue) = replacement_gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let replacement_caps =
            DeviceCapabilities::capture(&replacement_gpu.adapter, &replacement, 2);
        let mut candidate = live
            .device_prepare_inputs()
            .expect("retained rows")
            .prepare(&replacement_caps, &replacement_queue, None)
            .await
            .expect("new device");
        live.publish_device(&mut candidate).expect("publish");
        assert_eq!(settle(&mut live, &stats, first), reference);
        assert_eq!(
            stats.lock().expect("stats").drawn_device_generation,
            Some(2)
        );
        assert_eq!(
            updates.report().submitted_bytes,
            0,
            "reconstruction lost a repeated product's row identity"
        );
    });
}

#[test]
fn supersession_discarded_protected_switch_and_retired_space_reuse() {
    let Some(gpu) =
        thrum_test_support::gpu("row supersession and reuse", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new("supersession");
        let cook = config(root.path());
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        cooker.cook_all().expect("cook");
        let (mut live, stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let first = generation(&cook);
        let mut old = settle(&mut live, &stats, first);
        let updates = live.cooked_rows().expect("rows").updates();
        // Every edit here is half the grid it was (512 and 256 cells became 256 and 128): the
        // superseded upload still spans five bounded slices, the protected and reused rows two,
        // and each frame draws a quarter of the triangles, which the entities repeat.
        let uploading = edit(&cook, &mut cooker, 256, 1.8);
        staged(&stats, uploading);
        // First capture takes the mailbox; the second records only the first bounded slice.
        for _ in 0..2 {
            assert_eq!(live.capture_pixels().expect("partial write"), old);
        }
        assert_eq!(
            updates.status(uploading),
            Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Writing)
        );
        assert!(updates.report().last_write_bytes > 0);
        let superseding = edit(&cook, &mut cooker, 5, 0.04);
        staged(&stats, superseding);
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let new = settle(&mut fresh, &fresh_stats, superseding);
        only_old_or_new(&mut live, &stats, superseding, &old, &new);
        assert!(
            !stats
                .lock()
                .expect("stats")
                .swaps
                .iter()
                .any(|s| s.generation == uploading)
        );
        old = new;
        let protected = edit(&cook, &mut cooker, 128, 0.8);
        staged(&stats, protected);
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let middle = settle(&mut fresh, &fresh_stats, protected);
        let deadline = Instant::now() + Duration::from_secs(90);
        while updates.status(protected)
            != Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Ready)
        {
            assert_eq!(live.capture_pixels().expect("readying frame"), old);
            assert!(Instant::now() < deadline, "row never became ready");
        }
        live.advance(Duration::from_millis(16))
            .expect("advance")
            .encode_scene()
            .expect("protected switch")
            .skip();
        assert_eq!(
            updates.status(protected),
            Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Ready)
        );
        let behind = edit(&cook, &mut cooker, 4, 0.16);
        staged(&stats, behind);
        assert_eq!(live.capture_pixels().expect("replayed switch"), middle);
        assert_eq!(stats.lock().expect("stats").drawn, Some(protected));
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let newest = settle(&mut fresh, &fresh_stats, behind);
        only_old_or_new(&mut live, &stats, behind, &middle, &newest);
        old = newest;
        let high_water = updates.report().high_water;
        for (size, amplitude) in [(128, 0.9), (4, 0.1), (128, 0.7), (4, 0.05)] {
            let next = edit(&cook, &mut cooker, size, amplitude);
            let prepared = rows(&cook, &device).selection(Vec::new());
            let mesh = &prepared.meshes[0];
            let expected_bytes = (mesh.vertices.len() * 48 + mesh.indices.len() * 4) as u64;
            let (mut fresh, fresh_stats) = host(
                &cook,
                &device,
                &queue,
                &caps,
                CullMode::Occlusion,
                true,
                None,
            )
            .await;
            let reference = settle(&mut fresh, &fresh_stats, next);
            let before = updates.report().submitted_bytes;
            let pipelines = live.pipeline_creation_count();
            only_old_or_new(&mut live, &stats, next, &old, &reference);
            let report = updates.report();
            assert_eq!(
                report.submitted_bytes - before,
                expected_bytes,
                "an edit wrote unchanged rows"
            );
            assert_eq!(
                report.high_water, high_water,
                "arena grew instead of reusing retired space"
            );
            assert!(report.max_write_bytes <= thrum_render::synthetic::mesh_rows::ROW_WRITE_BYTES);
            assert_eq!(live.pipeline_creation_count(), pipelines);
            old = reference;
        }
        let report = updates.report();
        let range = |times: &[Duration]| {
            let mut times = times.to_vec();
            times.sort_unstable();
            (
                times.first().copied(),
                times.get(times.len() / 2).copied(),
                times.last().copied(),
            )
        };
        eprintln!(
            "row-write bound={} reserved={} high_water={:?} uploaded={} write_times(min,median,max)={:?} steady_times={:?}; contended Metal run",
            thrum_render::synthetic::mesh_rows::ROW_WRITE_BYTES,
            report.reserved_bytes,
            report.high_water,
            report.submitted_bytes,
            range(&report.write_times),
            range(&report.steady_times)
        );
    });
}

#[test]
fn refusal_keeps_the_whole_publication_then_recovers_resizes_and_replaces_device() {
    let Some(gpu) =
        thrum_test_support::gpu("row refusal and lifecycle", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new("refusal");
        let cook = config(root.path());
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        cooker.cook_all().expect("cook");
        let first = generation(&cook);
        let (mut live, stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            Some(64 << 10),
        )
        .await;
        let old = settle(&mut live, &stats, first);
        demo::write_checker_png(&cook.source.join("colour.png"), 16, [30, 170, 70])
            .expect("texture edit");
        let refused = edit(&cook, &mut cooker, 256, 1.4);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            assert_eq!(
                live.capture_pixels().expect("refused frame"),
                old,
                "a refused publication changed pixels"
            );
            let stats = stats.lock().expect("stats");
            if !stats.errors.is_empty() {
                assert!(stats.errors.last().expect("reason").contains("Capacity"));
                assert_eq!(stats.drawn, Some(first));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "arena refusal was never recorded"
            );
        }
        for _ in 0..3 {
            assert_eq!(live.capture_pixels().expect("held frame"), old);
        }
        assert_eq!(
            stats.lock().expect("stats").failures,
            1,
            "refusal recorded more than once"
        );
        let good = edit(&cook, &mut cooker, 6, 0.5);
        assert!(good > refused);
        stats.lock().expect("stats").errors.clear();
        let new = settle(&mut live, &stats, good);
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        assert_eq!(new, settle(&mut fresh, &fresh_stats, good));
        assert_ne!(new, old);
        std::fs::write(cook.source.join("meshes/terrain.gltf"), b"broken source")
            .expect("bad save");
        assert_ne!(cooker.cook_all().expect("bad cook").failed, []);
        assert_eq!(live.capture_pixels().expect("bad save frame"), new);
        let latest = edit(&cook, &mut cooker, 5, 0.2);
        let newest = settle(&mut live, &stats, latest);
        let (mut fresh, fresh_stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        assert_eq!(newest, settle(&mut fresh, &fresh_stats, latest));
        live.resize_surface([480, 270]).expect("resize");
        assert_eq!(live.capture_pixels().expect("resized scene"), newest);
        let replacement_gpu = gpu.fresh_adapter();
        let (replacement, replacement_queue) = replacement_gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let replacement_caps =
            DeviceCapabilities::capture(&replacement_gpu.adapter, &replacement, 2);
        let mut candidate = live
            .device_prepare_inputs()
            .expect("retained scene")
            .prepare(
                &replacement_caps,
                &replacement_queue,
                Some(thrum_render::present::SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: [480, 270],
                }),
            )
            .await
            .expect("replacement resources");
        live.publish_device(&mut candidate)
            .expect("publish replacement");
        assert_eq!(settle(&mut live, &stats, latest), newest);
        assert_eq!(
            stats.lock().expect("stats").drawn_device_generation,
            Some(2)
        );
    });
}

#[test]
fn conversion_preserves_product_geometry_and_named_admission() {
    let root = Scratch::new("conversion");
    let cook = config(root.path());
    demo::write_mirrored_grid_gltf(&cook.source.join("meshes"), "terrain", 8, 0.25)
        .expect("varied authored normals");
    Cooker::open(cook.clone())
        .expect("cooker")
        .cook_all()
        .expect("cook");
    let store = AssetStore::new(&cook.store);
    let manifest = store
        .load_manifest(&store.read_head().expect("HEAD").expect("published"))
        .expect("manifest");
    let hash = manifest
        .resolve("meshes/terrain", &[Profile::Portable])
        .expect("mesh")
        .product;
    let product = MeshProduct::open(store.map(&hash).expect("mapping")).expect("product");
    let geometry = thrum_asset_gpu::rows::mesh_row(&product, 128 << 20).expect("convert");
    assert_eq!(geometry.mesh().indices, product.indices());
    let mut directions = std::collections::BTreeSet::new();
    for ((vertex, position), attributes) in geometry
        .mesh()
        .vertices
        .iter()
        .zip(product.positions())
        .zip(product.attributes())
    {
        assert_eq!(&vertex.position, position);
        assert_eq!(vertex.uv, attributes.uv);
        directions.insert(attributes.normal_oct);
        // Independent double-precision octahedral unfolding oracle. Unlike the
        // production decoder's correction term, unfold the lower diamond by
        // exchanging its absolute coordinates, then normalize in f64.
        let [x, y] = attributes
            .normal_oct
            .map(|v| (f64::from(v) / 32767.0).max(-1.0));
        let z = 1.0 - x.abs() - y.abs();
        let (x, y) = if z < 0.0 {
            ((1.0 - y.abs()).copysign(x), (1.0 - x.abs()).copysign(y))
        } else {
            (x, y)
        };
        let length = (x * x + y * y + z * z).sqrt();
        let expected = [x / length, y / length, z / length];
        let error = vertex
            .normal
            .iter()
            .zip(expected)
            .map(|(actual, expected)| (f64::from(*actual) - expected).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            error < 1e-6,
            "decoded normal differs: {:?} / {expected:?}",
            vertex.normal
        );
    }
    assert!(
        directions.len() > 8,
        "fixture must reject a constant unit normal"
    );
    let good = geometry.mesh().clone();
    let mut bad = good.clone();
    bad.vertices[0].position[0] = f32::NAN;
    assert!(matches!(
        MeshRowGeometry::new(bad, 128 << 20),
        Err(MeshRowError::NonFinite)
    ));
    let mut bad = good.clone();
    bad.indices[0] = u32::MAX;
    assert!(matches!(
        MeshRowGeometry::new(bad, 128 << 20),
        Err(MeshRowError::IndexOutOfRange { .. })
    ));
    let mut bad = good.clone();
    let index = bad.indices[0] as usize;
    bad.vertices[index].position[0] = 1e31;
    assert!(matches!(
        MeshRowGeometry::new(bad, 128 << 20),
        Err(MeshRowError::CoordinateLimit)
    ));
    assert!(matches!(
        MeshRowGeometry::new(good, 4),
        Err(MeshRowError::Capacity)
    ));
}

// Each culling mode, with and without a texture placement on the edited mesh, is one test, so
// the runner spreads them over the cores.
#[test]
fn edited_rows_equal_fresh_game_direct_without_placement() {
    edited_rows_equal_fresh_game(CullMode::Direct, false);
}

#[test]
fn edited_rows_equal_fresh_game_direct_with_placement() {
    edited_rows_equal_fresh_game(CullMode::Direct, true);
}

#[test]
fn edited_rows_equal_fresh_game_frustum_without_placement() {
    edited_rows_equal_fresh_game(CullMode::Frustum, false);
}

#[test]
fn edited_rows_equal_fresh_game_frustum_with_placement() {
    edited_rows_equal_fresh_game(CullMode::Frustum, true);
}

#[test]
fn edited_rows_equal_fresh_game_occlusion_without_placement() {
    edited_rows_equal_fresh_game(CullMode::Occlusion, false);
}

#[test]
fn edited_rows_equal_fresh_game_occlusion_with_placement() {
    edited_rows_equal_fresh_game(CullMode::Occlusion, true);
}

fn edited_rows_equal_fresh_game(mode: CullMode, coupled: bool) {
    let _turn = CASES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(gate) = thrum_test_support::gpu("cooked rows", Some(wgpu::Backend::Metal)) else {
        return;
    };
    pollster::block_on(async {
        let (device, queue) = gate.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let caps = DeviceCapabilities::capture(&gate.adapter, &device, 1);
        let root = Scratch::new(&format!("{mode:?}-{coupled}"));
        let cook = config(root.path());
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        cooker.cook_all().expect("first cook");
        let first = AssetStore::new(&cook.store)
            .read_head()
            .expect("HEAD")
            .expect("first")
            .generation;
        let (mut live, stats) = host(&cook, &device, &queue, &caps, mode, coupled, None).await;
        let mut old = settle(&mut live, &stats, first);
        let updates = live.cooked_rows().expect("rows").updates();
        let mut clock = Duration::ZERO;
        for (size, amplitude) in [(256, 1.4), (4, 0.07)] {
            demo::write_grid_gltf(&cook.source.join("meshes"), "terrain", size, amplitude)
                .expect("edit");
            cooker.cook_all().expect("recook");
            let generation = AssetStore::new(&cook.store)
                .read_head()
                .expect("HEAD")
                .expect("edit")
                .generation;
            let (mut fresh, fresh_stats) =
                host(&cook, &device, &queue, &caps, mode, coupled, None).await;
            let new = settle(&mut fresh, &fresh_stats, generation);
            let pipelines = live.pipeline_creation_count();
            assert_ne!(old, new, "fixture must visibly change {mode:?}");
            let deadline = Instant::now() + Duration::from_secs(90);
            let mut discarded_switch = false;
            let mut row_only_frames = 0;
            loop {
                let ready = updates.status(generation)
                    == Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Ready);
                if ready && !discarded_switch {
                    clock += Duration::from_millis(16);
                    let before = stats.lock().expect("stats").drawn;
                    live.advance(clock)
                        .expect("advance")
                        .encode_scene()
                        .expect("switch recording")
                        .skip();
                    assert_eq!(
                        stats.lock().expect("stats").drawn,
                        before,
                        "discard committed a switch"
                    );
                    assert_eq!(
                        updates.status(generation),
                        Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Ready)
                    );
                    discarded_switch = true;
                }
                let image = live.capture_pixels().expect("edit frame");
                assert!(
                    updates.report().max_write_bytes
                        <= thrum_render::synthetic::mesh_rows::ROW_WRITE_BYTES
                );
                assert!(
                    image == old || image == new,
                    "mixed or non-fresh frame {mode:?}, coupled {coupled}, size {size}"
                );
                let observed = stats.lock().expect("stats");
                if observed.last_frame_copy_bytes == 0
                    && updates.report().last_write_bytes != 0
                    && updates.status(generation)
                        == Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Writing)
                {
                    row_only_frames += 1;
                }
                assert!(observed.errors.is_empty(), "{:?}", observed.errors);
                if observed.drawn == Some(generation) {
                    assert_eq!(image, new);
                    break;
                }
                assert!(Instant::now() < deadline, "edit timed out");
                drop(observed);
                std::thread::yield_now();
            }
            assert!(discarded_switch, "the switch frame must be discarded once");
            if coupled && size == 256 {
                assert!(
                    row_only_frames >= 2,
                    "the row must still be writing for several frames after the placement's last copy"
                );
            }
            assert_eq!(
                live.pipeline_creation_count(),
                pipelines,
                "edit created a pipeline"
            );
            old = new;
        }
    });
}

fn environment(revision: u8) -> thrum_content_runtime::EnvironmentProduct {
    let (x, y) = if revision.is_multiple_of(2) {
        (0.6, 0.8)
    } else {
        (0.0, 1.0)
    };
    let values = thrum_content_runtime::EnvironmentValues::from_numbers([
        x, y, 0.0, 100_000.0, 1.0, 1.0, 1.0, 0.00465, 100.0, 100.0, 100.0, 1.0, 1.0, 1.0,
    ])
    .expect("environment");
    thrum_content_runtime::EnvironmentProduct::new(
        thrum_content_runtime::EnvironmentProductRevision::from_bytes([revision; 32]),
        values,
    )
}

async fn environment_reference(
    cook: &CookConfig,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    mode: CullMode,
    product: thrum_content_runtime::EnvironmentProduct,
) -> Vec<u8> {
    let (mut fresh, stats) = host(cook, device, queue, caps, mode, true, None).await;
    settle(&mut fresh, &stats, generation(cook));
    let mut candidate = fresh
        .environment_prepare_inputs()
        .prepare(product)
        .await
        .expect("fresh environment");
    assert!(
        fresh
            .publish_environment(&mut candidate)
            .expect("fresh publish")
    );
    fresh.capture_pixels().expect("fresh environment frame")
}

// A visual rebuild preserves a writing, ready, discarded or newly committed row: each culling
// mode and each stage is one test, so the runner spreads them over the cores.
#[test]
fn visual_rebuild_direct_partial_write() {
    visual_rebuild_preserves_rows(CullMode::Direct, 0);
}

#[test]
fn visual_rebuild_direct_ready_row() {
    visual_rebuild_preserves_rows(CullMode::Direct, 1);
}

#[test]
fn visual_rebuild_direct_discarded_switch() {
    visual_rebuild_preserves_rows(CullMode::Direct, 2);
}

#[test]
fn visual_rebuild_direct_candidate_prepared_before_the_commit() {
    visual_rebuild_preserves_rows(CullMode::Direct, 3);
}

#[test]
fn visual_rebuild_frustum_partial_write() {
    visual_rebuild_preserves_rows(CullMode::Frustum, 0);
}

#[test]
fn visual_rebuild_frustum_ready_row() {
    visual_rebuild_preserves_rows(CullMode::Frustum, 1);
}

#[test]
fn visual_rebuild_frustum_discarded_switch() {
    visual_rebuild_preserves_rows(CullMode::Frustum, 2);
}

#[test]
fn visual_rebuild_frustum_candidate_prepared_before_the_commit() {
    visual_rebuild_preserves_rows(CullMode::Frustum, 3);
}

#[test]
fn visual_rebuild_occlusion_partial_write() {
    visual_rebuild_preserves_rows(CullMode::Occlusion, 0);
}

#[test]
fn visual_rebuild_occlusion_ready_row() {
    visual_rebuild_preserves_rows(CullMode::Occlusion, 1);
}

#[test]
fn visual_rebuild_occlusion_discarded_switch() {
    visual_rebuild_preserves_rows(CullMode::Occlusion, 2);
}

#[test]
fn visual_rebuild_occlusion_candidate_prepared_before_the_commit() {
    visual_rebuild_preserves_rows(CullMode::Occlusion, 3);
}

/// `phase` 0: partial write; 1: ready; 2: discarded switch; 3: a candidate prepared before the
/// edit, then published after the edit committed.
fn visual_rebuild_preserves_rows(mode: CullMode, phase: u8) {
    let _turn = CASES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(gpu) = thrum_test_support::gpu("row visual rebuilds", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new(&format!("visual-{mode:?}-{phase}"));
        let cook = config(root.path());
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        cooker.cook_all().expect("initial cook");
        let product = environment(if phase == 0 || phase == 2 { 2 } else { 1 });
        let old = Box::pin(environment_reference(
            &cook, &device, &queue, &caps, mode, product,
        ))
        .await;
        let (mut live, stats) = host(&cook, &device, &queue, &caps, mode, true, None).await;
        settle(&mut live, &stats, generation(&cook));
        let updates = live.cooked_rows().expect("rows").updates();
        let mut candidate = if phase == 3 {
            Some(
                live.environment_prepare_inputs()
                    .prepare(product)
                    .await
                    .expect("early candidate"),
            )
        } else {
            None
        };
        // 128 cells, 1.2 MB of rows, still more than one bounded slice, so the partial-write
        // stage is reached; it was 256 cells, at four times each frame's triangles.
        let next = edit(&cook, &mut cooker, 128, 1.4);
        staged(&stats, next);
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let status = updates.status(next);
            let reached = match phase {
                0 => {
                    status == Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Writing)
                        && updates.report().last_write_bytes != 0
                }
                1 | 2 => status == Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Ready),
                _ => status == Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Committed),
            };
            if reached {
                break;
            }
            render_frame(&mut live);
            assert!(
                Instant::now() < deadline,
                "rebuild stage {phase} never reached"
            );
        }
        if phase == 2 {
            live.advance(Duration::from_millis(16))
                .expect("advance")
                .encode_scene()
                .expect("discard switch")
                .skip();
        }
        let before_status = updates.status(next);
        let before_bytes = updates.report().submitted_bytes;
        let mut candidate = match candidate.take() {
            Some(candidate) => candidate,
            None => live
                .environment_prepare_inputs()
                .prepare(product)
                .await
                .expect("candidate"),
        };
        assert!(live.publish_environment(&mut candidate).expect("publish"));
        assert_eq!(
            updates.status(next),
            before_status,
            "preparation belongs to the retained arena"
        );
        assert_eq!(
            updates.report().submitted_bytes,
            before_bytes,
            "rebuild must not rewrite row geometry"
        );
        let new = Box::pin(environment_reference(
            &cook, &device, &queue, &caps, mode, product,
        ))
        .await;
        assert_ne!(old, new, "fixture must expose changed geometry");
        only_old_or_new(&mut live, &stats, next, &old, &new);
        assert_eq!(live.capture_pixels().expect("following frame"), new);
        assert_eq!(live.capture_pixels().expect("persistent frame"), new);

        // The inherited allocator and retirement state must also support
        // the next replacement after the rebuilt scene has published.
        let shrink = edit(&cook, &mut cooker, 4, 0.07);
        let smaller = Box::pin(environment_reference(
            &cook, &device, &queue, &caps, mode, product,
        ))
        .await;
        only_old_or_new(&mut live, &stats, shrink, &new, &smaller);
        assert_eq!(live.capture_pixels().expect("shrunk frame"), smaller);
        eprintln!(
            "visual row rebuild: {mode:?}, phase={phase}, exact old/new and subsequent shrink passed"
        );
    });
}

#[test]
fn a_device_candidate_prepared_before_a_row_commit_is_refused_without_changing_the_host() {
    let Some(gpu) =
        thrum_test_support::gpu("stale device row snapshot", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let root = Scratch::new("stale-device-rows");
        let cook = config(root.path());
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        cooker.cook_all().expect("initial cook");
        let (mut live, stats) = host(
            &cook,
            &device,
            &queue,
            &caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        let old = settle(&mut live, &stats, generation(&cook));
        let replacement_gpu = gpu.fresh_adapter();
        let (replacement, replacement_queue) = replacement_gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let replacement_caps =
            DeviceCapabilities::capture(&replacement_gpu.adapter, &replacement, 2);
        let surface = Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: EXTENT,
        });
        let mut stale = live
            .device_prepare_inputs()
            .expect("inputs")
            .prepare(&replacement_caps, &replacement_queue, surface)
            .await
            .expect("early candidate");
        let next = edit(&cook, &mut cooker, 16, 0.8);
        let current = settle(&mut live, &stats, next);
        assert_ne!(old, current);
        let error = live
            .publish_device(&mut stale)
            .expect_err("stale geometry must not replace current rows");
        assert!(error.to_string().contains("stale mesh rows"), "{error}");
        assert_eq!(
            live.capture_pixels().expect("healthy host after refusal"),
            current
        );
        let mut newest = live
            .device_prepare_inputs()
            .expect("newest inputs")
            .prepare(&replacement_caps, &replacement_queue, surface)
            .await
            .expect("current candidate");
        live.publish_device(&mut newest)
            .expect("publish current rows");
        let (mut fresh, fresh_stats) = host(
            &cook,
            &replacement,
            &replacement_queue,
            &replacement_caps,
            CullMode::Occlusion,
            true,
            None,
        )
        .await;
        assert_eq!(
            settle(&mut live, &stats, next),
            settle(&mut fresh, &fresh_stats, next)
        );
    });
}
