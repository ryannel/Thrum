use std::path::{Path, PathBuf};
use thrum_asset::{AssetStore, ProductBytes, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};

pub(super) fn root(path: &Path) -> PathBuf {
    let folder = path.join("source/actors");
    std::fs::create_dir_all(&folder).expect("sheet folder");
    folder
}

pub(super) fn cook(path: &Path, store: &str) -> Cooker {
    Cooker::open(CookConfig {
        source: path.join("source"),
        store: path.join(store),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker")
}

pub(super) fn sheet_bytes(store: &AssetStore, profile: Profile) -> ProductBytes {
    let head = store
        .read_head()
        .expect("read HEAD")
        .expect("published sheet");
    let manifest = store.load_manifest(&head).expect("manifest");
    assert_eq!(manifest.entries().count(), 3);
    let entry = manifest.resolve("actors", &[profile]).expect("sheet name");
    assert_eq!(entry.kind, thrum_asset::ProductKind::SpriteSheet);
    store.map(&entry.product).expect("cooked sheet bytes")
}
