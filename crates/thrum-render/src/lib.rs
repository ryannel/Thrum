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
mod apple_tile;
pub mod atmosphere;
pub mod binding;
pub mod capabilities;
pub mod contribution;
pub mod error;
pub mod extract;
pub mod frame;
pub mod governor;
pub mod graph;
mod graph_record;
pub mod history;
mod light;
mod pass_key;
pub mod present;
mod records;
#[doc(hidden)]
pub mod registry_audit;
pub mod render_config;
pub mod runtime;
mod shader_constants;
mod shaders;
pub mod sprite;
#[doc(hidden)]
pub use records::{RecordLayout, RecordMember, RecordScalar, layouts as record_layouts};
#[doc(hidden)]
pub use shaders::composed_sources;
pub mod synthetic;
#[cfg(test)]
mod test_device;
#[cfg(test)]
mod timestamp_permit;
pub mod timing;
mod visibility;
pub mod wetness;

mod shared_gpu;
pub(crate) use shared_gpu::SharedGpu;
