//! The V1 game's side of the engine's browser shell: its world lifecycle and its CPU edits.
//!
//! Every edit here changes world or presentation-policy state only, so it stays allowed while
//! the device is lost and the replacement draws it. Scene edits compare through the shell's
//! read access first, because the shell's mutable source access resets presentation history.

use std::time::Duration;

use thrum_render::synthetic::{CullingSelection, DisplayOverlay};

use super::{
    GameScene, GameSetup, Runtime, WorldPort, camera_pitch_edit, camera_yaw_edit, day_phase_edit,
};
use crate::SceneCamera;

/// The engine's browser shell over the V1 world and scene.
pub(crate) type Shell = thrum_app::web::WebShell<WorldPort, GameScene>;

/// The first device's generation; replacements count up from it.
pub(crate) const DEVICE_GENERATION: u64 = super::FrameHost::DEVICE_GENERATION;

impl thrum_app::web::WebWorld for WorldPort {
    fn pause(&mut self) -> Result<(), String> {
        WorldPort::pause(self)
    }
    fn resume_after_drain(&mut self, now: Duration) -> Result<(), String> {
        self.inline_lifecycle()?.resume_after_drain(now)
    }
}

/// Prepare the V1 runtime on the shell's first device.
/// # Errors
/// Refuses a mismatched device or game/engine preparation failure.
pub(crate) async fn prepare(
    gpu: thrum_app::web::WebGpu,
    setup: GameSetup,
) -> Result<Runtime, String> {
    let host = super::FrameHost::new_with_setup(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        gpu.frame_config,
        setup,
    )
    .await?;
    Ok(host.inner)
}

/// Committed solar time: midnight is zero, noon is one half.
pub(crate) fn time_of_day(shell: &Shell) -> f32 {
    super::time_of_day(shell.host())
}

/// Change the game-authored day phase.
/// # Errors
/// Refuses a nonfinite or out-of-range normalized time.
pub(crate) fn set_time_of_day(shell: &mut Shell, fraction: f32) -> Result<(), String> {
    let host = shell.host();
    if let Some(offset) = day_phase_edit(host.world().tick(), host.source(), fraction)? {
        shell.source_mut().day_phase_offset = offset;
    }
    Ok(())
}

/// Current presentation camera pitch and yaw, in radians.
pub(crate) fn camera_orientation(shell: &Shell) -> [f32; 2] {
    let source = shell.host().source();
    [source.camera_pitch, source.camera_yaw]
}

/// Change the game camera elevation.
/// # Errors
/// Refuses a nonfinite pitch outside the supported scene range.
pub(crate) fn set_camera_pitch(shell: &mut Shell, radians: f32) -> Result<(), String> {
    if let Some(pitch) = camera_pitch_edit(shell.host().source(), radians)? {
        shell.continuous_source_mut().camera_pitch = pitch;
    }
    Ok(())
}

/// Turn the game camera.
/// # Errors
/// Refuses a nonfinite yaw or one outside ±π radians.
pub(crate) fn set_camera_yaw(shell: &mut Shell, radians: f32) -> Result<(), String> {
    if let Some(yaw) = camera_yaw_edit(shell.host().source(), radians)? {
        shell.continuous_source_mut().camera_yaw = yaw;
    }
    Ok(())
}

/// Select a checked authored camera.
pub(crate) fn set_scene_camera(shell: &mut Shell, camera: SceneCamera) -> Result<(), String> {
    shell.source_mut().select_camera(camera)
}

/// Draw a display diagnostic in place of the tone-mapped scene.
pub(crate) fn set_display_overlay(shell: &mut Shell, overlay: DisplayOverlay) {
    shell.source_mut().display_overlay = overlay;
}

/// Validate and replace continuous local input.
/// # Errors
/// Refuses invalid controls.
pub(crate) fn set_controls(shell: &mut Shell, axes: [f32; 3]) -> Result<(), String> {
    shell.host().source().route_guard()?;
    shell.world_mut().set_controls(axes)
}

/// Position of the game fixture's controlled actor.
pub(crate) fn controlled_position(shell: &Shell) -> Option<[f64; 3]> {
    shell.host().world().controlled_position()
}

