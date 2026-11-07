//! Reusable immutable endpoint leases and engine-owned page dirty unions.
//! Held pages accumulate sidecar changes without mutating their leased payloads.

use std::{mem::size_of, sync::Arc};

pub(super) const PAGE_COUNT: usize = 4;

fn reusable<T>(page: &Arc<T>) -> bool {
    Arc::strong_count(page) == 1 && Arc::weak_count(page) == 0
}

struct DirtyUnion {
    components: Vec<Vec<u64>>,
    structural: Vec<u64>,
}

impl DirtyUnion {
    fn new(capacity: usize, component_count: usize) -> Self {
        Self {
            components: vec![vec![0; capacity.div_ceil(64)]; component_count],
            structural: vec![0; capacity.div_ceil(64)],
        }
    }

    fn view(&self) -> DirtyPage<'_> {
        DirtyPage {
            components: &self.components,
            structural: &self.structural,
        }
    }

    fn clear(&mut self) {
        for words in &mut self.components {
            words.fill(0);
        }
        self.structural.fill(0);
    }

    fn merge(&mut self, source: &Self) {
        for (destination, source) in self.components.iter_mut().zip(&source.components) {
            for (destination, &source) in destination.iter_mut().zip(source) {
                *destination |= source;
            }
        }
        for (destination, &source) in self.structural.iter_mut().zip(&source.structural) {
            *destination |= source;
        }
    }

    fn reserved_bytes(&self) -> usize {
        self.components.capacity() * size_of::<Vec<u64>>()
            + (self.components.iter().map(Vec::capacity).sum::<usize>()
                + self.structural.capacity())
                * size_of::<u64>()
    }
}

struct PageChanges {
    dirty: DirtyUnion,
    captured: bool,
}

fn word_slots(index: usize, mut word: u64) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        if word == 0 {
            return None;
        }
        let slot = index * 64 + word.trailing_zeros() as usize;
        word &= word - 1;
        Some(u32::try_from(slot).expect("marked slots have u32 addresses"))
    })
}

/// Borrowed engine-owned dirty unions for one page or the publication endpoint.
/// Each iterator visits physical slots once in ascending order without allocation.
#[derive(Clone, Copy)]
pub struct DirtyPage<'a> {
    components: &'a [Vec<u64>],
    structural: &'a [u64],
}

impl<'a> DirtyPage<'a> {
    /// Number of registered components represented by this dirty view.
    pub fn component_count(self) -> usize {
        self.components.len()
    }

    /// Component-only bitset, excluding structural changes.
    ///
    /// # Panics
    ///
    /// `component` must be less than `component_count()`.
    pub fn component_words(self, component: usize) -> &'a [u64] {
        &self.components[component]
    }

    /// Structural membership and identity bitset.
    pub fn structural_words(self) -> &'a [u64] {
        self.structural
    }

    /// Selected component's changed slots, including structural changes that
    /// require every published component to refresh.
    ///
    /// # Panics
    ///
    /// `component` must be less than `component_count()`.
    pub fn component_slots(self, component: usize) -> impl Iterator<Item = u32> + 'a {
        self.components[component]
            .iter()
            .zip(self.structural)
            .enumerate()
            .flat_map(|(index, (&component, &structural))| {
                word_slots(index, component | structural)
            })
    }

    /// Slots whose membership or entity identity changed.
    pub fn structural_slots(self) -> impl Iterator<Item = u32> + 'a {
        self.structural
            .iter()
            .enumerate()
            .flat_map(|(index, &word)| word_slots(index, word))
    }

    /// Union of all component and structural changes, in ascending slot order.
    pub fn slots(self) -> impl Iterator<Item = u32> + 'a {
        self.structural
            .iter()
            .enumerate()
            .flat_map(move |(index, &structural)| {
                let union = self
                    .components
                    .iter()
                    .fold(structural, |word, component| word | component[index]);
                word_slots(index, union)
            })
    }
}

/// Every preallocated page is held by a consumer or retained as a recent endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationBusy;

/// Bounded pool of reusable committed pages. A capture mutates only an unleased
/// page, then publishes it atomically as the latest endpoint. Dirty sidecars
/// retain each page's changes separately from the last-publication change union.
pub struct SnapshotRing<T> {
    pages: Vec<Arc<T>>,
    changes: Vec<PageChanges>,
    outbound: DirtyUnion,
    scratch: DirtyUnion,
    capacity: usize,
    cursor: usize,
    latest: Option<Arc<T>>,
    previous: Option<Arc<T>>,
    retired_source: Option<Arc<T>>,
}

