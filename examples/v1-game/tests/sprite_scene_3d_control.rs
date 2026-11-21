//! RENDER-2D's retained before/after witness from the V1 game's actual 3D setup.

use std::{fmt::Write as _, path::Path, time::Duration};
use thrum_asset::AssetStore;
use thrum_asset_gpu::CookedSource;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::runtime::FrameConfig;
use thrum_render::{
    present::SurfaceProfile,
    synthetic::{CullMode, DisplayOverlay, Reconstruction},
};
use v1_game::{CloudSelection, GameContent, GameHost, GameSetup, MeshRowSource};

const EXTENT: [u32; 2] = [64, 48];
const EXIT_MESHES: [&str; 5] = [
    "exit-boulder",
    "exit-scrub",
    "exit-cactus",
    "exit-standing-rock",
    "exit-balloon",
];

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn cook(root: &Path) -> AssetStore {
    let source = root.join("source");
    std::fs::create_dir_all(&source).expect("graph proof cook source");
    for folder in ["assets/showcase", "assets/exit"] {
        let game = Path::new(env!("CARGO_MANIFEST_DIR"));
        for entry in std::fs::read_dir(game.join(folder)).expect("game asset directory") {
            let entry = entry.expect("asset directory entry");
            if entry
                .path()
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|extension| matches!(extension, "png" | "gltf" | "bin"))
            {
                std::fs::copy(entry.path(), source.join(entry.file_name()))
                    .expect("copy authored asset");
            }
        }
    }
    let store = root.join("store");
    Cooker::open(CookConfig {
        source,
        store: store.clone(),
        profiles: vec![thrum_asset::Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open game cooker")
    .cook_all()
    .expect("cook game assets");
    AssetStore::new(store)
}

fn snapshot(gpu: &thrum_test_support::Gpu) -> String {
    let scratch = Scratch(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp")
            .join(format!("render2d-graph-{}", std::process::id())),
    );
    let store = cook(&scratch.0);
    let mut records = String::new();
    for recipe in ["portable", "apple-tile"] {
        let (device, queue, caps) = gpu
            .device_with_hardware(
                thrum_test_support::Profile::Native,
                &thrum_test_support::Restriction::default(),
                1,
                &[],
            )
            .expect("test device selects real Metal");
        let caps = caps
            .with_recipe_override(recipe)
            .expect("Metal recipe override");
        let default = GameSetup::default();
        let valley = GameSetup {
            camera_culling: CullMode::Frustum,
            valley_stars: true,
            ..default
        };
        let field = GameSetup {
            content: GameContent::Field,
            camera_culling: CullMode::Frustum,
            ..default
        };
        let exit = GameSetup {
            content: GameContent::Exit,
            cooked_mesh_rows: MeshRowSource::Cooked,
            ..valley
        };
        let arms = [
            ("grid", default, false),
            (
                "grid temporal inputs",
                GameSetup {
                    display_overlay: DisplayOverlay::TemporalInputs,
                    ..default
                },
                false,
            ),
            (
                "grid cloud shade",
                GameSetup {
                    clouds: CloudSelection::Shade,
                    ..default
                },
                false,
            ),
            (
                "grid cloud sky",
                GameSetup {
                    clouds: CloudSelection::Sky,
                    ..default
                },
                false,
            ),
            ("field cubes frustum", field, false),
            (
                "field meshes occlusion",
                GameSetup {
                    field_meshes: true,
                    camera_culling: CullMode::Occlusion,
                    ..field
                },
                false,
            ),
            (
                "valley day",
                GameSetup {
                    valley_stars: false,
                    ..valley
                },
                true,
            ),
            ("valley stars", valley, true),
            (
                "valley taau",
                GameSetup {
                    reconstruction: Reconstruction::Taau,
                    ..valley
                },
                true,
            ),
            (
                "valley reference",
                GameSetup {
                    reconstruction: Reconstruction::Reference,
                    ..valley
                },
                true,
            ),
            ("exit cooked entities", exit, true),
        ];
        for (label, setup, cooked) in arms {
            let config = FrameConfig {
                render_extent: EXTENT,
                surface: Some(SurfaceProfile {
                    extent: EXTENT,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                }),
            };
            let rows = (setup.content == GameContent::Exit).then(|| {
                thrum_asset_gpu::rows::CookedRows::from_store(
                    &store,
                    EXIT_MESHES.map(str::to_owned).to_vec(),
                    device.limits().max_storage_buffer_binding_size,
                )
                .expect("prepared exit mesh rows")
            });
            let mut host =
                pollster::block_on(setup.prepare_with_rows(&device, &queue, &caps, config, rows))
                    .expect("prepare the actual V1 variant");
            if cooked {
                let (camera, mut assets) =
                    v1_game::prepare_cooked_valley(CookedSource::from(scratch.0.join("store")))
                        .expect("game's cooked valley recipe");
                if let Some(rows) = host.cooked_rows() {
                    assets = assets.with_rows(rows).expect("couple cooked entity rows");
                }
                host.set_game_contribution(Box::new(assets))
                    .expect("attach cooked placements");
                host.set_scene_camera(camera)
                    .expect("install valley camera");
            }
            if setup.valley_stars {
                host.set_time_of_day(0.0).expect("sample the midnight view");
            }
            let frame = host
                .advance(Duration::ZERO)
                .expect("admit initial frame")
                .encode_scene()
                .expect("record the held variant");
            frame.skip();
            let graph = host.scene_graph().expect("compiled V1 graph");
            writeln!(records, "{recipe} {label}").expect("write snapshot");
            writeln!(
                records,
                "{:?} order={:?} slots={}",
                graph.pass_names().collect::<Vec<_>>(),
                graph.pass_order,
                graph.transient_slots
            )
            .expect("write snapshot");
            let slots: Vec<_> = graph
                .lifetimes
                .iter()
                .filter_map(|life| {
                    Some((life.transient_slot?, &life.resource, life.first, life.last))
                })
                .collect();
            let light = thrum_test_support::BeforeLightSplit::new(&graph);
            records.push_str(&light.resource_indices(&format!(
                "storage={:?}\ntransients={slots:?}\n",
                graph.attachment_storage
            )));
        }
    }
    records
}

#[test]
fn v1_game_3d_graphs_match_before_sprite_preparation() {
    let Some(gpu) = thrum_test_support::gpu("V1 3D scene variants", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    assert_eq!(
        snapshot(&gpu),
        include_str!("support/scene-3d-starting.txt")
    );
}
