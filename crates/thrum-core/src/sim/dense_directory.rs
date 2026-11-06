//! Logical identity stays fixed while active component rows occupy a dense prefix.

use std::{
    hash::{Hash, Hasher},
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_LAYOUT_ID: AtomicU64 = AtomicU64::new(1);

fn next_layout_id() -> u64 {
    NEXT_LAYOUT_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("layout identities exhausted")
}

/// Full-capacity inverse maps, including inactive and currently vacant slots.
/// Component columns must apply every returned row swap in the same order.
pub(super) struct DenseDirectory {
    slot_to_row: Vec<usize>,
    row_to_slot: Vec<usize>,
    active_len: usize,
    layout_id: u64,
}

impl Hash for DenseDirectory {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.slot_to_row.hash(state);
        self.row_to_slot.hash(state);
        self.active_len.hash(state);
    }
}

/// One membership change, including changes that do not move any row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RowSwap {
    pub(super) row: usize,
    pub(super) boundary_row: usize,
    previous_active_len: usize,
    resulting_active_len: usize,
}

impl Clone for DenseDirectory {
    fn clone(&self) -> Self {
        Self {
            slot_to_row: self.slot_to_row.clone(),
            row_to_slot: self.row_to_slot.clone(),
            active_len: self.active_len,
            layout_id: self.layout_id,
        }
    }

    /// Reuses the allocated maps when the destination has sufficient capacity.
    fn clone_from(&mut self, source: &Self) {
        self.slot_to_row.clone_from(&source.slot_to_row);
        self.row_to_slot.clone_from(&source.row_to_slot);
        self.active_len = source.active_len;
        self.layout_id = source.layout_id;
    }
}

