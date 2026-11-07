//! Typed V1 presentation view over the engine-owned immutable publication page.

use std::sync::Arc;

use thrum_core::{
    Handle, StableId,
    sim::{PublicationChanges, PublicationPage, SnapshotId, StructuralChange},
};

use super::types::{Health, Ownership, Position, Velocity, Visual};

pub(crate) type Page = PublicationPage;

/// The V1 game's named component views over engine-owned exact differences.
#[derive(Clone, Copy)]
pub struct DirtySlots<'a>(&'a PublicationChanges);

impl<'a> DirtySlots<'a> {
    /// Position endpoints changed since the preceding publication.
    pub fn position(self) -> &'a [u32] {
        self.0.component::<Position>()
    }
    /// Position interpolation pairs (previous or current) changed since the
    /// preceding publication, including a body that stopped.
    pub fn position_pair(self) -> &'a [u32] {
        self.0.temporal::<Position>()
    }
    /// Velocity endpoints changed since the preceding publication.
    pub fn velocity(self) -> &'a [u32] {
        self.0.component::<Velocity>()
    }
    /// Health endpoints changed since the preceding publication.
    pub fn health(self) -> &'a [u32] {
        self.0.component::<Health>()
    }
    /// Visual selector or material changed since the preceding publication.
    pub fn visual(self) -> &'a [u32] {
        self.0.component::<Visual>()
    }
    /// Membership or identity changed since the preceding publication.
    pub fn structural(self) -> &'a [u32] {
        self.0.structural()
    }
}

/// Shared immutable lease of one complete V1 game endpoint.
#[derive(Clone)]
pub struct PublishedPage(pub(crate) Arc<Page>);

impl PublishedPage {
    /// Old branch endpoint this inverse patch can update.
    pub fn patch_source(&self) -> Option<(SnapshotId, u64)> {
        self.0.patch_source()
    }
    /// Complete published endpoint used as the source for this page's delta.
    pub fn source_endpoint(&self) -> Option<(SnapshotId, u64)> {
        self.0.source_endpoint()
    }
    /// Whether this page starts a complete membership stream.
    pub fn is_baseline(&self) -> bool {
        self.0.is_baseline()
    }
    /// Committed world generation and fixed tick.
    pub fn id(&self) -> SnapshotId {
        self.0.id()
    }
    /// Replay branch generation for temporal presentation history.
    pub fn timeline(&self) -> u64 {
        self.0.timeline()
    }
    /// Live entity count.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Whether this endpoint has no live entities.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Physical slot capacity of this authority.
    pub fn slot_capacity(&self) -> usize {
        self.0.slot_capacity()
    }
    /// Stable identity at one live physical slot.
    pub fn stable_id(&self, slot: u32) -> Option<StableId> {
        self.0.stable_id(slot)
    }
    /// Checked handle at one live physical slot.
    pub fn handle(&self, slot: u32) -> Option<Handle> {
        self.0.handle(slot)
    }
    /// Position at one matching live handle.
    pub fn position(&self, handle: Handle) -> Option<Position> {
        self.0.component::<Position>(handle).copied()
    }
    /// Previous adjacent position for a matching live handle.
    pub fn previous_position(&self, handle: Handle) -> Option<Position> {
        self.0.previous_component::<Position>(handle).copied()
    }
    /// Velocity at one matching live handle; zero when absent.
    pub fn velocity(&self, handle: Handle) -> Option<Velocity> {
        self.handle(handle.slot)
            .filter(|current| *current == handle)
            .map(|_| {
                self.0
                    .component::<Velocity>(handle)
                    .copied()
                    .unwrap_or_default()
            })
    }
    /// Health at one matching live handle.
    pub fn health(&self, handle: Handle) -> Option<u16> {
        self.0.component::<Health>(handle).map(|value| value.0)
    }
    /// Ownership at one matching live handle.
    pub fn ownership(&self, handle: Handle) -> Option<Ownership> {
        self.0.component::<Ownership>(handle).copied()
    }
    /// Visual selector at one matching live handle.
    pub fn visual(&self, handle: Handle) -> Option<u64> {
        self.0.component::<Visual>(handle).map(|value| value.0)
    }
    /// Exact component and structural endpoint changes.
    pub fn dirty(&self) -> DirtySlots<'_> {
        DirtySlots(self.0.dirty())
    }
    /// Ordered structural changes.
    pub fn structural_events(&self) -> &[StructuralChange] {
        self.0.structural_events()
    }
    /// Diagnostic, not page content: columns whose warm capture ran on the
    /// Rayon pool for this page.
    pub fn parallel_capture_columns(&self) -> usize {
        self.0.parallel_capture_columns()
    }
    /// Visit live visual rows in physical slot order.
    ///
    /// # Panics
    ///
    /// Only if an engine-owned live page lacks the V1 required Position.
    pub fn visit(&self, mut visit: impl FnMut(Handle, StableId, Position, Option<u64>)) {
        self.0
            .visit2::<Position, Visual>(|handle, id, position, visual| {
                let position = *position.expect("V1 live entity has position");
                visit(handle, id, position, visual.map(|value| value.0));
            });
    }
}
