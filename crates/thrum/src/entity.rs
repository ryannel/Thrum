use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use thrum_core::StableId;

use crate::Data;

/// The handle for one spawned entity.
/// ```
/// use thrum::Entity;
/// fn remember(entity: Entity) -> Option<Entity> { Some(entity) }
/// ```
#[derive(Data, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct Entity(pub(crate) u128);

impl Entity {
    pub(crate) fn stable(self) -> StableId {
        StableId(self.0)
    }
}

use thrum_core::{
    Handle,
    sim::{Edit, EditWriter, EntityWriter, RegisteredWorldError, StructuralPermit, WorldRead},
};

use crate::{
    Component, Error, Result,
    data::Stored,
    rule::{
        access::{Bindings, Column},
        params::{Context, Param, ParameterList, ReadContext},
    },
    runtime::{Candidate, spawn_edit},
};

/// A tuple of components with which to spawn an entity.
/// ```
/// use thrum::{Component, Entities, Spawn};
/// #[derive(Component)] struct Health(u32);
/// fn actor() -> impl Spawn { (Health(3),) }
/// fn start(mut entities: Entities) { entities.spawn(actor()); }
/// ```
#[expect(private_bounds, reason = "spawn bundles are implemented by the SDK")]
pub trait Spawn: SpawnValue + Sized + 'static {}

pub(crate) trait SpawnValue: Send + Sync {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn fill(&self, writer: &mut EntityWriter<'_>)
    -> core::result::Result<(), RegisteredWorldError>;
    fn columns(&self, expect: &mut dyn FnMut(Column));
}

impl Spawn for () {}
impl SpawnValue for () {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn fill(&self, _: &mut EntityWriter<'_>) -> core::result::Result<(), RegisteredWorldError> {
        Ok(())
    }
    fn columns(&self, _: &mut dyn FnMut(Column)) {}
}

macro_rules! bundle {
    ($(($($ty:ident $index:tt),+)),+ $(,)?) => {$(
        impl<$($ty: Component),+> Spawn for ($($ty,)+) {}
        impl<$($ty: Component),+> SpawnValue for ($($ty,)+) {
            fn as_any_mut(&mut self) -> &mut dyn Any { self }
            fn fill(&self, writer: &mut EntityWriter<'_>) -> core::result::Result<(), RegisteredWorldError> {
                $(writer.add(Stored(self.$index.clone()))?;)+
                Ok(())
            }
            fn columns(&self, expect: &mut dyn FnMut(Column)) {
                $(expect(Column::of::<$ty>());)+
            }
        }
    )+};
}

bundle!((A 0), (A 0, B 1), (A 0, B 1, C 2), (A 0, B 1, C 2, D 3), (A 0, B 1, C 2, D 3, E 4), (A 0, B 1, C 2, D 3, E 4, F 5));

/// Spawn, despawn, insert and remove at the end of a rule.
/// ```
/// use thrum::{Component, Entities};
/// #[derive(Component)] struct Health(u32);
/// fn start(mut entities: Entities) { entities.spawn((Health(2),)); }
/// ```
pub struct Entities<'t>(&'t mut Changes);

impl Entities<'_> {
    /// Reserve a new entity handle and stage the tuple that it will carry.
    /// ```
    /// use thrum::{Component, Entities, Entity};
    /// #[derive(Component)] struct Health(u32);
    /// fn spawn(mut entities: Entities) -> Entity { entities.spawn((Health(2),)) }
    /// ```
    /// # Panics
    /// Only if the structural edit pool is poisoned or the entity ID space is exhausted.
    pub fn spawn(&mut self, tuple: impl Spawn) -> Entity {
        self.0.next_entity += 1;
        let entity = Entity(self.0.next_entity);
        let value = self
            .0
            .pool
            .lock()
            .expect("structural edit pool")
            .spawn(tuple);
        self.0.spawns.push((entity, value));
        entity
    }

    /// Stage the retirement of a life, once.
    /// ```
    /// use thrum::{Entities, Entity};
    /// fn retire(mut entities: Entities, entity: Entity) -> bool { entities.despawn(entity) }
    /// ```
    /// # Panics
    /// Only if the structural edit pool is poisoned.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if let Some(index) = self
            .0
            .spawns
            .iter()
            .position(|(spawned, _)| *spawned == entity)
        {
            let (_, bundle) = self.0.spawns.remove(index);
            self.0
                .pool
                .lock()
                .expect("structural edit pool")
                .births
                .push(bundle);
            return true;
        }
        if self.0.contains(entity) && !self.0.despawns.contains(&entity) {
            self.0.despawns.push(entity);
            true
        } else {
            false
        }
    }

    /// Insert or replace a component.
    /// ```
    /// use thrum::{Component, Entities, Entity};
    /// #[derive(Component)] struct Health(u32);
    /// fn heal(mut entities: Entities, entity: Entity) { entities.insert(entity, Health(10)); }
    /// ```
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.0
            .set(entity, ComponentChange::<T>(Some(Stored(value))));
    }

    /// Remove a component from an entity when the rule finishes.
    /// ```
    /// use thrum::{Component, Entities, Entity};
    /// #[derive(Component)] struct Wounded;
    /// fn cure(mut entities: Entities, entity: Entity) { entities.remove::<Wounded>(entity); }
    /// ```
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.0.set(entity, ComponentChange::<T>(None));
    }

    /// Whether this entity was alive when the rule began.
    /// ```
    /// use thrum::{Entities, Entity};
    /// fn living(entities: Entities, entity: Entity) -> bool { entities.alive(entity) }
    /// ```
    pub fn alive(&self, entity: Entity) -> bool {
        self.0.contains(entity)
    }
}

