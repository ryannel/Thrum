//! Submitted island sand retains a damp band after the tide falls.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn wet_shore_follows_the_tide() {
    use water_scene::island::{Wind, build_wet_shore};
    let name = "wet_shore_follows_the_tide";
    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let build = || {
            let (game, report, _, _) = build_wet_shore(Wind::Calm, false).expect("wet shore");
            let mut f = support::Fixture::from_game(&gpu, [256, 144], game, report);
            f.parameters(r#"{"camera.island":0,"sea.stage":0}"#);
            f.warm_cooked();
            f
        };
        let mut control = build();
        let mut ebb = build();
        control.parameters(r#"{"sea.stage":-0.5}"#);
        ebb.parameters(r#"{"sea.stage":0.5}"#);
        for i in 1..=4 {
            let t = 2.0 + f64::from(i) / 60.0;
            control.frame_without_timings(t);
            ebb.frame_without_timings(t);
        }
        ebb.parameters(r#"{"sea.stage":-0.5}"#);
        for i in 5..=12 {
            let t = 2.0 + f64::from(i) / 60.0;
            ebb.frame_without_timings(t);
            control.frame_without_timings(t);
        }
        let dry = control.image();
        let damp = ebb.image();
        let darker = damp.as_chunks::<4>().0.iter().zip(dry.as_chunks::<4>().0.iter())
            .filter(|(a,b)| a[0] < b[0] && a[1] < b[1] && a[2] < b[2]).count();
        assert!(darker >= 100, "falling tide left {darker} darker sand pixels");
        assert!(ebb.host.scene_shader_source().expect("forward source").contains("fn wet_shore("));
    });
}
}
