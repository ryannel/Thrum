//! Submitted depth, motion and reset behavior against the independent f64 model.
#[cfg(target_os = "macos")]
use crate::support;
use thrum_render::{contribution::SceneContributions, synthetic::Reconstruction};
thrum_test_support::macos_gpu_test! {
#[expect(
    clippy::cast_possible_truncation,
    reason = "the independent f64 oracle is projected into the renderer's bounded f32 camera"
)]
fn tick_and_repeat_motion_follow_the_cpu_surface() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "tick_and_repeat_motion_follow_the_cpu_surface",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::temporal(&test_gpu, [384, 216]);
    let _map = fixture.signal(0.45);
    assert_eq!(fixture.host.reconstruction(), Reconstruction::Taau);
    let cpu = thrum_sea_model::LongWaves::new(vec![support::component(0.45)], 0.1).expect("oracle");
    let matrix = support::matrix(fixture.extent, 2.0);
    let mut previous = 0.2 - 1.0 / 60.0;
    fixture.frame(0.2);
    assert!(
        fixture
            .witness()
            .iter()
            .any(|p| p[2] > 65000.0 && p[0] > 0.1)
    );
    let creations = fixture.host.pipeline_creation_count();
    for seconds in [0.208_333_333, 0.216_666_666, 0.225] {
        let sample = fixture.frame(seconds);
        let time = (sample.tick as f64 - 1.0 + f64::from(sample.presentation_alpha)) / 60.0;
        let witness = fixture.witness();
        for [x, y] in [[192, 160], [150, 190], [230, 210]] {
            let p = witness[(y * fixture.extent[0] + x) as usize];
            assert!(
                p[0] > 0.1 && p[1] > 0.0 && p[2].abs() < 100.0,
                "frame inputs {p:?}"
            );
            let world = support::world_at(matrix, fixture.extent, [x, y], p[1]);
            let expected = cpu
                .evaluate([f64::from(world.x), f64::from(world.z)], time)
                .expect("surface");
            assert!(
                (f64::from(world.y) - expected.height).abs() < 0.035,
                "height {world:?} vs {}",
                expected.height
            );
            let velocity = glam::Vec3::from_array(expected.velocity.map(|v| v as f32));
            let current = matrix.project_point3(world);
            let old = matrix.project_point3(world - velocity * (time - previous) as f32);
            let motion = (old - current)
                * glam::vec3(
                    fixture.extent[0] as f32 * 0.5,
                    -(fixture.extent[1] as f32) * 0.5,
                    1.0,
                );
            assert!(
                (p[2] - motion.x).abs() < 0.12 && (p[3] - motion.y).abs() < 0.12,
                "motion {p:?} vs {motion:?}"
            );
        }
        previous = time;
    }
    assert_eq!(
        fixture.host.pipeline_creation_count(),
        creations,
        "no pipelines during frames"
    );
    fixture.frame(0.5);
    assert!(
        fixture
            .witness()
            .iter()
            .any(|p| p[2] > 65000.0 && p[0] > 0.1),
        "multi-tick reset"
    );
    let _map = fixture.signal(0.45);
    fixture.frame(0.508_333_333);
    assert!(
        fixture
            .witness()
            .iter()
            .any(|p| p[2] > 65000.0 && p[0] > 0.1),
        "rebuild reset"
    );
    fixture
        .host
        .set_scene_contribution(Box::<SceneContributions>::default())
        .expect("absent");
    let before = fixture.host.storage_report();
    let names: Vec<_> = fixture
        .host
        .scene_graph()
        .expect("absent graph")
        .pass_names()
        .map(str::to_owned)
        .collect();
    assert!(
        !names
            .iter()
            .any(|n| n.contains("water") || n.contains("sea "))
    );
    fixture.frame(0.52);
    assert_eq!(
        before,
        fixture.host.storage_report(),
        "absent graph storage stays unchanged"
    );
}
}
