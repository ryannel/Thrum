//! Registry-selected immutable page columns. Type erasure is resolved once per
//! column operation; selected slot loops use direct typed indexing and equality.

use std::{any::Any, mem::size_of};

use crate::{Handle, StableId, WorldEpoch};

use super::{
    ComponentKey, ComponentRegistry, DirtyPage, SnapshotId, StructuralChange,
    dense_directory::DenseDirectory,
    registered_world::{RegisteredWorld, WorldUndo},
};

pub(super) struct PublicationCopy<T> {
    pub(super) snapshot: fn(&T) -> T,
    pub(super) nested_bytes: fn(&T) -> usize,
}

pub(super) type PublicationFactory =
    Box<dyn Fn(usize) -> Box<dyn ErasedPublicationColumn> + Send + Sync>;

pub(super) struct WarmColumnCapture<'a> {
    world: &'a RegisteredWorld,
    component: usize,
    source: &'a dyn ErasedPublicationColumn,
    directory: Option<&'a DenseDirectory>,
    source_directory: Option<&'a DenseDirectory>,
    page_matches_world: bool,
    page_matches_source: bool,
    page_words: &'a [u64],
    page_structural: &'a [u64],
    outbound_words: &'a [u64],
    outbound_structural: &'a [u64],
    identities: &'a [u64],
    pair_valid: &'a [u64],
    final_undo: Option<&'a WorldUndo>,
    scratch: &'a mut [u64],
    endpoint: &'a mut [u64],
    pair: &'a mut [u64],
    changed: &'a mut ChangedColumn,
}

pub(super) trait ErasedPublicationColumn: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn capture(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        directory: Option<&DenseDirectory>,
    );
    fn capture_warm(&mut self, capture: WarmColumnCapture<'_>);
    fn refresh_words(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        component_words: &[u64],
        structural_words: &[u64],
        directory: Option<&DenseDirectory>,
    );
    fn reset_previous(&mut self, slots: &[u32], directory: Option<&DenseDirectory>);
    fn clear_previous_overrides(&mut self);
    fn prior_slots(&self) -> &[u32];
    fn is_temporal(&self) -> bool;
    fn apply_previous(
        &mut self,
        undo: &WorldUndo,
        component: usize,
        seen_words: &mut [u64],
        directory: Option<&DenseDirectory>,
    );
    fn diff(
        &self,
        baseline: &dyn ErasedPublicationColumn,
        candidates: &[u64],
        dirty: &mut [u64],
        temporal_pair: bool,
        identities: Option<&[u64]>,
        directories: (Option<&DenseDirectory>, Option<&DenseDirectory>),
    );
    fn mark_present(&self, dirty: &mut [u64], directory: Option<&DenseDirectory>);
    fn patch(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        slots: &[u32],
        dirty: &mut [u64],
        identities: &[u64],
        directory: Option<&DenseDirectory>,
    );
    fn swap_rows(&mut self, row: usize, other: usize);
    fn reserved_bytes(&self) -> usize;
}

/// Physical-row values owned by an immutable publication page. A previous
/// column exists only when the registry selected temporal sampling for `T`.
pub(super) struct PublicationColumn<T> {
    current: Vec<Option<T>>,
    previous: Option<Vec<Option<T>>>,
    // Overrides address physical rows; candidates and undo identities stay logical.
    override_words: Vec<u64>,
    prior_slots: Vec<u32>,
    snapshot: fn(&T) -> T,
    nested_bytes: fn(&T) -> usize,
}

impl<T> PublicationColumn<T> {
    pub(super) fn new(
        capacity: usize,
        temporal: bool,
        snapshot: fn(&T) -> T,
        nested_bytes: fn(&T) -> usize,
    ) -> Self {
        Self {
            current: std::iter::repeat_with(|| None).take(capacity).collect(),
            previous: temporal.then(|| std::iter::repeat_with(|| None).take(capacity).collect()),
            override_words: if temporal {
                vec![0; capacity.div_ceil(64)]
            } else {
                Vec::new()
            },
            prior_slots: Vec::with_capacity(if temporal { capacity } else { 0 }),
            snapshot,
            nested_bytes,
        }
    }

    /// Whether this selected column stores adjacent-tick previous values.
    fn supports_previous(&self) -> bool {
        self.previous.is_some()
    }

    fn previous_at(&self, index: usize) -> Option<&T> {
        let previous = self.previous.as_ref()?;
        let value = self.current.get(index)?;
        if self.override_words[index / 64] & (1 << (index % 64)) == 0 {
            value.as_ref()
        } else {
            previous[index].as_ref()
        }
    }

    /// Clear selected previous-tick overrides so previous equals current.
    fn reset_previous(&mut self, slots: &[u32], directory: Option<&DenseDirectory>) {
        if let Some(previous) = &mut self.previous {
            for &slot in slots {
                let index = row_of(directory, slot as usize);
                self.override_words[index / 64] &= !(1 << (index % 64));
                previous[index] = None;
            }
        }
    }

