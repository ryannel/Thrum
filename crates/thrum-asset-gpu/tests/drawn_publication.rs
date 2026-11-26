//! The publication identity follows the submitted draw on Metal.

use std::{
    fs,
    future::{Future, ready},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use super::static_scene::{Source, World};
use thrum_asset::{
    AssetError, AssetStore, ContentHash, DevClient, Fetch, FetchReply, Head, ProductBytes, Profile,
};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedDrawPath, CookedSource, CookedStats, Drawn, MeshPlacement,
    RemoteFeed, await_drawn,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost},
    synthetic::{SyntheticScene, SyntheticView},
};
use thrum_test_support::{Opened, Restriction, Scratch, target, wait};

fn fixture() -> AssetStore {
    AssetStore::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data"))
}

fn head(store: &AssetStore) -> Head {
    store
        .read_head()
        .expect("read HEAD")
        .expect("published HEAD")
}

fn open(test: &str) -> Option<Opened> {
    thrum_test_support::open(
        test,
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    )
}

fn host(
    device: &Opened,
    source: impl Into<CookedSource>,
) -> (FrameHost<World, Source>, Arc<Mutex<CookedStats>>) {
    let extent = [target::SIZE; 2];
    let view = SyntheticView::perspective(
        [0.4, 1.7, 5.0],
        [0.0, 0.75, 0.0],
        [0.0, 1.0, 0.0],
        0.92,
        1.0,
        0.1,
        30.0,
    )
    .expect("camera");
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.view = view;
    scene.instances[0].deactivate();
    let config = FrameConfig {
        render_extent: extent,
        surface: Some(SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent,
        }),
    };
    let mut host = pollster::block_on(FrameHost::new(
        &device.capabilities,
        &device.queue,
        config,
        World,
        Source(view),
        scene,
        1.0,
    ))
    .expect("host");
    let mut config = CookedConfig::new(
        source,
        vec![MeshPlacement::new("meshes/rock", "textures/white").on_ground()],
    );
    config.draw_path = CookedDrawPath::Direct;
    let assets = CookedAssets::new(config).expect("assets");
    let stats = assets.stats();
    host.set_scene_contribution(Box::new(assets))
        .expect("attach");
    (host, stats)
}

fn assert_identity(stats: &CookedStats, expected: Option<Head>) {
    assert_eq!(
        (stats.drawn, stats.drawn_manifest_hash),
        (
            expected.map(|head| head.generation),
            expected.map(|head| head.manifest)
        )
    );
}

fn assert_drawn(stats: &Mutex<CookedStats>, expected: Head) {
    let stats = stats.lock().expect("stats");
    assert_identity(&stats, Some(expected));
    assert_eq!(stats.indexed_direct_draws, 1, "one cooked placement drawn");
}

#[test]
fn stores_report_their_own_roots_and_a_reload_switches_on_submit() {
    let Some(device) = open("drawn store publication") else {
        return;
    };
    let first_store = fixture();
    let first = head(&first_store);
    let (mut first_host, stats) = host(&device, first_store.root().to_path_buf());
    assert_identity(&stats.lock().expect("stats"), None);
    await_drawn(&mut first_host, &stats, Drawn::Generation(1)).expect("first store");
    assert_drawn(&stats, first);
    drop(first_host);

    let scratch = Scratch::new("drawn-publication");
    let source = scratch.join("source");
    fs::create_dir_all(source.join("meshes")).expect("meshes");
    fs::create_dir_all(source.join("textures")).expect("textures");
    demo::write_grid_gltf(&source.join("meshes"), "rock", 1, 0.0).expect("mesh");
    let texture = source.join("textures/white.png");
    demo::write_checker_png(&texture, 1, [200, 40, 20]).expect("texture");
    let store = AssetStore::new(scratch.join("store"));
    let mut cooker = Cooker::open(CookConfig {
        source,
        store: store.root().to_path_buf(),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    assert_eq!(cooker.cook_all().expect("first cook").failed, []);
    let original = head(&store);
    assert_eq!(original.generation, 1);
    assert_ne!(original.manifest, first.manifest);
    let (mut host, stats) = host(&device, store.root().to_path_buf());
    await_drawn(&mut host, &stats, Drawn::Generation(1)).expect("second store");
    assert_drawn(&stats, original);

    demo::write_checker_png(&texture, 1, [20, 100, 200]).expect("edit");
    assert_eq!(cooker.cook_all().expect("recook").failed, []);
    let edited = head(&store);
    assert_eq!(edited.generation, 2);
    assert_ne!(edited.manifest, original.manifest);
    assert_drawn(&stats, original);

    let mut clock = Duration::ZERO;
    let mut discarded = false;
    wait::until(
        "edited publication submitted",
        Duration::from_secs(5),
        || {
            clock += Duration::from_millis(16);
            let advanced = host.advance(clock).expect("advance");
            if !advanced.can_acquire_serial() {
                return None;
            }
            let frame = advanced.encode_scene().expect("encode scene");
            assert!(frame.can_acquire_surface());
            let switch = {
                let stats = stats.lock().expect("stats");
                assert!(stats.errors.is_empty(), "{:?}", stats.errors);
                assert_identity(&stats, Some(original));
                stats.recorded_last_copy == Some(edited.generation)
            };
            if switch && !discarded {
                frame.skip();
                assert_drawn(&stats, original);
                discarded = true;
                return None;
            }
            assert!(frame.submit(None).expect("submit frame").submitted);
            assert_drawn(&stats, if switch { edited } else { original });
            switch.then_some(())
        },
    );
    assert!(discarded, "the first recorded switch must be discarded");
}

// Serve the cooked fixture bytes through the real client and its verification,
// without starting a server or opening a network connection.
struct FixtureFetch(AssetStore);

impl Fetch for FixtureFetch {
    fn get(&self, url: &str) -> impl Future<Output = Result<FetchReply, AssetError>> + use<> {
        let path = if let Some(hash) = url.split_once("/cas/").map(|(_, hash)| hash) {
            let hash = ContentHash::from_hex(hash).expect("content address");
            self.0.product_path(&hash)
        } else {
            assert!(url.ends_with("/head"), "unexpected URL {url}");
            self.0.head_path()
        };
        let bytes = fs::read(path).expect("fixture bytes");
        ready(Ok(FetchReply {
            status: 200,
            body: ProductBytes::copy_from(&bytes),
        }))
    }
}

#[test]
fn remote_feed_reports_the_head_its_drawn_set_was_fetched_with() {
    let Some(device) = open("drawn remote publication") else {
        return;
    };
    let store = fixture();
    let expected = head(&store);
    let mut client = DevClient::new(FixtureFetch(store), "https://fixture.test").expect("client");
    let fetched = pollster::block_on(client.poll())
        .expect("verified fetch")
        .expect("fetched publication");
    assert_eq!(fetched.head(), expected);
    let (mut host, stats) = host(&device, CookedSource::Remote(RemoteFeed::new(fetched)));
    assert_identity(&stats.lock().expect("stats"), None);
    await_drawn(&mut host, &stats, Drawn::Generation(expected.generation)).expect("remote draw");
    assert_drawn(&stats, expected);
}
