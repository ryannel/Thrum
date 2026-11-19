//! Short workspace proof using the real reference rule set and conditioned links.

#[test]
fn a_match_keeps_stepping_through_join_reconnect_repair_and_a_stalled_link() {
    match_proof::run_fault("short", 160).expect("each independent replica caught up");
}
