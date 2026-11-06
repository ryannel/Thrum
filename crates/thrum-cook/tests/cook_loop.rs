//! End-to-end cook cycles over a real temporary source tree and store.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use thrum_asset::{AssetStore, HeadWatcher, Ktx2, Ktx2Format, MeshProduct, ProductKind, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

static NEXT: AtomicU32 = AtomicU32::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "thrum-cook-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("source/meshes")).expect("create source");
        std::fs::create_dir_all(root.join("source/textures")).expect("create source");
        demo::write_grid_gltf(&root.join("source/meshes"), "terrain", 24, 0.3).expect("write gltf");
        demo::write_checker_png(&root.join("source/textures/checker.png"), 64, [200, 40, 40])
            .expect("write png");
        demo::write_cloud_volume(&root.join("source/cloud.tvol"), 16, 1).expect("write volume");
        Self { root }
    }

    fn source(&self) -> PathBuf {
        self.root.join("source")
    }

    fn config(&self) -> CookConfig {
        CookConfig {
            source: self.source(),
            store: self.root.join("store"),
            profiles: Profile::ALL.to_vec(),
            quality: Quality::Fast,
            durability: Durability::Process,
        }
    }

    fn store(&self) -> AssetStore {
        AssetStore::new(self.root.join("store"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        make_writable(&self.root);
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn make_writable(path: &Path) {
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
                make_writable(&entry.path());
            }
        }
    }
}

#[test]
fn full_cook_publishes_every_kind_and_profile_and_the_runtime_accepts_them() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    let report = cooker.cook_all().expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(report.cooked, 1 + 3 + 3);
    let head = report.published.expect("first publish");
    assert_eq!(head.generation, 1);

    let store = fixture.store();
    let manifest = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    assert_eq!(manifest.entries().count(), 7);

    let mesh = manifest
        .resolve("meshes/terrain", &[Profile::AppleAstc, Profile::Portable])
        .expect("mesh entry");
    assert_eq!(mesh.kind, ProductKind::Mesh);
    let mesh = MeshProduct::open(store.map(&mesh.product).expect("map")).expect("mesh");
    mesh.validate_references().expect("references");
    assert!(
        mesh.meshlets().len() > 4,
        "a 24×24 grid spans several meshlets"
    );
    assert_eq!(mesh.indices().len(), 24 * 24 * 6);
    assert!(
        mesh.meshlets()
            .iter()
            .all(|m| m.vertex_count <= 64 && m.triangle_count <= 124)
    );
    assert!(mesh.meta().bounds_max[1] > 0.1);

    for (profile, format) in [
        (Profile::Portable, Ktx2Format::Rgba8Srgb),
        (Profile::DesktopBc, Ktx2Format::Bc7Srgb),
        (Profile::AppleAstc, Ktx2Format::Astc4x4Srgb),
    ] {
        let entry = manifest
            .resolve("textures/checker", &[profile])
            .expect("texture entry");
        let texture = Ktx2::open(store.map(&entry.product).expect("map")).expect("texture");
        assert_eq!(texture.format(), format);
        assert_eq!(texture.levels().len(), 7, "64 px has a 7-level chain");
        assert!(!texture.is_3d());
    }
    for (profile, format) in [
        (Profile::DesktopBc, Ktx2Format::Bc4Unorm),
        (Profile::Portable, Ktx2Format::R8Unorm),
    ] {
        let entry = manifest.resolve("cloud", &[profile]).expect("volume entry");
        assert_eq!(entry.kind, ProductKind::Volume);
        let volume = Ktx2::open(store.map(&entry.product).expect("map")).expect("volume");
        assert!(volume.is_3d());
        assert_eq!(volume.format(), format);
        assert_eq!(volume.levels()[0].depth, 16);
        assert_eq!(volume.levels().len(), 5);
    }
}

