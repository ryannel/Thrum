//! Independent vertical quadrature of the public density at interiors and edges.
use crate::support;
use support::{close, field, state};
use thrum_cloud_model::Density;

#[test]
fn procedural_edges_are_removed_rather_than_uniformly_attenuated() {
    let field = field(7, 0.35);
    let mut state = state(0.75, 0.5);
    state.step(1, 8000.0).expect("time");
    let mut removed = [0; 2];
    let mut retained = [0; 2];
    let mut different = 0;
    // Sample the same band more closely across the bank. The broad shaping
    // leaves fewer surviving billow edges where the detail term can differ.
    for x in -110..=110 {
        for y in 0..=100 {
            let p = [f64::from(x) * 13.0, 700.0 + f64::from(y) * 5.0, 417.0];
            let u = field.density(p, &state, Density::Uneroded, |_| None, 0.0);
            let densities = [Density::Coarse, Density::Full]
                .map(|f| field.density(p, &state, f, |_| None, 0.0));
            for (i, d) in densities.into_iter().enumerate() {
                assert!(d >= 0.0 && d <= u);
                if u > 0.0 {
                    removed[i] += usize::from(d == 0.0);
                    retained[i] += usize::from(d > 0.0);
                }
            }
            different += usize::from((densities[0] - densities[1]).abs() > 1e-8);
        }
    }
    assert!(removed.iter().all(|&n| n > 100), "{removed:?}");
    assert!(retained.iter().all(|&n| n > 100), "{retained:?}");
    assert!(different > 100, "detail changed {different} edge samples");
}

#[test]
fn envelope_sweeps_fixed_noise_and_detail_preserves_solid_interiors() {
    use support::{directed, formed};
    use thrum_cloud_model::Shape;
    let field = field(7, 0.35);
    let shape = Shape::new([8; 3], 20.0, vec![1.0; 512]).expect("solid fixture");
    let mut rising = state(0.0, 0.0);
    rising.add(directed(&shape, 1, [0.0; 2])).expect("admit");
    // A half-second is 0.05 envelope for this ten-second linear response.
    rising.step(1, 0.5).expect("thin envelope");
    let solid = formed(&shape);
    let mut born = [0; 2];
    let mut cores = 0;
    for x in -16_i32..=16 {
        for y in 0..16 {
            let p = [f64::from(x) * 5.0, 705.0 + f64::from(y) * 10.0, 0.0];
            let u = field.density(p, &rising, Density::Uneroded, |_| Some(&shape), 1.0);
            for (i, f) in [Density::Coarse, Density::Full].into_iter().enumerate() {
                let thin = field.density(p, &rising, f, |_| Some(&shape), 1.0);
                let full = field.density(p, &solid, f, |_| Some(&shape), 1.0);
                if u > 0.0 && thin == 0.0 && full > 0.0 {
                    born[i] += 1;
                }
                // Descending through the same envelope retraces the same field:
                // noise never receives weather time or an envelope translation.
                assert!(thin <= full);
                if x.abs() <= 12 {
                    close(full, 0.05, 1e-15);
                    cores += 1;
                }
            }
        }
    }
    assert!(born.iter().all(|&n| n > 100), "{born:?}");
    assert!(cores > 100);
    let mut falling = formed(&shape);
    falling.direct(1, 0.0, 1.0).expect("dissolve");
    falling.step(2, 9.5).expect("thin again");
    for f in [Density::Coarse, Density::Full] {
        for x in -16..=16 {
            let p = [f64::from(x) * 5.0, 750.0, 0.0];
            close(
                field.density(p, &rising, f, |_| Some(&shape), 1.0),
                field.density(p, &falling, f, |_| Some(&shape), 1.0),
                1e-15,
            );
        }
    }
}

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
