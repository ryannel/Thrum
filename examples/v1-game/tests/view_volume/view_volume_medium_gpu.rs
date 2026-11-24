//! Read the joint view-volume transmittance through a slab on Metal.

#![cfg(feature = "proving")]

#[expect(
    dead_code,
    reason = "the shared fixture also checks sky pixels in other binaries"
)]
mod fixture;
mod medium_fixture;

use medium_fixture::{centre_layer, render};

const SLAB: [f32; 2] = [25.3125, 45.0];

#[test]
fn metal_joint_medium_counts_air_and_slab_once_at_both_slice_counts() {
    let Some(gpu) = fixture::Gpu::open(7, "joint slab transport") else {
        return;
    };
    let scene = fixture::scene(false, false, false);
    let slab = format!("select(0.0, 1.0, x >= {:?} && x < {:?})", SLAB[0], SLAB[1]);
    for factor in [1, 2] {
        let light = fixture::daylight(true, 1);
        let light = if factor == 2 {
            light.with_double_view_volume_slices()
        } else {
            light
        };
        let air = render(&gpu, &scene, light, None);
        let probe = 20 * factor;
        let base = centre_layer(&air, probe, false)[0];
        let scatter = centre_layer(&air, probe, true)[0];
        for range in [30.0_f32, 150.0, 2000.0] {
            let sigma = -0.02_f32.ln() / range;
            let lit = render(
                &gpu,
                &scene,
                light.with_view_volume_medium(1),
                Some((&slab, [sigma, sigma * 0.24, sigma * 4000.0])),
            );
            let attenuation = (-sigma * (SLAB[1] - SLAB[0])).exp();
            let expected = base * attenuation;
            let actual = centre_layer(&lit, probe, false)[0];
            let tolerance = 0.0016 * expected;
            let agrees = |value: f32| (actual - value).abs() <= tolerance;
            assert_eq!(lit.grid[2], 80 * factor + 2);
            assert!(
                agrees(expected),
                "range={range}, slices={}: T={actual} expected={expected}",
                80 * factor
            );
            assert!(!agrees(base * base * attenuation), "doubled air survived");
            assert!(
                !agrees(base * attenuation * attenuation),
                "doubled slab survived"
            );
            assert!(centre_layer(&lit, probe, true)[0] > scatter);
            println!(
                "range={range} slices={} T={actual} expected={expected} tolerance={tolerance}",
                80 * factor
            );
        }
    }
}