#[test]
fn unchanged_sources_reuse_everything_and_do_not_republish() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("first cook");
    let again = cooker.cook_all().expect("second cook");
    assert_eq!(again.cooked, 0);
    assert_eq!(again.reused, 7);
    assert!(again.published.is_none());
    drop(cooker);

    // A fresh process reuses the persisted action cache and stat cache.
    let mut reopened = Cooker::open(fixture.config()).expect("reopen");
    assert_eq!(reopened.generation(), 1);
    let report = reopened.cook_all().expect("cook after reopen");
    assert_eq!(report.cooked, 0);
    assert!(report.published.is_none());
}

#[test]
fn an_edit_recooks_only_its_asset_and_old_readers_stay_valid() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let store = fixture.store();
    let mut watcher = HeadWatcher::new(store.clone());
    let first = watcher.poll().expect("poll").expect("first head");
    let old_manifest = store.load_manifest(&first).expect("old manifest");
    let old_texture = old_manifest
        .resolve("textures/checker", &[Profile::DesktopBc])
        .expect("entry")
        .product;
    let old_mesh = old_manifest
        .resolve("meshes/terrain", &[Profile::Portable])
        .expect("entry")
        .product;
    let old_bytes = store.map(&old_texture).expect("map old texture");
    assert!(watcher.poll().expect("poll").is_none());

    let path = fixture.source().join("textures/checker.png");
    demo::write_checker_png(&path, 64, [40, 40, 200]).expect("edit png");
    let report = cooker.cook_changed(&[path]).expect("incremental cook");
    assert_eq!(report.cooked, 3, "three texture profiles only");
    assert_eq!(report.scan.hashed, 1);
    let second = watcher.poll().expect("poll").expect("new head");
    assert_eq!(second.generation, 2);

    let manifest = store.load_manifest(&second).expect("new manifest");
    assert_ne!(
        manifest
            .resolve("textures/checker", &[Profile::DesktopBc])
            .expect("entry")
            .product,
        old_texture
    );
    assert_eq!(
        manifest
            .resolve("meshes/terrain", &[Profile::Portable])
            .expect("entry")
            .product,
        old_mesh
    );
    // The previous generation's products are immutable and still readable.
    assert!(old_texture.verify(&old_bytes).is_ok());
    assert!(Ktx2::open(store.map(&old_texture).expect("map")).is_ok());
}

#[test]
fn a_broken_source_keeps_the_previous_product_published() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let store = fixture.store();
    let before = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    let product = before
        .resolve("textures/checker", &[Profile::Portable])
        .expect("entry")
        .product;

    let path = fixture.source().join("textures/checker.png");
    std::fs::write(&path, b"not a png").expect("break source");
    let report = cooker.cook_changed(&[path]).expect("cycle");
    assert_eq!(report.failed.len(), 1, "{:?}", report.failed);
    assert!(report.published.is_none(), "nothing valid changed");
    let after = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    assert_eq!(
        after
            .resolve("textures/checker", &[Profile::Portable])
            .expect("entry")
            .product,
        product
    );

    // A deleted asset disappears from the next publication.
    std::fs::remove_file(fixture.source().join("cloud.tvol")).expect("delete");
    let report = cooker
        .cook_changed(&[fixture.source().join("cloud.tvol")])
        .expect("cycle");
    let head = report.published.expect("publish removal");
    assert!(
        store
            .load_manifest(&head)
            .expect("manifest")
            .resolve("cloud", &Profile::ALL)
            .is_none()
    );
}

#[test]
fn reopening_removes_damaged_products_and_recooks_them() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    drop(cooker);
    let store = fixture.store();
    let manifest = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    let mesh = manifest
        .resolve("meshes/terrain", &[Profile::Portable])
        .expect("entry")
        .product;
    let path = store.product_path(&mesh);
    make_writable(&path);
    let mut bytes = std::fs::read(&path).expect("read");
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&path, bytes).expect("damage");

    let mut cooker = Cooker::open(fixture.config()).expect("reopen");
    assert!(!path.exists(), "damaged product removed");
    let report = cooker.cook_all().expect("recook");
    assert_eq!(report.cooked, 1);
    assert!(
        mesh.verify(&std::fs::read(&path).expect("restored"))
            .is_ok()
    );
}

