//! Sparse, owned component preimages for a committed tick. Values move into
//! history after whole-batch validation and move back on reverse application.

use std::{any::Any, mem::size_of};

use super::column_store::{StorageColumn, TypedStore};

pub(super) trait UndoColumn: Send + Sync {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn restore_into(&mut self, storage: &mut dyn StorageColumn);
    fn clear(&mut self);
    fn reserve(&mut self, additional: usize);
    fn reserved_bytes(&self) -> usize;
}

pub(super) struct TypedUndo<T> {
    rows: Vec<(u32, Option<T>)>,
}

impl<T> TypedUndo<T> {
    pub(super) fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub(super) fn push(&mut self, slot: u32, prior: Option<T>) {
        self.rows.push((slot, prior));
    }
}

impl<T: Send + Sync + 'static> UndoColumn for TypedUndo<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn restore_into(&mut self, storage: &mut dyn StorageColumn) {
        let storage = storage
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered undo column type");
        for (slot, prior) in self.rows.drain(..).rev() {
            match prior {
                Some(value) => {
                    storage.insert(slot as usize, value);
                }
                None => {
                    storage.remove(slot as usize);
                }
            }
        }
    }

    fn clear(&mut self) {
        self.rows.clear();
    }

    fn reserve(&mut self, additional: usize) {
        self.rows.reserve(additional);
    }

    fn reserved_bytes(&self) -> usize {
        size_of::<Self>() + self.rows.capacity() * size_of::<(u32, Option<T>)>()
    }
}
