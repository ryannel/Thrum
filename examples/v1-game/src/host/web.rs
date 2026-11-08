//! The V1 game's side of the engine's browser shell: its world lifecycle and its CPU edits.
//!
//! Every edit here changes world or presentation-policy state only, so it stays allowed while
//! the device is lost and the replacement draws it. Scene edits compare through the shell's
//! read access first, because the shell's mutable source access resets presentation history.

use std::time::Duration;

use thrum_render::synthetic::{CullMode, DisplayOverlay};

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
        shell.source_mut().camera_pitch = pitch;
    }
    Ok(())
}

/// Turn the game camera.
/// # Errors
/// Refuses a nonfinite yaw or one outside ±π radians.
pub(crate) fn set_camera_yaw(shell: &mut Shell, radians: f32) -> Result<(), String> {
    if let Some(yaw) = camera_yaw_edit(shell.host().source(), radians)? {
        shell.source_mut().camera_yaw = yaw;
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
pub(crate) fn camera_culling(shell: &Shell) -> CullMode {
    shell.host().camera_culling()
}

/// Optional scene views shed and undrained profiler rows discarded by the engine.
pub(crate) fn dropped(shell: &Shell) -> (u64, u64) {
    (shell.host().dropped_views(), shell.host().dropped_reports())
}
