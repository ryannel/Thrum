//! Explicit submitted-sample and accepted-history metadata. No global frame
//! parity, skipped draw or submission completion can manufacture valid history.

/// All epochs whose changes invalidate this bounded spatial/temporal handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryKey {
    /// Stable view identity, independent of profile identity.
    pub view: u64,
    /// Effect/geometry history provider.
    pub provider: u64,
    /// Output pixel grid; render scale alone does not invalidate history.
    pub extent: [u32; 2],
    /// Device generation.
    pub device_generation: u64,
    /// Source/product semantic revision.
    pub content_epoch: u64,
    /// Selected prepared presentation profile.
    pub profile_epoch: u64,
    /// Origin convention epoch.
    pub origin_epoch: u64,
    /// Incremented on camera cut or invalid correspondence.
    pub correspondence_epoch: u64,
}

/// One immutable sample. Times belong to presentation/extraction, not host parity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// Monotonic within the source epoch.
    pub serial: u64,
    /// Finite presentation seconds.
    pub time_seconds: f64,
    /// Committed endpoint identity supplied by the world producer.
    pub committed_endpoint: u64,
}

/// Tracks successful single-queue submission-order acceptance.
#[derive(Clone, Debug)]
pub struct ViewHistory {
    key: HistoryKey,
    submitted: Option<Sample>,
    accepted: Option<Sample>,
}
impl ViewHistory {
    /// Start without usable prior history.
    pub fn new(key: HistoryKey) -> Self {
        Self {
            key,
            submitted: None,
            accepted: None,
        }
    }
    /// Reset all metadata on any key change. Reprojection needs a separately
    /// documented conversion and is intentionally absent from this first profile.
    pub fn synchronize(&mut self, key: HistoryKey) {
        if self.key != key {
            *self = Self::new(key);
        }
    }
    /// Advance submitted metadata only after the executor actually submits this
    /// exact view. Reject nonfinite times, wrong epochs and stale/duplicate serials.
    pub fn submitted(&mut self, key: HistoryKey, sample: Sample) -> bool {
        if key != self.key
            || !sample.time_seconds.is_finite()
            || self
                .submitted
                .is_some_and(|old| sample.serial <= old.serial)
        {
            return false;
        }
        self.submitted = Some(sample);
        // A single queue orders this history write before its successor's read.
        // Delayed execution faults reset the entire device generation.
        self.accepted = Some(sample);
        true
    }
    /// Accept only the latest exact submitted sample. Queue completion alone
    /// must not call this: acceptance belongs to the history-producing provider.
    pub fn accept(&mut self, key: HistoryKey, sample: Sample) -> bool {
        if key != self.key || self.submitted != Some(sample) {
            return false;
        }
        self.accepted = Some(sample);
        true
    }
    /// Latest actually submitted sample; skips/failures leave it unchanged.
    pub fn last_submitted(&self) -> Option<Sample> {
        self.submitted
    }
    /// Explicitly accepted usable image/pose sample, or reset/unknown.
    pub fn accepted(&self) -> Option<Sample> {
        self.accepted
    }
}
