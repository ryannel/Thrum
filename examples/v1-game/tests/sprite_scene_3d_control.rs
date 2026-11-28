//! Before/after graph witnesses from the V1 game's actual 3D setups.

use std::{fmt::Write as _, path::Path, time::Duration};
use thrum_asset::AssetStore;
use thrum_asset_gpu::CookedSource;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::runtime::FrameConfig;
use thrum_render::{
    present::SurfaceProfile,
    synthetic::{CullMode, DisplayOverlay, Reconstruction, TwinReason},
};
use thrum_test_support::Scratch;
use v1_game::{CloudSelection, GameContent, GameHost, GameSetup, MeshRowSource};

const EXTENT: [u32; 2] = [64, 48];
const EXIT_MESHES: [&str; 5] = [
    "exit-boulder",
    "exit-scrub",
    "exit-cactus",
    "exit-standing-rock",
    "exit-balloon",
];

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

fn graph_cases() -> [(&'static str, GameSetup, bool); 12] {
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
    [
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
        (
            "valley MetalFX",
            GameSetup {
                reconstruction: Reconstruction::MetalFx,
                ..valley
            },
            true,
        ),
    ]
}

fn snapshot(
    gpu: &thrum_test_support::Gpu,
    recipes: &[&str],
    format: wgpu::TextureFormat,
    historical: bool,
) -> String {
    let scratch = Scratch::new("v1-graph");
    let store = cook(scratch.path());
    let mut records = String::new();
    for &recipe in recipes {
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
        for (label, setup, cooked) in graph_cases() {
            if historical && setup.reconstruction == Reconstruction::MetalFx {
                continue;
            }
            let config = FrameConfig {
                render_extent: EXTENT,
                surface: Some(SurfaceProfile {
                    extent: EXTENT,
                    format,
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
                    v1_game::prepare_cooked_valley(CookedSource::from(scratch.join("store")))
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
            if historical {
                writeln!(records, "{recipe} {label}").expect("write snapshot");
            } else {
                let creation = host.creation_report();
                assert!(
                    creation
                        .rows
                        .iter()
                        .all(|row| !row.label.contains("interface"))
                );
                assert!(
                    creation
                        .pipelines
                        .iter()
                        .all(|row| !row.label.contains("interface"))
                );
                let resolved = host.reconstruction();
                let provider = host.native_provider_report();
                let native = setup.reconstruction == Reconstruction::MetalFx;
                if native {
                    let provider = provider.expect("prepared MetalFX request");
                    assert_eq!(provider.pass, "MetalFX temporal");
                    if recipe == "apple-tile" {
                        assert_eq!(resolved, Reconstruction::MetalFx);
                        assert!(provider.twin.is_none());
                        assert!(graph.pass_names().any(|name| name == provider.pass));
                    } else {
                        assert_eq!(resolved, Reconstruction::Taau);
                        let twin = provider.twin.expect("portable MetalFX twin");
                        assert_eq!(
                            (twin.name, twin.reason),
                            ("TAAU", TwinReason::PortableRecipe)
                        );
                    }
                } else {
                    assert_eq!(resolved, setup.reconstruction, "{recipe} {label}");
                }
                writeln!(records, "{recipe} {format:?} {label} resolved={resolved:?}")
                    .expect("write snapshot");
            }
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
            let storage = format!(
                "storage={:?}\ntransients={slots:?}\n",
                graph.attachment_storage
            );
            let chart = thrum_test_support::BeforeChartSnapshot::new(&graph);
            let storage = chart.resource_indices(&storage);
            if historical {
                let light = thrum_test_support::BeforeLightSplit::new(&graph);
                records.push_str(&light.resource_indices(&storage));
            } else {
                records.push_str(&storage);
            }
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
        snapshot(
            &gpu,
            &["portable", "apple-tile"],
            wgpu::TextureFormat::Bgra8UnormSrgb,
            true,
        ),
        include_str!("support/scene-3d-starting.txt")
    );
}

fn compare_presentation(recipe: &str, format: wgpu::TextureFormat, baseline: &str) {
    let Some(gpu) =
        thrum_test_support::gpu("V1 3D presented graph variants", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (commit, expected) = baseline.split_once('\n').expect("baseline revision");
    assert_eq!(commit, "b96e70604");
    let actual = snapshot(&gpu, &[recipe], format, false);
    for (line, (before, after)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(
            before, after,
            "{recipe} {format:?} graph snapshot line {line}"
        );
    }
    assert_eq!(expected.lines().count(), actual.lines().count());
}

#[test]
fn portable_v1_sdr_graphs_match_before_the_interface() {
    compare_presentation(
        "portable",
        wgpu::TextureFormat::Bgra8UnormSrgb,
        include_str!("support/interface-absent-portable-sdr.txt"),
    );
}

#[test]
fn portable_v1_float_graphs_match_before_the_interface() {
    compare_presentation(
        "portable",
        wgpu::TextureFormat::Rgba16Float,
        include_str!("support/interface-absent-portable-float.txt"),
    );
}

#[test]
fn apple_tile_v1_sdr_graphs_match_before_the_interface() {
    compare_presentation(
        "apple-tile",
        wgpu::TextureFormat::Bgra8UnormSrgb,
        include_str!("support/interface-absent-apple-tile-sdr.txt"),
    );
}

#[test]
fn apple_tile_v1_float_graphs_match_before_the_interface() {
    compare_presentation(
        "apple-tile",
        wgpu::TextureFormat::Rgba16Float,
        include_str!("support/interface-absent-apple-tile-float.txt"),
    );
}
