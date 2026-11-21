//! A live stage changes submitted geometry and scene depth together.
#[cfg(target_os = "macos")]
use crate::support;
thrum_test_support::macos_gpu_test! {
fn stage_raises_the_surface_by_two_metres() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "stage_raises_the_surface_by_two_metres",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let mut fixture = support::Fixture::new(&test_gpu, [384, 216]);
    fixture.parameters(r#"{"camera.height":10}"#);
    let matrix = support::matrix(fixture.extent, 10.0);
    for amplitude in [0.0, 0.45] {
        let map = fixture.signal(amplitude);
        let cpu = thrum_sea_model::LongWaves::new(vec![support::component(amplitude)], 0.1)
            .expect("oracle");
        let sample = fixture.frame(0.2);
        let time = (sample.tick as f64 - 1.0 + f64::from(sample.presentation_alpha)) / 60.0;
        let before = fixture.witness();
        map.lock().expect("map").set_stage(0, 2.0).expect("stage");
        fixture.frame(0.2);
        let after = fixture.witness();
        for y in [150, 180, 210] {
            for x in [80, 192, 300] {
                let at = (y * fixture.extent[0] + x) as usize;
                let a = support::world_at(matrix, fixture.extent, [x, y], before[at][1]);
                let b = support::world_at(matrix, fixture.extent, [x, y], after[at][1]);
                let first = cpu
                    .evaluate([f64::from(a.x), f64::from(a.z)], time)
                    .expect("before wave");
                let raised = cpu
                    .evaluate([f64::from(b.x), f64::from(b.z)], time)
                    .expect("after wave");
                assert!(
                    (f64::from(a.y) - first.height).abs() < 0.035,
                    "initial {a:?}"
                );
                assert!(
                    (f64::from(b.y) - raised.height - 2.0).abs() < 0.035,
                    "stage {b:?}"
                );
            }
        }
    }
}
}
