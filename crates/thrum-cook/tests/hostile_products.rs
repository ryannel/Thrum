//! Runtime parsers reject damaged or hostile products without panicking or reading out of bounds.
//!
//! Deterministic mutations of real cooked products: every truncation length near structure
//! boundaries, single-byte flips, and extreme values written over each aligned word of the
//! headers and section tables. The `fuzz_products` example in `thrum-asset` takes the same parsers
//! to an external fuzzer.

use std::sync::atomic::{AtomicU32, Ordering};

use thrum_asset::{Head, Ktx2, Manifest, MeshProduct, ProductBytes, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

static NEXT: AtomicU32 = AtomicU32::new(0);

fn exercise(bytes: &[u8]) {
    std::hint::black_box(Head::decode(bytes).is_ok());
    if let Ok(manifest) = Manifest::parse(ProductBytes::copy_from(bytes)) {
        for entry in manifest.entries() {
            std::hint::black_box((entry.name.len(), entry.byte_len));
        }
        std::hint::black_box(manifest.resolve("meshes/terrain", &Profile::ALL).is_some());
    }
    if let Ok(mesh) = MeshProduct::open(ProductBytes::copy_from(bytes)) {
        std::hint::black_box(mesh.validate_references().is_ok());
        std::hint::black_box((mesh.positions().len(), mesh.meshlet_triangles().len()));
    }
    if let Ok(texture) = Ktx2::open(ProductBytes::copy_from(bytes)) {
        for level in texture.levels() {
            std::hint::black_box(texture.level_data(level).len());
        }
    }
}

#[test]
fn mutated_products_are_rejected_or_parsed_safely() {
    let root = std::env::temp_dir().join(format!(
        "thrum-g5-hostile-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("dir");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 6, 0.2).expect("gltf");
    demo::write_checker_png(&source.join("checker.png"), 16, [1, 2, 3]).expect("png");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 8, 3).expect("volume");
    let mut cooker = Cooker::open(CookConfig {
        source,
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open");
    let head = cooker.cook_all().expect("cook").published.expect("publish");
    let store = cooker.store().clone();
    let manifest = store.load_manifest(&head).expect("manifest");
    let mut samples = vec![
        head.encode().to_vec(),
        store.map(&head.manifest).expect("map").to_vec(),
    ];
    samples.extend(
        manifest
            .entries()
            .map(|entry| store.map(&entry.product).expect("map").to_vec()),
    );
    assert_eq!(samples.len(), 2 + 7);

    // A level index whose extent product would wrap 64 bits must not match a zero-length level.
    for sample in &samples[2..] {
        if sample.starts_with(&[0xab, b'K', b'T', b'X']) {
            let mut wrapped = sample.clone();
            for (offset, value) in [
                (20, 4_194_304_u32),
                (24, 2_097_152),
                (28, 2_097_152),
                (40, 1),
            ] {
                wrapped[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            wrapped[80 + 8..80 + 24].fill(0);
            assert!(Ktx2::open(ProductBytes::copy_from(&wrapped)).is_err());
        }
    }

    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut random = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut cases = 0_usize;
    for original in &samples {
        exercise(original);
        let header = original.len().min(1024);
        for length in (0..header).chain((original.len().saturating_sub(64))..original.len()) {
            exercise(&original[..length]);
            cases += 1;
        }
        for offset in (0..header.saturating_sub(3)).step_by(4) {
            for value in [0_u32, 1, 0x7fff_ffff, u32::MAX, 0x8000_0000, 64] {
                let mut mutated = original.clone();
                mutated[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                exercise(&mutated);
                cases += 1;
            }
        }
        for _ in 0..2_000 {
            let mut mutated = original.clone();
            let position = usize::try_from(random() % original.len() as u64).expect("index");
            mutated[position] ^= u8::try_from(random() % 255 + 1).expect("byte");
            exercise(&mutated);
            cases += 1;
        }
    }
    assert!(cases > 20_000, "{cases}");
    drop(cooker);
    writable(&root);
    std::fs::remove_dir_all(&root).ok();
}

fn writable(path: &std::path::Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut permissions = meta.permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "test cleanup of read-only store files"
        )]
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions).ok();
        if meta.is_dir() {
            for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
                writable(&entry.path());
            }
        }
    }
}
