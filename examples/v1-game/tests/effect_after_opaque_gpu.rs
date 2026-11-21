//! Submitted late-stage qualification on both native recipes.
#![cfg(not(target_arch = "wasm32"))]
/// Shared native diagnostic device policy.
use crate::support;
#[test]
fn submitted_slab_depth_refraction_motion_and_resets() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_slab_depth_refraction_motion_and_resets",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        for recipe in ["portable", "apple-tile"] {
            let (instance, device, queue, caps) =
                support::device(&test_gpu.fresh_adapter(), recipe, false);
            let record = Box::pin(v1_game::effect::after_opaque::run(
                &device, &queue, &caps, &instance,
            ))
            .await
            .expect("late pass qualification");
            let output =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m3");
            std::fs::create_dir_all(&output).expect("output");
            std::fs::write(output.join(format!("after-opaque-{recipe}.txt")), record)
                .expect("target record");
        }
    });
}
