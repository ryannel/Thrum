//! The boundary between the platform adapters and one game.

use std::{
    error::Error,
    fmt,
    future::Future,
    time::{Duration, Instant},
};

use thrum_controls::Controls;
#[cfg(not(target_arch = "wasm32"))]
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{FrameError, FrameHost, FrameWorld, SceneSource},
};

/// Failure a game reports to its host while preparing or handling a request.
#[derive(Debug)]
pub enum GameError {
    /// Authored scene values are invalid.
    InvalidScene(String),
    /// The frame runtime refused the operation.
    Frame(FrameError),
    /// The game refused a request with its own diagnostic.
    Rejected(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScene(message) | Self::Rejected(message) => formatter.write_str(message),
            Self::Frame(error) => error.fmt(formatter),
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            _ => None,
        }
    }
}

impl From<FrameError> for GameError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

/// The platform's first device, queue and configured surface, lent to [`Game::prepare`].
pub struct Gpu<'a> {
    /// Device created by the shell's device recipe.
    pub device: &'a wgpu::Device,
    /// Queue paired with `device`.
    pub queue: &'a wgpu::Queue,
    /// Immutable capability record of `device`.
    pub capabilities: &'a DeviceCapabilities,
    /// Format and extent of the configured presentation target.
    pub surface: SurfaceProfile,
}

/// A game's prepared frame runtime, handed to the render owner thread.
pub struct Prepared<W: FrameWorld, V: SceneSource<W::Page>> {
    /// The runtime host; the world can run inline or on a resident lane.
    pub host: FrameHost<W, V>,
    /// Monotonic origin if the world already has a clock. The adapter supplies time for an inline world.
    pub origin: Option<Instant>,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> Prepared<W, V> {
    /// A runtime with an inline world, whose time is supplied by its adapter.
    pub fn inline(host: FrameHost<W, V>) -> Self {
        Self { host, origin: None }
    }
}

/// The world's lifecycle as the shell drives it. The world owns its clock and the drain of
/// accepted ticks; the shell decides only when to pause and when to resume.
pub trait InlineWorld: FrameWorld {
    /// Stop admitting wall time, keeping accepted ticks.
    /// # Errors
    /// Returns a world lifecycle refusal.
    fn pause(&mut self) -> Result<(), GameError>;
    /// Resume from `now` after the accepted ticks have drained.
    /// # Errors
    /// Returns a clock state or timestamp refusal.
    fn resume(&mut self, now: Duration) -> Result<(), GameError>;
}

impl<D: thrum_core::sim::WorldDriver> InlineWorld for thrum_core::sim::DrivenWorld<D> {
    fn pause(&mut self) -> Result<(), GameError> {
        self.pause();
        Ok(())
    }

    fn resume(&mut self, _now: Duration) -> Result<(), GameError> {
        self.resume();
        Ok(())
    }
}

/// Lifecycle control of a world that runs on its own lane, used for pause and overload.
#[cfg(not(target_arch = "wasm32"))]
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

/// The runtime half of a game, shared by window, browser and offscreen adapters.
/// The adapter owns presentation, GPU recovery and the control view.
pub trait Game {
    /// The runtime's world adapter.
    type World: FrameWorld + 'static;
    /// The runtime's game-owned scene source.
    type Scene: SceneSource<<Self::World as FrameWorld>::Page> + 'static;
    /// Edits applied by the render owner before a frame.
    type Edits: Edits;
    /// A small value read on the render owner after each job and observed on the main thread.
    type Readback: Send + 'static;

    /// Explicit launch hardware preferences and requirements, retained on device recovery.
    fn hardware_requests(&self) -> &[thrum_render::capabilities::HardwareRequest] {
        &[]
    }

    /// Declare EDR support to receive a float surface on an eligible display.
    /// A 2D game makes the same declaration as a 3D game. Return `true` to opt in.
    fn supports_edr(&self) -> bool {
        false
    }

