//! Shared profiler control declarations. Live windows are owned by Mac and web shells.
use thrum_controls::{ControlError, Controls};

#[cfg(any(target_os = "macos", target_arch = "wasm32", test))]
mod window;
#[cfg(any(target_os = "macos", target_arch = "wasm32", test))]
pub(crate) use window::ProfileWindow;

const TOGGLE: &str = "profiler.enabled";
const TABLE: &str = "profiler.rows";

/// Declare the shared profiler group, off by default. Shells supply its rows.
/// # Errors
/// Refuses keys already declared by the app.
pub fn declare(controls: &mut Controls) -> Result<(), ControlError> {
    controls.toggle(TOGGLE, "Frame profiler", false)?;
    let table = controls.table(TABLE, "Recent frame timings")?;
    controls.show_rows(table, vec![("Profiler".into(), "off".into())]);
    Ok(())
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
pub(crate) fn enabled(controls: &Controls) -> Option<bool> {
    controls
        .find(TOGGLE)
        .map(|id| matches!(controls.value(id), thrum_controls::Value::Flag(true)))
}

#[cfg(target_os = "macos")]
pub(crate) fn revision(controls: &Controls) -> u64 {
    controls
        .find(TOGGLE)
        .map_or(0, |id| controls.revision_of(id))
}
