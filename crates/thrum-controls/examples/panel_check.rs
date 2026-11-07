//! Build a demo panel on the main thread, drive it through AppKit's target/action path,
//! and write before/after PNGs for visual review: `cargo run -p thrum-controls --example
//! panel_check -- <output-dir>`. Works while the Mac is locked.

#[cfg(target_os = "macos")]
fn main() -> Result<(), String> {
    use thrum_controls::{Controls, Display, macos::MacPanel};

    let output = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: panel_check <output-dir>")?,
    );
    std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
    let mut controls = Controls::new();
    let error = |error: thrum_controls::ControlError| error.to_string();
    let time = controls
        .slider("sky.solar_time", "Solar time", 0.0..=1.0)
        .default(0.5)
        .step(1.0 / 2400.0)
        .display(Display::clock())
        .add()
        .map_err(error)?;
    let one_degree = 1.0_f32.to_radians();
    let yaw = controls
        .slider(
            "camera.yaw",
            "Look around",
            -std::f32::consts::PI..=std::f32::consts::PI,
        )
        .default(0.0)
        .step(one_degree)
        .display(Display::degrees())
        .add()
        .map_err(error)?;
    let paused = controls
        .toggle("world.paused", "Pause world", false)
        .map_err(error)?;
    let sun = controls
        .readout("sky.sun_elevation", "Sun elevation", Display::degrees())
        .map_err(error)?;
    controls.show(sun, 45.0_f32.to_radians());
    let mut panel = MacPanel::new(&controls, "Thrum demo", None)?;
    std::fs::write(output.join("panel-initial.png"), panel.render_png()?)
        .map_err(|error| error.to_string())?;
    // A person drags solar time to 18:29 and yaw past its end, and presses Pause.
    let moved = [
        panel.simulate(time.id(), 18.48),
        panel.simulate(yaw.id(), 200.0),
        panel.simulate(paused.id(), 1.0),
    ];
    let applied = panel.apply_edits(&mut controls);
    // The game then publishes a new reading, as it would after a frame.
    controls.show(sun, (-2.0_f32).to_radians());
    panel.sync(&controls);
    std::fs::write(output.join("panel-edited.png"), panel.render_png()?)
        .map_err(|error| error.to_string())?;
    let report = format!(
        "{{\"moved\":{moved:?},\"applied\":{applied},\"solar_time\":\"{}\",\"yaw\":\"{}\",\"paused\":{},\"clamped_edits\":{}}}",
        controls.formatted(time.id()),
        controls.formatted(yaw.id()),
        controls.is_on(paused),
        controls.clamped_edits()
    );
    std::fs::write(output.join("panel-check.json"), &report).map_err(|error| error.to_string())?;
    if applied != 3 || controls.formatted(time.id()) != "18:29" || !controls.is_on(paused) {
        return Err(format!("panel edits did not reach the model: {report}"));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {}
