//! An admitted dry provider preserves submitted pixels and the existing water readers.
#[cfg(target_os = "macos")]
mod support;

#[cfg(target_os = "macos")]
use water_scene::island::Wind;

thrum_test_support::macos_gpu_test! {
fn dry_wetness_provider_is_pixel_exact() {
    compare_dry("dry_wetness_provider_is_pixel_exact", Wind::Calm);
}
}

thrum_test_support::macos_gpu_test! {
fn dry_wetness_provider_is_pixel_exact_in_gale() {
    compare_dry("dry_wetness_provider_is_pixel_exact_in_gale", Wind::Gale);
}
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stderr,
    reason = "retain each submitted Metal comparison"
)]
fn compare_dry(name: &str, wind: Wind) {
    use std::time::Duration;
    use water_scene::island::build_rain_ground;

    let Some(gpu) = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal)) else {
        return;
    };
    support::island::with_cooked_island(name, || {
        let make = |enabled| {
            let (game, report, _, _) = build_rain_ground(wind, true, enabled).expect("yard");
            let mut f = support::Fixture::from_game(&gpu, [384, 216], game, report);
            f.parameters(r#"{"camera.island":0,"rain.rate":0,"rain.drying":0,"sun.elevation":30}"#);
            let mut drawn = false;
            thrum_test_support::wait::until(
                "cooked grass texture",
                Duration::from_secs(30),
                || {
                    // Keep the simulation endpoint fixed while streaming both arms.
                    f.frame_without_timings(0.0);
                    match f.game.poll_status() {
                        Some(thrum_app::sky::ContentStatus::Drawn(_)) => drawn = true,
                        Some(thrum_app::sky::ContentStatus::Failed(error)) => panic!("{error}"),
                        _ => {}
                    }
                    if !drawn {
                        return None;
                    }
                    let pixels = f.image();
                    let grass = (60 * 384 + 32) * 4;
                    (pixels[grass + 1].saturating_sub(pixels[grass + 2]) > 15).then_some(())
                },
            );
            f.frame_without_timings(1.0);
            f
        };
        let mut absent = make(false);
        let mut dry = make(true);
        for (fixture, registered) in [(&absent, false), (&dry, true)] {
            assert_eq!(
                fixture
                    .host
                    .scene_shader_source()
                    .expect("scene shader")
                    .contains("fn ground_wetness("),
                registered
            );
        }
        for (index, (camera, pan, tilt)) in [
            (0, 0, 0),
            (3, 0, 0),
            (6, 0, 0),
            (7, 0, 0),
            (12, 0, 0),
            (15, 0, 0),
            (18, 0, 0),
            (7, 155, 50),
        ]
        .into_iter()
        .enumerate()
        {
            let get = |f: &mut support::Fixture| {
                f.parameters(&format!(r#"{{"camera.island":{camera},"sun.elevation":8,"camera.pan":{pan},"camera.tilt":{tilt}}}"#));
                let frame =
                    f.frame(2.0 + f64::from(u32::try_from(index).expect("camera count")) / 120.0);
                let gather = frame
                    .gpu_pass_ns
                    .as_ref()
                    .expect("Metal timestamps")
                    .passes()
                    .iter()
                    .any(|pass| pass.name.ends_with("water gather"));
                (f.image(), gather)
            };
            let (ordinary, ordinary_gather) = get(&mut absent);
            let (registered, registered_gather) = get(&mut dry);
            let changed = ordinary
                .as_chunks::<4>()
                .0
                .iter()
                .zip(registered.as_chunks::<4>().0)
                .filter(|(left, right)| left != right)
                .count();
            if changed != 0 {
                let first = ordinary
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(registered.as_chunks::<4>().0)
                    .enumerate()
                    .find(|(_, (left, right))| left != right);
                eprintln!("Metal first mismatch at {first:?}");
            }
            assert_eq!(
                changed, 0,
                "{wind:?} camera {camera} changed submitted pixels"
            );
            assert_eq!(
                registered_gather, ordinary_gather,
                "{wind:?} camera {camera} changed gather admission"
            );
            eprintln!(
                "Metal {wind:?} camera {camera}, pan {pan}: {changed} changed pixels, gather {ordinary_gather} / {registered_gather}"
            );
            if pan != 0 && matches!(wind, Wind::Calm) {
                assert!(!registered_gather, "{wind:?} camera away from all readers");
            }
        }
    });
}
