//! One request and reply for a shell's render owner.

use std::time::Duration;

use thrum_render::{frame::TargetLeadStatus, runtime::FrameSample};

/// Maximum number of completed GPU samples collected in a render job.
pub const SAMPLE_SLOTS: usize = 2;

/// One frame requested for the shell's latest surface and display target.
pub struct FrameRequest<P> {
    /// The shell's sequence number for this opportunity.
    pub sequence: u64,
    /// The device generation the shell believes current.
    pub device_generation: u64,
    /// The intended presentation time in the platform's media clock.
    pub target_media_seconds: f64,
    /// Requested presentation extent in pixels.
    pub extent: [u32; 2],
    /// The shell's current surface generation or lease epoch.
    pub surface_generation: u64,
    /// Additional shell input and render-owner instructions.
    pub platform: P,
}

/// The render owner's answer, including completions from earlier submissions.
pub struct FrameReply<O, P> {
    /// The request's opportunity number.
    pub sequence: u64,
    /// The owner's device generation when it replied.
    pub device_generation: u64,
    /// The owner outcome, including the frame's drive report when available.
    pub outcome: O,
    /// Completed GPU samples, in collection order.
    pub samples: [Option<FrameSample>; SAMPLE_SLOTS],
    /// Whether the late-sampled display target was usable.
    pub target_status: Option<TargetLeadStatus>,
    /// Time inside the drawable acquire call, when attempted.
    pub acquire_wall: Option<Duration>,
    /// The surface extent reported by the owner.
    pub surface_extent: [u32; 2],
    /// Additional shell observations and game readback.
    pub platform: P,
}
