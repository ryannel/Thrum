//! Disc-energy conservation and renderer f32 table measurements.
pub mod support;
use std::f64::consts::PI;
use support::close;
use thrum_cloud_model::{Phase, PhaseTable};

fn sphere(phase: impl Fn(f64) -> f64, n: u32) -> [f64; 2] {
    let mut total = 0.0;
    let mut moment = 0.0;
    for i in 0..n {
        let u = (f64::from(i) + 0.5) / f64::from(n);
        let mass = phase(u.powi(4)) * 16.0 * PI * u.powi(3) / f64::from(n);
        total += mass;
        moment += mass * (1.0 - 2.0 * u.powi(4));
    }
    [total, moment / total]
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "exercise the renderer's f32 sampling"
)]
fn table_phase(table: &PhaseTable, s2: f64) -> f64 {
    f64::from(table.sample(s2 as f32))
}

#[test]
fn disc_and_table_conserve_energy_and_match_the_forward_angles() {
    let radius = 0.2667_f64.to_radians();
    let mut tables = Vec::new();
    for diameter in [5.0, 20.0, 50.0] {
        let phase = Phase::fit(diameter).expect("fit");
        let integral = sphere(|s2| phase.disc(2.0 * s2.sqrt().asin(), radius), 3000);
        let expected_moment = phase.mean_cosine() * (1.0 + radius.cos()) * 0.5;
        close(integral[0], 1.0, 2e-6);
        close(integral[1], expected_moment, 2e-6);
        // Own polar midpoint quadrature at the centre; no model disc quadrature.
        let centre: f64 = (0..100_000)
            .map(|i| {
                let angle = radius * (f64::from(i) + 0.5) / 100_000.0;
                phase.point((angle * 0.5).sin().powi(2)) * angle.sin() * radius
                    / 100_000.0
                    / (2.0 * (radius * 0.5).sin().powi(2))
            })
            .sum();
        close(phase.disc(0.0, radius) / centre, 1.0, 2e-7);
        let table = PhaseTable::new(phase, radius, 1025).expect("table");
        let integrated = sphere(|s2| table_phase(&table, s2), 100_000);
        let mut worst = 0.0_f64;
        for degrees in [0.0_f64, 0.25, 0.5, 2.0] {
            let theta = degrees.to_radians();
            let exact = phase.disc(theta, radius);
            let value = table_phase(&table, (theta * 0.5).sin().powi(2));
            worst = worst.max((value / exact - 1.0).abs());
        }
        // Bounds measured first in M1-phase.md; widened lobe loses >0.003
        // even at 5 um, and >0.07 at 20 um, so cannot pass this energy bound.
        close(integrated[0], 1.0, 0.00015);
        close(integrated[1], expected_moment, 0.00015);
        assert!(worst < 0.002);
        println!(
            "diameter={diameter}: disc_integral={:.9}, table_integral={:.9}, table_mean={:.9}, expected_mean={expected_moment:.9}, peak={centre:.6}, angle_relative_error={worst:.9}",
            integral[0], integrated[0], integrated[1]
        );
        tables.push(table);
    }
    let blend = sphere(
        |s2| 0.37 * table_phase(&tables[0], s2) + 0.63 * table_phase(&tables[2], s2),
        100_000,
    );
    close(blend[0], 1.0, 0.00015);
}
