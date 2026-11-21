//! Distinct interiors with identical footprints have distinct slant optical paths.
use crate::support;
use support::{close, directed, field, state};
use thrum_cloud_model::{Density, Sample, Shape};

fn values(alternate: bool) -> Vec<f64> {
    let mut v = vec![0.0; 8 * 8 * 8];
    for z in 0..8 {
        for x in 0..8 {
            for y in 0..8 {
                // Equal bottom/top and sum at each x/z. Interior sheet slopes in opposite
                // directions; a ray following one sheet crosses the other briefly.
                let sheet = if alternate { 7 - x } else { x };
                v[(z * 8 + y) * 8 + x] = if y == 0 || y == 7 || y == sheet {
                    1.0
                } else {
                    0.0
                };
            }
        }
    }
    v
}

fn own_density(v: &[f64], point: [f64; 3]) -> f64 {
    if !(0.0..160.0).contains(&point[1]) {
        return 0.0;
    }
    let x = point[0] / 20.0 + 3.5;
    let z = point[2] / 20.0 + 3.5;
    let mut sum = 0.0;
    // Deliberately independent interpolation: tent kernels over all voxel centres.
    for iz in 0..8 {
        for ix in 0..8 {
            let weight =
                (1.0 - (x - ix as f64).abs()).max(0.0) * (1.0 - (z - iz as f64).abs()).max(0.0);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "vertical coordinate bounded above"
            )]
            let iy = (point[1] / 20.0) as usize;
            sum += weight * v[(iz * 8 + iy) * 8 + ix];
        }
    }
    sum * 0.05
}

#[test]
fn slant_path_reads_voxels_not_their_column_sum() {
    let a_values = values(false);
    let b_values = values(true);
    let a = Shape::new([8; 3], 20.0, a_values.clone()).expect("shape");
    let b = Shape::new([8; 3], 20.0, b_values.clone()).expect("shape");
    assert_eq!(a.footprint(), b.footprint());
    let field = field(7, 0.0); // no erosion: isolate the two forms' geometry
    let mut state = state(0.0, 0.0);
    state.add(directed(&a, 1, [0.0; 2])).expect("add");
    state.step(1, 10.0).expect("form");
    let sample = Sample {
        tick: 1,
        origin: [-70.0, 710.0, 0.0],
        direction: [0.6, 0.8, 0.0],
        length: 180.0,
        steps: 7200,
    };
    let mut depths = Vec::new();
    for (shape, voxels) in [(&a, &a_values), (&b, &b_values)] {
        let ds = sample.length / f64::from(sample.steps);
        let expected: f64 = (0..sample.steps)
            .map(|i| {
                let s = (f64::from(i) + 0.5) * ds;
                own_density(
                    voxels,
                    [
                        sample.origin[0] + 0.6 * s,
                        sample.origin[1] - 700.0 + 0.8 * s,
                        0.0,
                    ],
                ) * ds
            })
            .sum();
        for fidelity in [Density::Full, Density::Coarse] {
            close(
                field.optical_depth(&sample, &state, fidelity, |_| Some(shape), 1.0),
                expected,
                1e-10,
            );
        }
        depths.push(expected);
        let column = field.optical_depth(&sample, &state, Density::Coarse, |_| Some(shape), 0.0);
        let other = field.optical_depth(&sample, &state, Density::Coarse, |_| None, 0.0);
        close(column, other, 0.0);
    }
    assert!((depths[0] - depths[1]).abs() > 0.5, "{depths:?}");
}

#[test]
fn eroded_slant_paths_integrate_both_public_density_variants() {
    let shape = Shape::new([8; 3], 20.0, values(false)).expect("shape");
    let field = field(7, 0.35);
    let state = support::formed(&shape);
    let sample = Sample {
        tick: 1,
        origin: [-70.0, 710.0, 0.0],
        direction: [0.6, 0.8, 0.0],
        length: 180.0,
        steps: 7200,
    };
    let uneroded = field.optical_depth(&sample, &state, Density::Uneroded, |_| Some(&shape), 1.0);
    let mut results = Vec::new();
    for fidelity in [Density::Full, Density::Coarse] {
        // Independently use a trapezoid rule at four times the path resolution.
        // Horizontal tent kernels are linear within a voxel; profile/noise
        // introduce continuous bounded curvature. Halve again to verify the
        // numerical allowance of 0.001 optical depth on this stated fixture.
        let integrate = |steps: u32| {
            let ds = sample.length / f64::from(steps);
            (0..=steps)
                .map(|i| {
                    let distance = f64::from(i) * ds;
                    let p =
                        std::array::from_fn(|j| sample.origin[j] + sample.direction[j] * distance);
                    let endpoint = if i == 0 || i == steps { 0.5 } else { 1.0 };
                    field.density(p, &state, fidelity, |_| Some(&shape), 1.0) * ds * endpoint
                })
                .sum::<f64>()
        };
        let reference = integrate(sample.steps * 4);
        close(reference, integrate(sample.steps * 8), 0.0005);
        let path = field.optical_depth(&sample, &state, fidelity, |_| Some(&shape), 1.0);
        close(path, reference, 0.001);
        assert!(path > 0.0 && path < uneroded);
        results.push(path);
    }
    assert!((results[0] - results[1]).abs() > 0.001, "{results:?}");
}
