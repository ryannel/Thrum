//! End-to-end check of the Wasm dev-server client against a running `thrum-cook watch --serve`.
//!
//! Driven by `scripts/check_g5_wasm_dev_client.sh`. Run under `wasm-bindgen-test-runner` (Node's global `fetch`) with `THRUM_G5_DEV_SERVER` set at
//! compile time to the served origin. The driver edits one source after the first load; the test
//! then long-polls for that publication. Without the variable the test reports a skip.

#![cfg(target_arch = "wasm32")]

use std::collections::HashSet;

use thrum_asset::{BrowserFetch, DevClient, Ktx2, MeshProduct, ProductKind};
use wasm_bindgen_test::{console_log, wasm_bindgen_test};

#[wasm_bindgen_test]
async fn loads_a_publication_then_long_polls_the_next_one() {
    let Some(origin) = option_env!("THRUM_G5_DEV_SERVER") else {
        console_log!("skipped: THRUM_G5_DEV_SERVER was not set at build time");
        return;
    };
    let mut client = DevClient::new(BrowserFetch, origin).expect("origin");
    let first = loop {
        if let Some(generation) = client.poll().await.expect("first poll") {
            break generation;
        }
    };
    let mut opened = HashSet::new();
    for entry in first.manifest().entries() {
        let Some(bytes) = first.product(&entry.product) else {
            continue;
        };
        // Products are named by content, so profiles with identical bytes share one.
        if !opened.insert(entry.product) {
            continue;
        }
        assert_eq!(bytes.as_ptr().align_offset(64), 0);
        match entry.kind {
            ProductKind::Mesh => {
                MeshProduct::open(bytes)
                    .and_then(|mesh| mesh.validate_references())
                    .expect("mesh");
            }
            ProductKind::Texture | ProductKind::Volume => {
                Ktx2::open(bytes).expect("ktx2");
            }
        }
    }
    let opened = opened.len();
    assert_eq!(opened, 3, "one portable product per demo asset");
    console_log!(
        "E2E_READY generation {} opened {opened}",
        first.head().generation
    );
    let second = loop {
        if let Some(generation) = client.poll().await.expect("long poll") {
            break generation;
        }
    };
    assert!(second.head().generation > first.head().generation);
    let mut seen = HashSet::new();
    let mut shared = 0;
    let mut replaced = 0;
    for entry in second.manifest().entries() {
        let Some(bytes) = second.product(&entry.product) else {
            continue;
        };
        if !seen.insert(entry.product) {
            continue;
        }
        match first.product(&entry.product) {
            Some(old) if old.as_ptr() == bytes.as_ptr() => shared += 1,
            _ => replaced += 1,
        }
    }
    console_log!(
        "E2E_DONE generation {} shared {shared} replaced {replaced}",
        second.head().generation
    );
    assert_eq!(
        (shared, replaced),
        (2, 1),
        "only the edited volume is fetched"
    );
}
