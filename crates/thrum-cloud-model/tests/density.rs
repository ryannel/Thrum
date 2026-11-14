//! Independent vertical quadrature of the public density at interiors and edges.
pub mod support;
use support::{close, field, state};
use thrum_cloud_model::Density;

#[test]
fn query_bounds_full_and_coarse_integrals_pointwise() {
    let field = field(7, 0.35);
    let mut state = state(0.75, 0.5);
    state.step(1, 8000.0).expect("time");
    let mut eroded = 0;
    for x in -20..=20 {
        let point = [f64::from(x) * 71.0, 0.0, 417.0];
        let c = field.query(point, &state);
        let n = 512;
        let dy = (c.top - c.base) / f64::from(n);
        let integral = |fidelity| {
            (0..n)
                .map(|i| {
                    field.density(
                        [point[0], c.base + (f64::from(i) + 0.5) * dy, point[2]],
                        &state,
                        fidelity,
                        |_| None,
                        0.0,
                    ) * dy
                })
                .sum::<f64>()
        };
        // Midpoint error ≤ max |profile''|/(24 n² integral(profile)).
        // Here min fade=0.2, max second derivative=6/0.2², integral=0.75.
        close(
            integral(Density::Uneroded),
            c.optical_depth,
            c.optical_depth * 150.0 / (24.0 * 0.75 * f64::from(n * n)) + 1e-12,
        );
        for f in [Density::Coarse, Density::Full] {
            let tau = integral(f);
            assert!(tau <= c.optical_depth + 1e-10);
            if c.optical_depth == 0.0 {
                close(tau, 0.0, 0.0);
            }
            if tau < c.optical_depth {
                eroded += 1;
            }
        }
    }
    assert!(eroded > 20);
    close(
        thrum_cloud_model::Type {
            base: 0.0,
            depth: [1.0, 1.0],
            liquid_water: 0.0004,
            droplet_radius: 10e-6,
            deck: 0.0,
            profile: [0.2, 0.3],
        }
        .extinction(),
        0.06,
        1e-15,
    );
}