impl<T> SnapshotRing<T> {
    /// Prepare four page payloads and dirty sidecars before ticking. `make` is
    /// called once per page. Two pages retain endpoints and two remain available
    /// for capture or external leases. Component and structural bitsets are allocated
    /// once for the fixed slot capacity.
    pub fn new(capacity: usize, component_count: usize, mut make: impl FnMut() -> T) -> Self {
        Self {
            pages: (0..PAGE_COUNT).map(|_| Arc::new(make())).collect(),
            changes: (0..PAGE_COUNT)
                .map(|_| PageChanges {
                    dirty: DirtyUnion::new(capacity, component_count),
                    captured: false,
                })
                .collect(),
            outbound: DirtyUnion::new(capacity, component_count),
            scratch: DirtyUnion::new(capacity, component_count),
            capacity,
            cursor: 0,
            latest: None,
            previous: None,
            retired_source: None,
        }
    }

    /// Record a committed or reversed component change for every page and for
    /// the next publication's endpoint diff. Duplicate marks have no extra effect.
    ///
    /// # Panics
    ///
    /// `slot` and `component` must be within their construction capacities.
    pub fn mark_component(&mut self, component: usize, slot: u32) {
        let (word, bit) = self.slot_bit(slot);
        self.outbound.components[component][word] |= bit;
        for page in &mut self.changes {
            page.dirty.components[component][word] |= bit;
        }
    }

    /// Record changed slot membership or entity identity. Every component's slot
    /// iterator includes these structural changes.
    ///
    /// # Panics
    ///
    /// `slot` must be within the capacity supplied at construction.
    pub fn mark_structural(&mut self, slot: u32) {
        let (word, bit) = self.slot_bit(slot);
        for page in &mut self.changes {
            page.dirty.structural[word] |= bit;
        }
        self.outbound.structural[word] |= bit;
    }

    /// Accumulate a component change in reusable scratch. Call `merge_changes`
    /// once after collecting a complete committed or reversed operation. Pending
    /// scratch does not alter page sidecars or the outbound publication union.
    ///
    /// # Panics
    ///
    /// `slot` and `component` must be within their construction capacities.
    pub fn accumulate_component(&mut self, component: usize, slot: u32) {
        let (word, bit) = self.slot_bit(slot);
        self.scratch.components[component][word] |= bit;
    }

    /// Accumulate a structural change in reusable scratch for `merge_changes`.
    ///
    /// # Panics
    ///
    /// `slot` must be within the capacity supplied at construction.
    pub fn accumulate_structural(&mut self, slot: u32) {
        let (word, bit) = self.slot_bit(slot);
        self.scratch.structural[word] |= bit;
    }

    /// Merge accumulated changes into outbound and every page's sidecar with
    /// sequential word loops, then clear scratch for the next operation. This
    /// never mutates leased payloads or allocates. Immediate `mark_component` and
    /// `mark_structural` changes remain in their unions.
    pub fn merge_changes(&mut self) {
        self.outbound.merge(&self.scratch);
        for page in &mut self.changes {
            page.dirty.merge(&self.scratch);
        }
        self.scratch.clear();
    }

    fn slot_bit(&self, slot: u32) -> (usize, u64) {
        let slot = slot as usize;
        assert!(slot < self.capacity, "publication slot capacity");
        (slot / 64, 1_u64 << (slot % 64))
    }

    /// Whether an unleased page exists for the next complete publication.
    pub fn can_capture(&self) -> bool {
        self.pages.iter().any(reusable)
    }

