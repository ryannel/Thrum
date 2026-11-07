//! Native macOS application shell shared by Thrum games: one window and its Metal device
//! recipe, the per-view display link, a render owner thread, world pause and GPU device
//! recovery, the typed demo-control panel with `THRUM_PARAMS` files, and a bounded frames
//! report. A game supplies its prepared frame runtime, controls and keys through [`Game`].
//!
//! [`sky`] is a small kit for still scenes: a physically based sky and Sun, a look-at camera
//! and an optional engine contribution such as cooked meshes, mapped from controls each frame.
//!
//! The crate is empty on other targets; browser and mobile shells own their own loops.

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod game;
#[cfg(target_os = "macos")]
pub mod sky;

#[cfg(target_os = "macos")]
pub use app::{run, run_until_closed};
#[cfg(target_os = "macos")]
pub use game::{Config, Edits, Game, Gpu, KeyResponse, Prepared, WorldLink};
/// Physical key codes delivered to [`Game::key`].
#[cfg(target_os = "macos")]
pub use winit::keyboard::KeyCode;
