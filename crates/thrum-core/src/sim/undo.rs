//! Sparse, owned component preimages for a committed tick. Values move into
//! history after whole-batch validation and move back on reverse application.

use std::{
    any::Any,
    mem::{needs_drop, size_of},
};

use super::column_store::{StorageColumn, TypedStore};

pub(super) trait UndoColumn: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn restore_into(
        &mut self,
        storage: &mut dyn StorageColumn,
        change_ticks: &mut [u64],
        component: usize,
        stride: usize,
    );
    fn restore_first_seen_into(
        &mut self,
        storage: &mut dyn StorageColumn,
        change_ticks: &mut [u64],
        component: usize,
        stride: usize,
        seen: &mut [bool],
        visited: &mut Vec<u32>,
    );
    fn visit_changed_slots(&self, visit: &mut dyn FnMut(u32));
    fn has_rows(&self) -> bool;
    fn can_coalesce_without_drop_order(&self) -> bool;
    fn clear(&mut self);
    fn copy_from(&mut self, source: &dyn UndoColumn);
    fn reserve(&mut self, additional: usize);
    fn reserved_bytes(&self) -> usize;
}

pub(super) struct TypedUndo<T> {
    rows: Vec<(u32, Option<T>)>,
    prior_ticks: Vec<u64>,
    snapshot: fn(&T) -> T,
}

impl<T> TypedUndo<T> {
    pub(super) fn new(snapshot: fn(&T) -> T) -> Self {
        Self {
            rows: Vec::new(),
            prior_ticks: Vec::new(),
            snapshot,
        }
    }

    pub(super) fn push_component(&mut self, slot: u32, prior: Option<T>, tick: u64) {
        self.rows.push((slot, prior));
        self.prior_ticks.push(tick);
    }

    pub(super) fn pop_tentative(&mut self, slot: u32) {
        let (recorded, _) = self.rows.pop().expect("tentative preimage exists");
        assert_eq!(recorded, slot, "tentative preimage is the last row");
        self.prior_ticks
            .pop()
            .expect("tentative change tick exists");
    }

    pub(super) fn visit_first_preimages(
        &self,
        seen_words: &mut [u64],
        mut visit: impl FnMut(u32, Option<&T>),
    ) {
        debug_assert!(seen_words.iter().all(|&word| word == 0));
        for (slot, prior) in &self.rows {
            let word = &mut seen_words[*slot as usize / 64];
            let bit = 1_u64 << (*slot % 64);
            if *word & bit == 0 {
                *word |= bit;
                visit(*slot, prior.as_ref());
            }
        }
        for (slot, _) in &self.rows {
            seen_words[*slot as usize / 64] &= !(1_u64 << (*slot % 64));
        }
    }

    pub(super) fn push_structural(&mut self, slot: u32, prior: Option<T>) {
        self.rows.push((slot, prior));
        self.prior_ticks.push(u64::MAX);
    }
}

impl<T: Send + Sync + 'static> UndoColumn for TypedUndo<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn restore_into(
        &mut self,
        storage: &mut dyn StorageColumn,
        change_ticks: &mut [u64],
        component: usize,
        stride: usize,
    ) {
        let storage = storage
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered undo column type");
        for ((slot, prior), &tick) in self
            .rows
            .iter_mut()
            .rev()
            .zip(self.prior_ticks.iter().rev())
        {
            match prior.take() {
                Some(value) => {
                    storage.insert(*slot as usize, value);
                }
                None => {
                    storage.remove(*slot as usize);
                }
            }
            if tick != u64::MAX {
                change_ticks[*slot as usize * stride + component] = tick;
            }
        }
    }

    fn restore_first_seen_into(
        &mut self,
        storage: &mut dyn StorageColumn,
        change_ticks: &mut [u64],
        component: usize,
        stride: usize,
        seen: &mut [bool],
        visited: &mut Vec<u32>,
    ) {
        let storage = storage
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered undo column type");
        for ((slot, prior), &tick) in self.rows.iter_mut().zip(&self.prior_ticks) {
            debug_assert_ne!(tick, u64::MAX, "structural rows use sequential undo");
            let index = *slot as usize;
            if seen[index] {
                drop(prior.take());
                continue;
            }
            seen[index] = true;
            visited.push(*slot);
            match prior.take() {
                Some(value) => {
                    storage.insert(index, value);
                }
                None => {
                    storage.remove(index);
                }
            }
            change_ticks[index * stride + component] = tick;
        }
    }

    fn visit_changed_slots(&self, visit: &mut dyn FnMut(u32)) {
        for (&(slot, _), &tick) in self.rows.iter().zip(&self.prior_ticks) {
            if tick != u64::MAX {
                visit(slot);
            }
        }
    }

    fn has_rows(&self) -> bool {
        !self.rows.is_empty()
    }

    fn can_coalesce_without_drop_order(&self) -> bool {
        !needs_drop::<T>()
    }

    fn clear(&mut self) {
        self.rows.clear();
        self.prior_ticks.clear();
    }

    fn copy_from(&mut self, source: &dyn UndoColumn) {
        let source = source
            .as_any()
            .downcast_ref::<Self>()
            .expect("registered undo column type");
        self.rows.clear();
        self.rows.extend(
            source
                .rows
                .iter()
                .map(|(slot, prior)| (*slot, prior.as_ref().map(self.snapshot))),
        );
        self.prior_ticks.clone_from(&source.prior_ticks);
    }

    fn reserve(&mut self, additional: usize) {
        self.rows.reserve(additional);
        self.prior_ticks.reserve(additional);
    }

    fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.rows.capacity() * size_of::<(u32, Option<T>)>()
            + self.prior_ticks.capacity() * size_of::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::TypedUndo;

    #[test]
    fn first_preimage_wins_after_multiple_writes_and_seen_storage_is_reusable() {
        let mut undo = TypedUndo::new(|value| *value);
        undo.push_component(3, Some(10_u32), 1);
        undo.push_component(3, Some(20), 2);
        undo.push_component(5, None, 0);
        undo.push_component(3, Some(30), 3);
        let mut seen = [0_u64; 1];
        for _ in 0..2 {
            let mut preimages = Vec::new();
            undo.visit_first_preimages(&mut seen, |slot, prior| {
                preimages.push((slot, prior.copied()));
            });
            assert_eq!(preimages, [(3, Some(10)), (5, None)]);
            assert_eq!(seen, [0]);
        }
    }
}
