//! Fast projectile interaction through the reference game's real ticks.
use crowd_2d::{Simulation, run_trace};

#[test]
fn crossing_whole_enemy_hits_while_endpoint_misses() {
    let mut simulation = Simulation::mixed(2000, 1);
    let result = run_trace(&mut simulation, 240, true, |_| 0);
    // The witness is counted at the actual hit: endpoint predicate is false
    // for that same enemy and displacement exceeds its whole diameter.
    assert!(result.swept_only_hits > 0);
    assert!(result.hits >= result.swept_only_hits);
}
