//! Runtime proof through the public game and water interfaces.
#[cfg(target_os = "macos")]
mod support;
use thrum_render::runtime::FrameWorld;
thrum_test_support::macos_gpu_test! {
fn scene_ticks_and_declares_one_water_contribution() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "scene_ticks_and_declares_one_water_contribution",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::new(&test_gpu, [384, 216]);
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
        21
    );
    let witness = fixture.witness();
    let water = witness.iter().filter(|p| p[0] > 0.1 && p[1] > 0.0).count();
    assert!(water > witness.len() / 4, "surface coverage {water}");
}
}
