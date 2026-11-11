//! The boundary between the generic window shell and one game.

use std::time::Instant;

use thrum_controls::Controls;
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{FrameHost, FrameWorld, SceneSource},
};
use winit::keyboard::KeyCode;

/// The platform's first device, queue and configured surface, lent to [`Game::prepare`].
pub struct Gpu<'a> {
    /// Device created by the shell's Metal recipe.
    pub device: &'a wgpu::Device,
    /// Queue paired with `device`.
    pub queue: &'a wgpu::Queue,
    /// Immutable capability record of `device`.
    pub capabilities: &'a DeviceCapabilities,
    /// Format and extent of the configured window surface.
    pub surface: SurfaceProfile,
}

/// A game's prepared frame runtime, handed to the render owner thread.
pub struct Prepared<W: FrameWorld, V: SceneSource<W::Page>> {
    /// The runtime host; its world should already run on its own lane.
    pub host: FrameHost<W, V>,
    /// Monotonic origin of the world clock; render times are measured from it.
    pub origin: Instant,
}

/// Lifecycle control of a world that runs on its own lane, used for pause and overload.
pub trait WorldLink {
    /// Request a transition without waiting for it.
    /// # Errors
    /// `Busy` while an earlier request or reply occupies the slot, or the world's fault.
    fn request_lifecycle(&self, request: WorldLifecycle) -> Result<(), WorldLaneFault>;
    /// Consume the acknowledgement of the requested transition, if it has arrived.
    /// # Errors
    /// A refused transition or a failed world owner.
    fn poll_lifecycle_reply(&self) -> Result<Option<WorldLifecycleReply>, WorldLaneFault>;
    /// The latest committed tick, or the world's overload or failure.
    /// # Errors
    /// A failed owner, fixed-clock overload or synchronization fault.
    fn check_health(&self) -> Result<u64, WorldLaneFault>;
}

/// Presentation edits carried from the main thread to the render owner with a frame request.
pub trait Edits: Copy + Default + Send + 'static {
    /// Combine pending edits; a newer value replaces an older one.
    #[must_use]
    fn merge(self, newer: Self) -> Self;
    /// Whether the edits change nothing.
    fn is_empty(&self) -> bool;
}

/// What the shell does after a game handles a key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyResponse {
    /// Nothing changed.
    Ignored,
    /// The controls model changed: take its edits and refresh the panel.
    Edited,
    /// Continuous world input changed: request a new frame.
    Input,
}

/// One game hosted by the shell. The shell owns the window, device, render thread, pause,
/// recovery and panel; the game owns its runtime, controls model and game keys.
///
/// The shell handles `R` while the GPU is unavailable, `P` when [`Game::world`] is present,
/// and `L` (reload `THRUM_PARAMS`, then [`Game::reload`]); every other key reaches [`Game::key`].
pub trait Game {
    /// The runtime's world adapter.
    type World: FrameWorld + Send + 'static;
    /// The runtime's game-owned scene source.
    type Scene: SceneSource<<Self::World as FrameWorld>::Page> + Send + 'static;
    /// Edits applied by the render owner before a frame.
    type Edits: Edits;
    /// A small value read on the render owner after each job and observed on the main thread.
    type Readback: Copy + Send + 'static;

    /// Prepare the runtime on the shell's first device. `world_advanced` wakes the shell when
    /// a resident world lane commits.
    /// # Errors
    /// A game, content or engine preparation failure, which ends the run.
    fn prepare(
        &mut self,
        gpu: Gpu<'_>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<Self::World, Self::Scene>, String>;
    /// Lifecycle control of a resident world, once prepared; `None` disables pause.
    fn world(&self) -> Option<&dyn WorldLink> {
        None
    }
    /// The controls model shown in the panel and edited by parameter files.
    fn controls(&self) -> &Controls;
    /// Mutable access for panel edits and parameter files.
    fn controls_mut(&mut self) -> &mut Controls;
    /// Take presentation edits made since the last call, and a requested pause state.
    fn take_edits(&mut self) -> (Self::Edits, Option<bool>);
    /// Presentation edits computed on the main thread just before each frame request, for
    /// continuous animation or polling. The default edits nothing.
    fn frame_edits(&mut self) -> Self::Edits {
        Self::Edits::default()
    }
    /// The whole desired presentation state, sent again after GPU recovery.
    fn replay(&self) -> Self::Edits;
    /// Apply edits to the runtime on the render owner thread.
    /// # Errors
    /// A refused edit, which ends the run.
    fn apply(
        host: &mut FrameHost<Self::World, Self::Scene>,
        edits: Self::Edits,
    ) -> Result<(), String>;
    /// Read back a value on the render owner thread after each job.
    fn readback(host: &FrameHost<Self::World, Self::Scene>) -> Self::Readback;
    /// Observe a readback on the main thread. Replies to frames requested before the latest
    /// edits are not observed, so they cannot revert those edits.
    fn observe(&mut self, readback: Self::Readback);
    /// Reload the game's own authored files, such as a material hook, when the window opens, on
    /// `L` after the parameter file, and when one of [`Game::reload_files`] is saved. The game
    /// reports what became of them; the default reloads nothing.
    fn reload(&mut self) {}
    /// The files [`Game::reload`] reads. The window watches them and reloads when a save
    /// settles, as `L` would; the default watches none.
    fn reload_files(&self) -> Vec<std::path::PathBuf> {
        Vec::new()
    }
    /// Observe the pause state the `P` key now asks for.
    fn observe_paused(&mut self, paused: bool) {
        let _ = paused;
    }
    /// Handle a game key.
    /// # Errors
    /// A refused input, which ends the run.
    fn key(&mut self, code: KeyCode, pressed: bool) -> Result<KeyResponse, String>;
    /// Release held input when the window loses focus; the shell then requests a frame.
    /// # Errors
    /// A refused input, which ends the run.
    fn focus_lost(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Read a parameter file and apply it to `controls`: the report names any refused entry, which
/// keeps its value while the others apply. The window and a live capture share it.
/// # Errors
/// The file cannot be read or is not a parameter file; every control keeps its value.
pub(crate) fn apply_params_file(
    path: &std::path::Path,
    controls: &mut Controls,
) -> Result<thrum_controls::FileReport, String> {
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| {
            controls
                .apply_file(&text)
                .map_err(|error| error.to_string())
        })
}