#[test]
fn the_stat_cache_notices_an_atomic_save_with_identical_size_and_time() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let mut cache = thrum_cook::StatCache::default();
    let path = source.join("textures/checker.png");
    let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
    for file in [
        "textures/checker.png",
        "meshes/terrain.gltf",
        "meshes/terrain.bin",
        "cloud.tvol",
    ] {
        std::fs::File::options()
            .write(true)
            .open(source.join(file))
            .expect("open")
            .set_modified(old_time)
            .expect("age file");
    }
    cache.scan_all(&source).expect("scan");
    let original = cache
        .hash(Path::new("textures/checker.png"))
        .expect("tracked");
    let second = cache.scan_all(&source).expect("rescan");
    assert_eq!(second.hashed, 0, "aged files are trusted by stamp");

    // Same length and modification time, different bytes, new inode.
    let mut bytes = std::fs::read(&path).expect("read");
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    let staged = source.join("textures/.checker.png.tmp");
    std::fs::write(&staged, &bytes).expect("stage");
    std::fs::File::options()
        .write(true)
        .open(&staged)
        .expect("open")
        .set_modified(old_time)
        .expect("age file");
    std::fs::rename(&staged, &path).expect("atomic save");
    let third = cache.scan_all(&source).expect("rescan");
    assert_eq!(third.hashed, 1);
    assert_ne!(
        cache.hash(Path::new("textures/checker.png")),
        Some(original)
    );
}

fn manifest_of(store: &AssetStore) -> thrum_asset::Manifest {
    store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest")
}

#[test]
fn two_sources_for_one_name_fail_and_keep_the_previous_product() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let before = manifest_of(&fixture.store())
        .resolve("textures/checker", &[Profile::Portable])
        .expect("entry")
        .product;
    demo::write_checker_png(
        &fixture.source().join("textures/checker.linear.png"),
        64,
        [0, 200, 0],
    )
    .expect("second source");
    let report = cooker.cook_all().expect("cook");
    assert!(
        report
            .failed
            .iter()
            .any(|(name, reason)| name == "textures/checker" && reason.contains("several sources")),
        "{:?}",
        report.failed
    );
    assert_eq!(
        manifest_of(&fixture.store())
            .resolve("textures/checker", &[Profile::Portable])
            .expect("kept")
            .product,
        before
    );
}

#[test]
fn a_planning_failure_keeps_the_previous_mesh_published() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let before = manifest_of(&fixture.store())
        .resolve("meshes/terrain", &[Profile::Portable])
        .expect("entry")
        .product;
    std::fs::remove_file(fixture.source().join("meshes/terrain.bin")).expect("remove buffer");
    let report = cooker.cook_all().expect("cook");
    assert!(
        report.failed.iter().any(
            |(name, reason)| name == "meshes/terrain" && reason.contains("missing glTF buffer")
        ),
        "{:?}",
        report.failed
    );
    assert!(report.published.is_none(), "nothing valid changed");
    assert_eq!(
        manifest_of(&fixture.store())
            .resolve("meshes/terrain", &[Profile::Portable])
            .expect("kept")
            .product,
        before
    );
}

#[test]
fn a_damaged_product_behind_an_action_record_is_recooked_not_republished() {
    let fixture = Fixture::new();
    let path = fixture.source().join("textures/checker.png");
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook red");
    let red = manifest_of(&fixture.store())
        .resolve("textures/checker", &[Profile::Portable])
        .expect("entry")
        .product;
    demo::write_checker_png(&path, 64, [40, 40, 200]).expect("blue");
    cooker
        .cook_changed(std::slice::from_ref(&path))
        .expect("cook blue");
    drop(cooker);
    // The red product is now referenced only by its action record; damage it as power loss might.
    let product = fixture.store().product_path(&red);
    make_writable(&product);
    let length = std::fs::metadata(&product).expect("meta").len();
    std::fs::write(&product, vec![0; usize::try_from(length).expect("len")]).expect("zero-fill");

    let mut cooker = Cooker::open(fixture.config()).expect("reopen");
    demo::write_checker_png(&path, 64, [200, 40, 40]).expect("back to red");
    let report = cooker.cook_changed(&[path]).expect("cook red again");
    assert!(report.cooked >= 1, "the damaged product is recooked");
    let published = manifest_of(&fixture.store())
        .resolve("textures/checker", &[Profile::Portable])
        .expect("entry")
        .product;
    assert_eq!(published, red);
    assert!(
        red.verify(&std::fs::read(fixture.store().product_path(&red)).expect("read"))
            .is_ok()
    );
}