#[derive(Default)]
#[doc(hidden)]
/// Scratch used by the structural edits of one rule.
/// ```
/// use thrum::{Entities, __derive::{Changes, Param}};
/// let _: <Entities<'static> as Param>::State = Changes::default();
/// ```
pub struct Changes {
    live: Vec<Entity>,
    next_entity: u128,
    spawns: Vec<(Entity, Box<dyn SpawnValue>)>,
    despawns: Vec<Entity>,
    sets: Vec<(Entity, Box<dyn QueuedComponent>)>,
    pool: Arc<Mutex<EditPool>>,
}

impl Changes {
    pub(crate) fn reset(&mut self) {
        if !self.spawns.is_empty() || !self.sets.is_empty() {
            let mut pool = self.pool.lock().expect("structural edit pool");
            pool.births
                .extend(self.spawns.drain(..).map(|(_, bundle)| bundle));
            pool.components
                .extend(self.sets.drain(..).map(|(_, edit)| edit));
        }
        self.despawns.clear();
    }

    fn contains(&self, entity: Entity) -> bool {
        self.live.binary_search(&entity).is_ok()
    }

    fn set<T: Component>(&mut self, entity: Entity, change: ComponentChange<T>) {
        if let Some((_, stored)) = self.sets.iter_mut().find(|(known, change)| {
            *known == entity && change.column().key == Column::of::<T>().key
        }) {
            *stored
                .as_any_mut()
                .downcast_mut::<ComponentChange<T>>()
                .expect("same component type") = change;
        } else {
            let change = self
                .pool
                .lock()
                .expect("structural edit pool")
                .component(change);
            self.sets.push((entity, change));
        }
    }

    fn check(&self, world: &WorldRead<'_>, columns: &[Column], rule: &str) -> Result {
        let mut unknown = None;
        let mut check = |column: Column| {
            if !columns.iter().any(|known| known.key == column.key) {
                unknown.get_or_insert(column.name);
            }
        };
        for (_, value) in &self.spawns {
            value.columns(&mut check);
        }
        for (_, change) in &self.sets {
            check(change.column());
        }
        if let Some(name) = unknown {
            return Err(Error::refused(
                "T0301",
                rule,
                format!("component {name} is not registered"),
                "Declare it in a rule signature or register it with Game::component::<T>().",
            ));
        }
        let real_despawns = self
            .despawns
            .iter()
            .filter(|id| world.handle(id.stable()).is_some())
            .count();
        if world.capacity() - self.live.len() + real_despawns < self.spawns.len() {
            return Err(Error::refused(
                "T0302",
                rule,
                "spawn exceeds the world's entity capacity",
                "Increase Game::max_entities(..) or despawn an entity first.",
            ));
        }
        Ok(())
    }

