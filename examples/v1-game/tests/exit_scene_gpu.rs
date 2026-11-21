//! Metal witness for the cooked scene's visible bucket counts at the six presets.

use std::{fs, path::Path, time::Duration};

use glam::{Mat3, Mat4, Vec3, Vec4};
use serde::Deserialize;
use thrum_asset::{AssetStore, MeshProduct, Profile};
use thrum_asset_gpu::{
    CookedSource,
    rows::{CookedRows, mesh_row},
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::runtime::{FrameConfig, SurfaceOutput};
use thrum_render::{
    capabilities::DeviceCapabilities,
    synthetic::{CullMode, SyntheticView},
};
use thrum_test_support::{Profile as TestProfile, Restriction, gpu};
use v1_game::{GameContent, GameHost, GameScene, GameSetup, GameWorld, MeshRowSource};

type Host = thrum_render::runtime::FrameHost<GameWorld, GameScene>;

use crate::support;

const EXTENT: [u32; 2] = [1280, 720];
const MESHES: [&str; 5] = [
    "exit-boulder",
    "exit-scrub",
    "exit-cactus",
    "exit-standing-rock",
    "exit-balloon",
];

#[derive(Deserialize)]
struct Layout {
    entities: Vec<(usize, u64, f32, f32, f32, f32, f32)>,
}

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

fn cook(root: &Path) -> AssetStore {
    let game = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("source");
    fs::create_dir_all(&source).expect("cook source");
    for dir in ["assets/showcase", "assets/exit"] {
        for entry in fs::read_dir(game.join(dir)).expect("checked-in asset directory") {
            let entry = entry.expect("source entry");
            if entry
                .path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|extension| matches!(extension, "png" | "gltf" | "bin"))
            {
                fs::copy(entry.path(), source.join(entry.file_name())).expect("authored source");
            }
        }
    }
    let store = root.join("store");
    let mut cooker = Cooker::open(CookConfig {
        source,
        store: store.clone(),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    cooker.cook_all().expect("exit store");
    AssetStore::new(store)
}

fn bounds(store: &AssetStore, limit: u64) -> [(Vec3, Vec3); 5] {
    let head = store
        .read_head()
        .expect("read HEAD")
        .expect("exit generation");
    let manifest = store.load_manifest(&head).expect("exit manifest");
    MESHES.map(|name| {
        let product = manifest
            .resolve(name, &[Profile::Portable])
            .expect("cooked prop");
        let mesh =
            MeshProduct::open(store.map(&product.product).expect("product bytes")).expect("mesh");
        let row = mesh_row(&mesh, limit).expect("whole mesh row");
        assert_ne!(row.mesh().indices.len(), 0);
        row.mesh().vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), vertex| {
                let point = Vec3::from(vertex.position);
                (min.min(point), max.max(point))
            },
        )
    })
}

fn visible(layout: &Layout, bounds: &[(Vec3, Vec3); 5], eye: Vec3, pitch: f32, yaw: f32) -> u32 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let forward = Vec3::new(cos_pitch * sin_yaw, sin_pitch, -cos_pitch * cos_yaw);
    let view = SyntheticView::perspective(
        eye.to_array(),
        (eye + forward * 76.105_194).to_array(),
        [0.0, 1.0, 0.0],
        46_f32.to_radians(),
        16.0 / 9.0,
        0.1,
        30_000.0,
    )
    .expect("view");
    let matrix = Mat4::from_cols_array_2d(&view.view_proj_columns());
    let [x, y, z, w] = std::array::from_fn(|row| matrix.row(row));
    let planes = [w + x, w - x, w + y, w - y, w - z, z];
    let count = layout
        .entities
        .iter()
        .filter(|&&(mesh, _, x, y, z, turn, scale)| {
            let (min, max) = bounds[mesh];
            let basis = Mat3::from_rotation_y(turn) * scale;
            let point = Vec3::new(x, y, z);
            let center = point + basis * ((max + min) * 0.5);
            let radius = ((max - min) * 0.5) * 1.0001;
            let half = basis.x_axis.abs() * radius.x
                + basis.y_axis.abs() * radius.y
                + basis.z_axis.abs() * radius.z;
            planes.iter().all(|&plane| {
                let corner = center + half * plane.truncate().signum();
                Vec4::dot(corner.extend(1.0), plane) + 1.0e-5 * plane.w.abs() >= 0.0
            })
        })
        .count();
    u32::try_from(count).expect("10,000 slots")
}

