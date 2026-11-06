//! Dev-server protocol and watcher-driven publication over a real store.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::{AssetStore, ContentHash, Head, Profile};
use thrum_cook::{CookConfig, Cooker, DevServer, Durability, Publications, Quality, demo, watch};

static NEXT: AtomicU32 = AtomicU32::new(0);

fn fixture() -> (PathBuf, CookConfig) {
    let root = std::env::temp_dir().join(format!(
        "thrum-g5-serve-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let source = root.join("source");
    std::fs::create_dir_all(source.join("textures")).expect("create");
    demo::write_checker_png(&source.join("textures/checker.png"), 32, [200, 40, 40]).expect("png");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 8, 1).expect("volume");
    let config = CookConfig {
        source,
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    (root, config)
}

fn get(address: std::net::SocketAddr, target: &str) -> (u16, Vec<u8>, String) {
    get_with(address, target, "Host: localhost\r\n")
}

fn get_with(address: std::net::SocketAddr, target: &str, headers: &str) -> (u16, Vec<u8>, String) {
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("timeout");
    write!(stream, "GET {target} HTTP/1.1\r\n{headers}\r\n").expect("request");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("response");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("header end");
    let header = String::from_utf8_lossy(&response[..split]).into_owned();
    let status = header
        .split_ascii_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status");
    (status, response[split + 4..].to_vec(), header)
}

#[test]
fn the_dev_server_serves_head_verified_products_and_long_polls() {
    let (root, config) = fixture();
    let store = AssetStore::new(&config.store);
    let publications = Arc::new(Publications::default());
    let server =
        DevServer::start("127.0.0.1:0", store.clone(), Arc::clone(&publications)).expect("bind");
    assert_eq!(
        get(server.address(), "/head").0,
        204,
        "nothing published yet"
    );

    let mut cooker = Cooker::open(config.clone()).expect("open");
    cooker.cook_all().expect("cook");
    publications.announce(cooker.generation());
    let (status, body, _) = get(server.address(), "/head");
    assert_eq!(status, 200);
    let head = Head::decode(&body).expect("head");
    assert_eq!(head.generation, 1);
    let (status, manifest, header) = get(
        server.address(),
        &format!("/cas/{}", head.manifest.to_hex()),
    );
    assert_eq!(status, 200);
    assert!(
        head.manifest.verify(&manifest).is_ok(),
        "client verifies served bytes"
    );
    assert!(header.contains("immutable"));

    for target in [
        "/cas/../HEAD",
        "/cas/ABCDEF",
        &format!("/cas/{}", ContentHash::of(b"absent").to_hex()),
        "/HEAD",
        "/head?after=x",
    ] {
        assert_eq!(get(server.address(), target).0, 404, "{target}");
    }

    // A long poll returns as soon as a later generation is published.
    let address = server.address();
    let waiter = std::thread::spawn(move || {
        let start = Instant::now();
        let (status, body, _) = get(address, "/head?after=1");
        (
            status,
            Head::decode(&body).expect("head").generation,
            start.elapsed(),
        )
    });
    std::thread::sleep(Duration::from_millis(100));
    demo::write_checker_png(
        &config.source.join("textures/checker.png"),
        32,
        [40, 40, 200],
    )
    .expect("edit");
    let report = cooker
        .cook_changed(&[PathBuf::from("textures/checker.png")])
        .expect("cook");
    publications.announce(report.published.expect("publish").generation);
    let (status, generation, waited) = waiter.join().expect("waiter");
    assert_eq!((status, generation), (200, 2));
    assert!(
        waited < Duration::from_secs(10),
        "woken by publication, not timeout"
    );
    drop(server);
    cleanup(&root);
}

#[test]
fn web_pages_cannot_read_the_dev_server() {
    let (root, config) = fixture();
    let store = AssetStore::new(&config.store);
    let server =
        DevServer::start("127.0.0.1:0", store, Arc::new(Publications::default())).expect("bind");
    let address = server.address();
    // DNS rebinding: the page's own host name arrives in Host.
    assert_eq!(
        get_with(address, "/head", "Host: evil.example:80\r\n").0,
        421
    );
    assert_eq!(get_with(address, "/head", "").0, 421, "Host is required");
    // A cross-origin page is refused outright, not merely denied CORS.
    let (status, _, header) = get_with(
        address,
        "/head",
        "Host: localhost\r\nOrigin: https://evil.example\r\n",
    );
    assert_eq!(status, 403);
    assert!(!header.contains("Access-Control-Allow-Origin"));
    // A loopback page (the browser game's dev host) is allowed by name, never by wildcard.
    let (status, _, header) = get_with(
        address,
        "/head",
        &format!(
            "Host: 127.0.0.1:{}\r\nOrigin: http://localhost:5173\r\n",
            address.port()
        ),
    );
    assert_eq!(status, 204);
    assert!(header.contains("Access-Control-Allow-Origin: http://localhost:5173"));
    assert!(!header.contains("Access-Control-Allow-Origin: *"));
    // Native clients send no Origin and get no CORS header.
    let (status, _, header) = get(address, "/head");
    assert_eq!(status, 204);
    assert!(!header.contains("Access-Control"));
    drop(server);
    cleanup(&root);
}

#[test]
fn the_watcher_publishes_an_edited_source() {
    let (root, config) = fixture();
    let source = config.source.clone();
    let store = AssetStore::new(&config.store);
    let stop = Arc::new(AtomicBool::new(false));
    let published = Arc::new(Mutex::new(Vec::new()));
    let thread = {
        let (stop, published, source) = (Arc::clone(&stop), Arc::clone(&published), source.clone());
        std::thread::spawn(move || {
            let mut cooker = Cooker::open(config).expect("open");
            watch(&mut cooker, &source, &stop, |event| {
                if let Ok(report) = &event.report
                    && let Some(head) = report.published
                {
                    published.lock().expect("log").push(head.generation);
                }
            })
            .expect("watch");
        })
    };
    let wait_for = |generation: u64| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !published.lock().expect("log").contains(&generation) {
            assert!(
                Instant::now() < deadline,
                "generation {generation} not published"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    wait_for(1);
    let before = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    let old = before
        .resolve("textures/checker", &[Profile::Portable])
        .expect("entry")
        .product;

    demo::write_checker_png(&source.join("textures/checker.png"), 32, [40, 200, 40]).expect("edit");
    wait_for(2);
    let after = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    assert_ne!(
        after
            .resolve("textures/checker", &[Profile::Portable])
            .expect("entry")
            .product,
        old
    );

    // A new file in a new directory is picked up too.
    std::fs::create_dir_all(source.join("more")).expect("dir");
    demo::write_cloud_volume(&source.join("more/puff.tvol"), 8, 2).expect("new volume");
    wait_for(3);
    let latest = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    assert!(latest.resolve("more/puff", &[Profile::Portable]).is_some());

    stop.store(true, Ordering::Relaxed);
    thread.join().expect("watcher");
    cleanup(&root);
}

fn cleanup(root: &Path) {
    fn writable(path: &Path) {
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
    writable(root);
    std::fs::remove_dir_all(root).ok();
}
