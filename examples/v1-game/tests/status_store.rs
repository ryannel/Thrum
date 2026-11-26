use std::path::{Path, PathBuf};

use thrum_asset::Profile;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};

pub(crate) fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/game-ui/font-source")
}

pub(crate) fn cook(source: PathBuf, store: PathBuf) -> Cooker {
    let mut cooker = Cooker::open(CookConfig {
        source,
        store,
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open cooker");
    assert_eq!(cooker.cook_all().expect("cook status products").failed, []);
    cooker
}