    fn clear_previous_overrides(&mut self) {
        if let Some(previous) = &mut self.previous {
            if std::mem::needs_drop::<T>() {
                for (word, &overrides) in self.override_words.iter().enumerate() {
                    let mut bits = overrides;
                    while bits != 0 {
                        let row = word * 64 + bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        previous[row] = None;
                    }
                }
            }
            self.prior_slots.clear();
            self.override_words.fill(0);
        }
    }

    /// Overlay an owned independent copy of one final-tick preimage. Bind this
    /// typed column once, then apply preimages without per-row type lookup.
    fn set_previous(&mut self, slot: u32, row: usize, prior: Option<&T>) {
        if let Some(previous) = &mut self.previous {
            let index = row;
            previous[index] = prior.map(self.snapshot);
            self.override_words[index / 64] |= 1 << (index % 64);
            self.prior_slots.push(slot);
        }
    }

    fn refresh_slot(&mut self, source: &[Option<T>], slot: usize, source_row: usize)
    where
        T: PartialEq,
    {
        if self.current[slot] != source[source_row] {
            self.current[slot] = source[source_row].as_ref().map(self.snapshot);
        }
    }
}

impl<T: PartialEq + Send + Sync + 'static> ErasedPublicationColumn for PublicationColumn<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn capture(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        directory: Option<&DenseDirectory>,
    ) {
        self.clear_previous_overrides();
        let source = world.column_at::<T>(component).values();
        assert_eq!(source.len(), self.current.len(), "publication capacity");
        for (page_row, destination) in self.current.iter_mut().enumerate() {
            let slot = directory.map_or(page_row, |directory| directory.slot_at(page_row));
            let row = world.row_of_slot(u32::try_from(slot).expect("validated capacity"));
            *destination = source[row].as_ref().map(self.snapshot);
        }
    }

    fn capture_warm(&mut self, capture: WarmColumnCapture<'_>) {
        let source = capture
            .source
            .as_any()
            .downcast_ref::<Self>()
            .expect("publication registry schema");
        if let Some(undo) = capture.final_undo {
            self.apply_previous(undo, capture.component, capture.scratch, capture.directory);
        } else {
            self.clear_previous_overrides();
        }
        let world = capture.world.column_at::<T>(capture.component).values();
        let temporal = self.supports_previous();
        capture.scratch.fill(0);
        if temporal {
            for &slot in self.prior_slots.iter().chain(&source.prior_slots) {
                capture.scratch[slot as usize / 64] |= 1 << (slot % 64);
            }
        }
        for word in 0..capture.page_words.len() {
            let refresh = capture.page_words[word] | capture.page_structural[word];
            let mut candidates =
                refresh | capture.outbound_words[word] | capture.outbound_structural[word];
            if temporal {
                candidates |= capture.scratch[word];
            }
            let mut endpoint = 0;
            let mut pair = 0;
            while candidates != 0 {
                let bit = candidates.trailing_zeros() as usize;
                let mask = 1_u64 << bit;
                candidates &= candidates - 1;
                let index = word * 64 + bit;
                if index >= self.current.len() {
                    continue;
                }
                let page_row = row_of(capture.directory, index);
                let source_row = if capture.page_matches_source {
                    page_row
                } else {
                    row_of(capture.source_directory, index)
                };
                if refresh & mask != 0 {
                    let row = if capture.page_matches_world {
                        page_row
                    } else {
                        capture
                            .world
                            .row_of_slot(u32::try_from(index).expect("validated capacity"))
                    };
                    self.refresh_slot(world, page_row, row);
                }
                if temporal && capture.pair_valid[word] & mask == 0 {
                    self.override_words[page_row / 64] &= !(1 << (page_row % 64));
                    self.previous.as_mut().expect("temporal column")[page_row] = None;
                }
                let current_changed = self.current[page_row] != source.current[source_row]
                    || (self.current[page_row].is_some() && capture.identities[word] & mask != 0);
                let slot = u32::try_from(index).expect("validated slot address");
                if current_changed {
                    endpoint |= mask;
                    capture.changed.slots.push(slot);
                }
                if temporal
                    && (current_changed
                        || self.previous_at(page_row) != source.previous_at(source_row))
                {
                    pair |= mask;
                    capture.changed.temporal.push(slot);
                }
            }
            capture.endpoint[word] = endpoint;
            capture.pair[word] = pair;
        }
    }

    fn refresh_words(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        component_words: &[u64],
        structural_words: &[u64],
        directory: Option<&DenseDirectory>,
    ) {
        let source = world.column_at::<T>(component).values();
        for (word, (&component, &structural)) in
            component_words.iter().zip(structural_words).enumerate()
        {
            let mut bits = component | structural;
            while bits != 0 {
                let index = word * 64 + bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if index < self.current.len() {
                    let row = world.row_of_slot(u32::try_from(index).expect("validated capacity"));
                    self.refresh_slot(source, row_of(directory, index), row);
                }
            }
        }
    }

    fn reset_previous(&mut self, slots: &[u32], directory: Option<&DenseDirectory>) {
        Self::reset_previous(self, slots, directory);
    }

    fn clear_previous_overrides(&mut self) {
        Self::clear_previous_overrides(self);
    }

    fn prior_slots(&self) -> &[u32] {
        &self.prior_slots
    }

    fn is_temporal(&self) -> bool {
        self.previous.is_some()
    }

    fn apply_previous(
        &mut self,
        undo: &WorldUndo,
        component: usize,
        seen_words: &mut [u64],
        directory: Option<&DenseDirectory>,
    ) {
        if self.previous.is_some() {
            self.clear_previous_overrides();
            seen_words.fill(0);
            undo.visit_first_preimages::<T>(component, seen_words, |slot, value| {
                self.set_previous(slot, row_of(directory, slot as usize), value);
            });
        }
    }

    fn diff(
        &self,
        baseline: &dyn ErasedPublicationColumn,
        candidates: &[u64],
        dirty: &mut [u64],
        temporal_pair: bool,
        identities: Option<&[u64]>,
        directories: (Option<&DenseDirectory>, Option<&DenseDirectory>),
    ) {
        let baseline = baseline
            .as_any()
            .downcast_ref::<Self>()
            .expect("publication registry schema");
        if temporal_pair && !(self.supports_previous() && baseline.supports_previous()) {
            return;
        }
        for (word, &candidates) in candidates.iter().enumerate() {
            let mut remaining = candidates;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                remaining &= remaining - 1;
                let index = word * 64 + bit;
                if index >= self.current.len() {
                    continue;
                }
                let row = row_of(directories.0, index);
                let baseline_row = row_of(directories.1, index);
                if self.current[row] != baseline.current[baseline_row]
                    || (temporal_pair
                        && self.previous_at(row) != baseline.previous_at(baseline_row))
                    || (self.current[row].is_some()
                        && identities.is_some_and(|words| words[word] & (1 << bit) != 0))
                {
                    dirty[word] |= 1_u64 << bit;
                }
            }
        }
    }

    fn mark_present(&self, dirty: &mut [u64], directory: Option<&DenseDirectory>) {
        for (row, value) in self.current.iter().enumerate() {
            if value.is_some() {
                let slot = directory.map_or(row, |directory| directory.slot_at(row));
                dirty[slot / 64] |= 1 << (slot % 64);
            }
        }
    }

    fn patch(
        &mut self,
        world: &RegisteredWorld,
        component: usize,
        slots: &[u32],
        dirty: &mut [u64],
        identities: &[u64],
        directory: Option<&DenseDirectory>,
    ) {
        let source = world.column_at::<T>(component).values();
        for &slot in slots {
            let index = slot as usize;
            let row = world.row_of_slot(slot);
            let page_row = row_of(directory, index);
            if self.current[page_row] != source[row]
                || (source[row].is_some() && identities[index / 64] & (1 << (index % 64)) != 0)
            {
                dirty[index / 64] |= 1 << (index % 64);
            }
            self.refresh_slot(source, row_of(directory, index), row);
        }
        self.clear_previous_overrides();
    }

    fn swap_rows(&mut self, row: usize, other: usize) {
        self.current.swap(row, other);
    }

    fn reserved_bytes(&self) -> usize {
        let values_bytes = |values: &Vec<Option<T>>| {
            values.capacity() * size_of::<Option<T>>()
                + values
                    .iter()
                    .flatten()
                    .map(self.nested_bytes)
                    .sum::<usize>()
        };
        size_of::<Self>()
            + values_bytes(&self.current)
            + self.previous.as_ref().map_or(0, values_bytes)
            + self.override_words.capacity() * size_of::<u64>()
            + self.prior_slots.capacity() * size_of::<u32>()
    }
}

