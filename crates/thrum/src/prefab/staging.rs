use std::{any::Any, collections::HashMap, sync::Mutex};

use thrum_core::sim::{EntityWriter, RegisteredWorldError};

use crate::{Component, Data, Entity, Error, Result, data::Stored, rule::access::Column};

use super::{Field, FieldTweak, Prepared, SpawnPatches};

pub(crate) trait StagedColumn: Send + Sync {
    fn append(&mut self, entity: Entity, value: &dyn Any);
    fn get(&self, entity: Entity) -> Option<&dyn Any>;
    fn fill(
        &self,
        entity: Entity,
        writer: &mut EntityWriter<'_>,
    ) -> std::result::Result<(), RegisteredWorldError>;
    fn clear(&mut self);
}

pub(crate) struct Values<T>(Vec<(Entity, T)>);

impl<T: Data> Values<T> {
    pub(crate) fn allocate(capacity: usize) -> Box<dyn StagedColumn> {
        Box::new(Self(Vec::with_capacity(capacity)))
    }

    fn index(&self, entity: Entity) -> Option<usize> {
        self.0.binary_search_by_key(&entity, |row| row.0).ok()
    }
}

impl<T: Data> StagedColumn for Values<T> {
    fn append(&mut self, entity: Entity, value: &dyn Any) {
        let value = value
            .downcast_ref::<T>()
            .expect("staging column type")
            .clone();
        if let Some(last) = self.0.last_mut()
            && last.0 == entity
        {
            last.1 = value;
        } else {
            self.0.push((entity, value));
        }
    }

    fn get(&self, entity: Entity) -> Option<&dyn Any> {
        self.index(entity)
            .map(|index| -> &dyn Any { &self.0[index].1 })
    }

    fn fill(
        &self,
        entity: Entity,
        writer: &mut EntityWriter<'_>,
    ) -> std::result::Result<(), RegisteredWorldError> {
        if let Some(index) = self.index(entity) {
            writer.add(Stored(self.0[index].1.clone()))?;
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Default)]
pub(crate) struct Staging {
    by_type: HashMap<std::any::TypeId, usize>,
    columns: Vec<Box<dyn StagedColumn>>,
    touched: Vec<usize>,
    used: Vec<bool>,
}

impl Staging {
    pub(crate) fn new(columns: &[Column], capacity: usize) -> Self {
        Self {
            by_type: columns
                .iter()
                .enumerate()
                .map(|(index, column)| (column.type_id, index))
                .collect(),
            columns: columns
                .iter()
                .map(|column| (column.stage)(capacity))
                .collect(),
            touched: Vec::with_capacity(columns.len()),
            used: vec![false; columns.len()],
        }
    }

    pub(crate) fn clear(&mut self) {
        for index in self.touched.drain(..) {
            self.columns[index].clear();
            self.used[index] = false;
        }
    }

    pub(crate) fn get(&self, entity: Entity, column: Column) -> Option<&dyn Any> {
        self.by_type
            .get(&column.type_id)
            .and_then(|&index| self.columns[index].get(entity))
    }

    pub(crate) fn fill(
        &self,
        entity: Entity,
        writer: &mut EntityWriter<'_>,
        omitted: impl Fn(usize) -> bool,
    ) -> std::result::Result<(), RegisteredWorldError> {
        for &index in &self.touched {
            if !omitted(index) {
                self.columns[index].fill(entity, writer)?;
            }
        }
        Ok(())
    }

    pub(crate) fn index(&self, column: Column) -> Option<usize> {
        self.by_type.get(&column.type_id).copied()
    }

    fn writable(&mut self, column: Column) -> Result<&mut dyn StagedColumn> {
        let index = self.index(column).ok_or_else(|| {
            Error::refused(
                "T0301",
                column.name,
                "component is not registered",
                "Declare it in a rule signature or register it with Game::component::<T>().",
            )
        })?;
        if !self.used[index] {
            self.touched.push(index);
            self.used[index] = true;
        }
        Ok(self.columns[index].as_mut())
    }
}

pub(crate) struct SpawnPatch<'a> {
    pub(crate) staging: &'a Mutex<Staging>,
    pub(crate) entity: Entity,
    pub(crate) template: Option<&'a Prepared>,
    pub(crate) changed: bool,
}

impl SpawnPatch<'_> {
    pub(crate) fn baseline<T: Component>(&self) -> Result<T> {
        let column = Column::of::<T>();
        let staging = self.staging.lock().expect("prefab spawn staging");
        staging
            .get(self.entity, column)
            .and_then(|value| value.downcast_ref::<T>())
            .copied()
            .or_else(|| self.template.and_then(Prepared::get))
            .ok_or_else(|| {
                super::missing_tweak(self.template.map_or("spawn", Prepared::name), column)
            })
    }
}

impl SpawnPatches for SpawnPatch<'_> {
    fn set(&mut self, field: &dyn Field) -> Result {
        self.staging
            .lock()
            .expect("prefab spawn staging")
            .writable(field.column())?
            .append(self.entity, field.read());
        self.changed = true;
        Ok(())
    }

    fn tweak(&mut self, tweak: &dyn FieldTweak) -> Result {
        tweak.stage(self)
    }
}
