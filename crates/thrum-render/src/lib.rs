//! V1 GPU execution foundations. The platform owns one device and queue; this
//! crate prepares resources and encodes work using that device, never creates
//! another device or submits background work. Compile graphs at setup or
//! configuration/generation changes, retain their plans, then reuse per frame.
//! Timestamp resolve/readback is a later host submission after a nonblocking
//! completion callback; it never waits in the frame encode/submit path.
//! Synthetic output is diagnostic,
//! not the lit PBR/CSM scene or a cooked-content readiness claim.

pub mod abi;
pub mod graph;
pub mod history;
pub mod synthetic;
pub mod timing;