struct SelectedColumn {
    key: ComponentKey,
    component: usize,
    values: Box<dyn ErasedPublicationColumn>,
}

/// Registry-selected page storage. Construction and initial capture are cold;
/// refresh and diff reuse fixed-capacity storage. Snapshot callbacks for values
/// with nested allocations remain responsible for their own copying costs.
struct PublicationColumns {
    selected: Vec<SelectedColumn>,
    directory: Option<DenseDirectory>,
    capacity: usize,
}

impl PublicationColumns {
    /// Allocate every selected component's page storage during runtime setup.
    fn new(registry: &ComponentRegistry, capacity: usize) -> Self {
        Self {
            selected: registry
                .entries
                .iter()
                .enumerate()
                .filter_map(|(component, entry)| {
                    entry.publication.as_ref().map(|factory| SelectedColumn {
                        key: entry.key,
                        component,
                        values: factory(capacity),
                    })
                })
                .collect(),
            directory: registry
                .activity_partition
                .map(|_| DenseDirectory::new(capacity)),
            capacity,
        }
    }

    /// Resolve a selected type once before reading a page's slot loop.
    fn column<T: 'static>(&self) -> Option<&PublicationColumn<T>> {
        self.selected
            .iter()
            .find(|column| column.key == ComponentKey::of::<T>())?
            .values
            .as_any()
            .downcast_ref()
    }

    pub(super) fn capture(&mut self, world: &RegisteredWorld) {
        match (&mut self.directory, world.directory()) {
            (Some(directory), Some(source)) => directory.clone_from(source),
            (None, None) => {}
            _ => panic!("publication activity partition schema"),
        }
        for column in &mut self.selected {
            column
                .values
                .capture(world, column.component, self.directory.as_ref());
        }
    }

    fn reconcile_layout(&mut self, world: &RegisteredWorld, component: &[u64], structural: &[u64]) {
        // Only current rows move. Final-tick preimages rebuild the adjacent pair
        // after this page's independent directory has reached current membership.
        self.clear_previous_overrides();
        let Some(directory) = &mut self.directory else {
            return;
        };
        for slot in dirty_slots(component, structural, self.capacity) {
            if let Some(swap) = directory.transition(slot as usize, world.active_at_slot(slot)) {
                for column in &mut self.selected {
                    column.values.swap_rows(swap.row, swap.boundary_row);
                }
            }
        }
    }

    /// Refresh one typed component from component/structural dirty bitsets.
    /// Both bitsets have one word per 64 slots; unused padding bits are ignored.
    pub(super) fn refresh_component_words(
        &mut self,
        world: &RegisteredWorld,
        key: ComponentKey,
        component_words: &[u64],
        structural_words: &[u64],
    ) {
        assert_eq!(
            component_words.len(),
            self.capacity.div_ceil(64),
            "component dirty capacity"
        );
        assert_eq!(
            structural_words.len(),
            component_words.len(),
            "structural dirty capacity"
        );
        if let Some(column) = self.selected.iter_mut().find(|column| column.key == key) {
            column.values.refresh_words(
                world,
                column.component,
                component_words,
                structural_words,
                self.directory.as_ref(),
            );
        }
    }

    /// Reset stale temporal entries from the refreshed current endpoint. This
    /// union also includes entries whose preceding pair had a different prior
    /// value, so a stopped component does not retain an old interpolation pose.
    pub(super) fn reset_previous(&mut self, slots: &[u32]) {
        for column in &mut self.selected {
            column.values.reset_previous(slots, self.directory.as_ref());
        }
    }

    pub(super) fn clear_previous_overrides(&mut self) {
        for column in &mut self.selected {
            column.values.clear_previous_overrides();
        }
    }

    /// Apply final-tick first-write preimages after resetting temporal entries.
    /// Scratch is one fixed-capacity slot bitset, reused for every component.
    pub(super) fn apply_previous(&mut self, undo: &WorldUndo, seen_words: &mut [u64]) {
        assert!(
            seen_words.len() >= self.capacity.div_ceil(64),
            "publication scratch capacity"
        );
        for column in &mut self.selected {
            column.values.apply_previous(
                undo,
                column.component,
                seen_words,
                self.directory.as_ref(),
            );
        }
    }

    /// Inline and nested component storage retained by this page.
    fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self
                .directory
                .as_ref()
                .map_or(0, DenseDirectory::reserved_bytes)
            + self.selected.capacity() * size_of::<SelectedColumn>()
            + self
                .selected
                .iter()
                .map(|column| column.values.reserved_bytes())
                .sum::<usize>()
    }
}

