//! Shipping-shaped G5 viewer process: maps a cooked store, renders at a fixed 120 Hz cadence and
//! swaps in new publications as they appear. Links no authoring code.

use std::io::Write;
use std::time::{Duration, Instant};

use asset_loop::{Viewer, ViewerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let store = args.next().ok_or("usage: asset-loop <store> [seconds]")?;
    let seconds: f64 = args.next().map_or(Ok(5.0), |value| value.parse())?;
    let started = Instant::now();
    let mut viewer = Viewer::new(ViewerConfig::demo(store))?;
    let period = Duration::from_secs_f64(1.0 / 120.0);
    let mut next = Instant::now();
    let mut first_visible = None;
    while started.elapsed().as_secs_f64() < seconds {
        let frame = viewer.frame(started.elapsed().as_secs_f32());
        if first_visible.is_none() && frame.generation > 0 {
            first_visible = Some(started.elapsed());
        }
        next += period;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    viewer.settle();
    let records = viewer.records();
    let records = records.lock().map_err(|_| "records poisoned")?;
    let mut cpu: Vec<f64> = records
        .frames
        .iter()
        .map(|f| f.cpu.as_secs_f64() * 1e3)
        .collect();
    cpu.sort_by(f64::total_cmp);
    let percentile = |per_mille: usize| {
        cpu.get(cpu.len().saturating_sub(1) * per_mille / 1000)
            .copied()
            .unwrap_or(0.0)
    };
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "frames {} swaps {} errors {}",
        records.frames.len(),
        records.swaps.len(),
        records.errors.len()
    )?;
    writeln!(
        out,
        "render cpu ms p50 {:.3} p99 {:.3} max {:.3}",
        percentile(500),
        percentile(990),
        percentile(1000)
    )?;
    writeln!(out, "first resident frame after start {first_visible:?}")?;
    for error in &records.errors {
        writeln!(out, "error: {error}")?;
    }
    Ok(())
}
