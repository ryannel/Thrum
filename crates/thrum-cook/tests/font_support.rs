use std::path::{Path, PathBuf};

use thrum_asset::{AssetStore, ProductBytes, ProductKind, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};

pub(super) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/game-ui/font-source")
        .join(name)
}

pub(super) fn copy(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("source directory");
    for entry in std::fs::read_dir(source).expect("font source") {
        let entry = entry.expect("font file");
        std::fs::copy(entry.path(), destination.join(entry.file_name())).expect("copy font source");
    }
}

pub(super) fn cooker(source: PathBuf, store: PathBuf) -> Cooker {
    Cooker::open(CookConfig {
        source,
        store,
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open cooker")
}

pub(super) fn product(store: &AssetStore, name: &str) -> ProductBytes {
    let head = store.read_head().expect("HEAD").expect("publication");
    let manifest = store.load_manifest(&head).expect("manifest");
    let entry = manifest
        .resolve(name, &[Profile::Portable])
        .expect("font name");
    assert_eq!(entry.kind, ProductKind::Font);
    store.map(&entry.product).expect("font product")
}
