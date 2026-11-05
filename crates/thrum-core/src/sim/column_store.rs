//! Private typed slot columns and reusable spawn bundles. Registration resolves
//! type erasure once; queries and sparse accesses use checked slot indices.

use std::any::Any;

use super::ComponentRegistry;

pub(super) trait StorageColumn: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear_slot(&mut self, slot: usize);
    fn clear_all(&mut self);
}

pub(super) struct TypedStore<T> {
    values: Vec<Option<T>>,
    presence: Vec<u64>,
}

impl<T> TypedStore<T> {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            values: std::iter::repeat_with(|| None).take(capacity).collect(),
            presence: vec![0; capacity.div_ceil(64)],
        }
    }

    pub(super) fn get(&self, slot: usize) -> Option<&T> {
        self.values[slot].as_ref()
    }

    pub(super) fn get_mut(&mut self, slot: usize) -> Option<&mut T> {
        self.values[slot].as_mut()
    }

    pub(super) fn insert(&mut self, slot: usize, value: T) -> Option<T> {
        let prior = self.values[slot].replace(value);
        self.presence[slot / 64] |= 1_u64 << (slot % 64);
        prior
    }

    pub(super) fn remove(&mut self, slot: usize) -> Option<T> {
        let prior = self.values[slot].take();
        if prior.is_some() {
            self.presence[slot / 64] &= !(1_u64 << (slot % 64));
        }
        prior
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

    fn clear_slot(&mut self, slot: usize) {
        self.remove(slot);
    }

    fn clear_all(&mut self) {
        for value in &mut self.values {
            *value = None;
        }
        self.presence.fill(0);
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
