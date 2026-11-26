//! CPU oracle for the real window and browser status-display captures.

#[cfg(target_os = "macos")]
mod support {
    pub(crate) mod status_cost;
}

use v1_game::interface_status::proof;

use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

use thrum_ui::Viewport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(args.next().ok_or("expected status store path")?);
    #[cfg(target_os = "macos")]
    if args.next().is_some_and(|arg| arg == "costs") {
        writeln!(
            io::stdout().lock(),
            "{}",
            support::status_cost::measure(&path)
        )?;
        return Ok(());
    }
    let store = thrum_asset::AssetStore::new(path);
    let (font, art) = proof::open(&store);
    // One line per sample, followed by a line with the cropped bounds and binary
    // RGBA and one-byte-per-pixel tolerance. The caller supplies submitted tick.
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let [extent, scale, insets, tick]: [serde_json::Value; 4] = serde_json::from_str(&line)?;
        let viewport = Viewport {
            extent: serde_json::from_value(extent)?,
            scale: serde_json::from_value(scale)?,
            insets: serde_json::from_value(insets)?,
        };
        let tick = serde_json::from_value(tick)?;
        let (reference, region) = proof::compose(viewport, tick, &font, &art);
        let (rgba, tolerance) = reference.crop(region);
        writeln!(
            output,
            "{}",
            serde_json::json!([region, rgba.len(), tolerance.len()])
        )?;
        output.write_all(&rgba)?;
        output.write_all(&tolerance)?;
        output.flush()?;
    }
    Ok(())
}