    pub(crate) fn submit(
        &mut self,
        world: &WorldRead<'_>,
        bindings: &Bindings,
        permit: &StructuralPermit,
        candidate: &mut Candidate,
        edits: &mut EditWriter<'_, Candidate>,
    ) {
        for entity in &self.despawns {
            if let Some(handle) = world.handle(entity.stable()) {
                candidate.retired_entities.push(*entity);
                edits.push(Edit::despawn(permit, handle));
            }
        }
        for (entity, value) in self.spawns.drain(..) {
            candidate.spawned_entities.push(entity);
            candidate.changed_entities.push(entity);
            spawn_edit(permit, edits, candidate, entity.stable(), value);
        }
        for (entity, change) in self.sets.drain(..) {
            if self.despawns.contains(&entity) {
                self.pool
                    .lock()
                    .expect("structural edit pool")
                    .components
                    .push(change);
                continue;
            }
            if let Some(handle) = world.handle(entity.stable()) {
                let index = candidate.changes.len();
                let edit = change.edit(world, handle, bindings, index);
                if let Some(edit) = edit {
                    candidate.changed_entities.push(entity);
                    candidate.changes.push(change);
                    edits.push(edit);
                    continue;
                }
            }
            self.pool
                .lock()
                .expect("structural edit pool")
                .components
                .push(change);
        }
    }
}

pub(crate) trait QueuedComponent: Send + Sync {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn column(&self) -> Column;
    fn any(&self) -> &dyn Any;
    fn edit(
        &self,
        world: &WorldRead<'_>,
        handle: Handle,
        bindings: &Bindings,
        index: usize,
    ) -> Option<Edit<Candidate>>;
}

struct ComponentChange<T: Component>(Option<Stored<T>>);

impl<T: Component> QueuedComponent for ComponentChange<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn column(&self) -> Column {
        Column::of::<T>()
    }
    fn any(&self) -> &dyn Any {
        self.0
            .as_ref()
            .expect("only insert and replace use the value")
    }
    fn edit(
        &self,
        world: &WorldRead<'_>,
        handle: Handle,
        bindings: &Bindings,
        index: usize,
    ) -> Option<Edit<Candidate>> {
        fn value(candidate: &Candidate, index: usize) -> &dyn Any {
            candidate.changes[index].any()
        }
        let permit = bindings.write::<T>();
        let present = world.has_component(handle, self.column().key);
        match (&self.0, present) {
            (Some(next), true) => {
                let unchanged = world
                    .with(bindings.read::<T>(), handle, |current| current == next)
                    .expect("registered component on a live entity");
                (!unchanged).then(|| Edit::replace(permit, handle, index, value))
            }
            (Some(_), false) => Some(Edit::insert(permit, handle, index, value)),
            (None, true) => Some(Edit::remove(permit, handle)),
            (None, false) => None,
        }
    }
}

// SAFETY: This adapter only lends its own scratch and acquires no column loans.
#[expect(unsafe_code, reason = "trusted SDK parameter adapter")]
unsafe impl Param for Entities<'_> {
    type State = Changes;
    type Item<'t> = Entities<'t>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.writer::<Changes>(name);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        state.live.clear();
        state
            .live
            .extend(context.entities.iter().map(|&(entity, _)| entity));
        state.next_entity = context.candidate.state.next_entity;
        state.reset();
        state.pool = Arc::clone(&context.candidate.edit_pool);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Entities<'t> {
        Entities(state)
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        state.check(&context.world.read(), context.columns, context.name)?;
        context.candidate.state.next_entity = state.next_entity;
        std::mem::swap(context.changes, state);
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct EditPool {
    pub(crate) births: Vec<Box<dyn SpawnValue>>,
    pub(crate) components: Vec<Box<dyn QueuedComponent>>,
}

impl EditPool {
    fn spawn<S: Spawn>(&mut self, value: S) -> Box<dyn SpawnValue> {
        if let Some(index) = self
            .births
            .iter_mut()
            .position(|bundle| bundle.as_any_mut().is::<S>())
        {
            let mut bundle = self.births.swap_remove(index);
            *bundle
                .as_any_mut()
                .downcast_mut::<S>()
                .expect("same bundle type") = value;
            bundle
        } else {
            Box::new(value)
        }
    }

    fn component<T: Component>(&mut self, value: ComponentChange<T>) -> Box<dyn QueuedComponent> {
        if let Some(index) = self
            .components
            .iter_mut()
            .position(|edit| edit.as_any_mut().is::<ComponentChange<T>>())
        {
            let mut edit = self.components.swap_remove(index);
            *edit
                .as_any_mut()
                .downcast_mut::<ComponentChange<T>>()
                .expect("same component type") = value;
            edit
        } else {
            Box::new(value)
        }
    }
}

pub(crate) fn contains_entities(signature: &crate::rule::access::Signature) -> bool {
    signature.0.iter().any(|parameter| {
        if let crate::rule::access::Parameter::Exclusive(_, access) = parameter {
            access.id == std::any::TypeId::of::<Changes>()
        } else {
            false
        }
    })
}
