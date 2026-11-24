//! The medium ends at the handover and ordered clear transport carries on.
#[expect(dead_code, reason = "the shared fixture includes sky-pixel readers")]
mod fixture;
mod medium_fixture;

use medium_fixture::{centre_layer, render};

#[test]
fn metal_uses_only_air_beyond_the_provider_ramp() {
    let Some(gpu) = fixture::Gpu::open(8, "medium handover") else {
        return;
    };
    let scene = fixture::scene(false, false, false);
    let light = fixture::daylight(true, 1);
    let air = render(&gpu, &scene, light, None);
    let medium = render(
        &gpu,
        &scene,
        light.with_view_volume_medium(2),
        Some((
            "1.0 - smoothstep(300.0, VOLUME_NEAR_DISTANCE, x)",
            [0.0012, 0.0001, 0.0001],
        )),
    );
    let (near, far) = (40, 46);
    let ratio = centre_layer(&medium, near, false)[0] / centre_layer(&air, near, false)[0];
    let analytic = (-0.0012_f32 * 400.0).exp();
    assert!(
        (ratio - analytic).abs() < 0.006,
        "ramp R={ratio}, reference={analytic}"
    );
    for lane in 0..3 {
        let read = |w, p, s| centre_layer(w, p, s)[lane];
        let expected_transmission = ratio * read(&air, far, false);
        let expected_scatter =
            read(&medium, near, true) + ratio * (read(&air, far, true) - read(&air, near, true));
        assert!(
            (read(&medium, far, false) - expected_transmission).abs() < 0.001,
            "clear continuation T"
        );
        assert!(
            (read(&medium, far, true) - expected_scatter).abs() < 0.003 * expected_scatter,
            "clear continuation S"
        );
    }
    println!("ramp R={ratio}, analytic={analytic}, near={near} far={far}");
}
