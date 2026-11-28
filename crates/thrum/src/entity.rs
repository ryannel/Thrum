pub(crate) mod family;

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
    Component, Error, Part, Result, Transform2d, Transform3d,
    data::Stored,
    prefab::{
        Catalog, Pose, Prepared, SpawnPatches,
        staging::{SpawnPatch, Staging},
    },
    rule::{
        access::{Bindings, Column},
        params::{Context, Param, ParameterList, ReadContext},
    },
    runtime::{Candidate, spawn_edit},
    transform::{Transform, preserve},
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
    fn fields(&self, add: &mut dyn FnMut(&dyn crate::prefab::Field));
    fn patch(&self, _: &mut dyn SpawnPatches) -> Result {
        Ok(())
    }
    fn children(&self, _: &mut dyn FnMut(&dyn SpawnValue) -> Result) -> Result {
        Ok(())
    }
    fn prefab(&self) -> Option<&crate::prefab::SpawnSpec> {
        None
    }
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
    fn fields(&self, _: &mut dyn FnMut(&dyn crate::prefab::Field)) {}
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
            fn fields(&self, add: &mut dyn FnMut(&dyn crate::prefab::Field)) {
                $(add(&Stored(self.$index));)+
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
        let entity = self.0.reserve();
        if tuple.prefab().is_some() {
            if let Err(error) = self.0.spawn_prefab(entity, &tuple, None) {
                self.0.error.get_or_insert(error);
            }
        } else {
            let value = self
                .0
                .pool
                .lock()
                .expect("structural edit pool")
                .spawn(tuple);
            self.0.spawns.push((entity, Birth::tuple(value)));
        }
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
        if !self.0.knows(entity) || self.0.despawns.contains(&entity) {
            return false;
        }
        if self.0.family.has_retired_ancestor(entity, &self.0.despawns) {
            return false;
        }
        self.0.despawns.push(entity);
        if !self.0.family.tree.empty() || !self.0.family_edits.is_empty() {
            self.0.family_edits.push(FamilyEdit::Retire(entity));
        }
        true
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
    family: family::FamilyRows,
    family_edits: Vec<FamilyEdit>,
    family_retired: Vec<Entity>,
    error: Option<Error>,
    prefabs: Arc<Catalog>,
    staging: Arc<Mutex<Staging>>,
    staged_spawns: bool,
    next_entity: u128,
    spawns: Vec<(Entity, Birth)>,
    despawns: Vec<Entity>,
    sets: Vec<(Entity, Box<dyn QueuedComponent>)>,
    pool: Arc<Mutex<EditPool>>,
}

impl Changes {
    pub(crate) fn reset(&mut self) {
        if !self.spawns.is_empty() || !self.sets.is_empty() {
            let mut pool = self.pool.lock().expect("structural edit pool");
            for (_, spawn) in self.spawns.drain(..) {
                spawn.recycle(&mut pool);
            }
            pool.components
                .extend(self.sets.drain(..).map(|(_, edit)| edit));
        }
        self.despawns.clear();
        self.family_edits.clear();
        self.family_retired.clear();
        self.error = None;
        self.staged_spawns = false;
    }

    fn contains(&self, entity: Entity) -> bool {
        self.family.view().alive(entity)
    }

    fn knows(&self, entity: Entity) -> bool {
        self.contains(entity) || self.spawns.iter().any(|(id, _)| *id == entity)
    }

    fn reserve(&mut self) -> Entity {
        self.next_entity += 1;
        Entity(self.next_entity)
    }

