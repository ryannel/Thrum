//! M1.3: depth never negative and never not a number over seeded beds; a spring's water wets
//! its way down from a dry start to the mouth; refusals by name and place; narrow stretches
//! reported by place.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::unwrap_used,
    reason = "seeded test arithmetic; places are exact cell centres"
)]
use thrum_river_model::{
    Declaration, Grid, Manning, Profile, Receiver, RiverError, Source, Spring, State, Stepper,
    solve,
};

/// SplitMix64, so every bed is the same on every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }
}

#[test]
fn depth_stays_positive_and_finite_over_seeded_beds_with_dry_patches_steps_and_sources() {
    let (nx, nz, dx) = (24, 14, 0.25);
    for seed in 0..200_u64 {
        let mut rng = Rng(seed);
        let tilt = 0.3 * rng.next();
        let steps_at = [rng.next() * 6.0, rng.next() * 6.0];
        let mut heights = vec![0.0; nx * nz];
        for (i, h) in heights.iter_mut().enumerate() {
            let (x, z) = ((i % nx) as f64 * dx, (i / nx) as f64 * dx);
            *h = -tilt * x + 0.08 * rng.next();
            for &s in &steps_at {
                if x > s {
                    *h -= 0.2;
                }
            }
            if rng.next() < 0.12 {
                *h += 0.5 * rng.next();
            }
            if ((x - 3.0).powi(2) + (z - 1.5).powi(2)) < 0.4 {
                *h += 0.8;
            }
        }
        let grid = Grid::new([0.0; 2], dx, [nx, nz], heights.clone()).unwrap();
        let mut state = State::at_rest(&grid, |_| -1e9);
        for i in 0..nx * nz {
            if rng.next() < 0.6 {
                state.depth[i] = 0.3 * rng.next() * rng.next();
                state.qx[i] = (rng.next() - 0.3) * 0.4 * state.depth[i];
                state.qz[i] = (rng.next() - 0.5) * 0.4 * state.depth[i];
            }
        }
        let n = 0.01 + 0.06 * rng.next();
        let mut stepper =
            Stepper::new(&grid, &Manning::Uniform(n), 0.45, 1 + (seed % 4) as usize).unwrap();
        for _ in 0..=(rng.next() * 3.0) as usize {
            let cell = (rng.next() * (nx * nz) as f64) as usize % (nx * nz);
            stepper.add_source(cell, 0.5 * rng.next());
        }
        for step in 0..300 {
            stepper.step(&mut state);
            for i in 0..nx * nz {
                assert!(
                    state.depth[i] >= 0.0
                        && state.depth[i].is_finite()
                        && state.qx[i].is_finite()
                        && state.qz[i].is_finite(),
                    "seed {seed}, step {step}, cell {i}: depth {} discharge {} {}",
                    state.depth[i],
                    state.qx[i],
                    state.qz[i]
                );
            }
        }
    }
}

/// A channel down a 1% slope between steep banks, `width(x)` cells wide, with a spring at its
/// head and a pit held as a lake at its foot. Returns the grid and declaration.
fn channel(width: impl Fn(usize) -> usize) -> (Grid, Declaration) {
    let (nx, nz, dx) = (120, 16, 0.25);
    let heights = (0..nx * nz)
        .map(|i| {
            let (x, z) = (i % nx, i / nx);
            let floor = 2.0 - 0.01 * x as f64 * dx;
            let w = width(x);
            let lo = (nz - w) / 2;
            if x >= nx - 4 && (1..nz - 1).contains(&z) && x < nx - 1 {
                0.5
            } else if x == 0 || x >= nx - 1 || z < lo || z >= lo + w {
                floor + 2.0
            } else {
                floor
            }
        })
        .collect();
    let grid = Grid::new([0.0; 2], dx, [nx, nz], heights).unwrap();
    let declaration = Declaration {
        name: "channel".into(),
        springs: vec![Spring {
            name: "spring".into(),
            point: [0.5, 2.0],
            discharge: 0.05,
            radius: 0.3,
        }],
        sources: vec![],
        receivers: vec![Receiver {
            name: "pit".into(),
            seed: [(nx - 3) as f64 * dx, 2.0],
            extent: [
                [(nx - 4) as f64 * dx - 0.01, 0.0],
                [nx as f64 * dx, nz as f64 * dx],
            ],
            level: 1.8 - 0.01 * 30.0,
        }],
        manning: Manning::Uniform(0.035),
    };
    (grid, declaration)
}

#[test]
fn below_a_spring_the_water_wets_its_way_down_dry_ground_to_the_mouth() {
    // A plain slope has no hollow, so the start leaves every cell below the spring dry.
    let (grid, declaration) = channel(|_| 6);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    let [nx, nz] = grid.size();
    for x in 2..nx - 4 {
        assert!((0..nz).any(|z| s.wet[z * nx + x]), "column {x} is dry");
    }
    let mouth: f64 = (0..nz).map(|z| s.x_flux[z * (nx + 1) + nx - 4]).sum();
    println!(
        "{} steps; into the mouth {mouth:.5} m3/s of 0.05",
        s.report.steps
    );
    assert!((mouth / 0.05 - 1.0).abs() < 0.01);
}

