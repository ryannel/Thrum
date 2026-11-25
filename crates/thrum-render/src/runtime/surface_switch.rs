//! Configure and restore on the render owner, at a frame boundary.

use std::time::{Duration, Instant};

/// Mac render owner's observed surface lease and its configuration admission.
#[cfg(target_os = "macos")]
pub struct SurfaceLease {
    generation: u64,
    pending: bool,
    refused: Option<u64>,
}

#[cfg(target_os = "macos")]
impl SurfaceLease {
    /// Track a lease on startup or after a device replacement.
    #[must_use]
    pub fn new(generation: u64, pending: bool) -> Self {
        Self {
            generation,
            pending,
            refused: None,
        }
    }

    /// The platform has requested a replacement. The next drawable needs a fresh configure.
    pub fn observe(&mut self, generation: u64) {
        if generation != self.generation {
            self.generation = generation;
            self.invalidate();
        }
    }

    /// Close drawable admission until the render owner configures the surface.
    pub fn invalidate(&mut self) {
        self.pending = true;
    }

    /// Whether the render owner still needs to configure.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Whether this lease's latest configuration was refused.
    #[must_use]
    pub fn refused(&self, generation: u64) -> bool {
        self.refused == Some(generation)
    }

    /// An extent switch settled the current configuration.
    pub fn settle_extent(&mut self, taken: bool) {
        self.pending = false;
        if taken {
            self.refused = None;
        }
    }

    /// Recover the surface at the committed extent once the render owner has
    /// drained submitted frames and no extent preparation is using the device.
    /// The callback performs a real configure for the committed configuration.
    /// Returns None when recovery is unnecessary, deferred or already refused.
    pub fn recover(
        &mut self,
        preparation_busy: bool,
        pending_frames: usize,
        configure: impl FnMut() -> Result<(), wgpu::Error>,
    ) -> Option<Result<(), crate::capabilities::SurfaceConfigError>> {
        if !self.pending || preparation_busy || pending_frames != 0 || self.refused(self.generation)
        {
            return None;
        }
        let result = crate::capabilities::retry_surface_configuration(configure);
        if result.is_ok() {
            self.pending = false;
        } else {
            self.refused = Some(self.generation);
        }
        Some(result)
    }
}

/// Calls made at one admitted generation switch.
#[derive(Debug, Default)]
pub struct SurfaceSwitchRecord {
    /// Boundaries at which the candidate remained ready and no call was made.
    pub deferrals: u32,
    /// Calls made with the proposed configuration.
    pub configure_calls: u32,
    /// Successful switches to the proposed configuration.
    pub generations_taken: u32,
    /// Calls made with the old configuration after a failed switch.
    pub restore_calls: u32,
    /// Whether the attempted restore succeeded.
    pub restored: Option<bool>,
    /// Time in the proposed configure call, including wgpu's queue wait.
    pub configure_wall: Duration,
    /// Time in the restoring call, if made.
    pub restore_wall: Duration,
    /// Error from the proposed configure call.
    pub configure_error: Option<String>,
    /// Error from the restoring call.
    pub restore_error: Option<String>,
}

/// Surface acquisition eligibility and the record for the current candidate.
pub struct SurfaceSwitch {
    usable: bool,
    record: SurfaceSwitchRecord,
}

impl SurfaceSwitch {
    /// A newly configured surface may acquire immediately.
    #[must_use]
    pub fn configured() -> Self {
        Self::with_eligibility(true)
    }

    pub(super) fn with_eligibility(usable: bool) -> Self {
        Self {
            usable,
            record: SurfaceSwitchRecord::default(),
        }
    }

    /// The platform successfully configured this surface outside a resize switch.
    pub fn platform_configured(&mut self) {
        self.usable = true;
    }

    /// Whether an acquisition can use the current configuration.
    #[must_use]
    pub fn usable(&self) -> bool {
        self.usable
    }

    /// The history of attempts to take this candidate.
    #[must_use]
    pub fn record(&self) -> &SurfaceSwitchRecord {
        &self.record
    }

    /// Begin recording a new candidate, retaining the surface's current eligibility.
    pub fn begin(&mut self) {
        self.record = SurfaceSwitchRecord::default();
    }

    /// Configure once at an eligible boundary. A failure repairs the old
    /// configuration before acquisition can resume.
    ///
    /// The configuration callback is called synchronously on the render owner.
    /// It must return the result of one configure call, including validation.
    pub fn take<C, E: std::fmt::Display>(
        &mut self,
        drawable_held: bool,
        preparation_busy: bool,
        proposed: &C,
        current: &C,
        mut configure: impl FnMut(&C) -> Result<(), E>,
    ) -> SurfaceSwitchOutcome {
        if drawable_held || preparation_busy {
            self.record.deferrals += 1;
            return SurfaceSwitchOutcome::Deferred;
        }

        self.record.configure_calls += 1;
        let start = Instant::now();
        let result = configure(proposed);
        self.record.configure_wall = start.elapsed();
        match result {
            Ok(()) => {
                self.usable = true;
                self.record.generations_taken += 1;
                SurfaceSwitchOutcome::Taken
            }
            Err(error) => {
                self.record.configure_error = Some(error.to_string());
                self.record.restore_calls += 1;
                let start = Instant::now();
                let restore = configure(current);
                self.record.restore_wall = start.elapsed();
                self.usable = restore.is_ok();
                self.record.restored = Some(self.usable);
                self.record.restore_error = restore.err().map(|error| error.to_string());
                SurfaceSwitchOutcome::Refused
            }
        }
    }
}

/// Whether the candidate's configuration became current at this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceSwitchOutcome {
    /// The candidate stays ready for a later boundary.
    Deferred,
    /// Take the prepared generation.
    Taken,
    /// Drop the candidate. Acquire only if the restore succeeded.
    Refused,
}
