//! Silent, frame-addressed proofs for the mix and its future output adapters.

mod cost;
mod reference;

pub use cost::{FullPool, MovingEmitters};
pub use reference::{Digest, ReferenceRun};
