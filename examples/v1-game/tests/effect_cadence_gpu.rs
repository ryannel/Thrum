//! Cadence changes use the same compiled graph and prebuilt pipelines.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/effect.rs"]
mod support;
#[test]
fn both_history_consumers_follow_runtime_cadence_and_discard() {
    support::run("cadence");
}
