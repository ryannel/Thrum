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
//! page keeps its animation-frame loop, DOM events and input, and calls the shell; a game keeps
//! its report text, controls and authored scene. Mobile shells own their own loops.

#[cfg(target_os = "macos")]
mod app;
/// Bounded submitted-image sink shared by native platform adapters.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub mod capture_writer;
#[cfg(target_os = "macos")]
mod file_watch;
#[cfg(target_os = "macos")]
mod game;
#[cfg(target_os = "macos")]
pub mod offscreen;
pub mod sky;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(any(target_arch = "wasm32", test))]
mod web_clock;

#[cfg(target_os = "macos")]
pub use app::{REPORT_VARIABLE, run, run_until_closed};
#[cfg(target_os = "macos")]
pub use game::{Edits, Game, Gpu, KeyResponse, Prepared, WorldLink};
/// Physical key codes delivered to [`Game::key`].
#[cfg(target_os = "macos")]
pub use winit::keyboard::KeyCode;

mod config;
pub use config::Config;
