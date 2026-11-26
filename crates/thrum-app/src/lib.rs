//! Application shells shared by Thrum games. The native macOS shell: one window and its Metal device
//! recipe, the per-view display link, a render owner thread, world pause and GPU device
//! recovery, the typed demo-control panel with `THRUM_PARAMS` files, and a bounded frames
//! report. A game supplies its prepared frame runtime, controls and keys through [`Game`].
//!
//! [`sky`] is a small kit for still scenes: a physically based sky and Sun, a lit ground, a
//! look-at camera and cooked meshes, mapped from controls each frame. Start from
//! `examples/starter`.
//!
//! [`web`] (wasm32 only) is the browser shell: the WebGPU canvas surface and device, device-loss
//! gating and detached replacement, the page clock policy, the frame step and typed counters. The
//! shared page driver owns browser callbacks, visibility and recovery, and a game keeps its report
//! text, controls and authored scene. The iOS shell owns the UIKit scene, display link and render owner.

// platform-only tests: they drive the winit and AppKit shell, which exists only on macOS.
#[cfg(target_os = "macos")]
mod app;
/// Bounded submitted-image sink shared by native platform adapters.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    all(test, not(target_arch = "wasm32"))
))]
pub mod capture_writer;
// Its tests are portable; only the macOS shell uses it.
#[cfg(any(test, target_os = "macos"))]
#[cfg_attr(
    not(target_os = "macos"),
    expect(dead_code, reason = "only the macOS shell starts the watcher")
)]
mod file_watch;
// These portable tests exercise the same touch and lease code as UIKit.
#[cfg(any(target_os = "ios", test))]
mod ios_gesture;
#[cfg(any(target_os = "ios", test))]
mod ios_lease;

pub mod frame;
mod game;
#[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
mod inline_lifecycle;
#[cfg(feature = "interface")]
mod interface;
/// Shared UIKit application shell.
#[cfg(target_os = "ios")]
pub mod ios;
mod key;
#[cfg(target_os = "macos")]
pub mod offscreen;
pub mod profile;
pub mod sky;
pub mod surface;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(any(target_arch = "wasm32", test))]
mod web_cadence;
#[cfg(any(target_arch = "wasm32", test))]
mod web_clock;

#[cfg(all(feature = "interface", target_os = "macos"))]
pub use app::InterfaceScript;
#[cfg(target_os = "macos")]
pub use app::{
    DisplayOverride, DisplayOverrideError, ExtentWindow, GovernorProof, REPORT_VARIABLE,
    ScaleReplay, TransitionArm, WindowDisplayOptions, WindowExtentCapture, WindowGame,
    WindowQualification, WindowTransitionProof, run, run_until_closed, run_with_display,
    run_with_extent_capture, run_with_qualification, run_with_qualification_and_display,
    run_with_transition_proof, thermal_state,
};
#[cfg(feature = "interface")]
pub use game::OutputMetrics;
#[cfg(not(target_arch = "wasm32"))]
pub use game::WorldLink;
pub use game::{Edits, Game, GameError, Gpu, InlineWorld, KeyResponse, Prepared};
#[cfg(feature = "interface")]
pub use interface::{ClipboardAccess, interface_key};
pub use key::KeyCode;

mod config;
pub use config::Config;
#[cfg(target_os = "macos")]
pub use config::MacFrameRate;