    fn spawn_prefab(
        &mut self,
        entity: Entity,
        spawn: &dyn SpawnValue,
        parent: Option<Entity>,
    ) -> Result {
        let spec = spawn.prefab().expect("prefab spawn");
        let prepared = self.prefabs.spawn(spec)?;
        let overridden = self.apply_spawn_patches(entity, spawn, Some(&prepared))?;
        self.stage_prepared(
            entity,
            Some(&prepared),
            Placement {
                parent: parent.or(spec.parent),
                pose: spec.pose,
                disabled: spec.disabled,
                overridden,
                ..Placement::default()
            },
        );
        spawn.children(&mut |child| {
            let id = self.reserve();
            if child.prefab().is_some() {
                self.spawn_prefab(id, child, Some(entity))
            } else {
                let overridden = self.apply_spawn_patches(id, child, None)?;
                self.stage_prepared(
                    id,
                    None,
                    Placement {
                        parent: Some(entity),
                        overridden,
                        ..Placement::default()
                    },
                );
                Ok(())
            }
        })
    }

    fn apply_spawn_patches(
        &mut self,
        entity: Entity,
        spawn: &dyn SpawnValue,
        template: Option<&Prepared>,
    ) -> Result<bool> {
        let mut patch = SpawnPatch {
            staging: &self.staging,
            entity,
            template,
            changed: false,
        };
        let mut result = Ok(());
        spawn.fields(&mut |field| {
            if result.is_ok() {
                result = patch.set(field);
            }
        });
        result?;
        spawn.patch(&mut patch)?;
        self.staged_spawns |= patch.changed;
        Ok(patch.changed)
    }

