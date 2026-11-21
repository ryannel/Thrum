//! Independent relief, including a narrow ridge wholly inside the datum window.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{Density, Surface, SurfacePoint};

#[test]
fn hollows_fill_first_and_changes_converge() {
    let nodes: Vec<_> = (0..601)
        .map(|i| {
            let x = f64::from(i) * 10.0;
            SurfacePoint {
                ground: 70.0 * (x / 850.0).sin()
                    + 18.0 * (x / 93.0).sin()
                    + 5.0 * (x / 17.0).sin()
                    + if (295..305).contains(&i) { 50.0 } else { 0.0 },
                water: None,
            }
        })
        .collect();
    let surface = Surface::new([0.0; 2], [601, 1], 10.0, 3000.0, &nodes).expect("relief");
    // Independently average an interior datum, not a stored reference.
    support::close(
        surface.sample([3000.0, 0.0])[1],
        nodes[150..=450].iter().map(|p| p.ground).sum::<f64>() / 301.0,
        1e-12,
    );
    let mut row = ground_support::row();
    row.pooling = 1.0;
    let ground = ground_support::table(vec![row]);
    let mut previous_count = 0;
    for a in 1..=100 {
        let amount = f64::from(a) / 100.0;
        let mut samples: Vec<_> = (0..601)
            .map(|i| {
                let x = f64::from(i) * 10.0;
                let [h, d, _] = surface.sample([x, 0.0]);
                let c = ground.column(0, &surface, [x, 0.0], amount);
                (h - d, c[1] - c[0])
            })
            .collect();
        let count = samples.iter().filter(|s| s.1 > 0.0).count();
        assert!(count >= previous_count);
        previous_count = count;
        samples.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(samples.windows(2).all(|w| w[0].1 + 1e-12 >= w[1].1));
        for i in 1..601 {
            let x = f64::from(i) * 10.0;
            let p = ground.column(0, &surface, [x - 10.0, 0.0], amount);
            let q = ground.column(0, &surface, [x, 0.0], amount);
            if p[1] > p[0] && q[1] > q[0] {
                support::close(
                    q[1] - p[1],
                    surface.sample([x, 0.0])[1] - surface.sample([x - 10.0, 0.0])[1],
                    1e-12,
                );
            }
        }
    }
    assert!(previous_count > 0 && previous_count < 601);
    let field = support::field(1, 0.0);
    let measure = |delta: f64| {
        let mut a = support::state(0.0, 0.0);
        let mut b = a.clone();
        ground_support::controls(&mut a, vec![0.6], 0.0);
        ground_support::controls(&mut b, vec![0.6 + delta], 0.0);
        (0..601)
            .flat_map(|i| (0..10).map(move |j| [f64::from(i) * 10.0, f64::from(j) * 10.0, 0.0]))
            .map(|p| {
                (ground.density(&surface, &field, p, &a, Density::Uneroded)
                    - ground.density(&surface, &field, p, &b, Density::Uneroded))
                .abs()
            })
            .fold(0.0_f64, f64::max)
    };
    support::close(measure(0.0005) / measure(0.001), 0.5, 0.003);
    row.pooling = 0.0;
    let unpooled = ground_support::table(vec![row]);
    for i in 0..601 {
        let c = unpooled.column(0, &surface, [f64::from(i) * 10.0, 0.0], 0.7);
        support::close(c[1] - c[0], 70.0, 1e-12);
    }
}
