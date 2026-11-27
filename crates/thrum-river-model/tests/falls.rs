//! M2.2: water leaves a brink freely and enters the solve again where its path lands. The
//! expected depths are part 4 of `tests/fixtures/rivers/river_proof.py`, worked out
//! here: the critical depth `(q^2/g)^(1/3)`, Manning's normal depth `(q n / sqrt(S))^0.6`, and
//! the launch at 0.715 of the critical depth where the flow above is slow. A lower dry cell in
//! place of the lip's face, as part 4 shows, draws the lip down to about a fifth of the critical
//! depth, and fails the lip's bound.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it measured"
)]
use crate::support;
use support::fall::{Channel, Cliff, Foot, WIDTH, channel, critical, normal, through};
use thrum_river_model::{LAUNCH_SHARE, Profile, Solution, solve};
use thrum_water_model::{Below, EndKind};

const DX: f64 = 0.25;
const N: f64 = 0.04;
const Q: f64 = 0.15;

fn profile() -> Profile {
    Profile {
        level_rate: 1e-7,
        imbalance: 1e-4,
        ..Profile::default()
    }
}

fn fall(slope: f64, cliff: Cliff, foot: Foot) -> (Channel, Solution) {
    let c = channel(DX, 80, slope, Q, N, cliff, foot);
    let s = solve(&c.grid, &c.declaration, &profile()).unwrap();
    (c, s)
}

const CLIFF: Cliff = Cliff {
    height: 3.0,
    face_cells: 0,
};
const POOL: Foot = Foot::Pool {
    cells: 16,
    depth: 0.5,
};

/// The lip cells' depths across the channel.
fn lip_depths(c: &Channel, s: &Solution) -> Vec<f64> {
    c.rows().map(|z| s.state.depth[c.cell(c.lip, z)]).collect()
}

#[test]
fn above_a_lip_where_the_flow_is_slow_the_last_cell_stands_near_the_critical_depth() {
    let (c, s) = fall(0.005, CLIFF, POOL);
    let hc = critical(Q);
    let froude = Q / normal(Q, N, 0.005) / (9.81 * normal(Q, N, 0.005)).sqrt();
    assert!(froude < 1.0);
    for h in lip_depths(&c, &s) {
        assert!((0.95..=1.25).contains(&(h / hc)), "lip at {:.3} hc", h / hc);
    }
    let fall = &s.falls[0];
    assert_eq!(fall.segments.len(), WIDTH);
    for g in &fall.segments {
        // The launch is 0.715 of the critical depth of the face's own discharge.
        assert!((g.launch_depth / (LAUNCH_SHARE * critical(g.discharge)) - 1.0).abs() < 1e-12);
        assert!((g.launch_speed * g.launch_depth / g.discharge - 1.0).abs() < 1e-12);
        assert!((g.discharge / Q - 1.0).abs() < 0.01, "q {}", g.discharge);
    }
    println!(
        "slow (Froude {froude:.2} upstream): lip at {:.3} of the critical depth; launch {:.4} m",
        lip_depths(&c, &s)[2] / hc,
        fall.segments[2].launch_depth
    );
}

#[test]
fn above_a_lip_where_the_flow_is_fast_the_last_cell_stands_at_the_flows_own_depth() {
    let slope = 0.05;
    let (c, s) = fall(slope, CLIFF, POOL);
    let own = normal(Q, N, slope);
    let froude = Q / own / (9.81 * own).sqrt();
    assert!(froude > 1.0);
    for h in lip_depths(&c, &s) {
        assert!(
            (h / own - 1.0).abs() < 0.1,
            "lip at {:.3} of normal depth",
            h / own
        );
    }
    for g in &s.falls[0].segments {
        // The lip's cell is half a cell up the channel from the middle of its face.
        let m = g.centre();
        let lip = s.state.depth[c.grid.cell([m[0] - 0.5 * DX, m[1]]).unwrap()];
        assert!(
            (g.launch_depth / lip - 1.0).abs() < 1e-12,
            "launch at the lip's own depth"
        );
    }
    println!(
        "fast (Froude {froude:.2} upstream): lip at {:.3} of the normal depth, {:.3} of the critical",
        lip_depths(&c, &s)[2] / own,
        lip_depths(&c, &s)[2] / critical(Q)
    );
}

