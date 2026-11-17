//! Cooked entity rows exercised through the V1 game's ordinary frame host.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thrum_asset::{AssetStore, MeshProduct, Profile};
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedStats, MeshPlacement, rows::CookedRows};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::synthetic::mesh_rows::{MeshRowError, MeshRowGeometry};
use thrum_render::{capabilities::DeviceCapabilities, synthetic::CullMode};
use v1_game::{FrameConfig, FrameHost, GameSetup};

const EXTENT: [u32; 2] = [320, 180];
type Stats = Arc<Mutex<CookedStats>>;

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
) -> (FrameHost, Stats) {
    let rows = rows(cook, device);
    if let Some(binding) = binding {
        drop(rows.updates().with_binding_limit(binding));
    }
    let mut host = FrameHost::new_with_rows(
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
        GameSetup {
            field_meshes: true,
            camera_culling: mode,
            ..GameSetup::default()
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
    host.set_scene_contribution(Box::new(assets))
        .expect("attach");
    host.pause().expect("pause at same tick");
    (host, stats)
}

fn settle(host: &mut FrameHost, stats: &Stats, generation: u64) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        let image = host.capture_pixels().expect("game capture");
        let stats = stats.lock().expect("stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(generation) {
            return image;
        }
        assert!(
            Instant::now() < deadline,
            "generation {generation} timed out"
        );
        drop(stats);
        std::thread::yield_now();
    }
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

fn only_old_or_new(live: &mut FrameHost, stats: &Stats, generation: u64, old: &[u8], new: &[u8]) {
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
        let uploading = edit(&cook, &mut cooker, 512, 1.8);
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
        let protected = edit(&cook, &mut cooker, 256, 0.8);
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
        for (size, amplitude) in [(256, 0.9), (4, 0.1), (256, 0.7), (4, 0.05)] {
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
    for ((vertex, position), attributes) in geometry
        .mesh()
        .vertices
        .iter()
        .zip(product.positions())
        .zip(product.attributes())
    {
        assert_eq!(&vertex.position, position);
        assert_eq!(vertex.uv, attributes.uv);
        assert!((glam::Vec3::from_array(vertex.normal).length() - 1.0).abs() < 1e-6);
    }
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

#[test]
fn edited_rows_equal_fresh_game_in_every_culling_mode() {
    let Some(gate) = thrum_test_support::gpu("cooked rows", Some(wgpu::Backend::Metal)) else {
        return;
    };
    pollster::block_on(async {
        let (device, queue) = gate.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let caps = DeviceCapabilities::capture(&gate.adapter, &device, 1);
        for mode in [CullMode::Direct, CullMode::Frustum, CullMode::Occlusion] {
            for coupled in [false, true] {
                let root = Scratch::new(&format!("{mode:?}-{coupled}"));
                let cook = config(root.path());
                let mut cooker = Cooker::open(cook.clone()).expect("cooker");
                cooker.cook_all().expect("first cook");
                let first = AssetStore::new(&cook.store)
                    .read_head()
                    .expect("HEAD")
                    .expect("first")
                    .generation;
                let (mut live, stats) =
                    host(&cook, &device, &queue, &caps, mode, coupled, None).await;
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
            }
        }
    });
}
