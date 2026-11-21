//! Cook the same authored inputs through all three product profiles.

use thrum_asset::{AssetStore, Profile};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

// Recorded from the original per-profile cooker on the Mac.
const PRODUCTS: [(&str, [&str; 3]); 2] = [
    (
        "checker",
        [
            "1ae1e0ed7166050c01427e21adecd9b82fa937cbf4e4d611e7a15821eb96ff21",
            "27b9e0ddb88dc0fd3ebb94b6253920bf52b1536c49732745ce1ce5f31c947098",
            "5f315a4b87ccbb4cbd6681fe549c51fcbc182e2e48a0e4090e60c179f1d2a425",
        ],
    ),
    (
        "cloud",
        [
            "0e7411a643e5f00a29832c205a495d7605cf11c05445253af9333f4670353bd0",
            "8281e237d2c50ce0ceec4fb94c2975426c8107407d37f54af345200a4665c722",
            "0e7411a643e5f00a29832c205a495d7605cf11c05445253af9333f4670353bd0",
        ],
    ),
];

#[test]
fn products_keep_their_content_addresses() {
    let root = thrum_test_support::Scratch::new("decode-once");
    let source = root.join("source");
    let store = root.join("store");
    std::fs::create_dir_all(&source).unwrap();
    demo::write_checker_png(&source.join("checker.png"), 16, [200, 40, 40]).unwrap();
    demo::write_cloud_volume(&source.join("cloud.tvol"), 12, 1).unwrap();
    let mut config = CookConfig {
        source,
        store: store.clone(),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    let portable = Cooker::open(config.clone()).unwrap().cook_all().unwrap();
    assert_eq!(portable.cooked, 2);
    assert_eq!(
        (portable.texture_preparations, portable.volume_preparations),
        (1, 1)
    );
    config.profiles = Profile::ALL.to_vec();
    // Open a fresh store so all three profiles miss for both sources.
    config.store = root.join("all-profiles");
    let mut cooker = Cooker::open(config.clone()).unwrap();
    let report = cooker.cook_all().unwrap();
    assert_eq!(report.cooked, 6);
    assert_eq!(
        (report.texture_preparations, report.volume_preparations),
        (1, 1)
    );
    assert!(report.failed.is_empty() && report.refused.is_empty());
    println!("cold cook: 1 texture preparation and 1 volume preparation for 6 products");
    let assets = AssetStore::new(config.store);
    let manifest = assets.load_manifest(&report.published.unwrap()).unwrap();
    for (name, addresses) in PRODUCTS {
        for (profile, expected) in [Profile::Portable, Profile::DesktopBc, Profile::AppleAstc]
            .into_iter()
            .zip(addresses)
        {
            let entry = manifest.resolve(name, &[profile]).unwrap();
            assert_eq!(entry.profile, profile);
            println!("{name} {} {}", profile.name(), entry.product);
            assert_eq!(entry.product.to_hex(), expected);
        }
    }
    let cached = cooker.cook_all().unwrap();
    assert_eq!(cached.reused, 6);
    assert_eq!(
        (cached.texture_preparations, cached.volume_preparations),
        (0, 0)
    );
}
