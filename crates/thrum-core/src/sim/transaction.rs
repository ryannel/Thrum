//! Typed candidate edits. The engine preflights every operation and bundle
//! before it changes committed storage, then applies the checked batch once.

use std::{any::Any, collections::HashSet};

use crate::{Handle, StableId};

use super::{
    ComponentKey, EntityWriter, RegisteredWorldError, StructuralPermit, SystemId, WritePermit,
    registered_world::RegisteredWorld,
};

type ValueAt<C> = fn(&C, usize) -> &dyn Any;
type ApplyValue = fn(&mut RegisteredWorld, Handle, &dyn Any) -> Result<(), RegisteredWorldError>;
type Presence = fn(&RegisteredWorld, Handle) -> Result<bool, RegisteredWorldError>;
type Fill<C> = fn(&C, usize, &mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>;

/// One reusable candidate operation. Values remain in game-owned candidate
/// columns; the edit contains only checked identity and function pointers.
pub struct Edit<C> {
    inner: EditKind<C>,
}

enum EditKind<C> {
    Component {
        handle: Handle,
        key: ComponentKey,
        nonce: u64,
        system: SystemId,
        kind: ComponentEditKind,
        index: usize,
        value: Option<ValueAt<C>>,
        apply: ApplyValue,
        present: Presence,
        valid_value: fn(&dyn Any) -> bool,
    },
    Despawn {
        handle: Handle,
        nonce: u64,
        system: SystemId,
    },
    Spawn {
        id: StableId,
        nonce: u64,
        system: SystemId,
        index: usize,
        fill: Fill<C>,
    },
}

/// Presence semantics for one typed component edit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComponentEditKind {
    /// Require an existing component and replace its value.
    Replace,
    /// Require an absent component and add it.
    Insert,
    /// Require an existing component and remove it.
    Remove,
}

/// A candidate batch refused without modifying committed storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditError {
    /// A checked identity, bundle or component preflight failed.
    World(RegisteredWorldError),
    /// An access token belongs to another compiled runtime.
    WrongRuntime,
    /// A callback emitted an edit using another system's declared capability.
    WrongSystem,
    /// Two edits target the same field or structural handle.
    Conflict,
    /// An edit's accessor returned a value of another registered type.
    WrongValueType,
    /// Spawn count exceeds slots returned by validated despawns.
    Capacity,
}

fn present<T: Send + Sync + 'static>(
    world: &RegisteredWorld,
    handle: Handle,
) -> Result<bool, RegisteredWorldError> {
    match world.with::<T, _>(handle, |_| ()) {
        Ok(()) => Ok(true),
        Err(RegisteredWorldError::MissingComponent) => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_type<T: 'static>(value: &dyn Any) -> bool {
    value.is::<T>()
}

fn replace<T: Clone + Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    handle: Handle,
    value: &dyn Any,
) -> Result<(), RegisteredWorldError> {
    world.replace(
        handle,
        value
            .downcast_ref::<T>()
            .expect("prevalidated value type")
            .clone(),
    )?;
    Ok(())
}

fn insert<T: Clone + Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    handle: Handle,
    value: &dyn Any,
) -> Result<(), RegisteredWorldError> {
    world.insert(
        handle,
        value
            .downcast_ref::<T>()
            .expect("prevalidated value type")
            .clone(),
    )
}

fn remove<T: Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    handle: Handle,
    _value: &dyn Any,
) -> Result<(), RegisteredWorldError> {
    world.remove::<T>(handle)?;
    Ok(())
}

impl<C> Edit<C> {
    pub(super) fn system(&self) -> SystemId {
        match self.inner {
            EditKind::Component { system, .. }
            | EditKind::Despawn { system, .. }
            | EditKind::Spawn { system, .. } => system,
        }
    }
    /// Stage a replacement of an existing registered component.
    pub fn replace<T: Clone + Send + Sync + 'static>(
        permit: &WritePermit<T>,
        handle: Handle,
        index: usize,
        value: ValueAt<C>,
    ) -> Self {
        Self {
            inner: EditKind::Component {
                handle,
                key: ComponentKey::of::<T>(),
                nonce: permit.nonce,
                system: permit.system,
                kind: ComponentEditKind::Replace,
                index,
                value: Some(value),
                apply: replace::<T>,
                present: present::<T>,
                valid_value: is_type::<T>,
            },
        }
    }

    /// Stage insertion of one absent registered component.
    pub fn insert<T: Clone + Send + Sync + 'static>(
        permit: &WritePermit<T>,
        handle: Handle,
        index: usize,
        value: ValueAt<C>,
    ) -> Self {
        Self {
            inner: EditKind::Component {
                handle,
                key: ComponentKey::of::<T>(),
                nonce: permit.nonce,
                system: permit.system,
                kind: ComponentEditKind::Insert,
                index,
                value: Some(value),
                apply: insert::<T>,
                present: present::<T>,
                valid_value: is_type::<T>,
            },
        }
    }

    /// Stage removal of one present registered component.
    pub fn remove<T: Send + Sync + 'static>(permit: &WritePermit<T>, handle: Handle) -> Self {
        Self {
            inner: EditKind::Component {
                handle,
                key: ComponentKey::of::<T>(),
                nonce: permit.nonce,
                system: permit.system,
                kind: ComponentEditKind::Remove,
                index: 0,
                value: None,
                apply: remove::<T>,
                present: present::<T>,
                valid_value: is_type::<T>,
            },
        }
    }

    /// Stage one checked despawn.
    pub fn despawn(permit: &StructuralPermit, handle: Handle) -> Self {
        Self {
            inner: EditKind::Despawn {
                handle,
                nonce: permit.nonce,
                system: permit.system,
            },
        }
    }

    /// Stage a new registered bundle, filled once at preflight.
    pub fn spawn(permit: &StructuralPermit, id: StableId, index: usize, fill: Fill<C>) -> Self {
        Self {
            inner: EditKind::Spawn {
                id,
                nonce: permit.nonce,
                system: permit.system,
                index,
                fill,
            },
        }
    }
}

