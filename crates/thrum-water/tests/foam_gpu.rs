//! Whitewater transport and endpoint transitions on the submitted Metal field.
use crate::interaction_fixture;

use interaction_fixture::{Fixture, current, plane_beach, terrain};
use thrum_sea_model::Component;

#[derive(Debug)]
struct Patch {
    mass: f64,
    centre: [f64; 2],
    width: f64,
}

fn patch(fixture: &Fixture, stock: usize) -> Patch {
    let (witness, cells) = fixture.foam();
    let side = usize::try_from(witness.side).expect("foam grid");
    let plane = witness.banks[1] * 2 * side * side;
    let spacing = f64::from(witness.spacing[0]);
    let origin = witness.origins[1][0].map(f64::from);
    let mut mass = 0.0;
    let mut first = [0.0; 2];
    let mut second = 0.0;
    for (index, value) in cells[plane..plane + side * side].iter().enumerate() {
        let value = f64::from(value[stock]);
        let index = u32::try_from(index).expect("foam cell index");
        let position = [index % witness.side, index / witness.side];
        let [x, z] =
            std::array::from_fn(|axis| origin[axis] + (f64::from(position[axis]) + 0.5) * spacing);
        mass += value;
        first[0] += value * x;
        first[1] += value * z;
        second += value * (x * x + z * z);
    }
    let centre = first.map(|sum| sum / mass);
    let width = (second / mass - centre[0].powi(2) - centre[1].powi(2))
        .max(0.0)
        .sqrt();
    Patch {
        mass,
        centre,
        width,
    }
}

fn seed(fixture: &Fixture, spot: [f32; 2], stock: usize) {
    fixture.seed_foam(0, |p| {
        let mut stocks = [0.0; 4];
        if (p[0] - spot[0]).abs().max((p[1] - spot[1]).abs()) <= 0.25 {
            stocks[stock] = 0.25;
        }
        stocks
    });
}

fn aged_mass(before: f64, ticks: u32) -> f64 {
    before * (-(Fixture::STEP * ticks).as_secs_f64() / 13.0).exp()
}

fn assert_age(after: &Patch, before: &Patch, ticks: u32) {
    let expected = aged_mass(before.mass, ticks);
    assert!(
        (after.mass - expected).abs() < before.mass * 0.000_015,
        "{after:?}, expected mass {expected}"
    );
}

#[test]
fn wind_transport_and_shift_keep_the_interior_patch() {
    let Some(mut fixture) = Fixture::new_foam(terrain(|_| -8.0), None, [9.0, -2.0]) else {
        return;
    };
    fixture.draw(0);
    assert!(fixture.foam().1.iter().flatten().all(|value| *value == 0.0));
    seed(&fixture, [-4.0, 0.0], 2);
    let before = patch(&fixture, 2);
    let (witness, _) = fixture.foam();
    fixture.move_camera(-4.0);
    fixture.draw(8);
    let after = patch(&fixture, 2);
    let shifted = fixture.foam().0;
    assert_ne!(shifted.origins[1], witness.origins[1]);
    assert_age(&after, &before, 8);
    for (axis, wind) in [9.0, -2.0].into_iter().enumerate() {
        let expected = before.centre[axis] + 0.027 * wind * (Fixture::STEP * 8).as_secs_f64();
        assert!(
            (after.centre[axis] - expected).abs() < 0.000_04,
            "{after:?}, expected {expected}"
        );
    }
    assert!(after.width > before.width);
    for tick in (16..=8 * 151).step_by(8) {
        fixture.draw(tick);
    }
    let faded = patch(&fixture, 2);
    assert!(faded.mass < before.mass * 0.22);
    eprintln!(
        "Metal wind, shift and dispersal: seed {before:?}, shifted {after:?}, 20 s {faded:?}"
    );
}

#[test]
fn chart_current_carries_material_and_a_dry_patch_only_ages() {
    if let Some(mut fixture) = Fixture::new_foam(current(1.8), None, [0.0; 2]) {
        fixture.draw(0);
        seed(&fixture, [-4.0, 0.0], 2);
        let before = patch(&fixture, 2);
        for tick in (8..=64).step_by(8) {
            fixture.draw(tick);
        }
        let after = patch(&fixture, 2);
        let displacement = after.centre[0] - before.centre[0];
        let expected = 1.8 * (Fixture::STEP * 64).as_secs_f64();
        assert_age(&after, &before, 64);
        assert!(
            (displacement - expected).abs() < 0.000_1,
            "{after:?}, measured {displacement}"
        );
        eprintln!("Metal chart transport: {displacement:.6} m, expected {expected:.6} m");
    }
    if let Some(mut fixture) = Fixture::new_foam(terrain(|x| 0.1 * x), None, [18.0, 6.0]) {
        fixture.draw(0);
        seed(&fixture, [3.0, 0.0], 2);
        let before = patch(&fixture, 2);
        fixture.draw(8);
        let after = patch(&fixture, 2);
        assert_age(&after, &before, 8);
        assert!((after.centre[0] - before.centre[0]).abs() < 1e-8);
        assert!((after.centre[1] - before.centre[1]).abs() < 1e-8);
        assert!((after.width - before.width).abs() < 1e-8);
        eprintln!("Metal stranded film: {before:?} to {after:?}");
    }
}

