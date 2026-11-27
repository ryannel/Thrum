//! Continuous model transmission and the submitted incident meter.
#[cfg(target_os = "macos")]
mod support;

use thrum_water_model::meter_light_factor;

#[test]
fn a_centimetre_of_lens_travel_changes_the_meter_by_less_than_a_twentieth_stop() {
    for downwelling in [[0.16, 0.08, 0.06], [0.24, 0.12, 0.16]] {
        let mut previous = [1.0_f64; 3];
        let mut largest_step = 0.0_f64;
        for centimetres in -50..=50 {
            let next = meter_light_factor(f64::from(centimetres) * 0.01, downwelling, 0.96);
            for (before, after) in previous.into_iter().zip(next) {
                assert!((0.0..=before).contains(&after));
                largest_step = largest_step.max((before / after).log2());
            }
            previous = next;
        }
        assert!(largest_step < 0.05, "{downwelling:?}: {largest_step} EV");
    }
    let depth = 5.0;
    let [red, green, blue] = meter_light_factor(depth, [0.16, 0.08, 0.06], 0.96);
    assert!(0.0 < red && red < green && green < blue && blue < 0.96);
    assert!((green - 0.96 * (-0.08 * depth).exp()).abs() < 1e-12);
}

thrum_test_support::macos_gpu_test! {
fn metal_meter_applies_transmission_to_incident_light() {
    use support::optics::{Rig, camera, sun};

    let Some(gpu) = thrum_test_support::gpu("water meter", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, 1.0, 0.0], [0.0, 1.0, 1.0], 0.8, 1.0)
        .with_incident_exposure(0.0, -24.0, 24.0)
        .expect("incident meter");
    let mut rig = Rig::new(&gpu, &view, sun(1.0, 0.0), &[], None);
    let (_, baseline, _) = rig.scene_colour_with_exposure();
    rig.host.continuous_source_mut().view = view.with_meter_medium_factor([1.0; 3]).expect("unit factor");
    let (_, identity, _) = rig.scene_colour_with_exposure();
    assert_eq!(identity[..5], baseline[..5], "storage, exposure, EV, lux and flags");
    assert_eq!(identity[7], baseline[7], "display flags");
    let factor = 0.5;
    rig.host.continuous_source_mut().view = view.with_meter_medium_factor([factor; 3]).expect("half transmission");
    let (_, filtered, _) = rig.scene_colour_with_exposure();
    let lux = f32::from_bits(baseline[3]);
    assert!(lux > 0.0);
    assert_eq!(filtered[3], (lux * factor).to_bits());
    let gain = f32::from_bits(baseline[1]);
    let expected = gain * factor;
    let actual = f32::from_bits(filtered[1]);
    assert!(actual.to_bits().abs_diff(expected.to_bits()) <= 2, "{actual} vs {expected}");
}
}
