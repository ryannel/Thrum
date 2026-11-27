//! Witness the caustic draw through the ordinary effect graph on Metal.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn a_flat_surface_draws_a_unit_value_at_every_rung() {
    use support::optics::{Rig, attach, camera, sun, water_configured};
    use thrum_test_support::readback::{Mip, texture};
    use thrum_water::Profile;

    let Some(gpu) = thrum_test_support::gpu("caustic depth ladder", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, 1.0, 0.0], [0.0, 0.5, 1.0], 0.8, 1.0);
    let mut rig = Rig::new(&gpu, &view, sun(1.0, 0.0), &[], None);
    let mut water = water_configured(None, Profile { caustics: true, ..Profile::default() });
    let tap = water.tap_caustics();
    attach(&mut rig, water);
    rig.frame();
    let ladder = tap.texture().expect("prepared caustic ladder");
    assert_eq!(ladder.depth_or_array_layers(), 13);
    let contents = texture(&rig.device, &rig.queue, &ladder, Mip::level(0), ..);
    let unit = 0x3c00u16.to_le_bytes();
    for (index, &value) in contents.as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(value, unit, "flat caustic texel {index}");
    }
}
}

thrum_test_support::macos_gpu_test! {
#[expect(clippy::print_stderr, reason = "record the submitted rung means")]
fn a_ripple_moves_light_between_places_and_depths() {
    use std::time::Duration;
    use support::optics::{Rig, attach, camera, sun, water_with_spectrum};
    use thrum_app::sky::StillWorld;
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_test_support::readback::{Mip, texture};
    use thrum_water::Profile;

    let Some(gpu) = thrum_test_support::gpu("caustic ripple", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, 1.0, 0.0], [0.0, 0.5, 1.0], 0.8, 1.0);
    let mut rig = Rig::new(&gpu, &view, sun(1.0, 0.0), &[], None);
    *rig.host.world_mut() = StillWorld::with_tick_duration(Duration::from_millis(100)).expect("ripple clock");
    let spectrum = Spectrum {
        long: Vec::new(),
        cascades: [
            Cascade::new(160.0, &[]).unwrap(),
            Cascade::new(37.0, &[]).unwrap(),
            Cascade::new(9.0, &[(3, 0, 0.0018, 0.0), (0, 3, 0.00045, 0.7)]).unwrap(),
        ],
        tail_slope_variance: 0.0,
    };
    let profile = Profile { caustics: true, ..Profile::default() };
    let mut water = water_with_spectrum(None, profile, spectrum);
    let tap = water.tap_caustics();
    attach(&mut rig, water);
    let ladder = tap.texture().expect("prepared caustic ladder");
    let read = |rig: &Rig| {
        texture(&rig.device, &rig.queue, &ladder, Mip::level(0), ..)
            .as_chunks::<2>().0.iter()
            .map(|&value| f64::from(support::half_value(u16::from_le_bytes(value))))
            .collect::<Vec<_>>()
    };
    let first = rig.frame_at(Duration::from_millis(100));
    let initial = read(&rig);
    let last = rig.frame_at(Duration::from_millis(400));
    assert!(last.tick > first.tick);
    let moved = read(&rig);

    const RUNG_PIXELS: usize = 512 * 512;
    const RUNG_COUNT: usize = 13;
    assert_eq!(initial.len(), RUNG_COUNT * RUNG_PIXELS);
    let scale = f64::from(u32::try_from(RUNG_PIXELS).unwrap()).recip();
    let difference = |a: &[f64], b: &[f64]| {
        a.iter().zip(b).map(|(x, y)| (x-y).abs()).sum::<f64>() * scale
    };
    for data in [&initial, &moved] {
        for (rung, values) in data.chunks_exact(RUNG_PIXELS).enumerate() {
            let mean = values.iter().sum::<f64>() * scale;
            let contrast = values.iter().map(|v| (v-1.0).abs()).sum::<f64>() * scale;
            eprintln!("Rung {rung}: mean {mean:.6}, mean deviation from unity {contrast:.6}");
            assert!((mean - 1.0).abs() < 0.04, "rung {rung} mean {mean}");
            assert!(contrast > 0.005, "rung {rung} remains flat");
        }
    }
    let shallow = 2 * RUNG_PIXELS..3 * RUNG_PIXELS;
    let deep = 10 * RUNG_PIXELS..11 * RUNG_PIXELS;
    assert!(difference(&initial[shallow.clone()], &initial[deep]) > 0.04);
    assert!(difference(&initial[shallow.clone()], &moved[shallow]) > 0.01);
}
}
