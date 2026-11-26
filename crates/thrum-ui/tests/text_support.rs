use std::sync::OnceLock;

use thrum_asset::{FontProduct, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_test_support::Scratch;

static FONTS: OnceLock<[FontProduct; 2]> = OnceLock::new();

pub(crate) fn font(name: &str) -> &'static FontProduct {
    let fonts = FONTS.get_or_init(|| {
        let scratch = Scratch::new("text-runs");
        let mut cooker = Cooker::open(CookConfig {
            source: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/game-ui/font-source"),
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
        ["type", "pixel"].map(|name| {
            let entry = manifest.resolve(name, &[Profile::Portable]).expect("font");
            FontProduct::open(store.map(&entry.product).expect("map")).expect("font")
        })
    });
    &fonts[usize::from(name == "pixel")]
}