pub(super) struct EditScratch {
    marks: Vec<u64>,
    retired: Vec<u64>,
    generation: u64,
    components: usize,
    builders: Vec<hecs::EntityBuilder>,
    builders_used: usize,
    spawn_ids: HashSet<StableId>,
}

impl EditScratch {
    pub(super) fn new(capacity: usize, components: usize) -> Self {
        Self {
            marks: vec![0; capacity.saturating_mul(components)],
            retired: vec![0; capacity],
            generation: 0,
            components,
            builders: Vec::new(),
            builders_used: 0,
            spawn_ids: HashSet::new(),
        }
    }

    pub(super) fn validate<C>(
        &mut self,
        world: &RegisteredWorld,
        nonce: u64,
        edits: &[Edit<C>],
        candidate: &C,
    ) -> Result<(), EditError> {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.marks.fill(0);
            self.retired.fill(0);
            self.generation = 1;
        }
        self.builders_used = 0;
        self.spawn_ids.clear();
        let mut available = world.free_count();
        let mut spawns = 0;
        for edit in edits {
            match &edit.inner {
                EditKind::Component {
                    handle,
                    key,
                    nonce: token,
                    kind,
                    index,
                    value,
                    present,
                    valid_value,
                    ..
                } => {
                    if *token != nonce {
                        return Err(EditError::WrongRuntime);
                    }
                    world.entity(*handle).map_err(EditError::World)?;
                    let slot = handle.slot as usize;
                    if self.retired[slot] == self.generation {
                        return Err(EditError::Conflict);
                    }
                    let component = world.component_index(*key).ok_or(EditError::World(
                        RegisteredWorldError::UnregisteredComponent,
                    ))?;
                    let mark = slot * self.components + component;
                    if self.marks[mark] == self.generation {
                        return Err(EditError::Conflict);
                    }
                    self.marks[mark] = self.generation;
                    let exists = present(world, *handle).map_err(EditError::World)?;
                    if exists != (*kind != ComponentEditKind::Insert) {
                        return Err(EditError::World(if exists {
                            RegisteredWorldError::DuplicateComponent
                        } else {
                            RegisteredWorldError::MissingComponent
                        }));
                    }
                    if let Some(value) = value
                        && !valid_value(value(candidate, *index))
                    {
                        return Err(EditError::WrongValueType);
                    }
                }
                EditKind::Despawn {
                    handle,
                    nonce: token,
                    ..
                } => {
                    if *token != nonce {
                        return Err(EditError::WrongRuntime);
                    }
                    world.entity(*handle).map_err(EditError::World)?;
                    let slot = handle.slot as usize;
                    if self.retired[slot] == self.generation {
                        return Err(EditError::Conflict);
                    }
                    if (0..self.components).any(|component| {
                        self.marks[slot * self.components + component] == self.generation
                    }) {
                        return Err(EditError::Conflict);
                    }
                    self.retired[slot] = self.generation;
                    available += usize::from(world.reusable_after_despawn(*handle));
                }
                EditKind::Spawn {
                    id,
                    nonce: token,
                    index,
                    fill,
                    ..
                } => {
                    if *token != nonce {
                        return Err(EditError::WrongRuntime);
                    }
                    if world.handle(*id).is_some() || !self.spawn_ids.insert(*id) {
                        return Err(EditError::World(RegisteredWorldError::DuplicateId));
                    }
                    if self.builders_used == self.builders.len() {
                        self.builders.push(hecs::EntityBuilder::new());
                    }
                    let builder = &mut self.builders[self.builders_used];
                    world
                        .prepare_bundle(builder, |writer| fill(candidate, *index, writer))
                        .map_err(EditError::World)?;
                    self.builders_used += 1;
                    spawns += 1;
                }
            }
        }
        if spawns > available {
            return Err(EditError::Capacity);
        }
        Ok(())
    }

    pub(super) fn apply<C>(
        &mut self,
        world: &mut RegisteredWorld,
        edits: &[Edit<C>],
        candidate: &C,
    ) {
        for edit in edits {
            if let EditKind::Component {
                handle,
                index,
                value,
                apply,
                ..
            } = &edit.inner
            {
                let empty: &dyn Any = &();
                let value = value.map_or(empty, |get| get(candidate, *index));
                apply(world, *handle, value).expect("whole batch prevalidated");
            }
        }
        for edit in edits {
            if let EditKind::Despawn { handle, .. } = &edit.inner {
                world.despawn(*handle).expect("whole batch prevalidated");
            }
        }
        let mut spawn = 0;
        for edit in edits {
            if let EditKind::Spawn { id, .. } = &edit.inner {
                world
                    .spawn_prepared(*id, &mut self.builders[spawn])
                    .expect("whole batch prevalidated");
                spawn += 1;
            }
        }
    }
}
