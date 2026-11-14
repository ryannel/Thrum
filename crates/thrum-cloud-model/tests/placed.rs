//! Asymmetric voxel footprints bound volume, column and every tested blend.
pub mod support;
use support::{close, field, formed, shape};
use thrum_cloud_model::Density;

#[test]
fn footprint_is_the_pointwise_voxel_integral() {
    let shape = shape();
    let state = formed(&shape);
    let field = field(7, 0.3);
    for x in -12..=12 {
        for z in -10..=10 {
            let point = [f64::from(x) * 5.0, 700.0, f64::from(z) * 5.0];
            let c = field.query(point, &state);
            // Independent slab quadrature, including horizontal interpolation and hollow.
            let voxels: f64 = (0..6)
                .map(|y| {
                    shape.density([point[0], f64::from(y) * 20.0 + 10.0, point[2]]) * 20.0 / 20.0
                })
                .sum();
            close(c.optical_depth, voxels, 1e-12); // fixture extinction = 0.05 m^-1
            for weight in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for fidelity in [Density::Uneroded, Density::Full, Density::Coarse] {
                    let tau: f64 = (0..480)
                        .map(|i| {
                            field.density(
                                [point[0], 700.0 + (f64::from(i) + 0.5) * 0.25, point[2]],
                                &state,
                                fidelity,
                                |_| Some(&shape),
                                weight,
                            ) * 0.25
                        })
                        .sum();
                    // Midpoint bound of the independently integrated smooth profile.
                    assert!(
                        tau <= c.optical_depth * (1.0 + 2e-5) + 1e-9,
                        "{tau} > {}",
                        c.optical_depth
                    );
                    if weight > 0.99 && matches!(fidelity, Density::Uneroded) {
                        close(tau, c.optical_depth, 1e-10);
                    }
                    if c.optical_depth == 0.0 {
                        close(tau, 0.0, 0.0);
                    }
                }
            }
        }
    }
}