/// Reusable per-component page differences. Structural identity and generation
/// differences are owned by the runtime and are separate from these values.
struct PublicationDiff {
    columns: Vec<(ComponentKey, Vec<u64>)>,
}

impl PublicationDiff {
    /// Allocate bitsets once for the selected page schema and slot capacity.
    fn new(columns: &PublicationColumns) -> Self {
        Self {
            columns: columns
                .selected
                .iter()
                .map(|column| (column.key, vec![0; columns.capacity.div_ceil(64)]))
                .collect(),
        }
    }

    /// Clear the prior publication's changes while retaining every allocation.
    fn clear(&mut self) {
        for (_, words) in &mut self.columns {
            words.fill(0);
        }
    }
}

fn row_of(directory: Option<&DenseDirectory>, slot: usize) -> usize {
    directory.map_or(slot, |directory| directory.row_of(slot))
}

fn same_layout(left: Option<&DenseDirectory>, right: Option<&DenseDirectory>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.shares_layout(right),
        (None, None) => true,
        _ => false,
    }
}

fn dirty_slots<'a>(
    component: &'a [u64],
    structural: &'a [u64],
    capacity: usize,
) -> impl Iterator<Item = u32> + 'a {
    component.iter().zip(structural).enumerate().flat_map(
        move |(word, (&component, &structural))| {
            let mut remaining = component | structural;
            std::iter::from_fn(move || {
                if remaining == 0 {
                    return None;
                }
                let index = word * 64 + remaining.trailing_zeros() as usize;
                remaining &= remaining - 1;
                (index < capacity).then(|| u32::try_from(index).expect("validated slot address"))
            })
        },
    )
}

struct ChangedColumn {
    key: ComponentKey,
    slots: Vec<u32>,
    temporal: Vec<u32>,
}

/// Exact endpoint and temporal-pair changes against the last published page.
/// A component that returned to its published value is absent from its list.
pub struct PublicationChanges {
    columns: Vec<ChangedColumn>,
    structural: Vec<u32>,
}

