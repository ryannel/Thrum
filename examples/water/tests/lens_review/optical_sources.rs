//! Distinguish opaque samples on the optical ray from samples along the camera ray.
#[cfg(target_os = "macos")]
use crate::support::optics::{Panel, Rig, attach, camera, sun, water_with_profile};
#[cfg(target_os = "macos")]
use thrum_render::synthetic::SyntheticView;
#[cfg(target_os = "macos")]
use thrum_test_support::Gpu;

#[cfg(target_os = "macos")]
const EXTENT: [u32; 2] = [321, 181];

#[cfg(target_os = "macos")]
#[expect(
    clippy::print_stderr,
    reason = "record the two submitted scene-linear radiances in the proof log"
)]
fn emission_delta(
    gpu: &Gpu,
    view: &SyntheticView,
    panel: Panel,
    pixel: [u32; 2],
    underwater: bool,
    bed: Option<f64>,
) -> f32 {
    let samples = [false, true].map(|radiant| {
        let panel = Panel { radiant, ..panel };
        let mut rig = Rig::with_extent(gpu, view, sun(0.0, 0.0), &[panel], None, EXTENT);
        let bed = bed.map(|height| (height, [-6.0; 2], 12.0));
        let boxes = attach(&mut rig, water_with_profile(bed, true, underwater));
        assert!(boxes.publish(&[panel.bounds()]));
        let (scale, pixels) = rig.scene_colour();
        pixels[(pixel[1] * EXTENT[0] + pixel[0]) as usize][1] / scale
    });
    eprintln!(
        "at {pixel:?}, matte {:.3}, emissive {:.3}",
        samples[0], samples[1]
    );
    samples[1] - samples[0]
}

thrum_test_support::macos_gpu_test! {
fn only_the_refracted_ray_can_see_an_air_panel_from_below() {
    let Some((_serial, gpu)) = crate::metal::gpu("underwater source paths") else { return };
    let eye = [0.0, -1.0, 0.0];
    let view = camera(eye, [0.0, 0.0, 1.0], 105.0_f32.to_radians(), 1.0 / 32768.0);
    let panel = Panel {
        centre: [0.0, 0.30, 10.0],
        half_size: [0.9, 0.12, 0.06],
        radiant: false,
    };
    let reflected = emission_delta(&gpu, &view, panel, [161, 144], true, None);
    assert!(reflected.abs() < 1.0, "an air source leaked into the reflected ray: {reflected}");
    let along_refracted_ray = Panel {
        centre: [0.0, 0.18, 1.5],
        half_size: [0.25, 0.06, 0.04],
        ..panel
    };
    let transmitted = emission_delta(&gpu, &view, along_refracted_ray, [160, 90], true, None);
    assert!(transmitted > 100.0, "the refracted ray should see opaque geometry: {transmitted}");
}
}

thrum_test_support::macos_gpu_test! {
fn the_reflected_ray_can_see_a_submerged_panel() {
    let Some((_serial, gpu)) = crate::metal::gpu("underwater reflected source") else { return };
    let view = camera([0.0, -0.4, 0.0], [0.0, 0.0, 1.0], 105.0_f32.to_radians(), 1.0 / 32768.0);
    let panel = Panel {
        centre: [0.0, -0.2, 1.5],
        half_size: [0.25, 0.05, 0.04],
        radiant: false,
    };
    let delta = emission_delta(&gpu, &view, panel, [160, 90], true, None);
    assert!(delta > 100.0, "a submerged source on the reflected ray should be visible: {delta}");
}
}

thrum_test_support::macos_gpu_test! {
fn wet_side_of_a_split_lens_reflects_water_instead_of_the_dry_hull() {
    let Some((_serial, gpu)) = crate::metal::gpu("split lens source paths") else { return };
    let y = -0.06;
    let view = camera([0.0, y, 0.0], [0.0, y + 0.1, 1.0], 105.0_f32.to_radians(), 1.0 / 32768.0);
    let panel = Panel {
        centre: [0.0, 0.144, 2.04],
        half_size: [0.35, 0.025, 0.04],
        radiant: false,
    };
    let reflected = emission_delta(&gpu, &view, panel, [160, 90], true, None);
    assert!(reflected.abs() < 1.0, "the wet lens sees a dry source outside the window: {reflected}");
}
}

