//! M2.1: brinks are found from the bed alone. A face is a brink where the ground beyond it,
//! followed downhill, drops at least the least fall (0.75 m) at a slope steeper than the steepest
//! chart slope (1.5). A 3 m cliff is a brink along its whole lip; a 0.4 m ledge and a slope of 1
//! are not, and the water runs over them in the chart. The brinks are the same at every discharge.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unwrap_used,
    reason = "test arithmetic over cell indices; the test reports what it found"
)]
use crate::support;
use support::fall::{Cliff, Foot, LIP, WIDTH, channel};
use thrum_river_model::{Grid, Profile, Side, brinks, solve};

const DX: f64 = 0.25;

/// A plateau 4 m wide and 12 m long whose ground at x = `at` drops by `step(x)`: the bed is
/// `top - step(x)`, with no other change.
fn bed(step: impl Fn(f64) -> f64) -> Grid {
    let (nx, nz) = (48, 16);
    let heights = (0..nx * nz)
        .map(|i| 5.0 - step((i % nx) as f64 * DX))
        .collect();
    Grid::new([0.0; 2], DX, [nx, nz], heights).unwrap()
}

#[test]
fn a_3_m_cliff_is_a_brink_along_its_whole_lip() {
    // The ground drops 3 m between x = 5.0 and x = 5.25.
    let grid = bed(|x| if x > 5.1 { 3.0 } else { 0.0 });
    let found = brinks(&grid, &Profile::default());
    let [nx, nz] = grid.size();
    let lip = (5.0 / DX).round() as usize;
    assert_eq!(
        found.len(),
        nz,
        "one brink for each cell along the lip: {found:?}"
    );
    for (z, b) in found.iter().enumerate() {
        assert_eq!(b.cell, z * nx + lip);
        assert_eq!(b.side, Side::PlusX);
        assert!((b.drop - 3.0).abs() < 1e-12);
        assert_eq!(b.foot, z * nx + lip + 1);
        assert_eq!(b.face_cells(&grid), Vec::<usize>::new());
    }
    // A cliff whose face slopes at 2 over four cells is a brink at its top only.
    let grid = bed(|x| (2.0 * (x - 5.0)).clamp(0.0, 2.0));
    let found = brinks(&grid, &Profile::default());
    assert_eq!(found.len(), nz);
    for b in &found {
        assert_eq!(b.cell % nx, lip);
        assert!((b.drop - 2.0).abs() < 1e-12);
        assert_eq!(b.face_cells(&grid).len(), 3);
    }
    println!(
        "3 m cliff: {} brinks along a lip of {nz} cells",
        found.len()
    );
}

#[test]
fn a_ledge_of_0_4_m_and_a_slope_of_1_are_no_brinks() {
    // 0.4 m in one cell: steep (1.6), but under the least fall.
    let ledge = bed(|x| if x > 5.1 { 0.4 } else { 0.0 });
    assert_eq!(brinks(&ledge, &Profile::default()), Vec::new());
    // Ground falling 4 m at a slope of 1: far enough, but not steep.
    let slope = bed(|x| (x - 3.0).clamp(0.0, 4.0));
    assert_eq!(brinks(&slope, &Profile::default()), Vec::new());
    // Two ledges of 0.4 m a cell apart are no steep run of 0.8 m: a flat cell parts them.
    let steps = bed(|x| {
        if x > 5.6 {
            0.8
        } else if x > 5.1 {
            0.4
        } else {
            0.0
        }
    });
    assert_eq!(brinks(&steps, &Profile::default()), Vec::new());
}

/// A channel whose ground drops 0.4 m in one cell, then runs at a slope of 1 for 1 m, to a
/// pool: the water runs over both, in one chart.
#[test]
fn the_water_runs_over_a_ledge_and_a_slope_of_1_in_the_chart() {
    let c = channel(
        DX,
        40,
        0.01,
        0.15,
        0.04,
        Cliff {
            height: 0.4,
            face_cells: 0,
        },
        Foot::Ground {
            slope: 1.0,
            cells: 4,
        },
    );
    let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
    // The only brinks are the walls' tops; none lies in the channel.
    let nx = c.grid.size()[0];
    for b in &s.brinks {
        let (x, z) = (b.cell % nx, b.cell / nx);
        assert!(
            !(c.columns().contains(&x) && c.rows().contains(&z)),
            "{b:?}"
        );
        assert!(!s.wet[b.cell]);
    }
    assert_eq!(s.falls, Vec::new());
    let reach = s.reach().unwrap();
    // The ledge's foot and the slope's cells are wet and in the chart.
    for x in c.lip..c.foot + 4 {
        for z in c.rows() {
            let i = c.cell(x, z);
            assert!(s.wet[i], "cell ({x}, {z}) is wet");
            assert!(reach.footprint[i]);
        }
    }
    println!(
        "ledge of 0.4 m at {LIP} m and a slope of 1: one reach, {} wet cells",
        s.report.wet_cells
    );
}

#[test]
fn the_brinks_are_the_same_at_every_discharge() {
    let solve_at = |unit: f64| {
        let c = channel(
            DX,
            40,
            0.005,
            unit,
            0.04,
            Cliff {
                height: 3.0,
                face_cells: 0,
            },
            Foot::Pool {
                cells: 16,
                depth: 0.5,
            },
        );
        solve(&c.grid, &c.declaration, &Profile::default()).unwrap()
    };
    let low = solve_at(0.02);
    let high = solve_at(0.6);
    assert_eq!(low.brinks, high.brinks);
    assert_ne!(low.brinks.len(), 0);
    let ids = |s: &thrum_river_model::Solution| {
        s.falls
            .iter()
            .flat_map(|f| f.segments.iter().map(|g| g.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&low), ids(&high));
    assert_eq!(ids(&low).len(), WIDTH);
    println!(
        "{} brinks and the same {} fall segments at 0.02 and 0.6 m2/s",
        low.brinks.len(),
        ids(&low).len()
    );
}
