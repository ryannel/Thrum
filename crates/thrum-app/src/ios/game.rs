//! Game-owned preparation and input at the iOS shell's boundary.

use std::{
    future::Future,
    path::PathBuf,
    time::{Duration, Instant},
};

use thrum_render::runtime::{FrameHost, FrameWorld, SceneSource};

use crate::{GameError, Gpu, WorldLink};

#[cfg(feature = "interface")]
use crate::{KeyCode, OutputMetrics};
#[cfg(feature = "interface")]
use thrum_ui::{Event, Routed, Viewport};

/// Names and opt-in qualification settings supplied by the enclosing app.
pub struct MobileOptions {
    /// Title of the native window.
    pub title: &'static str,
    /// Render owner name used by diagnostics.
    pub render_thread: &'static str,
    /// Name of the qualification record in the application's temporary directory.
    pub witness: Option<&'static str>,
}

/// An optional request from the application's UIKit URL handler.
#[derive(Debug, Clone)]
pub struct SceneRequest {
    /// Decoded request as understood by the game.
    pub payload: String,
    /// Game-selected content location for this request.
    pub content: PathBuf,
}

/// Main-thread handles for a resident world and its analog input.
pub trait MobileControls: Send + 'static {
    /// Lifecycle handle, when the world advances independently of the render owner.
    fn world(&self) -> Option<&dyn WorldLink> {
        None
    }

    /// Send the translated analog axes.
    ///
    /// # Errors
    /// Returns a world input refusal.
    fn axes(&self, _axes: [f32; 3]) -> Result<(), GameError> {
        Ok(())
    }
}

impl MobileControls for () {}

/// The prepared game runtime and the app state returned to the shell.
pub struct MobilePrepared<G: MobileGame> {
    /// Frame runtime, kept on the render owner.
    pub host: MobileHost<G>,
    /// Per-scene presentation state, kept on the render owner.
    pub scene: G::Presentation,
    /// Handles retained on the main thread.
    pub controls: G::Controls,
    /// Game's first-chance interface state, retained on the main thread.
    pub input: G::Input,
    /// Whether this scene needs output metrics even if it has no interactive interface.
    pub output_metrics: bool,
}

/// Frame runtime prepared by a mobile game.
pub type MobileHost<G> = FrameHost<<G as MobileGame>::World, <G as MobileGame>::Scene>;

/// Game code called by the shared iOS shell. UIKit and drawable ownership stay in the shell.
pub trait MobileGame: Sized + Send + 'static {
    /// Simulation world read by the frame runtime.
    type World: FrameWorld + Send + 'static;
    /// Scene source that turns publications into render data.
    type Scene: SceneSource<<Self::World as FrameWorld>::Page> + Send + 'static;
    /// Scene setup and state retained on the render owner.
    type Presentation: Send + 'static;
    /// Main-thread interface state.
    type Input: Send + 'static;
    /// Main-thread lifecycle and analog input handles.
    type Controls: MobileControls;
    /// Optional observation from a presented frame.
    type Readback: Send + 'static;

    /// Prepare a scene on the render owner, optionally using an app request.
    /// The wake callback is for games that start a resident world lane.
    ///
    /// # Errors
    /// Returns game or renderer preparation failures.
    fn prepare(
        &self,
        gpu: Gpu<'_>,
        origin: Instant,
        wake: fn(),
        request: Option<SceneRequest>,
        qualification: bool,
    ) -> impl Future<Output = Result<MobilePrepared<Self>, GameError>>;

    /// Report the latest geometry to the game before encoding.
    #[cfg(feature = "interface")]
    fn output(_host: &mut MobileHost<Self>, _output: OutputMetrics) {}

    /// Apply a presentation update before encoding. An app may opt into its own
    /// qualification commands, which are delivered here by number.
    ///
    /// # Errors
    /// Returns a game presentation refusal.
    fn frame(
        _host: &mut MobileHost<Self>,
        _scene: &mut Self::Presentation,
        _command: u64,
        _now: Duration,
    ) -> Result<(), GameError> {
        Ok(())
    }

    /// Reset an inline world's clock when the app returns to the foreground.
    ///
    /// # Errors
    /// Returns a world lifecycle refusal.
    fn resume_inline(_world: &mut Self::World, _now: Duration) -> Result<(), GameError> {
        Ok(())
    }

    /// Read an app-provided sample on explicit request.
    fn sample_position(_host: &MobileHost<Self>) -> Option<[f64; 3]> {
        None
    }

    /// Read an observation after a successful present.
    fn readback(_host: &mut MobileHost<Self>) -> Option<Self::Readback> {
        None
    }

    /// Physical viewport for readbacks of an interface layout.
    #[cfg(feature = "interface")]
    fn viewport(_shown: &Self::Readback) -> Option<Viewport> {
        None
    }

    /// Present the readback to the app. Return true to write a qualification record now.
    fn submitted(
        _input: &mut Self::Input,
        _readback: &Self::Readback,
        _index: u64,
        _now: Duration,
    ) -> bool {
        false
    }

    /// Opt in to first-chance touch delivery.
    #[cfg(feature = "interface")]
    fn wants_input(_input: &Self::Input) -> bool {
        false
    }

    /// Route input against the last submitted interface layout.
    #[cfg(feature = "interface")]
    fn event(_input: &mut Self::Input, _event: Event<KeyCode>, _now: Duration) {}

    /// Advance game-owned interface delays.
    #[cfg(feature = "interface")]
    fn tick(_input: &mut Self::Input, _now: Duration) {}

    /// Deliver game-owned main-thread qualification commands.
    fn command(_input: &mut Self::Input, _command: u64, _now: Duration) {}

    /// Stable name of a game qualification command in the witness.
    fn command_name(_command: u64) -> &'static str {
        "Game"
    }

    /// Take input passed from the interface to the shell's analog gesture mapper.
    #[cfg(feature = "interface")]
    fn take_input(_input: &mut Self::Input) -> impl Iterator<Item = Routed<KeyCode>> {
        std::iter::empty()
    }

    /// Try to handle a scene request without replacing the current scene.
    fn scene_request(_input: &mut Self::Input, _request: &SceneRequest, _now: Duration) -> bool {
        false
    }

    /// Append game-owned key/value diagnostics to the shell qualification record.
    fn witness(_input: &Self::Input) -> String {
        String::new()
    }

    /// Record game-owned renderer diagnostics.
    fn frame_witness(_host: &MobileHost<Self>, _scene: &Self::Presentation) -> String {
        String::new()
    }

    /// Record the independently measured interface region for a still frame.
    fn still_bounds(_host: &mut MobileHost<Self>) -> Option<serde_json::Value> {
        None
    }
}