#[test]
fn each_stretch_narrower_than_four_cells_is_reported_by_place() {
    let (grid, declaration) = channel(|x| if (50..62).contains(&x) { 3 } else { 6 });
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    for n in &s.narrows {
        println!(
            "narrow: {:?}, {} cells, least {}",
            n.extent, n.cells, n.least_width
        );
    }
    let found = s.narrows.iter().any(|n| {
        n.extent[0][0] <= 50.0 * 0.25 + 0.3
            && n.extent[1][0] >= 61.0 * 0.25 - 0.3
            && n.least_width == 3
    });
    assert!(found, "the stretch from 12.5 m to 15.25 m is reported");
    // Every report lies in or beside the narrow stretch.
    assert!(
        s.narrows
            .iter()
            .all(|n| n.extent[0][0] >= 48.0 * 0.25 && n.extent[1][0] <= 64.0 * 0.25)
    );
}

#[test]
fn water_that_reaches_the_rectangles_edge_is_refused_by_name_and_place() {
    // The channel's floor runs out to the rectangle's side, 1 cm above the floor beside it.
    let (grid, declaration) = channel(|_| 6);
    let [nx, nz] = grid.size();
    let mut heights = grid.heights().to_vec();
    for x in 30..40 {
        for z in 0..nz {
            heights[z * nx + x] =
                2.0 - 0.01 * x as f64 * 0.25 + if (5..11).contains(&z) { 0.0 } else { 0.01 };
        }
    }
    let grid = Grid::new(grid.origin(), grid.spacing(), grid.size(), heights).unwrap();
    let err = solve(&grid, &declaration, &Profile::default()).unwrap_err();
    println!("{err}");
    let RiverError::ReachesEdge { name, place } = err else {
        panic!("{err}")
    };
    assert_eq!(name, "channel");
    assert!((7.5..=10.0).contains(&place[0]) && (place[1] == 0.0 || place[1] == 15.0 * 0.25));
}

#[test]
fn a_source_with_no_way_down_to_a_declared_body_is_refused_by_name() {
    // A ridge stands between the spring and the pit; the ground falls to the rectangle's head.
    let (grid, declaration) = channel(|_| 6);
    let [nx, nz] = grid.size();
    let mut heights = grid.heights().to_vec();
    for z in 0..nz {
        for x in 0..20 {
            heights[z * nx + x] = 1.5 + 0.02 * x as f64;
        }
        heights[z * nx + 20] = 5.0;
    }
    let grid = Grid::new(grid.origin(), grid.spacing(), grid.size(), heights).unwrap();
    let mut declaration = declaration;
    declaration.springs[0].point = [3.0, 2.0];
    let err = solve(&grid, &declaration, &Profile::default()).unwrap_err();
    println!("{err}");
    assert!(
        matches!(&err, RiverError::NoWayDown { name, place } if name == "spring" && *place == [3.0, 2.0])
    );
}

/// A lake at the head held behind a sill whose crest stands at 2.0 m, at the head of the channel.
fn lake_fed(range: [f64; 2]) -> (Grid, Declaration) {
    let (grid, mut declaration) = channel(|_| 6);
    let [nx, nz] = grid.size();
    let mut heights = grid.heights().to_vec();
    for z in 1..nz - 1 {
        for x in 1..12 {
            heights[z * nx + x] = 1.0;
        }
        heights[z * nx + 12] = if (5..11).contains(&z) { 2.0 } else { 4.0 };
    }
    let grid = Grid::new(grid.origin(), grid.spacing(), grid.size(), heights).unwrap();
    declaration.springs.clear();
    declaration.sources.push(Source {
        name: "upper lake".into(),
        seed: [1.0, 2.0],
        extent: [[0.0, 0.0], [2.9, 4.0]],
        level: range[1],
        range,
    });
    (grid, declaration)
}

#[test]
fn a_source_body_whose_range_reaches_under_its_outlets_sill_is_refused_by_name_and_place() {
    let (grid, declaration) = lake_fed([1.9, 2.1]);
    let err = solve(&grid, &declaration, &Profile::default()).unwrap_err();
    println!("{err}");
    let RiverError::BelowSill {
        name,
        place,
        sill,
        level,
    } = err
    else {
        panic!("{err}")
    };
    assert_eq!(name, "upper lake");
    assert!((sill - 2.0).abs() < 1e-12 && (level - 1.9).abs() < 1e-12);
    assert!(
        (place[0] - 3.0).abs() < 1e-9,
        "the sill at x = 3 m, not {place:?}"
    );
    // Over its sill through its whole range, the lake feeds the channel.
    let (grid, declaration) = lake_fed([2.05, 2.1]);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    println!("lake-fed: {:.4} m3/s over the sill", s.report.discharge);
    assert!(s.report.discharge > 0.0);
}