thrum_test_support::macos_gpu_test! {
fn ordinary_water_does_not_borrow_a_panel_the_optical_rays_miss() {
    let Some((_serial, gpu)) = crate::metal::gpu("ordinary water source paths") else { return };
    let panel = Panel {
        centre: [0.0, -0.103, 2.04],
        half_size: [0.35, 0.025, 0.04],
        radiant: false,
    };
    for distance in [1.0, 4.0] {
        let view = camera([0.0, 0.1 * distance, 1.0 - distance], [0.0, 0.0, 1.0], 0.55, 1.0 / 32768.0);
        let delta = emission_delta(&gpu, &view, panel, [160, 90], false, None);
        assert!(delta.abs() < 1.0, "the off-ray submerged source leaked at {distance} m: {delta}");
    }
}
}

thrum_test_support::macos_gpu_test! {
fn shallow_scenery_does_not_replace_the_refracted_ray_with_a_registered_panel() {
    let Some((_serial, gpu)) = crate::metal::gpu("shallow scenery source paths") else { return };
    let view = camera([0.0, 0.1, 0.0], [0.0, 0.0, 1.0], 0.55, 1.0 / 32768.0);
    let panel = Panel {
        centre: [0.0, -0.103, 2.04],
        half_size: [0.35, 0.025, 0.04],
        radiant: false,
    };
    let delta = emission_delta(&gpu, &view, panel, [160, 90], false, Some(-0.8));
    assert!(delta.abs() < 1.0, "the shallow scenery fallback borrowed a registered source: {delta}");
}
}

thrum_test_support::macos_gpu_test! {
fn a_raised_bed_without_opaque_geometry_cannot_colour_the_sky_window() {
    use std::sync::{Arc, Mutex};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Optics, Profile, Water};
    use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

    let Some((_serial, gpu)) = crate::metal::gpu("empty gather beyond the window") else { return };
    let view = camera([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], 1.1, 1.0 / 32768.0);
    let radiance = [false, true].map(|raised| {
        let heights = (0..21).flat_map(|z| {
            let rise = if raised { (3.0 * f64::from(z) - 30.0).max(0.0) * 1.2 } else { 0.0 };
            std::iter::repeat_n((-12.0 + rise).min(16.0), 21)
        });
        let map = Map::prepare(
            Bed::new([-30.0; 2], 3.0, [21, 21], heights.collect()).expect("calm bed"),
            vec![Body {
                name: "sea".into(),
                kind: BodyKind::Sea,
                stage: Stage { min: 0.0, max: 0.0, value: 0.0 },
                level: 0.0,
                gains: [0.0; 4],
                fetch: 100.0,
            }],
            thrum_water_model::Profile { margin_height: 1.0, swash_ratio: 0.7 },
        ).expect("bounded sea");
        let water = Water::mapped(
            Arc::new(Mutex::new(map)),
            Spectrum {
                long: vec![],
                cascades: [160.0, 37.0, 9.0].map(|length| Cascade::new(length, &[]).expect("calm cascade")),
                tail_slope_variance: 0.0,
            },
            Profile { underwater: true, optics: Some(Optics::default()), ..Profile::default() },
        ).expect("mapped water");
        let mut rig = Rig::with_extent(&gpu, &view, sun(0.0, 0.0), &[], None, EXTENT);
        attach(&mut rig, water);
        let (scale, pixels) = rig.scene_colour();
        pixels[pixels.len() / 2].map(|x| x / scale)
    });
    eprintln!("sky window with flat and raised beds and no opaque geometry: {radiance:?}");
    assert!(radiance[0][1] > 100.0);
    for (a, b) in radiance[0].into_iter().zip(radiance[1]) {
        assert!((a - b).abs() < 1.0, "a map height supplied scenery colour: {radiance:?}");
    }
}
}
