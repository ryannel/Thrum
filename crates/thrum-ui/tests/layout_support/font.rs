use std::sync::OnceLock;

use thrum_asset::{FontProduct, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_test_support::Scratch;

pub(super) fn pixel() -> &'static FontProduct {
    static FONT: OnceLock<FontProduct> = OnceLock::new();
    FONT.get_or_init(|| {
        let scratch = Scratch::new("tree-font");
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/game-ui/font-source/pixel");
        let source = scratch.join("source/pixel");
        std::fs::create_dir_all(&source).expect("font source");
        for name in ["grid.png", "font.json"] {
            std::fs::copy(fixture.join(name), source.join(name)).expect("font input");
        }
        let mut cooker = Cooker::open(CookConfig {
            source: scratch.join("source"),
            store: scratch.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker");
        assert_eq!(cooker.cook_all().expect("cook").failed, []);
        let store = cooker.store();
        let head = store.read_head().expect("HEAD").expect("publication");
        let manifest = store.load_manifest(&head).expect("manifest");
        let entry = manifest
            .resolve("pixel", &[Profile::Portable])
            .expect("font");
        FontProduct::open(store.map(&entry.product).expect("map")).expect("font")
    })
}
