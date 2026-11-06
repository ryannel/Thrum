//! Typed candidate edits. The engine preflights every operation and bundle
//! before it changes committed storage, then applies the checked batch once.

use std::{any::Any, collections::HashSet};

use crate::{Handle, StableId};

use super::{
    EntityWriter, RegisteredWorldError, StructuralChange, StructuralPermit, SystemId, WritePermit,
    column_store::EntityBundle,
    registered_world::{RegisteredWorld, WorldUndo},
};

type ValueAt<C> = fn(&C, usize) -> &dyn Any;
type ApplyValue = fn(&mut RegisteredWorld, usize, Handle, &dyn Any, Option<&mut WorldUndo>);
type Presence = fn(&RegisteredWorld, usize, u32) -> bool;
type Fill<C> = fn(&C, usize, &mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>;
type BatchValidate =
    fn(&RegisteredWorld, usize, &dyn Any, &mut EditScratch) -> Result<(), EditError>;
type BatchApply = fn(&mut RegisteredWorld, usize, &dyn Any, u64, Option<&mut WorldUndo>);
type BatchCount = fn(&dyn Any) -> usize;

/// One reusable candidate operation. Values remain in game-owned candidate
/// columns; the edit contains only checked identity and function pointers.
pub struct Edit<C> {
    inner: EditKind<C>,
}

/// Append-only access to this tick's engine-owned operation queue. Callbacks
/// cannot replace or remove operations emitted by an earlier system. The
/// runtime checks each callback's permits before accepting its output.
///
/// Values remain in shared game-owned candidate columns until final preflight;
/// this writer does not freeze payloads or isolate candidate fields by system.
///
/// ```compile_fail
/// use thrum_core::sim::{Edit, EditWriter};
/// fn replace_prior<C>(writer: &mut EditWriter<'_, C>, edit: Edit<C>) {
///     writer[0] = edit;
/// }
/// ```
pub struct EditWriter<'a, C> {
    edits: &'a mut Vec<Edit<C>>,
}

impl<'a, C> EditWriter<'a, C> {
    pub(super) fn new(edits: &'a mut Vec<Edit<C>>) -> Self {
        Self { edits }
    }

    /// Append an operation. A foreign permit refuses the complete tick at the
    /// callback boundary, even if the callback otherwise returns success.
    pub fn push(&mut self, edit: Edit<C>) {
        self.edits.push(edit);
    }
}

enum EditKind<C> {
    Component {
        handle: Handle,
        component: usize,
        nonce: u64,
        system: SystemId,
        kind: ComponentEditKind,
        index: usize,
        value: Option<ValueAt<C>>,
        apply: ApplyValue,
        present: Presence,
        valid_value: fn(&dyn Any) -> bool,
    },
    BatchReplace {
        component: usize,
        nonce: u64,
        system: SystemId,
        values: fn(&C) -> &dyn Any,
        validate: BatchValidate,
        apply: BatchApply,
        count: BatchCount,
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

fn present<T: Send + Sync + 'static>(world: &RegisteredWorld, component: usize, slot: u32) -> bool {
    world.present_at::<T>(component, slot)
}

fn is_type<T: 'static>(value: &dyn Any) -> bool {
    value.is::<T>()
}

fn replace<T: Clone + Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    component: usize,
    handle: Handle,
    value: &dyn Any,
    undo: Option<&mut WorldUndo>,
) {
    let old_tick = undo
        .as_ref()
        .map(|_| world.change_tick_at(component, handle.slot));
    let prior = world.replace_prevalidated(
        component,
        handle.slot,
        value
            .downcast_ref::<T>()
            .expect("prevalidated value type")
            .clone(),
    );
    if let (Some(undo), Some(old_tick)) = (undo, old_tick) {
        undo.record_component(component, handle.slot, old_tick, Some(prior));
    }
}

