//! Runtime proof through the public game and water interfaces.
mod support;
use thrum_render::runtime::FrameWorld;
#[test]
fn scene_ticks_and_declares_one_water_contribution() {
    let mut fixture = support::Fixture::new([384, 216]);
    let sample = fixture.frame(0.1);
    assert_eq!(fixture.host.world().source().tick, 5);
    assert_eq!(sample.tick, 5);
    let graph = fixture.host.scene_graph().expect("graph");
    let names: Vec<_> = graph.pass_names().collect();
    assert_eq!(
        names.iter().filter(|n| n.contains("water surface")).count(),
        1
    );
    assert_eq!(
        names
            .iter()
            .filter(|n| n.starts_with("effect ") && !n.starts_with("effect after"))
            .count(),
        13
    );
    let witness = fixture.witness();
    let water = witness.iter().filter(|p| p[0] > 0.1 && p[1] > 0.0).count();
    assert!(water > witness.len() / 4, "surface coverage {water}");
}
