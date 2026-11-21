//! Watcher batches may publish valid intermediate content. Finishing a save converges.

use std::fs;

use thrum_asset::{AssetStore, ContentHash, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

fn mesh_product(store: &AssetStore) -> ContentHash {
    let head = store.read_head().expect("read HEAD").expect("published");
    let manifest = store.load_manifest(&head).expect("load manifest");
    let entry = manifest
        .resolve("terrain", &[Profile::Portable])
        .expect("terrain");
    let bytes = store.map(&entry.product).expect("map terrain");
    entry.product.verify(&bytes).expect("verify terrain");
    entry.product
}

#[test]
fn a_valid_intermediate_mesh_publishes_and_the_finished_save_matches_a_fresh_cook() {
    let fixture = thrum_test_support::Scratch::new("preview-save");
    let source = fixture.join("source");
    let staged = fixture.join("staged");
    fs::create_dir_all(&source).expect("source directory");
    fs::create_dir_all(&staged).expect("staged directory");
    demo::write_grid_gltf(&source, "terrain", 16, 0.25).expect("initial mesh");
    demo::write_grid_gltf(&staged, "terrain", 16, 0.75).expect("edited mesh");

    // This transform changes the mesh with either buffer. The old buffer still fits all accessors.
    let edited_gltf = fs::read_to_string(staged.join("terrain.gltf")).expect("staged glTF");
    assert!(edited_gltf.contains(r#"{"mesh":0}"#));
    let edited_gltf = edited_gltf.replace(r#"{"mesh":0}"#, r#"{"mesh":0,"translation":[3,0,0]}"#);
    let config = CookConfig {
        source: source.clone(),
        store: fixture.join("working-store"),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    let store = AssetStore::new(&config.store);
    let mut cooker = Cooker::open(config.clone()).expect("open working cooker");
    let initial = cooker.cook_all().expect("initial cook");
    assert!(initial.failed.is_empty(), "{initial:?}");
    assert_eq!(
        initial.published.expect("initial publication").generation,
        1
    );
    let before = mesh_product(&store);

    // Drive the incremental cook entry point used by the watcher with two separate save batches.
    let gltf = source.join("terrain.gltf");
    fs::write(&gltf, edited_gltf).expect("save glTF only");
    let intermediate = cooker.cook_changed(&[gltf]).expect("intermediate cook");
    assert!(intermediate.failed.is_empty(), "{intermediate:?}");
    assert_eq!(
        intermediate
            .published
            .expect("intermediate publication")
            .generation,
        2
    );
    let preview = mesh_product(&store);
    assert_ne!(preview, before);

    let buffer = source.join("terrain.bin");
    fs::copy(staged.join("terrain.bin"), &buffer).expect("finish the save");
    let finished = cooker.cook_changed(&[buffer]).expect("finished cook");
    assert!(finished.failed.is_empty(), "{finished:?}");
    assert_eq!(
        finished.published.expect("finished publication").generation,
        3
    );
    let final_product = mesh_product(&store);
    assert_ne!(final_product, preview);

    let fresh_config = CookConfig {
        store: fixture.join("fresh-store"),
        ..config
    };
    let fresh_store = AssetStore::new(&fresh_config.store);
    let fresh = Cooker::open(fresh_config)
        .expect("open empty store")
        .cook_all()
        .expect("fresh cook");
    assert!(fresh.failed.is_empty(), "{fresh:?}");
    assert_eq!(fresh.published.expect("fresh publication").generation, 1);
    assert_eq!(final_product, mesh_product(&fresh_store));
}