    fn stage_prepared(
        &mut self,
        entity: Entity,
        prepared: Option<&Arc<Prepared>>,
        placement: Placement,
    ) {
        self.spawns.push((
            entity,
            Birth {
                value: BirthValue::Prefab(prepared.map(Arc::clone)),
                two: placement.pose.and_then(Pose::get),
                three: placement.pose.and_then(Pose::get),
                part: placement.part,
                defined_child: placement.defined_child,
                overridden: placement.overridden,
                entity: Some(entity),
            },
        ));
        if let Some(parent) = placement.parent {
            self.family_edits.push(FamilyEdit::Move(
                entity,
                Some(parent),
                Reparent::Local,
                placement.part,
            ));
        }
        if placement.disabled {
            self.family_edits.push(FamilyEdit::Disable(entity, true));
        }
        if let Some(prepared) = prepared {
            for child in &prepared.children {
                let id = self.reserve();
                self.stage_prepared(
                    id,
                    Some(&child.value),
                    Placement {
                        parent: Some(entity),
                        part: child.part,
                        disabled: child.value.disabled,
                        defined_child: true,
                        ..Placement::default()
                    },
                );
            }
        }
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
        if world.capacity() - self.family.len() + real_despawns < self.spawns.len() {
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
        candidate.staged_spawns = self.staged_spawns;
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
    fn value(&self) -> Option<&dyn Any>;
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
    fn value(&self) -> Option<&dyn Any> {
        self.0.as_ref().map(|value| -> &dyn Any { value })
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
        state.family.read(context);
        state.prefabs.clone_from(&context.candidate.prefabs);
        state.staging.clone_from(&context.candidate.staging);
        state.next_entity = context.candidate.state.next_entity;
        state.reset();
        state.pool = Arc::clone(&context.candidate.edit_pool);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Entities<'t> {
        Entities(state)
    }
    fn journal(state: &mut Self::State, _: &mut Context<'_, '_>) -> Result {
        state.family.release();
        Ok(())
    }

    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        if let Some(error) = state.error.take() {
            return Err(error);
        }
        state.apply_family(context.candidate)?;
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

enum BirthValue {
    Tuple(Box<dyn SpawnValue>),
    Prefab(Option<Arc<Prepared>>),
}

#[derive(Clone, Copy, Default)]
struct Placement {
    parent: Option<Entity>,
    part: Option<Part>,
    pose: Option<Pose>,
    disabled: bool,
    defined_child: bool,
    overridden: bool,
}

pub(crate) struct Birth {
    value: BirthValue,
    two: Option<Transform2d>,
    three: Option<Transform3d>,
    part: Option<Part>,
    defined_child: bool,
    overridden: bool,
    entity: Option<Entity>,
}

impl Birth {
    fn tuple(value: Box<dyn SpawnValue>) -> Self {
        Self {
            value: BirthValue::Tuple(value),
            two: None,
            three: None,
            part: None,
            defined_child: false,
            overridden: false,
            entity: None,
        }
    }

    pub(crate) fn recycle(self, pool: &mut EditPool) {
        if let BirthValue::Tuple(value) = self.value {
            pool.births.push(value);
        }
    }

    fn columns(&self, check: &mut impl FnMut(Column)) {
        match &self.value {
            BirthValue::Tuple(value) => value.columns(check),
            BirthValue::Prefab(_) => {} // Prepared and staged components were registered before this rule.
        }
    }

    fn get<T: Component>(&self, staging: &Mutex<Staging>) -> Option<T> {
        let two = self.two.and_then(|pose| Pose::Two(pose).get());
        let three = self.three.and_then(|pose| Pose::Three(pose).get());
        two.or(three).or_else(|| match &self.value {
            BirthValue::Prefab(value) => {
                if self.overridden {
                    let staging = staging.lock().expect("prefab spawn staging");
                    if let Some(value) =
                        staging.get(self.entity.expect("staged spawn"), Column::of::<T>())
                    {
                        return value.downcast_ref::<T>().copied();
                    }
                }
                value.as_ref().and_then(|value| value.get())
            }
            BirthValue::Tuple(value) => {
                let mut result = None;
                value.fields(&mut |field| {
                    if let Some(&value) = field.read().downcast_ref::<T>() {
                        result = Some(value);
                    }
                });
                result
            }
        })
    }

    pub(crate) fn fill(
        &self,
        writer: &mut EntityWriter<'_>,
        staging: &Mutex<Staging>,
    ) -> std::result::Result<(), RegisteredWorldError> {
        let poses = [self.two.map(Pose::Two), self.three.map(Pose::Three)];
        let at = |column: Column| {
            poses
                .iter()
                .flatten()
                .any(|pose| pose.column().key == column.key)
        };
        match &self.value {
            BirthValue::Tuple(value) if self.two.is_none() && self.three.is_none() => {
                value.fill(writer)?;
            }
            BirthValue::Tuple(value) => {
                let mut result = Ok(());
                value.fields(&mut |field| {
                    if !at(field.column()) && result.is_ok() {
                        result = field.fill(writer);
                    }
                });
                result?;
            }
            BirthValue::Prefab(value) if self.overridden => {
                let staging = staging.lock().expect("prefab spawn staging");
                let entity = self.entity.expect("staged spawn");
                if let Some(value) = value {
                    value.fill(writer, self.defined_child, self.part, |column| {
                        at(column) || staging.get(entity, column).is_some()
                    })?;
                }
                let omitted = poses.map(|pose| pose.and_then(|pose| staging.index(pose.column())));
                staging.fill(entity, writer, |index| omitted.contains(&Some(index)))?;
            }
            BirthValue::Prefab(value) => {
                if let Some(value) = value {
                    value.fill(writer, self.defined_child, self.part, at)?;
                }
            }
        }
        for pose in poses.into_iter().flatten() {
            pose.fill(writer)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Reparent {
    World,
    Local,
    At(Pose),
}

enum FamilyEdit {
    Move(Entity, Option<Entity>, Reparent, Option<Part>),
    Disable(Entity, bool),
    Retire(Entity),
}

impl Changes {
    fn apply_family(&mut self, candidate: &mut Candidate) -> Result {
        if self.family_edits.is_empty() {
            self.cancel_births();
            return Ok(());
        }
        let tree = Arc::make_mut(&mut candidate.state.tree);
        let mut edits = std::mem::take(&mut self.family_edits);
        for edit in edits.drain(..) {
            match edit {
                FamilyEdit::Move(child, parent, how, part) => {
                    for entity in [Some(child), parent].into_iter().flatten() {
                        if !self.knows(entity) || self.family_retired.contains(&entity) {
                            return Err(Error::refused(
                                "T0313",
                                format!("{entity:?}"),
                                "attachment needs a live or reserved entity",
                                "Use a handle that has not been despawned.",
                            ));
                        }
                    }
                    match how {
                        Reparent::World => {
                            self.preserve_in::<Transform2d>(tree, child, parent, Pose::Two)?;
                            self.preserve_in::<Transform3d>(tree, child, parent, Pose::Three)?;
                        }
                        Reparent::At(pose) => self.place(child, pose),
                        Reparent::Local => {}
                    }
                    if let Some(parent) = parent {
                        tree.attach(child, parent, part)?;
                    } else {
                        tree.detach(child);
                    }
                    mark_family(tree, child, &mut candidate.changed_entities);
                }
                FamilyEdit::Disable(entity, disabled) => {
                    if !self.family_retired.contains(&entity) {
                        tree.disable(entity, disabled);
                        mark_family(tree, entity, &mut candidate.changed_entities);
                    }
                }
                FamilyEdit::Retire(entity) => {
                    let start = self.family_retired.len();
                    mark_family(tree, entity, &mut self.family_retired);
                    for &retired in &self.family_retired[start..] {
                        tree.retire(retired);
                    }
                }
            }
        }

        self.family_edits = edits;
        self.despawns.append(&mut self.family_retired);
        self.despawns.sort_unstable();
        self.despawns.dedup();
        self.cancel_births();
        Ok(())
    }

    fn cancel_births(&mut self) {
        if self.despawns.is_empty() {
            return;
        }
        let mut index = 0;
        while index < self.spawns.len() {
            if self.despawns.contains(&self.spawns[index].0) {
                let (_, value) = self.spawns.remove(index);
                value.recycle(&mut self.pool.lock().expect("structural edit pool"));
            } else {
                index += 1;
            }
        }
    }

    fn preserve_in<T: Transform>(
        &mut self,
        tree: &family::Tree,
        child: Entity,
        parent: Option<Entity>,
        pose: impl FnOnce(T) -> Pose,
    ) -> Result {
        let local = |entity| {
            self.sets
                .iter()
                .find_map(|(id, change)| {
                    (*id == entity)
                        .then(|| {
                            change
                                .value()
                                .and_then(|value| value.downcast_ref::<Stored<T>>())
                                .map(|value| value.0)
                        })
                        .flatten()
                })
                .or_else(|| {
                    self.spawns
                        .iter()
                        .find(|(id, _)| *id == entity)
                        .and_then(|(_, birth)| birth.get::<T>(&self.staging))
                })
                .or_else(|| self.family.local::<T>(entity))
        };
        let has_transform = |start| {
            std::iter::successors(start, |&entity| tree.parent(entity))
                .any(|entity| local(entity).is_some())
        };
        let store_transform = has_transform(Some(child)) || has_transform(parent);
        let before = tree.world(child, local);
        let parent = parent.map_or(T::ORIGIN, |entity| tree.world(entity, local));
        let local = preserve(parent, before, child)?;
        if store_transform {
            self.place(child, pose(local));
        }
        Ok(())
    }

    fn place(&mut self, entity: Entity, pose: Pose) {
        if let Some((_, birth)) = self.spawns.iter_mut().find(|(id, _)| *id == entity) {
            match pose {
                Pose::Two(value) => birth.two = Some(value),
                Pose::Three(value) => birth.three = Some(value),
            }
        } else {
            match pose {
                Pose::Two(value) => self.set(entity, ComponentChange(Some(Stored(value)))),
                Pose::Three(value) => self.set(entity, ComponentChange(Some(Stored(value)))),
            }
        }
    }
}

fn mark_family(tree: &family::Tree, entity: Entity, output: &mut Vec<Entity>) {
    let mut index = output.len();
    output.push(entity);
    while index < output.len() {
        let parent = output[index];
        output.extend_from_slice(tree.children(parent));
        index += 1;
    }
}

impl Entities<'_> {
    /// Read the hierarchy as it stood when this rule started.
    pub fn parent(&self, child: Entity) -> Option<Entity> {
        self.0.family.view().parent(child)
    }

    /// Read direct children in attachment order.
    pub fn children(&self, parent: Entity) -> impl ExactSizeIterator<Item = Entity> + use<> {
        self.0.family.tree.child_iter(parent)
    }

    /// Find a named child, or refuse the tick when the part is missing.
    ///
    /// # Panics
    /// Only if the rule's request lock is poisoned.
    pub fn part(&self, parent: Entity, part: Part) -> Entity {
        self.0.family.view().part(parent, part)
    }

    /// Read an entity's composed 2D transform.
    pub fn world2d(&self, entity: Entity) -> Transform2d {
        self.0.family.view().world2d(entity)
    }

    /// Read an entity's composed 3D transform.
    pub fn world3d(&self, entity: Entity) -> Transform3d {
        self.0.family.view().world3d(entity)
    }

    /// Choose the parent and placement of an attachment.
    pub fn attach(&mut self, child: Entity) -> Attach<'_> {
        Attach {
            edits: self.0,
            child,
        }
    }

    /// Choose the placement of an entity to detach.
    pub fn detach(&mut self, child: Entity) -> Detach<'_> {
        Detach(Reparenting {
            edits: self.0,
            child,
            parent: None,
        })
    }

    /// Disable an entity and its descendants after this rule.
    pub fn disable(&mut self, entity: Entity) -> bool {
        self.0.set_disabled(entity, true)
    }

    /// Enable an entity after this rule, retaining any independently disabled descendants.
    pub fn enable(&mut self, entity: Entity) -> bool {
        self.0.set_disabled(entity, false)
    }
}

impl Changes {
    fn set_disabled(&mut self, entity: Entity, disabled: bool) -> bool {
        let current = self
            .family_edits
            .iter()
            .rev()
            .find_map(|edit| {
                if let FamilyEdit::Disable(id, value) = edit
                    && *id == entity
                {
                    Some(*value)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.family.tree.explicitly_disabled(entity));
        if current == disabled || !self.knows(entity) || self.despawns.contains(&entity) {
            return false;
        }
        self.family_edits
            .push(FamilyEdit::Disable(entity, disabled));
        true
    }
}

/// An attachment awaiting its parent.
#[must_use]
pub struct Attach<'a> {
    edits: &'a mut Changes,
    child: Entity,
}

impl<'a> Attach<'a> {
    /// Choose the entity that will own the child.
    pub fn to(self, parent: Entity) -> Reparenting<'a> {
        Reparenting {
            edits: self.edits,
            child: self.child,
            parent: Some(parent),
        }
    }
}

/// An attachment or detachment awaiting placement.
#[must_use]
pub struct Reparenting<'a> {
    edits: &'a mut Changes,
    child: Entity,
    parent: Option<Entity>,
}

impl Reparenting<'_> {
    /// Preserve the child's world transform within the hierarchy's bound.
    pub fn keep_world(self) {
        self.stage(Reparent::World);
    }

    /// Use the supplied pose relative to the new parent.
    pub fn keep_local(self, pose: impl Into<Pose>) {
        self.stage(Reparent::At(pose.into()));
    }

    fn stage(self, how: Reparent) {
        self.edits
            .family_edits
            .push(FamilyEdit::Move(self.child, self.parent, how, None));
    }
}

/// A detachment awaiting placement.
#[must_use]
pub struct Detach<'a>(Reparenting<'a>);

impl Detach<'_> {
    /// Leave the entity at its composed world transform.
    pub fn keep_world(self) {
        self.0.stage(Reparent::World);
    }

    /// Use the former local transform as the new world transform.
    pub fn keep_local(self) {
        self.0.stage(Reparent::Local);
    }

    /// Set the detached entity's world transform.
    pub fn at(self, pose: impl Into<Pose>) {
        self.0.stage(Reparent::At(pose.into()));
    }
}