fn insert<T: Clone + Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    component: usize,
    handle: Handle,
    value: &dyn Any,
    undo: Option<&mut WorldUndo>,
) {
    let old_tick = undo
        .as_ref()
        .map(|_| world.change_tick_at(component, handle.slot));
    world.insert_prevalidated(
        component,
        handle.slot,
        value
            .downcast_ref::<T>()
            .expect("prevalidated value type")
            .clone(),
    );
    if let (Some(undo), Some(old_tick)) = (undo, old_tick) {
        undo.record_component::<T>(component, handle.slot, old_tick, None);
    }
}

fn remove<T: Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    component: usize,
    handle: Handle,
    _value: &dyn Any,
    undo: Option<&mut WorldUndo>,
) {
    let old_tick = undo
        .as_ref()
        .map(|_| world.change_tick_at(component, handle.slot));
    let prior = world.remove_prevalidated::<T>(component, handle.slot);
    if let (Some(undo), Some(old_tick)) = (undo, old_tick) {
        undo.record_component(component, handle.slot, old_tick, Some(prior));
    }
}

fn validate_batch_replace<T: Clone + Send + Sync + 'static>(
    world: &RegisteredWorld,
    component: usize,
    values: &dyn Any,
    scratch: &mut EditScratch,
) -> Result<(), EditError> {
    let rows = values
        .downcast_ref::<Vec<(Handle, T)>>()
        .ok_or(EditError::WrongValueType)?;
    for (handle, _) in rows {
        scratch.validate_component(
            world,
            *handle,
            component,
            ComponentEditKind::Replace,
            || world.present_at::<T>(component, handle.slot),
        )?;
    }
    Ok(())
}

fn apply_batch_replace<T: Clone + Send + Sync + 'static>(
    world: &mut RegisteredWorld,
    component: usize,
    values: &dyn Any,
    tick: u64,
    undo: Option<&mut WorldUndo>,
) {
    let rows = values
        .downcast_ref::<Vec<(Handle, T)>>()
        .expect("prevalidated batch value type");
    if let Some(undo) = undo {
        let mut writer = undo.batch_writer::<T>(component);
        world.replace_batch_prevalidated(component, rows, tick, |slot, prior, old_tick| {
            writer.push(slot, prior, old_tick);
        });
    } else {
        world.replace_batch_prevalidated(component, rows, tick, |_, _, _| {});
    }
}

fn batch_count<T: 'static>(values: &dyn Any) -> usize {
    values
        .downcast_ref::<Vec<(Handle, T)>>()
        .expect("prevalidated batch value type")
        .len()
}