impl DenseDirectory {
    /// Reserves every logical slot, initially inactive and in logical order.
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            slot_to_row: (0..capacity).collect(),
            row_to_slot: (0..capacity).collect(),
            active_len: 0,
            layout_id: next_layout_id(),
        }
    }

    pub(super) fn row_of(&self, slot: usize) -> usize {
        self.slot_to_row[slot]
    }

    pub(super) fn slot_at(&self, row: usize) -> usize {
        self.row_to_slot[row]
    }

    pub(super) fn active_len(&self) -> usize {
        self.active_len
    }

    /// Equal IDs prove identical row maps; independently evolved but equal
    /// maps may have different IDs and take the general path.
    pub(super) fn shares_layout(&self, other: &Self) -> bool {
        self.layout_id == other.layout_id
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        self.slot_to_row.capacity() * size_of::<usize>()
            + self.row_to_slot.capacity() * size_of::<usize>()
    }

    /// Changes one checked slot's membership without allocating. A no-op returns
    /// `None`; a prefix-length change returns a record even for an equal-row swap.
    pub(super) fn transition(&mut self, slot: usize, active: bool) -> Option<RowSwap> {
        let row = self.row_of(slot);
        if (row < self.active_len) == active {
            return None;
        }
        let previous_active_len = self.active_len;
        let boundary_row = if active {
            self.active_len
        } else {
            self.active_len - 1
        };
        self.swap_rows(row, boundary_row);
        self.layout_id = next_layout_id();
        if active {
            self.active_len += 1;
        } else {
            self.active_len -= 1;
        }
        Some(RowSwap {
            row,
            boundary_row,
            previous_active_len,
            resulting_active_len: self.active_len,
        })
    }

    /// Reapplies a recorded transition from its prior layout. Records must be
    /// replayed in their original order against an identical starting layout;
    /// component columns must apply the same swaps.
    pub(super) fn replay(&mut self, swap: RowSwap) {
        assert_eq!(
            self.active_len, swap.previous_active_len,
            "layout replay starts at the recorded active prefix"
        );
        self.swap_rows(swap.row, swap.boundary_row);
        self.layout_id = next_layout_id();
        self.active_len = swap.resulting_active_len;
    }

    /// Restores the exact prior row order and membership. Records must be
    /// replayed in reverse order against the directory that produced them;
    /// component columns must reverse the same swaps before reuse.
    pub(super) fn undo(&mut self, swap: RowSwap) {
        self.swap_rows(swap.row, swap.boundary_row);
        self.layout_id = next_layout_id();
        self.active_len = swap.previous_active_len;
    }

    fn swap_rows(&mut self, row: usize, other: usize) {
        let slot = self.row_to_slot[row];
        let other_slot = self.row_to_slot[other];
        self.row_to_slot.swap(row, other);
        self.slot_to_row[slot] = other;
        self.slot_to_row[other_slot] = row;
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{Hash, Hasher};

    use super::DenseDirectory;

    fn assert_layout(directory: &DenseDirectory, active_len: usize, slots: &[usize]) {
        assert_eq!(directory.active_len(), active_len);
        for (row, &slot) in slots.iter().enumerate() {
            assert_eq!(directory.slot_at(row), slot);
            assert_eq!(directory.row_of(slot), row);
        }
    }

    #[test]
    fn interleaved_slots_pack_and_deactivate_at_the_boundary() {
        let mut directory = DenseDirectory::new(8);
        for slot in [5, 1, 7, 3] {
            directory.transition(slot, true).expect("inactive slot");
        }
        assert_layout(&directory, 4, &[5, 1, 7, 3, 4, 0, 6, 2]);
        let swap = directory.transition(1, false).expect("active slot");
        assert_eq!((swap.row, swap.boundary_row), (1, 3));
        assert_layout(&directory, 3, &[5, 3, 7, 1, 4, 0, 6, 2]);
        directory.transition(0, true).expect("inactive slot");
        assert_layout(&directory, 4, &[5, 3, 7, 0, 4, 1, 6, 2]);
    }

    #[test]
    fn same_state_is_a_noop_and_equal_row_changes_are_reversible() {
        let mut directory = DenseDirectory::new(2);
        assert!(directory.transition(0, false).is_none());
        let activate = directory.transition(0, true).expect("inactive slot");
        assert_eq!((activate.row, activate.boundary_row), (0, 0));
        assert!(directory.transition(0, true).is_none());
        let deactivate = directory.transition(0, false).expect("active slot");
        assert_eq!((deactivate.row, deactivate.boundary_row), (0, 0));
        assert_layout(&directory, 0, &[0, 1]);
        directory.undo(deactivate);
        assert_layout(&directory, 1, &[0, 1]);
        directory.undo(activate);
        assert_layout(&directory, 0, &[0, 1]);
    }

    #[test]
    fn inverse_replay_restores_every_intermediate_layout() {
        let mut directory = DenseDirectory::new(6);
        let mut history = Vec::new();
        for (slot, active) in [
            (4, true),
            (1, true),
            (5, true),
            (1, false),
            (0, true),
            (4, false),
        ] {
            let prior = directory.clone();
            let swap = directory
                .transition(slot, active)
                .expect("membership change");
            history.push((prior, swap));
        }
        for (prior, swap) in history.into_iter().rev() {
            directory.undo(swap);
            assert_layout(&directory, prior.active_len(), &prior.row_to_slot);
        }
        assert_layout(&directory, 0, &[0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn full_capacity_and_reuse_preserve_the_bijection() {
        let mut directory = DenseDirectory::new(5);
        for slot in [3, 1, 4, 0, 2] {
            directory.transition(slot, true).expect("inactive slot");
        }
        assert_layout(&directory, 5, &[3, 1, 4, 0, 2]);
        for slot in [4, 2, 3, 0, 1] {
            directory.transition(slot, false).expect("active slot");
            assert!(directory.row_of(slot) >= directory.active_len());
        }
        assert_layout(&directory, 0, &[1, 0, 3, 2, 4]);
        directory.transition(4, true).expect("reused slot");
        assert_layout(&directory, 1, &[4, 0, 3, 2, 1]);
    }

    #[test]
    fn forward_replay_matches_transitions_and_undo_restores_the_start() {
        let mut source = DenseDirectory::new(5);
        let mut replayed = source.clone();
        let mut records = Vec::new();
        for (slot, active) in [(0, true), (4, true), (0, false), (4, false)] {
            let record = source.transition(slot, active).expect("membership change");
            replayed.replay(record);
            assert_layout(&replayed, source.active_len(), &source.row_to_slot);
            records.push(record);
        }
        for record in records.into_iter().rev() {
            replayed.undo(record);
        }
        assert_layout(&replayed, 0, &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn clone_from_reuses_same_capacity_maps_and_copies_membership() {
        let mut source = DenseDirectory::new(4);
        source.transition(3, true).expect("inactive slot");
        let mut target = DenseDirectory::new(4);
        let pointers = (target.slot_to_row.as_ptr(), target.row_to_slot.as_ptr());
        target.clone_from(&source);
        assert_eq!(
            pointers,
            (target.slot_to_row.as_ptr(), target.row_to_slot.as_ptr())
        );
        assert_layout(&target, 1, &[3, 1, 2, 0]);
        assert_layout(&DenseDirectory::new(0), 0, &[]);
    }

    #[test]
    fn shared_layout_identity_is_conservative_and_does_not_change_world_hash() {
        let mut authority = DenseDirectory::new(5);
        let mut page = authority.clone();
        assert!(authority.shares_layout(&page));
        authority.transition(3, true).expect("authority transition");
        page.transition(3, true)
            .expect("independent page transition");
        assert!(!authority.shares_layout(&page));
        let hash = |directory: &DenseDirectory| {
            let mut state = std::collections::hash_map::DefaultHasher::new();
            directory.hash(&mut state);
            state.finish()
        };
        assert_eq!(hash(&authority), hash(&page));
        page.clone_from(&authority);
        assert!(authority.shares_layout(&page));
    }
}