#[test]
fn a_damaged_head_does_not_restart_generations() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let path = fixture.source().join("textures/checker.png");
    demo::write_checker_png(&path, 64, [40, 40, 200]).expect("edit");
    cooker
        .cook_changed(std::slice::from_ref(&path))
        .expect("cook");
    drop(cooker);
    std::fs::write(fixture.store().head_path(), b"garbage").expect("damage head");
    let mut cooker = Cooker::open(fixture.config()).expect("reopen");
    assert_eq!(cooker.generation(), 2);
    let report = cooker.cook_all().expect("cook");
    assert_eq!(report.published.expect("republish").generation, 3);
}

#[test]
fn only_one_cooker_may_write_a_store() {
    let fixture = Fixture::new();
    let _first = Cooker::open(fixture.config()).expect("open");
    let second = Cooker::open(fixture.config());
    assert!(second.is_err_and(|error| error.to_string().contains("locked")));
}

#[test]
fn hidden_files_named_by_the_watcher_are_not_sources() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let hidden = fixture.source().join("textures/._checker.png");
    demo::write_checker_png(&hidden, 64, [1, 2, 3]).expect("hidden");
    let report = cooker.cook_changed(&[hidden]).expect("cook");
    assert!(report.published.is_none() && report.cooked == 0);
}

#[test]
fn block_compressed_profiles_refuse_partial_blocks_but_the_portable_product_publishes() {
    let fixture = Fixture::new();
    demo::write_checker_png(&fixture.source().join("textures/odd.png"), 30, [9, 9, 9])
        .expect("odd size");
    let mut config = fixture.config();
    config.profiles = vec![Profile::DesktopBc, Profile::AppleAstc];
    let mut cooker = Cooker::open(config).expect("open");
    let report = cooker.cook_all().expect("cook");
    assert_eq!(
        report
            .failed
            .iter()
            .filter(|(name, _)| name == "textures/odd")
            .count(),
        1,
        "{:?}",
        report.failed
    );
    let manifest = manifest_of(&fixture.store());
    assert!(
        manifest
            .resolve("textures/odd", &[Profile::Portable])
            .is_some(),
        "portable is always cooked"
    );
    assert!(
        manifest
            .resolve("textures/odd", &[Profile::DesktopBc, Profile::AppleAstc])
            .is_none()
    );
}

#[test]
fn a_running_reader_follows_a_store_that_is_wiped_and_recreated() {
    let fixture = Fixture::new();
    let mut cooker = Cooker::open(fixture.config()).expect("open");
    cooker.cook_all().expect("cook");
    let path = fixture.source().join("textures/checker.png");
    demo::write_checker_png(&path, 64, [40, 40, 200]).expect("edit");
    cooker
        .cook_changed(std::slice::from_ref(&path))
        .expect("cook");
    let mut watcher = HeadWatcher::new(fixture.store());
    assert_eq!(watcher.poll().expect("poll").expect("head").generation, 2);
    drop(cooker);

    make_writable(&fixture.root.join("store"));
    std::fs::remove_dir_all(fixture.root.join("store")).expect("wipe store");
    let mut cooker = Cooker::open(fixture.config()).expect("recreate");
    let head = cooker.cook_all().expect("cook").published.expect("publish");
    assert_eq!(head.generation, 1);
    assert_eq!(
        watcher.poll().expect("poll"),
        Some(head),
        "a new store epoch is a new publication"
    );
    assert_eq!(watcher.poll().expect("poll"), None);
}