    /// Prepare the runtime on the shell's first device. `world_advanced` wakes the shell when
    /// a resident world lane commits.
    /// # Errors
    /// A game, content or engine preparation failure, which ends the run.
    fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> impl Future<Output = Result<Prepared<Self::World, Self::Scene>, GameError>> + 'a;
    /// Lifecycle control of a resident world, once prepared; `None` disables pause.
    #[cfg(not(target_arch = "wasm32"))]
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
    /// Opt in to output extent, density and safe-area reports from the platform shell.
    #[cfg(feature = "interface")]
    fn wants_interface_output(&self) -> bool {
        false
    }

    /// Convert the latest platform output report to an edit applied before drawing.
    #[cfg(feature = "interface")]
    fn interface_output(&mut self, _output: OutputMetrics) -> Self::Edits {
        Self::Edits::default()
    }

    /// Opt in to key, pointer, touch and text events from the shell.
    #[cfg(feature = "interface")]
    fn wants_interface_input(&self) -> bool {
        false
    }

    /// Route a shell event. Positions are in backing pixels and converted by the game
    /// using the scale of its last submitted interface layout.
    /// # Errors
    /// Returns a game input refusal.
    #[cfg(feature = "interface")]
    fn interface_input(
        &mut self,
        _event: thrum_ui::Event<crate::KeyCode>,
        _now: Duration,
    ) -> Result<KeyResponse, GameError> {
        Ok(KeyResponse::Ignored)
    }

    /// The full string the game asks the shell to copy.
    #[cfg(feature = "interface")]
    fn take_clipboard_copy(&mut self) -> Option<String> {
        None
    }

    /// Report whether the shell accepted the clipboard operation.
    #[cfg(feature = "interface")]
    fn clipboard_result(&mut self, _access: crate::ClipboardAccess, _allowed: bool) {}

    /// Observe every actual surface submission, even if a newer edit superseded its controls.
    #[cfg(feature = "interface")]
    fn interface_submitted(&mut self, _readback: &Self::Readback, _index: u64, _now: Duration) {}

    /// Advance game-owned delays in the interface on the shell's monotonic clock.
    #[cfg(feature = "interface")]
    fn interface_tick(&mut self, _now: Duration) {}

    /// Optional proof inputs for the window's ordinary translated event handlers.
    #[cfg(all(feature = "interface", target_os = "macos"))]
    fn interface_script(&self) -> Option<&crate::InterfaceScript> {
        None
    }

    /// Report game-owned observations for the window's input qualification script.
    #[cfg(all(feature = "interface", target_os = "macos"))]
    fn interface_witness(&self) -> Option<String> {
        None
    }

    /// The whole desired presentation state, sent again after GPU recovery.
    fn replay(&self) -> Self::Edits;
    /// Apply edits to the runtime on the render owner thread.
    /// # Errors
    /// A refused edit, which ends the run.
    fn apply(
        host: &mut FrameHost<Self::World, Self::Scene>,
        edits: Self::Edits,
    ) -> Result<(), GameError>;
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
    fn key(&mut self, _code: crate::KeyCode, _pressed: bool) -> Result<KeyResponse, GameError> {
        Ok(KeyResponse::Ignored)
    }
    /// Release held input when the host loses focus.
    /// # Errors
    /// A refused input, which ends the run.
    fn focus_lost(&mut self) -> Result<(), GameError> {
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
pub(crate) fn apply_frame_edits<G: Game>(
    game: &mut G,
    host: &mut FrameHost<G::World, G::Scene>,
    replay: bool,
) -> Result<(), GameError> {
    let replay = replay.then(|| game.replay());
    let frame = game.frame_edits();
    G::apply(host, replay.map_or(frame, |edits| edits.merge(frame)))
}

/// Read a parameter file and apply it to `controls`: the report names any refused entry, which
/// keeps its value while the others apply. The window and a live capture share it.
/// # Errors
/// The file cannot be read or is not a parameter file; every control keeps its value.
#[cfg(target_os = "macos")]
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

/// Window or canvas extent in pixels and its point-space safe area.
#[cfg(feature = "interface")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputMetrics {
    /// Physical output pixels.
    pub extent: [u32; 2],
    /// Device pixels per operating-system point.
    pub density: f64,
    /// Left, top, right and bottom in operating-system points.
    pub insets: [f64; 4],
}
