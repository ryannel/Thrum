//! Test the water's direct Sun separately from its sky illumination on Metal.
mod support;

#[cfg(target_os = "macos")]
use support::optics::{EXTENT, Rig, attach, camera, sun, water};
#[cfg(target_os = "macos")]
use thrum_render::atmosphere::SunOpticalDepth;

#[cfg(target_os = "macos")]
fn capture(gpu: &thrum_test_support::Gpu, tau: Option<f32>, optics: bool) -> Vec<[f32; 3]> {
    let provider = tau.map(|tau| {
        SunOpticalDepth::new(
            [2, 2],
            &[tau; 4],
            50.0,
            [-5000.0, -5000.0, 5000.0, 5000.0],
            0.01,
            1,
        )
        .expect("uniform stand-in optical depth")
    });
    let view = camera([0.0, 2.0, 0.0], [0.0, 0.1, 5.0], 1.15, 1.0 / 16384.0);
    let mut rig = Rig::new(gpu, &view, sun(2.0, 0.000_01), &[], provider);
    attach(&mut rig, water(None, optics));
    rig.scene_colour().1
}

thrum_test_support::macos_gpu_test! {
fn cloud_factor_is_linear_in_direct_sun_and_keeps_sky_light() {
    let Some(gpu) = thrum_test_support::gpu("water cloud factor", Some(wgpu::Backend::Metal)) else { return };
    let absent = capture(&gpu, None, true);
    let clear = capture(&gpu, Some(0.0), true);
    let half = capture(&gpu, Some(std::f32::consts::LN_2), true);
    let shade = capture(&gpu, Some(16.0), true);
    // Exclude the far horizon beyond 32 km, where the cloud-capable sky uses
    // the scene's extended aerial path. At fixed scale all terms are linear.
    let start = (EXTENT[0] * (EXTENT[1] * 3/4)) as usize;
    assert_eq!(&absent[start..], &clear[start..]);
    let mut affected = 0;
    for ((clear, half), shade) in clear[start..].iter().zip(&half[start..]).zip(&shade[start..]) {
        for ((&clear, &half), &shade) in clear.iter().zip(half).zip(shade) {
            if clear - shade > 0.0003 {
                let actual = (half-shade)/(clear-shade);
                assert!((actual-0.5).abs() < 0.03, "attenuation must be applied once: {clear} {half} {shade}, fraction {actual}");
                assert!(shade > 0.0, "indirect sky light remains under shade");
                affected += 1;
            }
        }
    }
    assert!(affected > 100, "direct sunlight must contribute in the inspected region");
}
}
