//! Reused CPU render columns derived from a committed simulation tick. These
//! presentation copies never become a second world authority.

/// Identity of the committed world endpoint used for one extraction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTick {
    /// World generation; a restart invalidates all prior samples.
    pub epoch: u128,
    /// Complete committed simulation tick.
    pub tick: u64,
    /// Replay branch generation; restore invalidates prior pose history.
    pub timeline: u64,
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
    /// Repeated physical slots dropped after the first sample.
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
    slots: Vec<u32>,
    current: Vec<[f32; 3]>,
    previous: Vec<[f32; 3]>,
    visuals: Vec<u64>,
    history_valid: Vec<bool>,
    index: Vec<Option<(u64, usize)>>,
}

impl RenderColumns {
    fn with_capacity(slot_capacity: usize, visual_capacity: usize) -> Self {
        Self {
            source: None,
            origin: [0.0; 3],
            previous_origin: [0.0; 3],
            ids: Vec::with_capacity(visual_capacity),
            slots: Vec::with_capacity(visual_capacity),
            current: Vec::with_capacity(visual_capacity),
            previous: Vec::with_capacity(visual_capacity),
            visuals: Vec::with_capacity(visual_capacity),
            history_valid: Vec::with_capacity(visual_capacity),
            index: vec![None; slot_capacity],
        }
    }

    fn clear(&mut self, source: SourceTick, origin: [f64; 3], previous_origin: [f64; 3]) {
        self.source = Some(source);
        self.origin = origin;
        self.previous_origin = previous_origin;
        self.ids.clear();
        self.slots.clear();
        self.current.clear();
        self.previous.clear();
        self.visuals.clear();
        self.history_valid.clear();
        self.index.fill(None);
    }

