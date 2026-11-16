//! Plain executable used on the Mac and through simctl spawn.
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "vectors") {
        writeln!(
            io::stdout().lock(),
            "{}",
            serde_json::to_string(&thrum_core::profile::conformance_vectors())?
        )?;
        return Ok(());
    }
    let count = args.first().ok_or("body count required")?.parse()?;
    if args.get(1).is_some_and(|arg| arg == "costs") {
        writeln!(
            io::stdout().lock(),
            "{}",
            crowd_2d_run::measure_trace(
                count,
                args.iter().any(|a| a == "--mixed"),
                args.iter().any(|a| a == "--parallel")
            )
        )?;
        return Ok(());
    }
    let ticks = args.get(1).ok_or("tick count required")?.parse()?;
    let has = |flag| args.iter().any(|arg| arg == flag);
    let result = crowd_2d_run::trace(
        count,
        ticks,
        has("--straight"),
        has("--wire"),
        has("--control"),
        has("--mixed"),
        has("--parallel"),
    );
    writeln!(io::stdout().lock(), "{result}")?;
    Ok(())
}