/// Latest committed authority tick.
pub(crate) fn tick(shell: &Shell) -> u64 {
    shell.host().world().tick()
}

/// The camera culling mode the prepared renderer runs.
pub(crate) fn camera_culling(shell: &Shell) -> CullingSelection {
    shell.host().culling_selection()
}

/// Optional scene views shed and undrained profiler rows discarded by the engine.
pub(crate) fn dropped(shell: &Shell) -> (u64, u64) {
    (shell.host().dropped_views(), shell.host().dropped_reports())
}

/// Install the moving route once the cooked valley's declared generation has drawn,
/// when the committed tick is the one before the route's origin. The browser admits
/// the route overlay only for this owned cooked route, as the native capture does.
/// # Errors
/// Refuses a route the game cannot install, or a cooked draw of another generation.
pub(crate) fn install_route(
    shell: &mut Shell,
    text: &str,
    drawn_generation: Option<u64>,
) -> Result<(), String> {
    let source = shell.host().source();
    source.route_guard()?;
    let camera = source
        .authored_camera()
        .ok_or("the moving route needs the cooked valley's authored camera")?;
    let tick_hz = thrum_core::ClockConfig::default().tick_hz.get();
    let route = crate::CameraRoute::parse(text, camera, tick_hz)?;
    if drawn_generation != Some(route.cooked_generation) {
        return Err(format!(
            "cooked generation {drawn_generation:?} has not drawn as the route's {}",
            route.cooked_generation
        ));
    }
    let tick = shell.host().world().tick();
    shell.source_mut().install_route(route, tick)
}

/// The route step target in page milliseconds from the clock origin: `(tick + 1 + φ)·P`.
pub(crate) fn route_step_ms(shell: &Shell, phi: f64) -> f64 {
    super::scene_source::route_step_ms(shell.host().world().tick(), phi)
}

/// Collect GPU completions without admitting time; whether a frame slot is free.
pub(crate) fn route_slot_ready(shell: &mut Shell) -> Result<bool, String> {
    shell.poll_completed()
}

/// Latest committed authority tick.
pub(crate) fn committed_tick(shell: &Shell) -> u64 {
    shell.host().world().tick()
}

/// Route identity for the page: counters, the committed solar hour, and every retained
/// GPU-complete sample at or after `from_tick` with its pose and rendered solar hour.
pub(crate) fn route_report(shell: &Shell, from_tick: u64) -> serde_json::Value {
    let source = shell.host().source();
    let status = shell.status();
    let (dropped_views, dropped_reports) = dropped(shell);
    let committed = time_of_day(shell) * 24.0;
    let samples: Vec<serde_json::Value> = shell
        .samples()
        .iter()
        .filter(|sample| sample.tick >= from_tick)
        .map(|sample| {
            let alpha = sample.presentation_alpha;
            let rendered = crate::environment::DayCycle::rendered_time_of_day(
                sample.tick,
                alpha,
                source.day_phase_offset,
            ) * 24.0;
            let pose = source.route_pose(sample.tick, alpha);
            serde_json::json!({
                "tick": sample.tick, "alpha": alpha, "alpha_bits": alpha.to_bits(),
                "submission": sample.submission_order,
                "history_sample": sample.history_identity[0], "history_predecessor": sample.history_identity[1],
                "rendered_solar_hours": rendered, "rendered_solar_bits": rendered.to_bits(),
                "eye": pose.map(|pose| pose.eye), "yaw": pose.map(|pose| pose.yaw),
                "pitch": pose.map(|pose| pose.pitch), "epoch": pose.map(|pose| pose.epoch),
            })
        })
        .collect();
    serde_json::json!({
        "tick": committed_tick(shell),
        "reconstruction": format!("{:?}",shell.host().reconstruction()),
        "committed_solar_hours": committed, "committed_solar_bits": committed.to_bits(),
        "callbacks": status.callbacks, "presented": status.presented,
        "timestamp_clamps": status.timestamp_clamps, "generation": status.generation,
        "unavailable": status.unavailable, "overload_events": status.overload_events,
        "dropped_views": dropped_views, "dropped_reports": dropped_reports,
        "extent": status.extent, "samples": samples,
    })
}
