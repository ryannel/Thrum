#[cfg(target_os = "macos")]
use crate::ray_shadow_support::terminator::{Sheet, scene};
#[cfg(target_os = "macos")]
use crate::{ray_shadow_support, ray_support::support};
#[cfg(target_os = "macos")]
use glam::Vec3;
#[cfg(target_os = "macos")]
use thrum_render::synthetic::SunShadowMethod;

thrum_test_support::macos_gpu_test! {
fn terminator_excuses_only_the_first_crossing() {
    let Some(gpu) = support::gate("forward Sun terminator and joined sheet") else {
        return;
    };
    let (device, queue, caps) = support::device(&gpu, "ray-query", 1);
    let receiver = Vec3::new(15.0_f32.to_radians().cos(), 0.23, 0.0);
    let (y, x) = 94.0_f32.to_radians().sin_cos();
    let sun = Vec3::new(x, y, 0.0);
    let lighting = ray_shadow_support::sun(sun, 8_000.0);
    // The triangle faces +X, but the radial normal faces the Sun. The far
    // face is about 34 mm ahead. Both sheets follow it and bracket the reach.
    let clear = receiver + Vec3::Z * 0.28;
    let distances = [0.25, 3.0];
    let cases = [false, true].into_iter().flat_map(|joined| {
        [false, true].into_iter().flat_map(move |reverse| {
            distances.map(move |distance| {
                let sheet = Sheet {
                    distance,
                    joined,
                    reverse,
                };
                (false, Some(sheet), false)
            })
        })
    });
    // With the far face removed, the joined back-face sheet becomes the
    // first hit. The passage excuses it only at the near distance. This
    // control makes a change in reach visible in the same forward operation.
    let reach_controls = distances.into_iter().zip([true, false]).map(|(distance, lit)| {
        let sheet = Sheet {
            distance,
            joined: true,
            reverse: true,
        };
        (true, Some(sheet), lit)
    });
    for (open_rim, sheet, lit) in std::iter::once((false, None, true))
        .chain(cases)
        .chain(reach_controls)
    {
        let scene = scene(receiver, sun, sheet, open_rim);
        let (prepared, target) =
            ray_shadow_support::prepare(&device, &caps, &queue, &scene, lighting);
        assert_eq!(prepared.shadow_selection().sun, SunShadowMethod::Traced);
        let image = support::submit_bytes(&device, &queue, &prepared, &target);
        let middle = ray_shadow_support::sample(&scene.view, &image, receiver);
        let beside = ray_shadow_support::sample(&scene.view, &image, clear);
        let case = format!("open rim {open_rim}, sheet {sheet:?}");
        assert!(beside[0] > 32, "{case}: beside {beside:?}");
        if lit {
            assert!(middle[0] > 32, "{case}: expected lit middle, got {middle:?}");
        } else {
            assert!(middle[0] <= 2, "{case}: expected dark middle, got {middle:?}");
        }
        println!("{case}: middle {middle:?}, beside {beside:?}");
    }
}
}
