//! Configure and restore on the render owner, at a frame boundary.

use std::time::{Duration, Instant};

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
        Self {
            usable: true,
            record: SurfaceSwitchRecord::default(),
        }
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
