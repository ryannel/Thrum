//! M1.5: the stopping rule and the report; a solve that runs out of steps is refused and
//! writes nothing; the work follows the water, not the rectangle; and the same input gives the
//! same bytes on one thread and on eight.
mod support;
use support::valley;
use thrum_river_model::{Profile, RiverError, key, solve, write};

#[test]
fn a_solve_stops_on_its_tolerances_and_reports_its_work() {
    let (grid, declaration) = valley::valley(0.25);
    let profile = Profile::default();
    let s = solve(&grid, &declaration, &profile).unwrap();
    let r = &s.report;
    println!("{r:?}");
    assert!(r.level_rate < profile.level_rate && r.imbalance < profile.imbalance);
    assert!(
        r.steps > 0 && r.steps.is_multiple_of(profile.check_every) && r.steps < profile.step_budget
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

#[test]
fn doubling_the_rectangle_round_the_valley_barely_changes_its_work() {
    let (grid, declaration) = valley::valley_in(0.25, 0.0, 14.0);
    let small = solve(&grid, &declaration, &Profile::default())
        .unwrap()
        .report;
    let (grid, declaration) = valley::valley_in(0.25, -7.0, 21.0);
    let large = solve(&grid, &declaration, &Profile::default())
        .unwrap()
        .report;
    let change = large.cell_updates as f64 / small.cell_updates as f64 - 1.0;
    println!(
        "{} cells: {} updates in {} steps; {} cells: {} updates in {} steps ({:+.2}%)",
        valley::valley_in(0.25, 0.0, 14.0).0.heights().len(),
        small.cell_updates,
        small.steps,
        grid.heights().len(),
        large.cell_updates,
        large.steps,
        100.0 * change
    );
    assert!(change.abs() < 0.05);
}

#[test]
fn the_same_input_gives_the_same_bytes_on_one_thread_and_on_eight() {
    let (grid, declaration) = valley::valley(0.25);
    let bytes = |threads| {
        let profile = Profile {
            threads,
            ..Profile::default()
        };
        let s = solve(&grid, &declaration, &profile).unwrap();
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