#[test]
fn a_regular_wave_carries_foam_up_a_plane_beach() {
    let wave = Component {
        wave_vector: [std::f64::consts::TAU / 36.0, 0.0],
        amplitude: 0.8,
        phase: 1.1,
    };
    let Some(mut fixture) = Fixture::new_foam(plane_beach(), Some(wave), [0.0; 2]) else {
        return;
    };
    fixture.draw(0);
    seed(&fixture, [-0.25, 0.0], 3);
    let before = patch(&fixture, 3);
    // No per-frame observation is needed between seeding and the final stock readback.
    for tick in (4..=40).step_by(4) {
        fixture.draw(tick);
    }
    let after = patch(&fixture, 3);
    let delta = after.centre[0] - before.centre[0];
    assert!(delta > 0.12, "swash displacement {delta}, {after:?}");
    let expected = before.mass * (-(Fixture::STEP * 40).as_secs_f64() / 8.0).exp();
    assert!(
        (after.mass - expected).abs() < 0.000_1,
        "{after:?}, expected {expected}"
    );
    eprintln!("Metal plane beach with one regular wave: upslope displacement {delta:.4} m");
}

#[test]
fn eight_tick_hitch_ages_and_transports_once_and_only_submissions_commit() {
    let Some(mut fixture) = Fixture::new_foam(terrain(|_| -8.0), None, [8.0, 0.0]) else {
        return;
    };
    fixture.draw(0);
    seed(&fixture, [-6.0, 0.0], 2);
    let before = patch(&fixture, 2);
    let old = fixture.foam();
    fixture.discard(4);
    fixture.capture();
    let unchanged = fixture.foam();
    assert_eq!(unchanged.0.report.ticks, [0; 2]);
    assert_eq!(old.1, unchanged.1);
    fixture.draw(8);
    let hitched = patch(&fixture, 2);
    assert_age(&hitched, &before, 8);
    assert_eq!(fixture.foam().0.report.ticks, [0, 8]);
    fixture.draw(8);
    assert_eq!(fixture.foam().0.report.ticks, [0, 8]);
    fixture.draw(20);
    assert_eq!(fixture.foam().0.report.dropped_ticks, 4);
    fixture.draw(1);
    assert_eq!(fixture.foam().0.report.ticks, [1; 2]);
    assert!(fixture.foam().1.iter().flatten().all(|value| *value == 0.0));
    fixture.timeline(2);
    fixture.draw(1);
    seed(&fixture, [-6.0, 0.0], 2);
    for tick in 2..=9 {
        fixture.draw(tick);
    }
    let stepped = patch(&fixture, 2);
    assert!(
        (hitched.mass - stepped.mass).abs() < before.mass * 0.000_015,
        "{hitched:?} vs {stepped:?}"
    );
    assert!((hitched.centre[0] - stepped.centre[0]).abs() < 0.000_04);
    assert!(
        (hitched.centre[0] - before.centre[0] - 0.027 * 8.0 * (Fixture::STEP * 8).as_secs_f64())
            .abs()
            < 0.000_04
    );
    assert!(hitched.mass < before.mass);
    fixture.timeline(3);
    fixture.draw(9);
    assert_eq!(fixture.foam().0.report.ticks, [9; 2]);
    assert!(fixture.foam().1.iter().flatten().all(|value| *value == 0.0));
    seed(&fixture, [-6.0, 0.0], 2);
    fixture.replace_device();
    fixture.draw(10);
    assert_eq!(fixture.foam().0.report.ticks, [10; 2]);
    assert!(fixture.foam().1.iter().flatten().all(|value| *value == 0.0));
    eprintln!("Metal hitch: seed {before:?}, eight at once {hitched:?}, eight singly {stepped:?}");
}

#[test]
fn air_feeds_both_films_over_a_hitch() {
    let Some(mut fixture) = Fixture::new_foam(terrain(|_| -8.0), None, [0.0; 2]) else {
        return;
    };
    let measure = |fixture: &mut Fixture, run: &[u64], air| {
        fixture.draw(0);
        fixture.seed_foam(0, |[x, z]| {
            if (x + 4.0).abs().max(z.abs()) <= 0.25 {
                [air, 0.16, 0.10, 0.0]
            } else {
                [0.0; 4]
            }
        });
        for &tick in run {
            fixture.draw(tick);
        }
        [0, 1, 2].map(|stock| patch(fixture, stock).mass / 4.0)
    };
    let hitched = measure(&mut fixture, &[8], 0.30);
    let stepped = measure(&mut fixture, &[1, 2, 3, 4, 5, 6, 7, 8], 0.30);
    let without_air = measure(&mut fixture, &[8], 0.0);
    for (once, repeated) in hitched.into_iter().zip(stepped) {
        assert!(
            (once - repeated).abs() <= 0.000_003,
            "{hitched:?} vs {stepped:?}"
        );
    }
    assert!(hitched[0] < 0.30 && hitched[1] > without_air[1]);
    assert!(without_air[2] > aged_mass(0.10, 8));
    assert!(hitched[2] > without_air[2] + 0.000_05);
    assert!(0.60 * hitched[0] + hitched[1] + hitched[2] < 0.60 * 0.30 + 0.16 + 0.10);
    eprintln!(
        "Metal air, fresh and aged stocks: hitch {hitched:?}, single steps {stepped:?}, no air {without_air:?}"
    );
}
