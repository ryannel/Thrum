//! M1.5: the stopping rule and the report; a solve that runs out of steps is refused and
//! writes nothing; the work follows the water, not the rectangle; and the same input gives the
//! same bytes on one thread and on eight.
use crate::support;
use support::valley;
use thrum_river_model::{
    Declaration, Grid, Manning, Profile, Receiver, RiverError, Source, key, solve, write,
};

#[test]
fn a_solve_stops_on_its_tolerances_and_reports_its_work() {
    let (grid, declaration) = support::section::short_reach();
    let profile = Profile::default();
    let s = solve(&grid, &declaration, &profile).unwrap();
    let r = &s.report;
    println!("{r:?}");
    assert!(r.level_rate < profile.level_rate && r.imbalance < profile.imbalance);
    assert!(
        r.steps > profile.check_every
            && r.steps.is_multiple_of(profile.check_every)
            && r.steps < profile.step_budget
    );
    assert!(r.cell_updates >= r.steps && r.seconds > 0.0 && r.wet_cells > 0);
    assert!((r.discharge - valley::DISCHARGE).abs() < 1e-12);
}

#[test]
fn a_solve_that_runs_out_of_steps_is_refused_by_name_and_writes_nothing() {
    let (grid, declaration) = valley::valley(0.25);
    let profile = Profile {
        step_budget: 500,
        ..Profile::default()
    };
    let err = solve(&grid, &declaration, &profile).unwrap_err();
    println!("{err}");
    assert!(
        matches!(&err, RiverError::NotConverged { name, steps, .. } if name == "valley" && *steps == 500)
    );
}

/// The budget is a hard limit: a final batch shorter than the check interval, or an interval
/// longer than the whole budget, never steps past it. A solve either reaches its tolerances
/// inside the budget or is refused at exactly the budget, and one that converges on its last
/// allowed step still succeeds.
///
/// The rule is the stepping loop's, whatever the grid, so the test solves the valley at 1 m
/// cells: 672 cells that converge in about 2,400 steps, which is still past every fixed budget
/// below, against 10,752 cells and about 8,000 steps at 0.25 m.
#[test]
fn no_step_is_taken_beyond_the_budget() {
    let (grid, declaration) = valley::valley(1.0);
    let run = |step_budget, check_every| {
        let profile = Profile {
            step_budget,
            check_every,
            ..Profile::default()
        };
        match solve(&grid, &declaration, &profile) {
            Ok(s) => Ok(s.report.steps),
            Err(RiverError::NotConverged { name, steps, .. }) if name == "valley" => Err(steps),
            Err(e) => panic!("budget {step_budget}, interval {check_every}: {e}"),
        }
    };
    assert_eq!(run(1, 10_000), Err(1));
    assert_eq!(run(1, 10), Err(1));
    assert_eq!(run(505, 10), Err(505));
    let needed = run(400_000, 10).unwrap();
    assert_eq!(run(needed / 2, 1_000_000), Err(needed / 2));
    assert_eq!(run(needed / 2 + 3, 10), Err(needed / 2 + 3));
    assert_eq!(run(needed, 10), Ok(needed));
    // Around the converging step, every outcome stays inside its budget.
    for budget in needed - 12..needed {
        for interval in [7, 10, 1_000_000] {
            match run(budget, interval) {
                Ok(steps) => assert!(steps <= budget, "{steps} > {budget}"),
                Err(steps) => assert_eq!(steps, budget),
            }
        }
    }
}

#[test]
fn doubling_the_rectangle_round_the_valley_barely_changes_its_work() {
    let dx = 0.5;
    let (grid, declaration) = valley::valley(dx);
    let size = grid.heights().len();
    let small = solve(&grid, &declaration, &Profile::default()).unwrap();
    let (padded, _) = valley::valley_in(dx, -7.0, 21.0);
    assert_eq!(padded.heights().len(), 2 * size);
    // The only change is extra dry rows of ground on either side of the reach.
    let offset = padded.cell(grid.origin()).unwrap();
    assert_eq!(padded.point(offset), grid.origin());
    assert_eq!(&padded.heights()[offset..offset + size], grid.heights());
    let large = solve(&padded, &declaration, &Profile::default()).unwrap();
    assert!(large.wet[..offset].iter().all(|&wet| !wet));
    assert!(large.wet[offset + size..].iter().all(|&wet| !wet));
    assert_eq!(&large.wet[offset..offset + size], small.wet);
    let (small, large) = (small.report, large.report);
    let change = large.cell_updates as f64 / small.cell_updates as f64 - 1.0;
    println!(
        "{} cells: {} updates in {} steps; {} cells: {} updates in {} steps ({:+.2}%)",
        size,
        small.cell_updates,
        small.steps,
        padded.heights().len(),
        large.cell_updates,
        large.steps,
        100.0 * change
    );
    assert!(change.abs() < 0.05);
}

/// A short reach across a wide channel, with held water at each end. The small
/// transverse bed change also gives the solver lateral discharge to reproduce.
fn wide_reach() -> (Grid, Declaration) {
    const NX: usize = 12;
    const NZ: usize = 94;
    let mut heights = vec![2.0; NX * NZ];
    for z in 1..NZ - 1 {
        for x in 1..NX - 1 {
            let cross_slope = if z < NZ / 2 { 0.0 } else { 0.01 };
            heights[z * NX + x] = -0.01 * x as f64 + cross_slope;
        }
    }
    let grid = Grid::new([0.0; 2], 0.5, [NX, NZ], heights).expect("wide reach grid");
    let declaration = Declaration {
        name: "wide reach".into(),
        springs: vec![],
        sources: vec![Source {
            name: "head".into(),
            seed: [0.5, 1.0],
            extent: [[0.25, 0.0], [0.75, 47.0]],
            level: 0.44,
            range: [0.44; 2],
        }],
        receivers: vec![Receiver {
            name: "pool".into(),
            seed: [5.0, 1.0],
            extent: [[4.75, 0.0], [5.25, 47.0]],
            level: 0.35,
        }],
        manning: Manning::Uniform(0.04),
    };
    (grid, declaration)
}

#[test]
fn the_same_input_gives_the_same_bytes_on_one_thread_and_on_eight() {
    let (grid, declaration) = wide_reach();
    let bytes = |threads| {
        let profile = Profile {
            threads,
            ..Profile::default()
        };
        let s = solve(&grid, &declaration, &profile).unwrap();
        let r = &s.report;
        // The report sums the exact active span count of each step. An average
        // above the stepper's 1,024-cell cutoff proves it took the parallel path
        // with eight threads. Rayon can bisect the 94 rows into 47-row
        // chunks (minimum 42), and neither half can contain 1,024 cells.
        assert!(r.cell_updates > 1_024 * r.steps, "{r:?}");
        assert!(s.state.qx.iter().any(|q| q.abs() > 1e-6));
        assert!(s.state.qz.iter().any(|q| q.abs() > 1e-6));
        assert!(s.loss.iter().any(|&loss| loss > 0.0));
        println!(
            "{threads} threads: {} steps, {} cell updates, {} wet cells",
            r.steps, r.cell_updates, r.wet_cells,
        );
        let mut out = write(&s.file(key(&grid, &declaration, &profile)).unwrap());
        for v in s
            .state
            .depth
            .iter()
            .chain(&s.state.qx)
            .chain(&s.state.qz)
            .chain(&s.loss)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    };
    let one = bytes(1);
    assert_eq!(one, bytes(1));
    assert_eq!(one, bytes(8));
}
