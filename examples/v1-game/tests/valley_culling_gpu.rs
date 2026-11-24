//! Explicit real-target proof; the launcher supplies a freshly cooked store.
#![cfg(all(feature = "proving", not(target_arch = "wasm32")))]
mod valley_route_support {
    mod valley_route;
    pub(super) use valley_route::run;
}

#[test]
#[ignore = "explicit cooked valley route; run scripts/run_v1_showcase.py culling"]
fn every_submitted_valley_frame_equals_direct() {
    valley_route_support::run("every_submitted_valley_frame_equals_direct");
}
