//! Times a solve of the M1.1 test valley and prints its record as JSON on one line.
//! `scripts/river_cost.py` runs it in a release build. Arguments: cell size (default 0.25) and
//! thread count (default 0, the machine's parallelism).
#[expect(dead_code, reason = "shared fixture")]
mod valley;
use std::io::Write;
use thrum_river_model::{Profile, solve};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let dx: f64 = args.next().map_or(Ok(0.25), |a| a.parse())?;
    let threads: usize = args.next().map_or(Ok(0), |a| a.parse())?;
    let (grid, declaration) = valley::valley(dx);
    let profile = Profile {
        threads,
        ..Profile::default()
    };
    let s = solve(&grid, &declaration, &profile)?;
    let r = &s.report;
    let [nx, nz] = grid.size();
    writeln!(
        std::io::stdout(),
        "{{\"cell_size_m\": {dx}, \"rectangle_cells\": {}, \"wet_cells\": {}, \"steps\": {}, \"cell_updates\": {}, \"seconds\": {:.4}, \"level_rate\": {:e}, \"imbalance\": {:e}, \"threads\": {}}}",
        nx * nz,
        r.wet_cells,
        r.steps,
        r.cell_updates,
        r.seconds,
        r.level_rate,
        r.imbalance,
        if threads == 0 {
            std::thread::available_parallelism().map_or(1, usize::from)
        } else {
            threads
        }
    )?;
    Ok(())
}
