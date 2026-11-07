//! Private typed slot columns and reusable spawn bundles. Registration resolves
//! type erasure once; queries and sparse accesses use checked slot indices.

use std::any::Any;

use super::ComponentRegistry;
use super::undo::{TypedUndo, UndoColumn};

pub(super) trait StorageColumn: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn active_presence(&self) -> Option<&[u64]>;
    fn swap_rows(&mut self, first: usize, second: usize);
    fn clear_slot(&mut self, slot: usize);
    fn clear_all(&mut self);
    fn take_into_undo(&mut self, slot: u32, row: usize, undo: &mut dyn UndoColumn);
}

pub(super) struct TypedStore<T> {
    values: Vec<Option<T>>,
    presence: Vec<u64>,
    active: Option<ActiveIndex<T>>,
}

/// Disjoint borrows of one store for a chunked in-place kernel.
pub(super) struct StoreParts<'a, T> {
    pub(super) values: &'a mut [Option<T>],
    pub(super) presence: &'a [u64],
    pub(super) active: Option<ActiveParts<'a, T>>,
}

/// Mutable active-index words and the predicate that refreshes them.
pub(super) type ActiveParts<'a, T> = (&'a mut [u64], fn(&T) -> bool);

struct ActiveIndex<T> {
    predicate: fn(&T) -> bool,
    words: Vec<u64>,
}

impl<T> TypedStore<T> {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            values: std::iter::repeat_with(|| None).take(capacity).collect(),
            presence: vec![0; capacity.div_ceil(64)],
            active: None,
        }
    }

    pub(super) fn new_indexed(capacity: usize, predicate: fn(&T) -> bool) -> Self {
        Self {
            active: Some(ActiveIndex {
                predicate,
                words: vec![0; capacity.div_ceil(64)],
            }),
            ..Self::new(capacity)
        }
    }

    pub(super) fn get(&self, slot: usize) -> Option<&T> {
        self.values[slot].as_ref()
    }

    pub(super) fn get_mut(&mut self, slot: usize) -> Option<&mut T> {
        self.values[slot].as_mut()
    }

    /// Reevaluate an indexed value after direct mutable access, before any
    /// active-presence query can observe the store. Ordinary columns do no work.
    pub(super) fn refresh_active(&mut self, slot: usize) {
        if let Some(active) = &mut self.active {
            let bit = 1_u64 << (slot % 64);
            let word = &mut active.words[slot / 64];
            if self.values[slot].as_ref().is_some_and(active.predicate) {
                *word |= bit;
            } else {
                *word &= !bit;
            }
        }
    }

    pub(super) fn swap_rows(&mut self, first: usize, second: usize) {
        self.values.swap(first, second);
        for row in [first, second] {
            let bit = 1_u64 << (row % 64);
            let presence = &mut self.presence[row / 64];
            if self.values[row].is_some() {
                *presence |= bit;
            } else {
                *presence &= !bit;
            }
            self.refresh_active(row);
        }
    }

    /// Present slots satisfying the registered predicate, when indexing was
    /// selected. Direct writes must call `refresh_active` before reading this.
    pub(super) fn active_presence(&self) -> Option<&[u64]> {
        self.active.as_ref().map(|active| active.words.as_slice())
    }

    pub(super) fn insert(&mut self, slot: usize, value: T) -> Option<T> {
        let prior = self.values[slot].replace(value);
        self.presence[slot / 64] |= 1_u64 << (slot % 64);
        self.refresh_active(slot);
        prior
    }

    pub(super) fn remove(&mut self, slot: usize) -> Option<T> {
        let prior = self.values[slot].take();
        if prior.is_some() {
            self.presence[slot / 64] &= !(1_u64 << (slot % 64));
        }
        if let Some(active) = &mut self.active {
            active.words[slot / 64] &= !(1_u64 << (slot % 64));
        }
        prior
    }

    /// Split values, presence and any active index for word-aligned chunked
    /// kernels. Callers must refresh the active words of every visited row.
    pub(super) fn split_for_chunks(&mut self) -> StoreParts<'_, T> {
        StoreParts {
            values: &mut self.values,
            presence: &self.presence,
            active: self
                .active
                .as_mut()
                .map(|active| (active.words.as_mut_slice(), active.predicate)),
        }
    }

    pub(super) fn values(&self) -> &[Option<T>] {
        &self.values
    }

    pub(super) fn presence(&self) -> &[u64] {
        &self.presence
    }
}

impl<T: Send + Sync + 'static> StorageColumn for TypedStore<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn active_presence(&self) -> Option<&[u64]> {
        Self::active_presence(self)
    }

    fn swap_rows(&mut self, first: usize, second: usize) {
        Self::swap_rows(self, first, second);
    }

    fn clear_slot(&mut self, slot: usize) {
        self.remove(slot);
    }

    fn clear_all(&mut self) {
        for value in &mut self.values {
            *value = None;
        }
        self.presence.fill(0);
        if let Some(active) = &mut self.active {
            active.words.fill(0);
        }
    }

    fn take_into_undo(&mut self, slot: u32, row: usize, undo: &mut dyn UndoColumn) {
        let prior = self.remove(row);
        undo.as_any_mut()
            .downcast_mut::<TypedUndo<T>>()
            .expect("registered undo column type")
            .push_structural(slot, prior);
    }
}

pub(super) trait PendingValue: Send + Sync {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn is_set(&self) -> bool;
    fn clear(&mut self);
    fn commit(&mut self, column: &mut dyn StorageColumn, slot: usize);
}

pub(super) struct Pending<T> {
    pub(super) value: Option<T>,
}