    /// Fill and publish one complete page. `fill` receives whether this payload
    /// is cold, its pending unions, the unions since publication and
    /// the immutable last-published source, including across a retired timeline.
    /// Only a cold payload needs a full capture; skipped publications do not.
    ///
    /// Validate before calling. A busy refusal leaves the cursor, endpoints and
    /// unions unchanged and never invokes `fill`. Successful capture clears only
    /// the selected page's pending union and the outbound union.
    ///
    /// # Errors
    ///
    /// Every page is leased, so optional publication must wait for a lease to drop.
    pub fn capture(
        &mut self,
        fill: impl FnOnce(&mut T, bool, DirtyPage<'_>, DirtyPage<'_>, Option<&T>),
    ) -> Result<(), PublicationBusy> {
        let index = (0..self.pages.len())
            .map(|offset| (self.cursor + offset) % self.pages.len())
            .find(|&index| reusable(&self.pages[index]))
            .ok_or(PublicationBusy)?;
        let page = Arc::get_mut(&mut self.pages[index]).ok_or(PublicationBusy)?;
        fill(
            page,
            !self.changes[index].captured,
            self.changes[index].dirty.view(),
            self.outbound.view(),
            self.latest.as_deref().or(self.retired_source.as_deref()),
        );
        self.changes[index].dirty.clear();
        self.changes[index].captured = true;
        self.outbound.clear();
        self.cursor = (index + 1) % self.pages.len();
        self.previous = self.latest.take();
        self.retired_source = None;
        self.latest = Some(Arc::clone(&self.pages[index]));
        Ok(())
    }

    /// Lease the latest complete endpoint, if one has been published.
    pub fn latest(&self) -> Option<Arc<T>> {
        self.latest.clone()
    }

    /// Whether this ring has sealed a complete endpoint on the current timeline.
    pub fn has_latest(&self) -> bool {
        self.latest.is_some()
    }

    /// Lease the endpoint immediately before latest, if available.
    pub fn previous(&self) -> Option<Arc<T>> {
        self.previous.clone()
    }

    /// Retire this timeline while retaining its last publication as a diff
    /// source. Mark reversed slots before the next capture. Existing leases and
    /// all page dirty unions remain intact; the new branch has no previous pair.
    pub(crate) fn reset_timeline(&mut self) {
        if let Some(latest) = self.latest.take() {
            self.retired_source = Some(latest);
        }
        self.previous = None;
    }

    /// Patch the newest unleased endpoint accepted by `source_tick` in place.
    /// The caller has already restored authority and invalidated old leases.
    /// A successful patch consumes that page's pending and outbound unions.
    pub(crate) fn patch_rewind(
        &mut self,
        source_tick: impl Fn(&T) -> Option<u64>,
        patch: impl FnOnce(&mut T, u64),
    ) -> Result<(), PublicationBusy> {
        let (index, tick) = self
            .pages
            .iter()
            .enumerate()
            .filter(|(_, page)| reusable(page))
            .filter_map(|(index, page)| source_tick(page).map(|tick| (index, tick)))
            .max_by_key(|&(_, tick)| tick)
            .ok_or(PublicationBusy)?;
        let page = Arc::get_mut(&mut self.pages[index]).ok_or(PublicationBusy)?;
        patch(page, tick);
        self.changes[index].dirty.clear();
        self.changes[index].captured = true;
        self.outbound.clear();
        self.retired_source = None;
        self.latest = Some(Arc::clone(&self.pages[index]));
        self.previous = None;
        self.cursor = (index + 1) % self.pages.len();
        Ok(())
    }

