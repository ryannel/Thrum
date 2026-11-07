//! Compare the shipping sky and incident reduction to the independent
//! finite-shell plus diffuse f64 quadrature in the M1 evidence.

use std::io::Write as _;

use super::readback::Probe;

const GAIN: f64 = 65_536.0;
const ELEVATIONS: [f32; 5] = [90.0, 20.0, 10.0, 5.0, -30.0];

struct Reference {
    height: f32,
    rays: [f64; 5],
    horizon: [f64; 2], // Ground limit, sky limit.
    horizontal_lux: f64,
}

// Each row sums the shell and diffuse columns of mixed-2.csv. The split
// horizon stores two distinct terminal solutions at the same elevation.
const REFERENCE: [Reference; 3] = [
    Reference {
        height: 0.0,
        rays: [
            1.710e-4,
            2.747_452e-4,
            2.989_057e-4,
            2.427_922e-4,
            2.135_841e-5,
        ],
        horizon: [2.135_841e-5, 1.034_311e-4],
        horizontal_lux: 6.709_942e-4,
    },
    Reference {
        height: 1800.0,
        rays: [
            1.727_028e-4,
            2.877_571e-4,
            3.294_886e-4,
            2.862_687e-4,
            2.475_446e-5,
        ],
        horizon: [9.189_556e-5, 9.973_511e-5],
        horizontal_lux: 6.912_419e-4,
    },
    Reference {
        height: 15_000.0,
        rays: [
            1.770_471e-4,
            3.294_208e-4,
            4.422_802e-4,
            4.742_667e-4,
            3.760_431e-5,
        ],
        horizon: [1.184_168e-4, 1.197_290e-4],
        horizontal_lux: 7.570_950e-4,
    },
];

fn error(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected
}

#[test]
fn night_profile_survives_sky_sampling_and_the_incident_reduction() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for night-profile reference");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("night-profile GPU device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let directions = ELEVATIONS.map(|degrees| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            [cos, sin, 0.0, 0.0]
        });
        let mut probe = Probe::new(&device, &queue, &directions, false);
        for reference in REFERENCE {
            // Toggle on an existing allocation. Keep the same solar input
            // and require that even its unexposed LUT values are unchanged.
            let off = probe.night(&device, &queue, reference.height, false);
            let on = probe.night(&device, &queue, reference.height, true);
            let restored = probe.night(&device, &queue, reference.height, false);
            for ((off, on), restored) in off.grid.iter().zip(&on.grid).zip(restored.grid) {
                assert_eq!(off.map(f64::to_bits), restored.map(f64::to_bits));
                assert_eq!(off[3].to_bits(), 0);
                assert!(
                    off[..3]
                        .iter()
                        .zip(on)
                        .all(|(a, b)| a.to_bits() == b.to_bits())
                );
            }
            let mut ray_error: f64 = 0.0;
            for (index, (actual, expected)) in on.rays.iter().zip(reference.rays).enumerate() {
                let actual = actual[3] / GAIN;
                ray_error = ray_error.max(error(actual, expected));
                assert!(
                    error(actual, expected) <= 0.10,
                    "{} m, {} degrees: {actual:e} vs {expected:e}",
                    reference.height,
                    ELEVATIONS[index]
                );
            }
            for (row, expected) in [53, 54].into_iter().zip(reference.horizon) {
                let actual = on.grid[row * 192][3] / GAIN;
                ray_error = ray_error.max(error(actual, expected));
                assert!(
                    error(actual, expected) <= 0.10,
                    "{} m, horizon row {row}: {actual:e} vs {expected:e}",
                    reference.height
                );
            }
            for (off, on) in off.irradiance[3][..3].iter().zip(on.irradiance[3]) {
                let actual = f64::from(on - off);
                assert!(
                    error(actual, reference.horizontal_lux) <= 0.05,
                    "{} m, incident night {actual:e} vs {:e}",
                    reference.height,
                    reference.horizontal_lux
                );
            }
            let incident = f64::from(on.irradiance[3][0] - off.irradiance[3][0]);
            writeln!(
                std::io::stdout().lock(),
                "height={} m max ray error={:.3}% incident={incident:e} lx",
                reference.height,
                ray_error * 100.0
            )
            .expect("write GPU observation");
            let ambient = on.irradiance[4][0] - off.irradiance[4][0];
            assert!(
                ambient > 0.0,
                "the same night light contributes to diffuse SH"
            );
        }
        assert!(
            scope.pop().await.is_none(),
            "night reference pipelines validate"
        );
    });
}
