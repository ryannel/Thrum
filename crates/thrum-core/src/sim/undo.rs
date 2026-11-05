//! Sparse, owned component preimages for a committed tick. Values move into
//! history after whole-batch validation and move back on reverse application.

use std::{
    any::Any,
    mem::{needs_drop, size_of},
};

use super::column_store::{StorageColumn, TypedStore};

pub(super) trait UndoColumn: Send + Sync {
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
    fn reserve(&mut self, additional: usize);
    fn reserved_bytes(&self) -> usize;
}

pub(super) struct TypedUndo<T> {
    rows: Vec<(u32, Option<T>)>,
    prior_ticks: Vec<u64>,
}

impl<T> TypedUndo<T> {
    pub(super) fn new() -> Self {
        Self {
            rows: Vec::new(),
            prior_ticks: Vec::new(),
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

    pub(super) fn push_structural(&mut self, slot: u32, prior: Option<T>) {
        self.rows.push((slot, prior));
        self.prior_ticks.push(u64::MAX);
    }
}

impl<T: Send + Sync + 'static> UndoColumn for TypedUndo<T> {
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
