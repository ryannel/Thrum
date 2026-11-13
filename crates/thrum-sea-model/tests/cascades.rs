//! Independent full-grid Fourier sum and transform checks.
use std::f64::consts::TAU;
use thrum_sea_model::{Cascade, Differential, Forcing, GRID_SIZE, Spectrum};

fn channels(s: Differential) -> [f64; 9] {
    [
        s.displacement[0],
        s.displacement[1],
        s.displacement[2],
        s.slope[0],
        s.slope[1],
        s.horizontal_derivative[0],
        s.horizontal_derivative[1],
        s.horizontal_derivative[2],
        s.jacobian,
    ]
}

// Direct Fourier sum, reordered by shared z frequency to avoid billions of
// redundant trig calls. No FFT/butterfly, production complex arithmetic, or
// production direct evaluator is used. Each vector contributes at every x;
// each z harmonic then contributes at every z. Complexity O(modes*N + N³).
fn direct_grid(band: &Cascade, time: f64) -> Vec<[f64; 9]> {
    let n = GRID_SIZE;
    let mut result = vec![[0.0; 9]; n * n];
    let trig: Vec<Vec<_>> = (-127..=127)
        .map(|k| {
            (0..n)
                .map(|z| (TAU * f64::from(k) * (z as f64 + 0.5) / n as f64).sin_cos())
                .collect()
        })
        .collect();
    for x in 0..n {
        let px = (x as f64 + 0.5) * band.length() / n as f64;
        let mut groups = vec![[[0.0; 2]; 8]; 255];
        for c in band.components() {
            let [kx, kz] = c.wave_vector;
            let k = kx.hypot(kz);
            let (s, cos) = (kx * px - c.frequency() * time + c.phase).sin_cos();
            let a = c.amplitude;
            let cosine = [
                0.0,
                a,
                0.0,
                0.0,
                0.0,
                -a * kx * kx / k,
                -a * kx * kz / k,
                -a * kz * kz / k,
            ];
            let sine = [
                -a * kx / k,
                0.0,
                -a * kz / k,
                -a * kx,
                -a * kz,
                0.0,
                0.0,
                0.0,
            ];
            #[expect(
                clippy::cast_possible_truncation,
                reason = "validated signed lattice mode plus 127 is in 0..255"
            )]
            let group = (kz * band.length() / TAU).round() as i32 + 127;
            let index = usize::try_from(group).expect("validated nonnegative lattice index");
            for axis in 0..8 {
                groups[index][axis][0] += cosine[axis] * cos + sine[axis] * s;
                groups[index][axis][1] += -cosine[axis] * s + sine[axis] * cos;
            }
        }
        for z in 0..n {
            let value = &mut result[z * n + x];
            for (group, t) in groups.iter().zip(&trig) {
                let (sin, cos) = t[z];
                for axis in 0..8 {
                    value[axis] += group[axis][0] * cos + group[axis][1] * sin;
                }
            }
            value[8] = (1.0 + value[5]) * (1.0 + value[7]) - value[6] * value[6];
        }
    }
    result
}

#[test]
fn full_256_band_matches_direct_sum_at_every_texel_centre() {
    let sea = Spectrum::build(&Forcing {
        wind_speed: 13.0,
        wind_direction: 0.41,
        fetch: 150_000.0,
        swells: vec![],
    })
    .unwrap();
    let band = &sea.cascades[0];
    assert!(band.components().len() > 40000);
    let tolerance = 1e-9 * band.variance().sqrt();
    let time = 19.25;
    let direct = direct_grid(band, time);
    let transformed = band.inverse_transform(time);
    let mut largest: f64 = 0.0;
    for (a, b) in direct.iter().zip(transformed) {
        for (a, b) in a.iter().zip(channels(b)) {
            largest = largest.max((a - b).abs());
        }
    }
    assert!(
        largest < tolerance,
        "largest {largest}, tolerance {tolerance}"
    );
    // The reordered reference is itself checked against ungrouped direct sums.
    for (x, z) in [(0, 0), (7, 131), (255, 255), (119, 22)] {
        let p = [
            (x as f64 + 0.5) * band.length() / 256.0,
            (z as f64 + 0.5) * band.length() / 256.0,
        ];
        for (a, b) in direct[z * 256 + x]
            .iter()
            .zip(channels(band.direct(p, time)))
        {
            assert!((a - b).abs() < tolerance);
        }
    }
}

#[test]
fn conjugates_axes_and_high_modes_are_not_lost() {
    let band = Cascade::new(
        37.0,
        &[
            (0, 3, 0.2, 0.4),
            (7, 0, 0.1, 1.0),
            (-127, 119, 0.0001, 2.0),
            (7, -8, 0.02, 0.9),
            (-7, 8, 0.01, 1.3),
        ],
    )
    .unwrap();
    let output = band.inverse_transform(0.0);
    for z in 0..256 {
        for x in 0..256 {
            let p = [
                (x as f64 + 0.5) * 37.0 / 256.0,
                (z as f64 + 0.5) * 37.0 / 256.0,
            ];
            for (a, b) in channels(output[z * 256 + x])
                .into_iter()
                .zip(channels(band.direct(p, 0.0)))
            {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
    assert!(Cascade::new(1.0, &[(128, 0, 1.0, 0.0)]).is_err());
}