impl<C> Edit<C> {
    pub(super) fn system(&self) -> SystemId {
        match self.inner {
            EditKind::Component { system, .. }
            | EditKind::BatchReplace { system, .. }
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
                component: permit.component,
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

    /// Stage a homogeneous replacement batch in caller-owned reusable storage.
    /// Every handle is checked with the rest of the tick before any edit applies.
    pub fn replace_batch<T: Clone + Send + Sync + 'static>(
        permit: &WritePermit<T>,
        values: fn(&C) -> &dyn Any,
    ) -> Self {
        Self {
            inner: EditKind::BatchReplace {
                component: permit.component,
                nonce: permit.nonce,
                system: permit.system,
                values,
                validate: validate_batch_replace::<T>,
                apply: apply_batch_replace::<T>,
                count: batch_count::<T>,
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
                component: permit.component,
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
                component: permit.component,
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
    builders: Vec<EntityBundle>,
    builders_used: usize,
    spawn_ids: HashSet<StableId>,
    undo_counts: Vec<usize>,
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
            undo_counts: vec![0; components],
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
                    component,
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
                    self.validate_component(world, *handle, *component, *kind, || {
                        present(world, *component, handle.slot)
                    })?;
                    if let Some(value) = value
                        && !valid_value(value(candidate, *index))
                    {
                        return Err(EditError::WrongValueType);
                    }
                }
                EditKind::BatchReplace {
                    component,
                    nonce: token,
                    values,
                    validate,
                    ..
                } => {
                    if *token != nonce {
                        return Err(EditError::WrongRuntime);
                    }
                    validate(world, *component, values(candidate), self)?;
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
                        self.builders.push(world.new_bundle());
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

    fn validate_component(
        &mut self,
        world: &RegisteredWorld,
        handle: Handle,
        component: usize,
        kind: ComponentEditKind,
        present: impl FnOnce() -> bool,
    ) -> Result<(), EditError> {
        world.entity(handle).map_err(EditError::World)?;
        let slot = handle.slot as usize;
        if self.retired[slot] == self.generation {
            return Err(EditError::Conflict);
        }
        let mark = slot * self.components + component;
        if self.marks[mark] == self.generation {
            return Err(EditError::Conflict);
        }
        self.marks[mark] = self.generation;
        let exists = present();
        if exists != (kind != ComponentEditKind::Insert) {
            return Err(EditError::World(if exists {
                RegisteredWorldError::DuplicateComponent
            } else {
                RegisteredWorldError::MissingComponent
            }));
        }
        Ok(())
    }

    pub(super) fn apply<C>(
        &mut self,
        world: &mut RegisteredWorld,
        edits: &[Edit<C>],
        candidate: &C,
        tick: u64,
        journal: &mut Vec<StructuralChange>,
        mut undo: Option<&mut WorldUndo>,
    ) {
        if let Some(undo) = undo.as_deref_mut() {
            self.undo_counts.fill(0);
            let mut structural = 0;
            let mut despawns = 0;
            for edit in edits {
                match &edit.inner {
                    EditKind::Component { component, .. } => self.undo_counts[*component] += 1,
                    EditKind::BatchReplace {
                        component,
                        values,
                        count,
                        ..
                    } => {
                        self.undo_counts[*component] += count(values(candidate));
                    }
                    EditKind::Despawn { .. } => {
                        structural += 1;
                        despawns += 1;
                    }
                    EditKind::Spawn { .. } => structural += 1,
                }
            }
            undo.reserve_for(&self.undo_counts, structural, despawns);
        }
        journal.clear();
        world.begin_tick(Some(tick));
        if let Some(undo) = undo.as_ref() {
            undo.visit_changed_components(|component, slot| {
                world.mark_changed(component, world.handle_at(slot), tick);
            });
        }
        for edit in edits {
            match &edit.inner {
                EditKind::Component {
                    handle,
                    index,
                    value,
                    apply,
                    component,
                    ..
                } => {
                    let empty: &dyn Any = &();
                    let value = value.map_or(empty, |get| get(candidate, *index));
                    apply(world, *component, *handle, value, undo.as_deref_mut());
                    world.mark_changed(*component, *handle, tick);
                }
                EditKind::BatchReplace {
                    component,
                    values,
                    apply,
                    ..
                } => apply(
                    world,
                    *component,
                    values(candidate),
                    tick,
                    undo.as_deref_mut(),
                ),
                EditKind::Despawn { .. } | EditKind::Spawn { .. } => {}
            }
        }
        for edit in edits {
            if let EditKind::Despawn { handle, .. } = &edit.inner {
                if let Some(undo) = undo.as_deref_mut() {
                    world
                        .despawn_into_undo(*handle, undo)
                        .expect("whole batch prevalidated");
                } else {
                    world.despawn(*handle).expect("whole batch prevalidated");
                }
                world.mark_structural(handle.slot);
                journal.push(StructuralChange::Despawn(*handle));
            }
        }
        let mut spawn = 0;
        for edit in edits {
            if let EditKind::Spawn { id, .. } = &edit.inner {
                if let Some(undo) = undo.as_deref_mut() {
                    undo.record_structural(world, world.next_free_slot());
                }
                let handle = world
                    .spawn_prepared(*id, &mut self.builders[spawn], tick)
                    .expect("whole batch prevalidated");
                world.mark_structural(handle.slot);
                journal.push(StructuralChange::Spawn(handle));
                spawn += 1;
            }
        }
        world.finish_tick(tick);
    }
}