impl PublicationChanges {
    /// Bind one component's endpoint differences before iterating its slots.
    pub fn component<T: 'static>(&self) -> &[u32] {
        self.columns
            .iter()
            .find_map(|column| {
                (column.key == ComponentKey::of::<T>()).then_some(column.slots.as_slice())
            })
            .unwrap_or(&[])
    }

    /// Bind temporal pair differences, including a stopped previous value.
    pub fn temporal<T: 'static>(&self) -> &[u32] {
        self.columns
            .iter()
            .find_map(|column| {
                (column.key == ComponentKey::of::<T>()).then_some(column.temporal.as_slice())
            })
            .unwrap_or(&[])
    }

    /// Slots with a net identity or generation transition.
    pub fn structural(&self) -> &[u32] {
        &self.structural
    }

    fn clear(&mut self) {
        self.structural.clear();
        for column in &mut self.columns {
            column.slots.clear();
            column.temporal.clear();
        }
    }
}

/// One immutable engine-owned committed endpoint and its adjacent temporal data.
/// Game types remain in registry-selected columns; page layout and lifecycle
/// differences belong to the engine.
pub struct PublicationPage {
    columns: PublicationColumns,
    id: SnapshotId,
    timeline: u64,
    previous_id: Option<SnapshotId>,
    ids: Vec<Option<StableId>>,
    generations: Vec<u64>,
    live_words: Vec<u64>,
    pair_valid: Vec<u64>,
    identities: Vec<u64>,
    scratch_words: Vec<u64>,
    candidates: Vec<u32>,
    endpoint: PublicationDiff,
    pair: PublicationDiff,
    dirty: PublicationChanges,
    events: Vec<StructuralChange>,
    live: usize,
    initialized: bool,
    baseline: bool,
    patch_source: Option<(SnapshotId, u64)>,
    source_endpoint: Option<(SnapshotId, u64)>,
}

/// Capture inputs are engine-owned dirty unions and final-tick undo data.
/// A missing final undo explicitly disables adjacent temporal sampling.
#[derive(Clone, Copy)]
pub(super) struct PublicationCapture<'a> {
    pub(super) cold: bool,
    pub(super) page_dirty: DirtyPage<'a>,
    pub(super) outbound_dirty: DirtyPage<'a>,
    pub(super) source: Option<&'a PublicationPage>,
    pub(super) tick: u64,
    pub(super) timeline: u64,
    pub(super) final_undo: Option<&'a WorldUndo>,
    pub(super) final_events: &'a [StructuralChange],
}

impl PublicationPage {
    /// Allocate all page and scratch storage before the world begins ticking.
    pub fn new(registry: &ComponentRegistry, epoch: WorldEpoch, capacity: usize) -> Self {
        let columns = PublicationColumns::new(registry, capacity);
        let endpoint = PublicationDiff::new(&columns);
        let pair = PublicationDiff::new(&columns);
        let dirty = PublicationChanges {
            columns: columns
                .selected
                .iter()
                .map(|column| ChangedColumn {
                    key: column.key,
                    slots: Vec::with_capacity(capacity),
                    temporal: Vec::with_capacity(if column.values.is_temporal() {
                        capacity
                    } else {
                        0
                    }),
                })
                .collect(),
            structural: Vec::with_capacity(capacity),
        };
        let words = capacity.div_ceil(64);
        Self {
            columns,
            endpoint,
            pair,
            dirty,
            id: SnapshotId { epoch, tick: 0 },
            timeline: 0,
            previous_id: None,
            ids: vec![None; capacity],
            generations: vec![0; capacity],
            live_words: vec![0; words],
            pair_valid: vec![0; words],
            identities: vec![0; words],
            scratch_words: vec![0; words],
            candidates: Vec::with_capacity(capacity),
            events: Vec::with_capacity(capacity.saturating_mul(2)),
            live: 0,
            initialized: false,
            baseline: false,
            patch_source: None,
            source_endpoint: None,
        }
    }

