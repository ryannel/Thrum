//! Fast projectile interaction through the reference game's real ticks.
use crowd_2d::{Simulation, run_trace};
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, Shape};

#[test]
fn crossing_whole_enemy_hits_while_endpoint_misses() {
    // A known straight shot crosses the complete diameter [-1, 1].
    let point = |x| Vector {
        x: N::from_i32(x),
        y: N::ZERO,
    };
    let mut index = CircleIndex::new(N::from_i32(4)).expect("cell size");
    index
        .build(&[Circle {
            slot: 42,
            centre: point(0),
            radius: N::ONE,
        }])
        .expect("enemy");
    let radius = N::ONE / N::from_i32(6);
    let mut swept = Vec::new();
    index
        .query(
            Shape::Capsule {
                start: point(-3),
                end: point(3),
                radius,
            },
            |enemy| swept.push(enemy.slot),
        )
        .expect("shot");
    let mut endpoint = Vec::new();
    index
        .overlap(point(3), radius, |enemy| endpoint.push(enemy.slot))
        .expect("endpoint");
    assert_eq!(swept, [42]);
    assert_eq!(endpoint, []);
    let mut simulation = Simulation::mixed(2000, 1);
    let result = run_trace(&mut simulation, 240, true, |_| 0);
    // The witness is counted at the actual hit: endpoint predicate is false
    // for that same enemy and displacement exceeds its whole diameter.
    assert!(result.swept_only_hits > 0);
    assert!(result.hits >= result.swept_only_hits);
}