    /// Reserved payload and dirty sidecar storage for diagnostics.
    /// `payload_bytes` reports one page including its nested allocations.
    pub fn reserved_bytes(&self, payload_bytes: impl Fn(&T) -> usize) -> usize {
        size_of::<Self>()
            + self.pages.capacity() * size_of::<Arc<T>>()
            + self.changes.capacity() * size_of::<PageChanges>()
            + self
                .changes
                .iter()
                .map(|page| page.dirty.reserved_bytes())
                .sum::<usize>()
            + self.outbound.reserved_bytes()
            + self.scratch.reserved_bytes()
            + self
                .pages
                .iter()
                .map(|page| payload_bytes(page) + 2 * size_of::<usize>())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{PublicationBusy, SnapshotRing};

    #[test]
    fn held_pages_refuse_without_calling_fill_and_resume_after_release() {
        let mut ring = SnapshotRing::new(8, 2, || 0_u64);
        for value in 1..=2 {
            ring.capture(|page, cold, _, _, _| {
                assert!(cold);
                *page = value;
            })
            .expect("first pages");
        }
        let second = ring.latest().expect("second lease");
        let first = ring.previous().expect("first lease");
        ring.capture(|page, _, _, _, _| *page = 3)
            .expect("third page");
        ring.capture(|page, _, _, _, _| *page = 4)
            .expect("fourth page");
        ring.mark_structural(7);
        ring.mark_structural(2);
        ring.mark_structural(7);
        assert_eq!(*first, 1);
        assert_eq!(*second, 2);
        assert_eq!(
            ring.capture(|_, _, _, _, _| panic!("must not fill")),
            Err(PublicationBusy)
        );
        ring.mark_structural(4);
        assert_eq!(ring.cursor, 0, "busy capture retains the cursor");
        drop(first);
        ring.capture(|page, cold, pending, outbound, source| {
            assert!(!cold, "a publication gap does not make a warm page cold");
            assert_eq!(pending.slots().collect::<Vec<_>>(), &[2, 4, 7]);
            assert_eq!(outbound.slots().collect::<Vec<_>>(), &[2, 4, 7]);
            assert_eq!(source, Some(&4));
            *page = 5;
        })
        .expect("released page reusable");
        assert_eq!(ring.cursor, 1);
        assert_eq!(*second, 2);
        assert_eq!(*ring.latest().expect("latest"), 5);
        assert_eq!(*ring.previous().expect("previous endpoint across gap"), 4);
    }

    #[test]
    fn page_unions_accumulate_independently_and_outbound_is_exact() {
        let mut next_page = 0;
        let mut ring = SnapshotRing::new(130, 2, || {
            let page = next_page;
            next_page += 1;
            page
        });
        for tick in 0..4 {
            ring.mark_structural(tick);
            ring.capture(|page, cold, pending, outbound, source| {
                assert_eq!(*page, tick);
                assert!(cold);
                assert_eq!(
                    pending.slots().collect::<Vec<_>>(),
                    (0..=tick).collect::<Vec<_>>()
                );
                assert_eq!(outbound.slots().collect::<Vec<_>>(), &[tick]);
                assert_eq!(source, tick.checked_sub(1).as_ref());
            })
            .expect("warm every payload");
        }
        ring.mark_structural(129);
        ring.mark_structural(64);
        ring.mark_structural(3);
        ring.mark_structural(64);
        ring.capture(|page, cold, pending, outbound, source| {
            assert_eq!(*page, 0, "cursor selects first reusable page");
            assert!(!cold);
            assert_eq!(pending.slots().collect::<Vec<_>>(), &[1, 2, 3, 64, 129]);
            assert_eq!(outbound.slots().collect::<Vec<_>>(), &[3, 64, 129]);
            assert_eq!(source, Some(&3));
            *page = 4;
        })
        .expect("first page refreshed");
        ring.capture(|page, cold, pending, outbound, source| {
            assert_eq!(*page, 1);
            assert!(!cold);
            assert_eq!(pending.slots().collect::<Vec<_>>(), &[2, 3, 64, 129]);
            assert!(outbound.slots().next().is_none());
            assert_eq!(source, Some(&4));
        })
        .expect("second page has its own accumulated union");
    }

    #[test]
    fn component_unions_are_separate_and_structural_changes_refresh_every_component() {
        let mut ring = SnapshotRing::new(130, 2, || ());
        ring.mark_component(1, 129);
        ring.mark_component(0, 64);
        ring.mark_component(0, 3);
        ring.mark_component(1, 3);
        ring.mark_component(0, 64);
        ring.mark_structural(7);
        ring.capture(|(), cold, pending, outbound, source| {
            assert!(cold);
            assert!(source.is_none());
            for dirty in [pending, outbound] {
                assert_eq!(dirty.component_count(), 2);
                assert_eq!(dirty.component_words(0), &[1 << 3, 1, 0]);
                assert_eq!(dirty.component_words(1), &[1 << 3, 0, 2]);
                assert_eq!(dirty.structural_words(), &[1 << 7, 0, 0]);
                assert_eq!(dirty.component_slots(0).collect::<Vec<_>>(), [3, 7, 64]);
                assert_eq!(dirty.component_slots(1).collect::<Vec<_>>(), [3, 7, 129]);
                assert_eq!(dirty.structural_slots().collect::<Vec<_>>(), [7]);
                assert_eq!(dirty.slots().collect::<Vec<_>>(), [3, 7, 64, 129]);
            }
        })
        .expect("capture independent unions");
        ring.mark_component(1, 65);
        ring.capture(|(), _, pending, outbound, _| {
            assert_eq!(pending.component_slots(0).collect::<Vec<_>>(), [3, 7, 64]);
            assert_eq!(
                pending.component_slots(1).collect::<Vec<_>>(),
                [3, 7, 65, 129]
            );
            assert!(outbound.component_slots(0).next().is_none());
            assert_eq!(outbound.component_slots(1).collect::<Vec<_>>(), [65]);
        })
        .expect("only outbound clears on the preceding capture");
    }

    #[test]
    fn batched_changes_merge_separate_components_and_preserve_immediate_marks() {
        let mut ring = SnapshotRing::new(130, 2, || ());
        ring.mark_component(0, 3);
        ring.mark_structural(4);
        ring.accumulate_component(0, 64);
        ring.accumulate_component(1, 129);
        ring.accumulate_component(1, 64);
        ring.accumulate_component(1, 64);
        ring.accumulate_structural(7);
        assert_eq!(ring.outbound.view().slots().collect::<Vec<_>>(), [3, 4]);
        ring.merge_changes();
        assert!(ring.scratch.view().slots().next().is_none());
        ring.capture(|(), _, pending, outbound, _| {
            for dirty in [pending, outbound] {
                assert_eq!(dirty.component_slots(0).collect::<Vec<_>>(), [3, 4, 7, 64]);
                assert_eq!(
                    dirty.component_slots(1).collect::<Vec<_>>(),
                    [4, 7, 64, 129]
                );
                assert_eq!(dirty.structural_slots().collect::<Vec<_>>(), [4, 7]);
            }
        })
        .expect("first merged batch");
        ring.accumulate_component(0, 65);
        ring.merge_changes();
        ring.merge_changes();
        ring.capture(|(), _, pending, outbound, _| {
            assert_eq!(
                pending.component_slots(0).collect::<Vec<_>>(),
                [3, 4, 7, 64, 65]
            );
            assert_eq!(
                pending.component_slots(1).collect::<Vec<_>>(),
                [4, 7, 64, 129]
            );
            assert_eq!(outbound.component_slots(0).collect::<Vec<_>>(), [65]);
            assert!(outbound.component_slots(1).next().is_none());
            assert!(outbound.structural_slots().next().is_none());
        })
        .expect("scratch clears between committed operations");
    }

    #[test]
    fn batched_changes_survive_busy_capture_and_rewind_without_mutating_leases() {
        let mut ring = SnapshotRing::new(130, 2, || 0_u64);
        for value in 1..=2 {
            ring.capture(|page, _, _, _, _| *page = value)
                .expect("first pages");
        }
        let first = ring.previous().expect("held first page");
        let second = ring.latest().expect("held second page");
        for value in 3..=4 {
            ring.capture(|page, _, _, _, _| *page = value)
                .expect("later pages");
        }
        ring.accumulate_component(0, 129);
        ring.accumulate_structural(64);
        ring.merge_changes();
        assert_eq!(
            ring.capture(|_, _, _, _, _| panic!("busy ring")),
            Err(PublicationBusy)
        );
        ring.accumulate_component(1, 7);
        ring.merge_changes();
        ring.reset_timeline();
        assert_eq!(*first, 1);
        assert_eq!(*second, 2);
        drop(first);
        ring.capture(|page, cold, pending, outbound, source| {
            assert!(!cold);
            assert_eq!(source, Some(&4));
            for dirty in [pending, outbound] {
                assert_eq!(dirty.component_slots(0).collect::<Vec<_>>(), [64, 129]);
                assert_eq!(dirty.component_slots(1).collect::<Vec<_>>(), [7, 64]);
            }
            *page = 2;
        })
        .expect("released page receives both merged operations");
        assert_eq!(*second, 2);
        assert!(ring.previous().is_none());
        assert!(ring.scratch.view().slots().next().is_none());
    }

    #[test]
    fn weak_lease_does_not_block_later_reusable_page_or_claim_false_readiness() {
        let mut ring = SnapshotRing::new(0, 0, || 0_u64);
        ring.capture(|page, _, _, _, _| *page = 1)
            .expect("first page");
        let weak = Arc::downgrade(&ring.latest().expect("first lease"));
        ring.capture(|page, _, _, _, _| *page = 2)
            .expect("second page");
        let second = ring.latest().expect("second lease");
        ring.capture(|page, _, _, _, _| *page = 3)
            .expect("third page");
        ring.capture(|page, _, _, _, _| *page = 4)
            .expect("fourth page");
        assert!(!ring.can_capture(), "weak first page must not be selected");
        assert_eq!(
            ring.capture(|_, _, _, _, _| panic!("must not fill")),
            Err(PublicationBusy)
        );
        drop(weak);
        assert!(ring.can_capture());
        ring.capture(|page, _, _, _, _| *page = 5)
            .expect("first page is now reusable");
        assert_eq!(*second, 2);
    }

    #[test]
    fn capture_skips_a_weak_leased_cursor_and_preserves_its_payload() {
        let mut ring = SnapshotRing::new(8, 1, || 0_u64);
        ring.capture(|page, _, _, _, _| *page = 1)
            .expect("first page");
        let weak = Arc::downgrade(&ring.latest().expect("first lease"));
        for value in 2..=4 {
            ring.capture(|page, _, _, _, _| *page = value)
                .expect("later pages");
        }
        assert_eq!(ring.cursor, 0);
        ring.mark_component(0, 6);
        assert!(ring.can_capture(), "a later payload is reusable");
        ring.capture(|page, cold, pending, outbound, source| {
            assert_eq!(*page, 2, "skip weak first page and select second");
            assert!(!cold);
            assert_eq!(pending.component_slots(0).collect::<Vec<_>>(), [6]);
            assert_eq!(outbound.component_slots(0).collect::<Vec<_>>(), [6]);
            assert_eq!(source, Some(&4));
            *page = 5;
        })
        .expect("later reusable page");
        assert_eq!(ring.cursor, 2);
        assert_eq!(*weak.upgrade().expect("weak lease remains valid"), 1);
    }

    #[test]
    fn rewind_retains_diff_source_without_exposing_a_stale_current_endpoint() {
        let mut ring = SnapshotRing::new(8, 2, || 0_u64);
        for value in 1..=4 {
            ring.mark_structural(u32::try_from(value).expect("small fixture"));
            ring.capture(|page, _, _, _, _| *page = value)
                .expect("published page");
        }
        let old = ring.latest().expect("retired source lease");
        ring.mark_structural(4);
        ring.mark_structural(3);
        ring.reset_timeline();
        ring.reset_timeline();
        assert!(!ring.has_latest());
        assert!(ring.latest().is_none());
        assert!(ring.previous().is_none());
        ring.capture(|page, cold, pending, outbound, source| {
            assert!(!cold);
            assert_eq!(pending.slots().collect::<Vec<_>>(), &[2, 3, 4]);
            assert_eq!(outbound.slots().collect::<Vec<_>>(), &[3, 4]);
            assert_eq!(
                source,
                Some(&4),
                "rewind diffs against its retired publication"
            );
            *page = 2;
        })
        .expect("rewound endpoint");
        assert_eq!(*old, 4);
        assert!(
            ring.previous().is_none(),
            "branches are not an adjacent pair"
        );
        ring.capture(|page, _, _, outbound, source| {
            assert!(outbound.slots().next().is_none());
            assert_eq!(source, Some(&2));
            *page = 3;
        })
        .expect("new branch continuation");
        assert_eq!(*ring.previous().expect("new previous"), 2);
    }

    #[test]
    fn rewind_patch_retires_source_and_consumes_only_selected_page_changes() {
        let mut ring = SnapshotRing::new(8, 2, || 0_u64);
        for value in 1..=4 {
            ring.capture(|page, _, _, _, _| *page = value)
                .expect("published page");
        }
        ring.mark_structural(7);
        ring.reset_timeline();
        ring.patch_rewind(
            |page| Some(*page),
            |page, tick| {
                assert_eq!(tick, 3, "newest unleased source");
                *page = 2;
            },
        )
        .expect("rewind patch");
        assert!(ring.retired_source.is_none());
        assert!(ring.previous().is_none());
        assert!(ring.outbound.view().slots().next().is_none());
        assert!(
            ring.changes[2]
                .dirty
                .structural
                .iter()
                .all(|word| *word == 0)
        );
        assert_eq!(ring.changes[0].dirty.structural, &[1_u64 << 7]);
        ring.capture(|page, cold, pending, outbound, source| {
            assert!(!cold);
            assert_eq!(pending.slots().collect::<Vec<_>>(), &[7]);
            assert!(outbound.slots().next().is_none());
            assert_eq!(source, Some(&2));
            *page = 3;
        })
        .expect("released retired page is next in cursor order");
        assert_eq!(ring.cursor, 0);
    }
}
