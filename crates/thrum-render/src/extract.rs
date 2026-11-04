//! Reused CPU render columns derived from a committed simulation tick. These
//! presentation copies never become a second world authority.

use std::collections::{HashMap, hash_map::Entry};

use foldhash::fast::RandomState;

/// Identity of the committed world endpoint used for one extraction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTick {
    /// World generation; a restart invalidates all prior samples.
    pub epoch: u128,
    /// Complete committed simulation tick.
    pub tick: u64,
}

/// Counts of accepted and degraded presentation samples for one extraction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExtractReport {
    /// Visual instances copied into the frame.
    pub accepted: usize,
    /// Samples dropped because the configured frame capacity was reached.
    pub dropped_capacity: usize,
    /// Samples dropped because their position cannot be represented relative to the view origin.
    pub dropped_invalid: usize,
    /// Repeated stable identities dropped after the first sample.
    pub dropped_duplicate: usize,
    /// Instances matched to the immediately preceding tick and visual revision.
    pub history_reused: usize,
}

/// Structure-of-arrays CPU frame. The current/previous positions and visual
/// revisions have the same index as each stable identity. Borrowing this frame
/// prevents the extractor from overwriting it during another extraction.
pub struct RenderColumns {
    source: Option<SourceTick>,
    origin: [f64; 3],
    previous_origin: [f64; 3],
    ids: Vec<u128>,
    current: Vec<[f32; 3]>,
    previous: Vec<[f32; 3]>,
    visuals: Vec<u64>,
    history_valid: Vec<bool>,
    index: HashMap<u128, usize, RandomState>,
}

impl RenderColumns {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            source: None,
            origin: [0.0; 3],
            previous_origin: [0.0; 3],
            ids: Vec::with_capacity(capacity),
            current: Vec::with_capacity(capacity),
            previous: Vec::with_capacity(capacity),
            visuals: Vec::with_capacity(capacity),
            history_valid: Vec::with_capacity(capacity),
            index: HashMap::with_capacity_and_hasher(capacity, RandomState::default()),
        }
    }

    fn clear(&mut self, source: SourceTick, origin: [f64; 3], previous_origin: [f64; 3]) {
        self.source = Some(source);
        self.origin = origin;
        self.previous_origin = previous_origin;
        self.ids.clear();
        self.current.clear();
        self.previous.clear();
        self.visuals.clear();
        self.history_valid.clear();
        self.index.clear();
    }

    /// Committed source of these columns, or `None` before the first extraction.
    pub fn source(&self) -> Option<SourceTick> {
        self.source
    }

    /// View origin used to convert current world positions to f32.
    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }

    /// View origin for valid previous positions. Invalid history entries use
    /// current positions and must not be reprojected.
    pub fn previous_origin(&self) -> [f64; 3] {
        self.previous_origin
    }

    /// Number of visual instances in this frame.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether this frame contains no visual instances.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Stable identities; never infer identity from row order.
    pub fn ids(&self) -> &[u128] {
        &self.ids
    }

    /// Current origin-relative world positions.
    pub fn current(&self) -> &[[f32; 3]] {
        &self.current
    }

    /// Previous origin-relative positions where history is valid; otherwise current.
    pub fn previous(&self) -> &[[f32; 3]] {
        &self.previous
    }

    /// Visual source revisions aligned with the positions.
    pub fn visuals(&self) -> &[u64] {
        &self.visuals
    }

    /// Validity of each previous-position sample.
    pub fn history_valid(&self) -> &[bool] {
        &self.history_valid
    }
}

/// Two reusable CPU frames: one committed read frame and one staging frame.
/// This first A2 path is synchronous; leased async packets and GPU resource
/// retirement remain separate construction work.
pub struct RenderExtractor {
    capacity: usize,
    front: RenderColumns,
    back: RenderColumns,
}

impl RenderExtractor {
    /// Reserve the maximum visual instance count in both reusable frames.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            front: RenderColumns::with_capacity(capacity),
            back: RenderColumns::with_capacity(capacity),
        }
    }

    /// Latest complete CPU render frame.
    pub fn frame(&self) -> &RenderColumns {
        &self.front
    }

    /// Copy visual samples from one committed tick and publish them together.
    /// Presentation overflow or invalid coordinates drop samples and count them.
    /// The caller supplies the authoritative world visit and cannot mutate the
    /// world while it is borrowed for this extraction.
    pub fn extract(
        &mut self,
        source: SourceTick,
        origin: [f64; 3],
        gather: impl FnOnce(&mut RenderSink<'_>),
    ) -> ExtractReport {
        let adjacent = self.front.source.is_some_and(|previous| {
            previous.epoch == source.epoch && previous.tick.checked_add(1) == Some(source.tick)
        });
        self.back.clear(
            source,
            origin,
            if adjacent { self.front.origin } else { origin },
        );
        let mut sink = RenderSink {
            previous: &self.front,
            staging: &mut self.back,
            origin,
            adjacent,
            capacity: self.capacity,
            report: ExtractReport::default(),
        };
        gather(&mut sink);
        let report = sink.report;
        std::mem::swap(&mut self.front, &mut self.back);
        report
    }
}

/// Restricted writer for one render extraction. It only copies presentation
/// fields; it cannot modify authoritative simulation state.
pub struct RenderSink<'a> {
    previous: &'a RenderColumns,
    staging: &'a mut RenderColumns,
    origin: [f64; 3],
    adjacent: bool,
    capacity: usize,
    report: ExtractReport,
}

impl RenderSink<'_> {
    /// Append one visible entity. Returns whether it was retained.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite view-origin-relative f64 positions intentionally round once into the f32 GPU domain"
    )]
    pub fn push(&mut self, id: u128, position: [f64; 3], visual: u64) -> bool {
        if self.staging.len() == self.capacity {
            self.report.dropped_capacity += 1;
            return false;
        }
        // Each axis uses its matching origin. Reject coordinates outside the
        // selected GPU-local range before the deliberate f64 -> f32 conversion.
        let relative = [
            position[0] - self.origin[0],
            position[1] - self.origin[1],
            position[2] - self.origin[2],
        ];
        if !relative
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 1e6)
        {
            self.report.dropped_invalid += 1;
            return false;
        }
        let index = self.staging.len();
        let Entry::Vacant(entry) = self.staging.index.entry(id) else {
            self.report.dropped_duplicate += 1;
            return false;
        };
        let current = relative.map(|value| value as f32);
        let prior = self
            .adjacent
            .then(|| self.previous.index.get(&id).copied())
            .flatten()
            .filter(|&old| self.previous.visuals[old] == visual);
        let previous = prior.map_or(current, |old| self.previous.current[old]);
        entry.insert(index);
        self.staging.ids.push(id);
        self.staging.current.push(current);
        self.staging.previous.push(previous);
        self.staging.visuals.push(visual);
        self.staging.history_valid.push(prior.is_some());
        self.report.accepted += 1;
        self.report.history_reused += usize::from(prior.is_some());
        true
    }
}
