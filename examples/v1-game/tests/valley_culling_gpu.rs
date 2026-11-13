//! Explicit real-target proof; the launcher supplies a freshly cooked store.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/valley_route.rs"]
mod valley_route;

#[test]
#[ignore = "explicit cooked valley route; run scripts/run_v1_showcase.py culling"]
fn every_submitted_valley_frame_equals_direct() {
    valley_route::run();
}
