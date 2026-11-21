//! Frame identity and a shore mask that must never travel with a bank.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{Affinity, Density};

#[test]
fn world_factors_stay_at_the_shore_while_banks_travel() {
    let surface = ground_support::shore([0.0; 2]);
    let field = support::field(3, 0.0);
    let mut row = ground_support::row();
    row.centre = [-240.0, 0.0];
    row.radius = [500.0, 500.0];
    row.drifting = true;
    row.affinity = Affinity::Water;
    row.noise = 0.8;
    let ground = ground_support::table(vec![row]);
    let mut zero = support::state(0.0, 0.0);
    ground_support::controls(&mut zero, vec![1.0], 0.0);
    let mut witnessed_wrong_shore = false;
    for travel in [-200.0, 0.0, 200.0, 400.0] {
        let mut moved = zero.clone();
        let mut p = moved.parameters();
        p.wind = [travel / 10.0, 0.0];
        moved.set_target(p).expect("wind");
        moved.step(1, 10.0).expect("travel");
        assert_eq!(moved.travel(), [travel, 0.0]);
        let shifted = ground_support::shore([-travel, 0.0]);
        // Keep weather time identical to isolate travel.
        let mut still = zero.clone();
        still.step(1, 10.0).expect("same time");
        for x in -35..=35 {
            for y in [-5.0, 5.0, 25.0, 70.0, 120.0] {
                let p = [f64::from(x) * 20.0, y, 0.0];
                for fidelity in [Density::Full, Density::Coarse, Density::Uneroded] {
                    let a = ground.density(&surface, &field, p, &moved, fidelity);
                    let b =
                        ground.density(&shifted, &field, [p[0] - travel, y, 0.0], &still, fidelity);
                    assert_eq!(a.to_bits(), b.to_bits(), "travel {travel}, point {p:?}");
                    if p[0] > 40.0 || y < surface.sample([p[0], 0.0])[0] {
                        assert_eq!(a, 0.0);
                    }
                }
                let wrong = ground.density(
                    &surface,
                    &field,
                    [p[0] - travel, y, 0.0],
                    &still,
                    Density::Uneroded,
                );
                witnessed_wrong_shore |=
                    (travel - 200.0).abs() < 1e-12 && p[0] > 40.0 && wrong > 0.0;
            }
        }
        if (travel - 200.0).abs() < 1e-12 {
            assert!(
                ground.density(
                    &surface,
                    &field,
                    [-80.0, 10.0, 0.0],
                    &moved,
                    Density::Uneroded
                ) > 0.0,
                "bank remains on water behind shore"
            );
        }
        row.drifting = false;
        row.affinity = Affinity::Any;
        let held = ground_support::table(vec![row]);
        for x in -20..20 {
            let p = [f64::from(x) * 20.0, 30.0, 0.0];
            assert_eq!(
                held.density(&surface, &field, p, &moved, Density::Uneroded),
                held.density(&surface, &field, p, &still, Density::Uneroded)
            );
        }
    }
    assert!(witnessed_wrong_shore);
    row.affinity = Affinity::Land;
    let land = ground_support::table(vec![row]);
    for x in -30..=0 {
        assert_eq!(
            land.density(
                &surface,
                &field,
                [f64::from(x) * 20.0, 10.0, 0.0],
                &zero,
                Density::Uneroded
            ),
            0.0
        );
    }
}
