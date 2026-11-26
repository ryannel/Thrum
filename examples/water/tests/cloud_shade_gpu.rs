//! Test the water's direct Sun separately from its sky illumination on Metal.
#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn foam_capture(gpu: &thrum_test_support::Gpu, tau: f32, foam: bool) -> Vec<[f32; 3]> {
    use std::time::Duration;
    use support::Fixture;
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_render::{runtime::CaptureLimits, synthetic::CaptureSource};
    use thrum_water::foam::Hull;

    let mut water = water(Some((-1000.0, [-2.0; 2], 4.0)), true);
    let births = foam.then(|| water.enable_foam([5.0, 0.0]).expect("foam on mapped water"));
    let report = water.report();
    let field = SunOpticalDepth::new(
        [2, 2],
        &[tau; 4],
        50.0,
        [-5000.0, -5000.0, 5000.0, 5000.0],
        0.01,
        1,
    )
    .expect("uniform field");
    let game = SkyGame::new(
        Controls::new(),
        Camera::look_at([0.0, 2.0, 0.0], [0.0, 0.0, 5.0], 65.0),
        |_, sky| {
            sky.sun_elevation_degrees = 68.0;
            sky.sun_azimuth_degrees = 90.0;
        },
    )
    .with_ground(None)
    .with_sun_optical_depth(field)
    .with_tick_duration(Duration::from_nanos(16_666_667))
    .expect("step")
    .with_fixed_update_endpoint(move |tick, _| {
        if let Some(births) = &births {
            births.post(
                tick.tick,
                [Hull {
                    place: [0.0, 3.0],
                    forward: [0.0, 1.0],
                    extent: [1.8, 0.65],
                    velocity: [0.0, 8.0],
                    drag_power: 30_000.0,
                }],
            );
        }
    })
    .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
    let mut fixture = Fixture::from_game(gpu, EXTENT, game, report);
    for tick in 0..32 {
        fixture.frame_without_timings(f64::from(tick) / 60.0);
    }
    fixture
        .host
        .enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
        .expect("scene-linear tap");
    fixture.frame_without_timings(32.0 / 60.0);
    let image =
        thrum_test_support::wait::until("foam shade readback", Duration::from_secs(30), || {
            fixture.device.poll(wgpu::PollType::Poll).expect("device");
            fixture.host.try_submitted_image().expect("image")
        });
    let exposure = f32::from_bits(image.exposure_gpu_words.expect("exposure provenance")[0]);
    image
        .rgba
        .as_chunks::<8>()
        .0
        .iter()
        .map(|p| {
            std::array::from_fn(|channel| {
                let lane = 2 * channel;
                support::half_value(u16::from_le_bytes([p[lane], p[lane + 1]])) / exposure
            })
        })
        .collect()
}

thrum_test_support::macos_gpu_test! {
fn the_foam_residual_at_half_sun_keeps_its_sky_term() {
    let Some(gpu) = thrum_test_support::gpu("lit whitewater", Some(wgpu::Backend::Metal)) else { return };
    let [lit, half, shaded] = [0.0, std::f32::consts::LN_2, 16.0].map(|tau| {
        let bare = foam_capture(&gpu, tau, false);
        let foam = foam_capture(&gpu, tau, true);
        foam.into_iter().zip(bare).map(|(foam, bare)| foam[0]-bare[0]).collect::<Vec<_>>()
    });
    let mut changed = 0;
    for ((&lit, &half), &shaded) in lit.iter().zip(&half).zip(&shaded) {
        if lit - shaded > 500.0 && shaded > 50.0 {
            let fraction = (half-shaded)/(lit-shaded);
            assert!((fraction - 0.5).abs() < 0.055, "whitewater path factor {fraction}: {lit} {half} {shaded}");
            changed += 1;
        }
    }
    assert!(changed > 40, "whitewater surplus under the sky and Sun in {changed} pixels");
}
}