#[test]
fn exit_scene_uses_cooked_rows_and_matches_the_cpu_frustum() {
    let Some(gpu) = gpu("exit_scene_gpu", Some(wgpu::Backend::Metal)) else {
        return;
    };
    pollster::block_on(async {
        let (device, queue) = gpu.device(TestProfile::Portable, &Restriction::default());
        let caps = DeviceCapabilities::capture(
            &gpu.adapter,
            &device,
            thrum_render::capabilities::FIRST_DEVICE_GENERATION,
        );
        let scratch = Scratch(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tmp")
                .join(format!("exit-gpu-{}", std::process::id())),
        );
        let store = cook(&scratch.0);
        let limit = device.limits().max_storage_buffer_binding_size;
        let bounds = bounds(&store, limit);
        let rows = CookedRows::from_store(&store, MESHES.map(str::to_owned).to_vec(), limit)
            .expect("all cooked rows");
        let (view, profile) = support::presentation(&device, EXTENT);
        let surface = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        let mut host = GameSetup {
            content: GameContent::Exit,
            camera_culling: GameSetup::valley_default_culling(),
            cooked_mesh_rows: MeshRowSource::Cooked,
            ..GameSetup::default()
        }
        .prepare_with_rows(
            &device,
            &queue,
            &caps,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(profile),
            },
            Some(rows),
        )
        .await
        .expect("exit host");
        assert_eq!(host.camera_culling(), CullMode::Frustum);
        let (camera, assets) =
            v1_game::prepare_cooked_valley(CookedSource::from(scratch.0.join("store")))
                .expect("authored valley");
        let rows = host.cooked_rows().expect("prepared cooked rows");
        let assets = assets.with_rows(rows).expect("coupled rows");
        let stats = assets.stats();
        host.set_game_contribution(Box::new(assets))
            .expect("cooked scene");
        host.set_scene_camera(camera).expect("authored camera");
        host.world_mut().pause().expect("hold tick zero");
        let sample = |host: &mut Host| {
            host.drive(Duration::ZERO, Some(&surface)).expect("submit");
            loop {
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .expect("frame completion");
                if let Some(sample) = host.poll_completed().expect("poll") {
                    return sample;
                }
            }
        };
        let ready = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            drop(sample(&mut host));
            let stats = stats.lock().expect("scene stats");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            if stats.drawn == Some(1) {
                break;
            }
            assert!(
                std::time::Instant::now() < ready,
                "cooked scene did not draw"
            );
        }
        let layout: Layout =
            serde_json::from_str(include_str!("../assets/exit/layout.json")).expect("layout");
        let initial_pipelines = host.pipeline_creation_count();
        for name in [
            "sunrise",
            "noon",
            "sunset-clear-sun",
            "sunset-sun",
            "twilight",
            "night-upslope",
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("params")
                .join(format!("showcase-{name}.params.json"));
            let text = fs::read_to_string(path).expect("unchanged preset");
            let mut controls = v1_game::controls::GameControls::new().expect("controls");
            assert_eq!(
                controls
                    .model_mut()
                    .apply_file(&text)
                    .expect("apply")
                    .problems
                    .len(),
                0
            );
            let knobs = controls.replay();
            host.set_time_of_day(knobs.time_of_day.expect("time"))
                .expect("time edit");
            host.set_camera_yaw(knobs.camera_yaw.expect("yaw"))
                .expect("yaw edit");
            host.set_camera_pitch(knobs.camera_pitch.expect("pitch"))
                .expect("pitch edit");
            let frame = sample(&mut host);
            assert_eq!(frame.tick, 0);
            if name == "noon" {
                let timing = frame.gpu_pass_ns.as_ref().expect("Metal timestamps");
                assert!(
                    timing.shadow_near() > 0.0 && timing.shadow_far() > 0.0,
                    "Sun shadow passes"
                );
            }
            let counts = frame.camera_culling.expect("indirect count from the GPU");
            assert_eq!(counts.active, 10_000);
            let [pitch, yaw] = host.camera_orientation();
            let expected = visible(&layout, &bounds, Vec3::from(camera.eye()), pitch, yaw);
            assert_eq!(counts.drawn, expected, "{name}");
            println!("{name}: {} cooked instances in view", counts.drawn);
        }
        assert_eq!(host.pipeline_creation_count(), initial_pipelines);
    });
}