    fn copy_from(&mut self, previous: &Self) {
        self.source = previous.source;
        self.origin = previous.origin;
        self.previous_origin = previous.previous_origin;
        self.ids.clone_from(&previous.ids);
        self.slots.clone_from(&previous.slots);
        self.current.clone_from(&previous.current);
        self.previous.clone_from(&previous.previous);
        self.visuals.clone_from(&previous.visuals);
        self.history_valid.clone_from(&previous.history_valid);
        self.index.clone_from(&previous.index);
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

    /// Physical world slots aligned with the visual rows.
    pub fn slots(&self) -> &[u32] {
        &self.slots
    }

    /// Generation of a visible physical slot, retained in the extraction index.
    pub fn generation_at_slot(&self, slot: usize) -> Option<u64> {
        self.index.get(slot)?.map(|(generation, _)| generation)
    }

    /// Whether this extraction retained a visible row at a physical slot.
    pub fn contains_slot(&self, slot: usize) -> bool {
        self.index.get(slot).is_some_and(Option::is_some)
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
    dropped_capacity: bool,
}

impl RenderExtractor {
    /// Reserve physical slot lookup and retained visual rows separately.
    pub fn new(slot_capacity: usize, visual_capacity: usize) -> Self {
        Self {
            capacity: visual_capacity,
            front: RenderColumns::with_capacity(slot_capacity, visual_capacity),
            back: RenderColumns::with_capacity(slot_capacity, visual_capacity),
            dropped_capacity: false,
        }
    }

    /// Latest complete CPU render frame.
    pub fn frame(&self) -> &RenderColumns {
        &self.front
    }

    /// Invalidate prior-pose matching after a visual generation changes.
    /// The next extraction rebuilds current columns without adjacent history,
    /// even if the world tick and per-row visual selectors are unchanged.
    pub fn invalidate_history(&mut self) {
        self.front.source = None;
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
        let same_source = self.front.source == Some(source);
        let adjacent = self.front.source.is_some_and(|previous| {
            previous.epoch == source.epoch
                && previous.timeline == source.timeline
                && previous.tick.checked_add(1) == Some(source.tick)
        });
        self.back.clear(
            source,
            origin,
            if same_source {
                self.front.previous_origin
            } else if adjacent {
                self.front.origin
            } else {
                origin
            },
        );
        let mut sink = RenderSink {
            previous: &self.front,
            staging: &mut self.back,
            origin,
            adjacent,
            same_source,
            capacity: self.capacity,
            report: ExtractReport::default(),
        };
        gather(&mut sink);
        let report = sink.report;
        std::mem::swap(&mut self.front, &mut self.back);
        self.dropped_capacity = report.dropped_capacity != 0;
        report
    }

    /// Reuse the current visual table when an adjacent page names that exact
    /// endpoint as its source. The changed writer visits only endpoint changes;
    /// full reconstruction handles skipped pages, replay and origin changes.
    pub fn extract_adaptive(
        &mut self,
        source: SourceTick,
        source_endpoint: Option<SourceTick>,
        origin: [f64; 3],
        full: impl FnOnce(&mut RenderSink<'_>),
        changed: impl FnOnce(&mut RenderPatchSink<'_>),
    ) -> ExtractReport {
        let can_patch = !self.dropped_capacity
            && self.front.source == source_endpoint
            && self.front.source.is_some_and(|previous| {
                previous.epoch == source.epoch
                    && previous.timeline == source.timeline
                    && previous.tick.checked_add(1) == Some(source.tick)
            })
            && self
                .front
                .origin
                .iter()
                .zip(origin)
                .all(|(previous, current)| previous.to_bits() == current.to_bits());
        if !can_patch {
            return self.extract(source, origin, full);
        }
        // Keep the prior endpoint intact until every admission succeeds. If
        // a new low slot meets a full table before a high slot retires, the
        // complete page must choose its rows in physical slot order.
        self.back.copy_from(&self.front);
        self.back.previous.copy_from_slice(&self.front.current);
        self.back.history_valid.fill(true);
        self.back.previous_origin = self.front.origin;
        self.back.source = Some(source);
        let mut sink = RenderPatchSink {
            frame: &mut self.back,
            origin,
            capacity: self.capacity,
            report: ExtractReport::default(),
        };
        changed(&mut sink);
        sink.report.accepted = sink.frame.len();
        sink.report.history_reused = sink
            .frame
            .history_valid
            .iter()
            .filter(|valid| **valid)
            .count();
        let report = sink.report;
        if report.dropped_capacity != 0 {
            return self.extract(source, origin, full);
        }
        std::mem::swap(&mut self.front, &mut self.back);
        self.dropped_capacity = false;
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
    same_source: bool,
    capacity: usize,
    report: ExtractReport,
}

impl RenderSink<'_> {
    /// Append one visible entity. Returns whether it was retained.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite view-origin-relative f64 positions intentionally round once into the f32 GPU domain"
    )]
    pub fn push(
        &mut self,
        slot: u32,
        generation: u64,
        id: u128,
        position: [f64; 3],
        visual: u64,
    ) -> bool {
        if self.staging.len() == self.capacity {
            self.report.dropped_capacity += 1;
            return false;
        }
        let Ok(slot_index) = usize::try_from(slot) else {
            self.report.dropped_invalid += 1;
            return false;
        };
        if slot_index >= self.staging.index.len() {
            self.report.dropped_invalid += 1;
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
        if self.staging.index[slot_index].is_some() {
            self.report.dropped_duplicate += 1;
            return false;
        }
        let current = relative.map(|value| value as f32);
        let prior = (self.adjacent || self.same_source)
            .then(|| self.previous.index[slot_index])
            .flatten()
            .filter(|&(old_generation, old)| {
                old_generation == generation
                    && self.previous.ids[old] == id
                    && self.previous.visuals[old] == visual
            })
            .map(|(_, old)| old)
            .filter(|&old| !self.same_source || self.previous.history_valid[old]);
        let previous = prior.map_or(current, |old| {
            if self.same_source {
                self.previous.previous[old]
            } else {
                self.previous.current[old]
            }
        });
        self.staging.index[slot_index] = Some((generation, index));
        self.staging.ids.push(id);
        self.staging.slots.push(slot);
        self.staging.current.push(current);
        self.staging.previous.push(previous);
        self.staging.visuals.push(visual);
        self.staging.history_valid.push(prior.is_some());
        self.report.accepted += 1;
        self.report.history_reused += usize::from(prior.is_some());
        true
    }
}

/// Changed physical slots from an adjacent committed publication. This writer
/// updates a derived visual table and cannot alter the authoritative page.
pub struct RenderPatchSink<'a> {
    frame: &'a mut RenderColumns,
    origin: [f64; 3],
    capacity: usize,
    report: ExtractReport,
}

impl RenderPatchSink<'_> {
    /// Replace the visible row at one physical slot. An invalid presentation
    /// coordinate retires its old row and increments the drop count.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite view-origin-relative f64 positions intentionally round once into the f32 GPU domain"
    )]
    pub fn set(
        &mut self,
        slot: u32,
        generation: u64,
        id: u128,
        position: [f64; 3],
        visual: u64,
    ) -> bool {
        let Ok(slot_index) = usize::try_from(slot) else {
            self.report.dropped_invalid += 1;
            return false;
        };
        if slot_index >= self.frame.index.len() {
            self.report.dropped_invalid += 1;
            return false;
        }
        let relative = [
            position[0] - self.origin[0],
            position[1] - self.origin[1],
            position[2] - self.origin[2],
        ];
        if !relative
            .iter()
            .all(|value| value.is_finite() && value.abs() <= 1e6)
        {
            self.remove(slot);
            self.report.dropped_invalid += 1;
            return false;
        }
        let current = relative.map(|value| value as f32);
        if let Some((old_generation, row)) = self.frame.index[slot_index] {
            let same_visual = old_generation == generation
                && self.frame.ids[row] == id
                && self.frame.visuals[row] == visual;
            self.frame.ids[row] = id;
            self.frame.current[row] = current;
            self.frame.visuals[row] = visual;
            self.frame.index[slot_index] = Some((generation, row));
            if !same_visual {
                self.frame.previous[row] = current;
                self.frame.history_valid[row] = false;
            }
            return true;
        }
        if self.frame.len() == self.capacity {
            self.report.dropped_capacity += 1;
            return false;
        }
        let row = self.frame.len();
        self.frame.index[slot_index] = Some((generation, row));
        self.frame.ids.push(id);
        self.frame.slots.push(slot);
        self.frame.current.push(current);
        self.frame.previous.push(current);
        self.frame.visuals.push(visual);
        self.frame.history_valid.push(false);
        true
    }

    /// Retire a missing or nonvisual physical slot, including a slot reused
    /// for an entity without a visual component.
    /// # Panics
    /// Only if a retained physical slot no longer fits the target's index width.
    pub fn remove(&mut self, slot: u32) {
        let Ok(slot_index) = usize::try_from(slot) else {
            self.report.dropped_invalid += 1;
            return;
        };
        let Some(entry) = self.frame.index.get_mut(slot_index) else {
            self.report.dropped_invalid += 1;
            return;
        };
        let Some((_, row)) = entry.take() else {
            return;
        };
        self.frame.ids.swap_remove(row);
        self.frame.slots.swap_remove(row);
        self.frame.current.swap_remove(row);
        self.frame.previous.swap_remove(row);
        self.frame.visuals.swap_remove(row);
        self.frame.history_valid.swap_remove(row);
        if let Some(&moved_slot) = self.frame.slots.get(row) {
            let moved_index = usize::try_from(moved_slot).expect("retained slot fits usize");
            if let Some((generation, _)) = self.frame.index[moved_index] {
                self.frame.index[moved_index] = Some((generation, row));
            }
        }
    }
}