impl<T: Send + Sync + 'static> PendingValue for Pending<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_set(&self) -> bool {
        self.value.is_some()
    }

    fn clear(&mut self) {
        self.value = None;
    }

    fn commit(&mut self, column: &mut dyn StorageColumn, slot: usize) {
        if let Some(value) = self.value.take() {
            let typed = column
                .as_any_mut()
                .downcast_mut::<TypedStore<T>>()
                .expect("registered bundle column type");
            assert!(typed.insert(slot, value).is_none(), "vacant spawn slot");
        }
    }
}

pub(super) struct EntityBundle {
    values: Vec<Box<dyn PendingValue>>,
}

impl EntityBundle {
    pub(super) fn new(registry: &ComponentRegistry) -> Self {
        Self {
            values: registry
                .entries
                .iter()
                .map(|entry| (entry.pending)())
                .collect(),
        }
    }

    pub(super) fn clear(&mut self) {
        for value in &mut self.values {
            value.clear();
        }
    }

    pub(super) fn add<T: Send + Sync + 'static>(&mut self, index: usize, value: T) -> bool {
        let cell = self.values[index]
            .as_any_mut()
            .downcast_mut::<Pending<T>>()
            .expect("registered bundle value type");
        if cell.is_set() {
            return false;
        }
        cell.value = Some(value);
        true
    }

    pub(super) fn commit(&mut self, columns: &mut [Box<dyn StorageColumn>], slot: usize) {
        for (value, column) in self.values.iter_mut().zip(columns) {
            value.commit(column.as_mut(), slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{StorageColumn, TypedStore, TypedUndo, UndoColumn};

    #[test]
    fn predicate_index_tracks_presence_replacement_and_direct_mutation_across_words() {
        let mut store = TypedStore::new_indexed(130, |value: &u32| *value != 0);
        assert_eq!(store.active_presence(), Some(&[0, 0, 0][..]));
        assert_eq!(store.insert(0, 1), None);
        assert_eq!(store.insert(64, 0), None);
        assert_eq!(store.insert(129, 9), None);
        assert_eq!(store.presence(), &[1, 1, 2]);
        assert_eq!(store.active_presence(), Some(&[1, 0, 2][..]));
        assert_eq!(store.insert(0, 0), Some(1));
        assert_eq!(store.insert(64, 3), Some(0));
        assert_eq!(store.active_presence(), Some(&[0, 1, 2][..]));
        *store.get_mut(0).expect("present inactive value") = 5;
        store.refresh_active(0);
        *store.get_mut(64).expect("present active value") = 0;
        store.refresh_active(64);
        let erased: &dyn StorageColumn = &store;
        assert_eq!(erased.active_presence(), Some(&[1, 0, 2][..]));
        assert_eq!(store.remove(129), Some(9));
        assert_eq!(store.remove(129), None);
        assert_eq!(store.presence(), &[1, 1, 0]);
        assert_eq!(store.active_presence(), Some(&[1, 0, 0][..]));
        store.clear_all();
        store.refresh_active(0);
        assert_eq!(store.presence(), &[0, 0, 0]);
        assert_eq!(store.active_presence(), Some(&[0, 0, 0][..]));
    }

    #[test]
    fn row_swap_carries_values_presence_and_activity_together() {
        let mut store = TypedStore::new_indexed(65, |value: &u32| *value != 0);
        store.insert(0, 7);
        store.insert(64, 0);
        store.swap_rows(0, 64);
        assert_eq!(store.get(0), Some(&0));
        assert_eq!(store.get(64), Some(&7));
        assert_eq!(store.presence(), &[1, 1]);
        assert_eq!(store.active_presence(), Some(&[0, 1][..]));
        store.swap_rows(0, 1);
        assert_eq!(store.get(0), None);
        assert_eq!(store.get(1), Some(&0));
        assert_eq!(store.presence(), &[2, 1]);
        store.swap_rows(0, 0);
        assert_eq!(store.active_presence(), Some(&[0, 1][..]));
    }

    #[test]
    fn undo_removal_and_restore_maintain_the_predicate_index() {
        let mut store = TypedStore::new_indexed(65, |value: &u32| *value >= 10);
        store.insert(64, 12);
        let mut undo = TypedUndo::<u32>::new(|value| *value);
        store.take_into_undo(64, 64, &mut undo);
        assert_eq!(store.get(64), None);
        assert_eq!(store.active_presence(), Some(&[0, 0][..]));
        undo.restore_into(&mut store, &mut [0; 65], 0, 1, None);
        assert_eq!(store.get(64), Some(&12));
        assert_eq!(store.active_presence(), Some(&[0, 1][..]));
        undo.clear();
        undo.push_component(64, Some(2), 0);
        undo.restore_into(&mut store, &mut [0; 65], 0, 1, None);
        assert_eq!(store.get(64), Some(&2));
        assert_eq!(store.active_presence(), Some(&[0, 0][..]));
    }

    #[test]
    fn ordinary_store_has_no_index_and_an_index_never_selects_absent_values() {
        let mut ordinary = TypedStore::new(1);
        ordinary.insert(0, 3_u32);
        *ordinary.get_mut(0).expect("ordinary value") = 4;
        ordinary.refresh_active(0);
        assert_eq!(ordinary.get(0), Some(&4));
        assert_eq!(ordinary.active_presence(), None);
        ordinary.remove(0);
        ordinary.clear_all();
        assert_eq!(ordinary.active_presence(), None);
        let mut indexed = TypedStore::new_indexed(1, |_: &u32| true);
        indexed.refresh_active(0);
        assert_eq!(indexed.active_presence(), Some(&[0][..]));
        indexed.insert(0, 0);
        assert_eq!(indexed.active_presence(), Some(&[1][..]));
        indexed.clear_slot(0);
        assert_eq!(indexed.active_presence(), Some(&[0][..]));
    }
}
