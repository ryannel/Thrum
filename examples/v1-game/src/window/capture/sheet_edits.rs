//! Save the prepared PNGs on an authoring thread against the capture's elapsed clock.

use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

pub(super) fn start(
    origin: Instant,
) -> Result<thread::JoinHandle<Result<Vec<Value>, String>>, String> {
    let staged = path("THRUM_SPRITE_SHEET_EDITS")?;
    let target = path("THRUM_SPRITE_SHEET_WATCHED_IMAGE")?;
    thread::Builder::new()
        .name("thrum-sprite-sheet-saves".into())
        .spawn(move || {
            let mut saves = Vec::new();
            for (period, file) in [(2, "recolour.png"), (4, "repack.png")] {
                let due = origin + Duration::from_secs(period);
                if let Some(wait) = due.checked_duration_since(Instant::now()) {
                    thread::sleep(wait);
                }
                let requested = origin.elapsed();
                std::fs::rename(staged.join(file), &target).map_err(|error| error.to_string())?;
                saves.push(json!({
                    "name": file,
                    "requested_ns": requested.as_nanos(),
                    "accepted_ns": origin.elapsed().as_nanos(),
                }));
            }
            Ok(saves)
        })
        .map_err(|error| error.to_string())
}

fn path(name: &str) -> Result<PathBuf, String> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("sprites-sheets needs {name}"))
}