    /// Committed world generation and tick.
    pub fn id(&self) -> SnapshotId {
        self.id
    }
    /// Replay branch generation.
    pub fn timeline(&self) -> u64 {
        self.timeline
    }
    /// Previous adjacent tick, absent when its preimages were unavailable.
    pub fn previous_id(&self) -> Option<SnapshotId> {
        self.previous_id
    }
    /// Whether this page starts a complete membership stream.
    pub fn is_baseline(&self) -> bool {
        self.baseline
    }
    /// Old branch endpoint updated by this inverse patch, when applicable.
    pub fn patch_source(&self) -> Option<(SnapshotId, u64)> {
        self.patch_source
    }
    /// Last published endpoint used to derive this page's exact differences.
    pub fn source_endpoint(&self) -> Option<(SnapshotId, u64)> {
        self.source_endpoint
    }
    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.live
    }
    /// Whether no entities are live.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }
    /// Fixed physical slot capacity.
    pub fn slot_capacity(&self) -> usize {
        self.ids.len()
    }
    /// Stable identity at one live physical slot.
    pub fn stable_id(&self, slot: u32) -> Option<StableId> {
        self.ids.get(slot as usize).copied().flatten()
    }
    /// Checked handle at one live physical slot.
    pub fn handle(&self, slot: u32) -> Option<Handle> {
        self.stable_id(slot).map(|_| Handle {
            epoch: self.id.epoch,
            slot,
            generation: self.generations[slot as usize],
        })
    }
    fn column<T: 'static>(&self) -> Option<&PublicationColumn<T>> {
        self.columns.column()
    }
    /// Component read guarded by this page's epoch and slot generation.
    pub fn component<T: 'static>(&self, handle: Handle) -> Option<&T> {
        (self.handle(handle.slot) == Some(handle))
            .then(|| {
                self.column::<T>()?
                    .current
                    .get(row_of(
                        self.columns.directory.as_ref(),
                        handle.slot as usize,
                    ))?
                    .as_ref()
            })
            .flatten()
    }
    /// Previous component only when the same entity existed in the adjacent tick.
    pub fn previous_component<T: 'static>(&self, handle: Handle) -> Option<&T> {
        (self.handle(handle.slot) == Some(handle) && self.pair_valid(handle.slot))
            .then(|| {
                self.column::<T>()?.previous_at(row_of(
                    self.columns.directory.as_ref(),
                    handle.slot as usize,
                ))
            })
            .flatten()
    }
    /// Whether temporal sampling preserves this slot's entity identity.
    pub fn pair_valid(&self, slot: u32) -> bool {
        self.previous_id.is_some()
            && self
                .pair_valid
                .get(slot as usize / 64)
                .is_some_and(|word| word & (1 << (slot % 64)) != 0)
    }
    /// Net component and identity changes against the last publication.
    pub fn dirty(&self) -> &PublicationChanges {
        &self.dirty
    }
    /// Net lifecycle transitions in slot order, Despawn before Spawn on reuse.
    pub fn structural_events(&self) -> &[StructuralChange] {
        &self.events
    }
    /// Visit live identities in physical slot order without an ID lookup.
    ///
    /// # Panics
    ///
    /// Panics only if runtime setup violated the u32 slot address bound.
    pub fn visit(&self, mut visit: impl FnMut(Handle, StableId)) {
        self.visit_rows(|handle, id, _| visit(handle, id));
    }

    /// Visit two selected game components while binding their typed columns
    /// once. Missing columns or absent values are reported as `None` for each
    /// live identity; the game decides which components it requires.
    ///
    /// # Panics
    ///
    /// Panics only if runtime setup violated the u32 slot address bound.
    pub fn visit2<T: 'static, U: 'static>(
        &self,
        mut visit: impl FnMut(Handle, StableId, Option<&T>, Option<&U>),
    ) {
        let first = self.column::<T>();
        let second = self.column::<U>();
        self.visit_rows(|handle, id, row| {
            visit(
                handle,
                id,
                first.and_then(|column| column.current[row].as_ref()),
                second.and_then(|column| column.current[row].as_ref()),
            );
        });
    }

    fn visit_rows(&self, mut visit: impl FnMut(Handle, StableId, usize)) {
        for (slot, id) in self.ids.iter().enumerate() {
            if let Some(id) = id {
                let slot = u32::try_from(slot).expect("validated slot capacity");
                visit(
                    Handle {
                        epoch: self.id.epoch,
                        slot,
                        generation: self.generations[slot as usize],
                    },
                    *id,
                    row_of(self.columns.directory.as_ref(), slot as usize),
                );
            }
        }
    }

    fn refresh_identity(&mut self, world: &RegisteredWorld, slot: u32) {
        let index = slot as usize;
        let live = world.live_at_slot(slot);
        match (self.ids[index].is_some(), live.is_some()) {
            (true, false) => self.live -= 1,
            (false, true) => self.live += 1,
            _ => {}
        }
        self.ids[index] = live.map(|(_, id)| id);
        if let Some((handle, _)) = live {
            self.generations[index] = handle.generation;
            self.live_words[index / 64] |= 1 << (index % 64);
        } else {
            self.live_words[index / 64] &= !(1 << (index % 64));
        }
    }

    fn lifecycle(&mut self, slot: u32, before: Option<(StableId, Handle)>) {
        let after = self
            .handle(slot)
            .map(|handle| (self.ids[slot as usize].expect("live identity"), handle));
        if before == after {
            return;
        }
        self.identities[slot as usize / 64] |= 1 << (slot % 64);
        self.dirty.structural.push(slot);
        if let Some((_, handle)) = before {
            self.events.push(StructuralChange::Despawn(handle));
        }
        if let Some((_, handle)) = after {
            self.events.push(StructuralChange::Spawn(handle));
        }
    }

    pub(super) fn capture(&mut self, world: &RegisteredWorld, capture: PublicationCapture<'_>) {
        assert_eq!(world.capacity(), self.ids.len(), "publication capacity");
        assert_eq!(world.epoch(), self.id.epoch, "publication epoch");
        let PublicationCapture {
            cold,
            page_dirty,
            outbound_dirty,
            source,
            tick,
            timeline,
            final_undo,
            final_events,
        } = capture;
        if cold {
            self.columns.capture(world);
            for slot in 0..self.ids.len() {
                self.refresh_identity(world, u32::try_from(slot).expect("validated capacity"));
            }
        } else {
            if let Some(component) = world.activity_component() {
                self.columns.reconcile_layout(
                    world,
                    page_dirty.component_words(component),
                    page_dirty.structural_words(),
                );
            }
            for slot in page_dirty.structural_slots() {
                self.refresh_identity(world, slot);
            }
        }
        self.previous_id = final_undo
            .and_then(|_| tick.checked_sub(1))
            .map(|tick| SnapshotId {
                epoch: self.id.epoch,
                tick,
            });
        self.pair_valid.copy_from_slice(&self.live_words);
        self.scratch_words.fill(0);
        self.candidates.clear();
        for event in final_events {
            let slot = match event {
                StructuralChange::Spawn(handle) | StructuralChange::Despawn(handle) => handle.slot,
            };
            self.pair_valid[slot as usize / 64] &= !(1 << (slot % 64));
            self.scratch_words[slot as usize / 64] |= 1 << (slot % 64);
        }
        self.candidates.extend(dirty_slots(
            &self.scratch_words,
            &self.scratch_words,
            self.ids.len(),
        ));
        self.id.tick = tick;
        self.timeline = timeline;
        self.baseline = source.is_none();
        self.source_endpoint = source.map(|source| (source.id, source.timeline));
        self.patch_source = self
            .source_endpoint
            .filter(|(_, source_timeline)| *source_timeline != timeline);
        self.dirty.clear();
        self.events.clear();
        self.identities.fill(0);
        if let Some(source) = source {
            for slot in outbound_dirty.structural_slots() {
                let before = source.handle(slot).map(|handle| {
                    (
                        source.ids[slot as usize].expect("source live identity"),
                        handle,
                    )
                });
                self.lifecycle(slot, before);
            }
        } else {
            for slot in 0..self.ids.len() {
                if self.ids[slot].is_some() {
                    self.lifecycle(u32::try_from(slot).expect("validated capacity"), None);
                }
            }
        }
        if let Some(source) = source.filter(|_| !cold) {
            let page_matches_world =
                same_layout(self.columns.directory.as_ref(), world.directory());
            let page_matches_source = same_layout(
                self.columns.directory.as_ref(),
                source.columns.directory.as_ref(),
            );
            for (index, column) in self.columns.selected.iter_mut().enumerate() {
                let baseline = &source.columns.selected[index];
                assert_eq!(column.key, baseline.key, "publication schema");
                column.values.capture_warm(WarmColumnCapture {
                    world,
                    component: column.component,
                    source: baseline.values.as_ref(),
                    directory: self.columns.directory.as_ref(),
                    source_directory: source.columns.directory.as_ref(),
                    page_matches_world,
                    page_matches_source,
                    page_words: page_dirty.component_words(column.component),
                    page_structural: page_dirty.structural_words(),
                    outbound_words: outbound_dirty.component_words(column.component),
                    outbound_structural: outbound_dirty.structural_words(),
                    identities: &self.identities,
                    pair_valid: &self.pair_valid,
                    final_undo,
                    scratch: &mut self.scratch_words,
                    endpoint: &mut self.endpoint.columns[index].1,
                    pair: &mut self.pair.columns[index].1,
                    changed: &mut self.dirty.columns[index],
                });
            }
        } else {
            if !cold {
                for index in 0..self.columns.selected.len() {
                    let column = &self.columns.selected[index];
                    self.columns.refresh_component_words(
                        world,
                        column.key,
                        page_dirty.component_words(column.component),
                        page_dirty.structural_words(),
                    );
                }
            }
            self.columns.clear_previous_overrides();
            if let Some(undo) = final_undo {
                self.columns.apply_previous(undo, &mut self.scratch_words);
            }
            self.columns.reset_previous(&self.candidates);
            self.endpoint.clear();
            self.pair.clear();
            if let Some(source) = source {
                self.compare_source(source, outbound_dirty);
            } else {
                for (index, column) in self.columns.selected.iter().enumerate() {
                    column.values.mark_present(
                        &mut self.endpoint.columns[index].1,
                        self.columns.directory.as_ref(),
                    );
                    if column.values.is_temporal() {
                        column.values.mark_present(
                            &mut self.pair.columns[index].1,
                            self.columns.directory.as_ref(),
                        );
                    }
                }
            }
            self.collect_changes();
        }
        self.initialized = true;
    }

    fn compare_source(&mut self, source: &Self, outbound_dirty: DirtyPage<'_>) {
        for (index, column) in self.columns.selected.iter().enumerate() {
            let baseline = &source.columns.selected[index];
            assert_eq!(column.key, baseline.key, "publication schema");
            for (word, (&component, &structural)) in self.scratch_words.iter_mut().zip(
                outbound_dirty
                    .component_words(column.component)
                    .iter()
                    .zip(outbound_dirty.structural_words()),
            ) {
                *word = component | structural;
            }
            for &slot in column
                .values
                .prior_slots()
                .iter()
                .chain(baseline.values.prior_slots())
            {
                self.scratch_words[slot as usize / 64] |= 1 << (slot % 64);
            }
            column.values.diff(
                baseline.values.as_ref(),
                &self.scratch_words,
                &mut self.endpoint.columns[index].1,
                false,
                Some(&self.identities),
                (
                    self.columns.directory.as_ref(),
                    source.columns.directory.as_ref(),
                ),
            );
            column.values.diff(
                baseline.values.as_ref(),
                &self.scratch_words,
                &mut self.pair.columns[index].1,
                true,
                Some(&self.identities),
                (
                    self.columns.directory.as_ref(),
                    source.columns.directory.as_ref(),
                ),
            );
        }
    }

    fn collect_changes(&mut self) {
        for (index, column) in self.dirty.columns.iter_mut().enumerate() {
            let words = &self.endpoint.columns[index].1;
            column
                .slots
                .extend(dirty_slots(words, words, self.ids.len()));
            let words = &self.pair.columns[index].1;
            column
                .temporal
                .extend(dirty_slots(words, words, self.ids.len()));
        }
    }

    /// Select an initialized old-branch endpoint in the retained rewind range.
    pub fn source_tick(&self, old_timeline: u64, target: u64, latest: u64) -> Option<u64> {
        (self.initialized
            && self.timeline == old_timeline
            && (target..=latest).contains(&self.id.tick))
        .then_some(self.id.tick)
    }

    /// Refresh an unleased old-branch page from a restored world. Only slots
    /// named by retained reverse records are examined; lifecycle and component
    /// output is the exact diff against this selected source page.
    ///
    /// # Panics
    ///
    /// The world and records must belong to this page's runtime and capacity.
    pub fn patch_rewind(
        &mut self,
        world: &super::WorldRead<'_>,
        steps: &[super::RuntimeUndo],
        source: u64,
        target: u64,
        timeline: u64,
    ) {
        self.patch_changed(
            world.world,
            |mark| {
                for step in steps.iter().take_while(|step| step.tick() <= source) {
                    step.visit_changed_slots(&mut *mark);
                }
            },
            target,
            timeline,
        );
    }

    fn patch_changed(
        &mut self,
        world: &RegisteredWorld,
        visit_changed: impl FnOnce(&mut dyn FnMut(u32)),
        target: u64,
        timeline: u64,
    ) {
        assert_eq!(
            world.capacity(),
            self.ids.len(),
            "rewind publication capacity"
        );
        assert_eq!(world.epoch(), self.id.epoch, "rewind publication epoch");
        self.scratch_words.fill(0);
        visit_changed(&mut |slot| {
            self.scratch_words[slot as usize / 64] |= 1 << (slot % 64);
        });
        self.candidates.clear();
        self.candidates.extend(dirty_slots(
            &self.scratch_words,
            &self.scratch_words,
            self.ids.len(),
        ));
        self.columns
            .reconcile_layout(world, &self.scratch_words, &self.scratch_words);
        self.endpoint.clear();
        self.pair.clear();
        self.dirty.clear();
        self.events.clear();
        self.identities.fill(0);
        for index in 0..self.candidates.len() {
            let slot = self.candidates[index];
            let before = self
                .handle(slot)
                .map(|handle| (self.ids[slot as usize].expect("old live identity"), handle));
            self.refresh_identity(world, slot);
            self.lifecycle(slot, before);
        }
        for (index, column) in self.columns.selected.iter_mut().enumerate() {
            column.values.patch(
                world,
                column.component,
                &self.candidates,
                &mut self.endpoint.columns[index].1,
                &self.identities,
                self.columns.directory.as_ref(),
            );
            if column.values.is_temporal() {
                self.pair.columns[index]
                    .1
                    .copy_from_slice(&self.endpoint.columns[index].1);
            }
        }
        self.collect_changes();
        self.patch_source = Some((self.id, self.timeline));
        self.source_endpoint = self.patch_source;
        self.id.tick = target;
        self.timeline = timeline;
        self.previous_id = None;
        self.baseline = false;
        self.initialized = true;
    }

    /// Page storage and reusable scratch retained by the runtime.
    pub fn reserved_bytes(&self) -> usize {
        let column_words = self
            .endpoint
            .columns
            .iter()
            .chain(&self.pair.columns)
            .map(|(_, words)| words.capacity() * size_of::<u64>())
            .sum::<usize>();
        let changes = self
            .dirty
            .columns
            .iter()
            .map(|column| (column.slots.capacity() + column.temporal.capacity()) * size_of::<u32>())
            .sum::<usize>();
        size_of::<Self>() + self.columns.reserved_bytes() - size_of::<PublicationColumns>()
            + self.ids.capacity() * size_of::<Option<StableId>>()
            + self.generations.capacity() * size_of::<u64>()
            + (self.live_words.capacity()
                + self.pair_valid.capacity()
                + self.identities.capacity()
                + self.scratch_words.capacity())
                * size_of::<u64>()
            + self.candidates.capacity() * size_of::<u32>()
            + self.events.capacity() * size_of::<StructuralChange>()
            + self.dirty.structural.capacity() * size_of::<u32>()
            + self.dirty.columns.capacity() * size_of::<ChangedColumn>()
            + (self.endpoint.columns.capacity() + self.pair.columns.capacity())
                * size_of::<(ComponentKey, Vec<u64>)>()
            + column_words
            + changes
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod page_tests;