#[test]
fn a_lower_dry_cell_in_place_of_the_lips_face_fails_the_lips_depth() {
    // The same bed, solved with no brink: the 3 m drop is an ordinary face to a dry cell.
    let c = channel(DX, 80, 0.005, Q, N, CLIFF, POOL);
    let no_brink = Profile {
        least_fall: 1e3,
        ..profile()
    };
    let s = solve(&c.grid, &c.declaration, &no_brink).unwrap();
    assert_eq!(s.falls, Vec::new());
    let hc = critical(Q);
    for h in lip_depths(&c, &s) {
        assert!(
            h / hc < 0.95,
            "without the brink the lip stands at {:.3} hc",
            h / hc
        );
    }
    let throw = LAUNCH_SHARE * hc / lip_depths(&c, &s)[2];
    println!(
        "a dry cell in place of the face: lip at {:.2} of the critical depth; a sheet launched from it \
         would be thrown {throw:.1} times too far",
        lip_depths(&c, &s)[2] / hc
    );
}

#[test]
fn the_reach_below_carries_the_discharge_of_the_reach_above() {
    let (c, s) = fall(
        0.005,
        CLIFF,
        Foot::Ground {
            slope: 0.02,
            cells: 60,
        },
    );
    let above = through(&s, c.lip - 20);
    let below = through(&s, c.foot + 40);
    let over: f64 = s.falls[0].segments.iter().map(|g| g.discharge * DX).sum();
    assert!(
        (below / above - 1.0).abs() < 0.01,
        "{above} above, {below} below"
    );
    assert!((over / above - 1.0).abs() < 0.01);
    println!("discharge {above:.5} m3/s above, {over:.5} over the lip, {below:.5} below");
}

#[test]
fn the_river_is_cut_into_reaches_at_its_fall_and_the_lower_reach_can_bank_on_the_rock() {
    // A face that slopes at 4 m over 2 m, so the cliff has cells on its face.
    let cliff = Cliff {
        height: 4.0,
        face_cells: 7,
    };
    let (c, s) = fall(
        0.005,
        cliff,
        Foot::Ground {
            slope: 0.02,
            cells: 60,
        },
    );
    assert_eq!(s.reach_names.len(), 2);
    let reaches = s.reaches().unwrap();
    let fall = &s.falls[0];
    assert_eq!(fall.above, s.reach_names[0]);
    let grid = &c.grid;
    let nx = grid.size()[0];
    for g in &fall.segments {
        // Lands on ground gentler than the steepest chart slope and starts the lower reach.
        let end = g.path.last().unwrap();
        assert_eq!(g.below, Below::Reach(s.reach_names[1].clone()));
        let landing = grid.cell(end.place).unwrap();
        assert_eq!(s.reach_of[landing], Some(1));
        assert!(landing % nx >= c.foot, "lands below the face");
    }
    let mut receiving_bank = 0;
    for b in s.brinks.iter().filter(|b| s.wet[b.cell]) {
        let below = b.below(grid);
        for r in &reaches {
            assert!(
                !(r.footprint[b.cell] && r.footprint[below]),
                "a chart crosses a brink"
            );
        }
        for f in b.face_cells(grid) {
            assert!(
                !reaches[0].footprint[f],
                "the upstream chart extends down the rock"
            );
            if reaches[1].footprint[f] {
                assert!(
                    grid.heights()[f] >= reaches[1].levels[f],
                    "receiving shore margin"
                );
                receiving_bank += 1;
            }
        }
    }
    // The reach above ends at the brink; the reach below has no source end.
    assert!(reaches[0].ends.iter().any(|e| e.kind == EndKind::Brink));
    assert!(reaches[1].ends.iter().all(|e| e.kind != EndKind::Source));
    assert!(reaches[1].ends.iter().any(|e| e.kind == EndKind::Mouth));
    // No cell is in two charts.
    for i in 0..grid.heights().len() {
        assert!(reaches.iter().filter(|r| r.footprint[i]).count() <= 1);
    }
    println!(
        "{} reaches; the fall lands at x = {:.2} m, {receiving_bank} face cells in the lower chart's bank",
        reaches.len(),
        fall.segments[0].path.last().unwrap().place[0]
    );
    assert!(
        receiving_bank > 0,
        "the receiving chart carries a bank onto the rock"
    );
}

#[test]
fn a_path_that_lands_in_a_declared_basin_ends_there() {
    let (c, s) = fall(0.005, CLIFF, POOL);
    assert_eq!(s.reach_names.len(), 1);
    for g in &s.falls[0].segments {
        assert_eq!(g.below, Below::Body("pool".into()));
        let end = g.path.last().unwrap();
        assert!((end.height - c.level).abs() < 1e-9);
        let p = end.place;
        assert!(c.basin[0][0] <= p[0] && p[0] <= c.basin[1][0]);
    }
    // The water that leaves over the lip reaches the pool: none lands in the solve.
    let below: f64 = (c.foot..c.columns().end)
        .flat_map(|x| c.rows().map(move |z| (x, z)))
        .map(|(x, z)| s.state.depth[c.cell(x, z)] * f64::from(u8::from(!s.is_held(c.cell(x, z)))))
        .sum();
    assert!(
        below < 1e-9,
        "no water in the solve below the fall: {below}"
    );
}
