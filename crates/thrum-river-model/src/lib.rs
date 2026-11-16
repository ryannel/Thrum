//! Steady river flow from the ground, worked out ahead of time on the CPU.
//!
//! A river is solved once, over a rectangle of the water map's bed, as the steady state of the
//! depth-averaged shallow-water equations with Manning friction: a finite-volume march with an
//! HLL flux, Chen and Noelle's hydrostatic reconstruction and friction linearised about the old
//! discharge, in pseudo-time with each cell's own step, from pools filled to their sills, on wet
//! cells and their neighbours only. The solve writes each reach as a chart `thrum-water-model`
//! declares, keeps an energy account whose whitewater share is the chart's loss, and is stored as
//! a river file under a key of its inputs. Nothing here needs a graphics device.
//!
//! Coordinates are metres in x/z order, as in `thrum-water-model`; heights are metres.
mod brink;
mod chart;
mod declare;
mod error;
mod fall;
mod scheme;
mod solve;
mod store;

pub use brink::{Brink, Side, brinks};
pub use declare::{Declaration, Grid, Manning, Profile, Receiver, Source, Spring};
pub use error::RiverError;
pub use fall::LAUNCH_SHARE;
pub use scheme::{State, Stepper};
pub use solve::{Narrow, Report, Solution, solve};
pub use store::{SOLVER_VERSION, key, key_for_version, write};
