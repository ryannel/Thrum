//! EFFECT-GRAPH M2 target script, using the external game's public diagnostic.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/effect.rs"]
mod support;
#[test]
fn effect_state_script_matches_literal_table() {
    support::run("state");
}
