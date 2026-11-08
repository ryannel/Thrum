//! V1 GPU execution foundations. The platform owns one device and queue; this
//! crate prepares resources and encodes work using that device. Its bounded
//! frame runtime coordinates submission; features never create another device
//! or submit independent work. Compile graphs at setup or
//! configuration/generation changes, retain their plans, then reuse per frame.
//! Timestamp resolve/readback is a later host submission after a nonblocking
//! completion callback; it never waits in the frame encode/submit path.
//! The synthetic lit output is diagnostic, not a complete clustered/CSM scene
//! or a cooked-content readiness claim.

pub mod abi;
pub mod atmosphere;
pub mod capabilities;
pub mod contribution;
pub mod extract;
pub mod frame;
pub mod graph;
mod graph_record;
pub mod history;
pub mod present;
pub mod runtime;
pub mod synthetic;
#[cfg(test)]
mod timestamp_permit;
pub mod timing;
mod visibility;
