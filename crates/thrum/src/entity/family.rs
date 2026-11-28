use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::{Arc, Mutex},
};

use thrum_core::{
    profile::{Digest, StateEncode},
    sim::WorldRead,
};

use crate::{
    Component, Data, Entity, Error, Result, Transform2d, Transform3d,
    data::{DataReader, DataWriter, InvalidData, schema_id},
    rule::params::{Param, ParameterList, ReadContext, ReadOnly},
    runtime::Requests,
    transform::Transform,
};

/// The name of a part within a prefab.
#[derive(Clone, Copy)]
pub struct Part {
    id: u128,
    label: Option<&'static str>,
}

impl Part {
    /// Name a part shared by a prefab definition and the rules using it.
    pub const fn new(name: &'static str) -> Self {
        Self {
            id: schema_id(name, &[]),
            label: Some(name),
        }
    }
}

impl fmt::Debug for Part {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.label {
            output.write_str(name)
        } else {
            write!(output, "{:032x}", self.id)
        }
    }
}

impl PartialEq for Part {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Part {}
impl PartialOrd for Part {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Part {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Data for Part {
    const SCHEMA: &'static str = "thrum::Part";
    fn same_encoding(&self, other: &Self) -> bool {
        self == other
    }
    fn write(&self, writer: &mut DataWriter<'_, '_>) {
        self.id.write(writer);
    }
    fn read(reader: &mut DataReader<'_, '_>) -> std::result::Result<Self, InvalidData> {
        Ok(Self {
            id: u128::read(reader)?,
            label: None,
        })
    }
}

#[derive(Clone, Default, PartialEq)]
pub(crate) struct Tree {
    parents: BTreeMap<Entity, Entity>,
    #[expect(
        clippy::rc_buffer,
        reason = "snapshots share the ordered children and later edits grow the buffer in place"
    )]
    children: BTreeMap<Entity, Arc<Vec<Entity>>>,
    parts: BTreeMap<Entity, BTreeMap<Part, Entity>>,
    disabled: BTreeSet<Entity>,
}

impl Tree {
    pub(crate) fn parent(&self, child: Entity) -> Option<Entity> {
        self.parents.get(&child).copied()
    }

    pub(crate) fn children(&self, parent: Entity) -> &[Entity] {
        self.children
            .get(&parent)
            .map_or(&[], |children| children.as_slice())
    }

    pub(super) fn child_iter(
        &self,
        parent: Entity,
    ) -> impl ExactSizeIterator<Item = Entity> + use<> {
        let children = self.children.get(&parent).cloned();
        (0..children.as_ref().map_or(0, |items| items.len())).map(move |index| {
            children
                .as_ref()
                .expect("this range is empty without children")[index]
        })
    }

    pub(crate) fn has_disabled(&self) -> bool {
        !self.disabled.is_empty()
    }

    pub(crate) fn explicitly_disabled(&self, entity: Entity) -> bool {
        self.disabled.contains(&entity)
    }

    pub(crate) fn is_disabled(&self, entity: Entity) -> bool {
        if self.disabled.is_empty() {
            return false;
        }
        for ancestor in std::iter::successors(Some(entity), |&entity| self.parent(entity)) {
            if self.disabled.contains(&ancestor) {
                return true;
            }
        }
        false
    }

    pub(crate) fn attach(&mut self, child: Entity, parent: Entity, part: Option<Part>) -> Result {
        if std::iter::successors(Some(parent), |&entity| self.parent(entity))
            .any(|ancestor| ancestor == child)
        {
            return Err(Error::refused(
                "T0310",
                format!("child {child:?}, parent {parent:?}"),
                "attachment would make a cycle",
                "Choose a parent outside the child's descendants.",
            ));
        }
        if self.parent(child) != Some(parent) {
            self.detach(child);
            self.parents.insert(child, parent);
            Arc::make_mut(self.children.entry(parent).or_default()).push(child);
        }
        if let Some(part) = part {
            self.parts.entry(parent).or_default().insert(part, child);
        }
        Ok(())
    }

    pub(crate) fn detach(&mut self, child: Entity) {
        if let Some(parent) = self.parents.remove(&child) {
            let siblings = self
                .children
                .get_mut(&parent)
                .expect("parent has this child");
            Arc::make_mut(siblings).retain(|&entity| entity != child);
            if siblings.is_empty() {
                self.children.remove(&parent);
            }
            if let Some(parts) = self.parts.get_mut(&parent) {
                parts.retain(|_, entity| *entity != child);
                if parts.is_empty() {
                    self.parts.remove(&parent);
                }
            }
        }
    }

    pub(crate) fn retire(&mut self, entity: Entity) {
        self.detach(entity);
        self.disabled.remove(&entity);
    }

    pub(crate) fn disable(&mut self, entity: Entity, disabled: bool) {
        if disabled {
            self.disabled.insert(entity);
        } else {
            self.disabled.remove(&entity);
        }
    }

    pub(crate) fn empty(&self) -> bool {
        self.parents.is_empty() && self.disabled.is_empty()
    }

    pub(crate) fn encode(&self, digest: &mut Digest<'_>) {
        // The digest visits attached children and explicitly disabled entities only.
        (self.children.len() as u64).encode(digest);
        for (&parent, children) in &self.children {
            parent.0.encode(digest);
            (children.len() as u64).encode(digest);
            for child in children.iter() {
                child.0.encode(digest);
            }
        }
        (self.parts.len() as u64).encode(digest);
        for (&parent, parts) in &self.parts {
            parent.0.encode(digest);
            (parts.len() as u64).encode(digest);
            for (&part, child) in parts {
                part.id.encode(digest);
                child.0.encode(digest);
            }
        }
        (self.disabled.len() as u64).encode(digest);
        for entity in &self.disabled {
            entity.0.encode(digest);
        }
    }

    pub(crate) fn world<T: Transform>(
        &self,
        entity: Entity,
        local: impl Fn(Entity) -> Option<T>,
    ) -> T {
        fn world<T: Transform>(
            tree: &Tree,
            entity: Entity,
            local: &impl Fn(Entity) -> Option<T>,
        ) -> T {
            let place = local(entity).unwrap_or(T::ORIGIN);
            tree.parent(entity)
                .map_or(place, |parent| world(tree, parent, local).compose(place))
        }
        world(self, entity, &local)
    }
}

static EMPTY_TREE: std::sync::LazyLock<Arc<Tree>> = std::sync::LazyLock::new(Arc::default);

#[doc(hidden)]
pub struct FamilyRows {
    pub(super) tree: Arc<Tree>,
    live: Vec<Entity>,
    two: Vec<(Entity, Transform2d)>,
    three: Vec<(Entity, Transform3d)>,
    requests: Arc<Mutex<Requests>>,
}

impl Default for FamilyRows {
    fn default() -> Self {
        Self {
            tree: Arc::clone(&EMPTY_TREE),
            live: Vec::new(),
            two: Vec::new(),
            three: Vec::new(),
            requests: Arc::default(),
        }
    }
}

impl FamilyRows {
    pub(super) fn read(&mut self, context: &ReadContext<'_>) {
        let world = context.world.expect("preparing family view");
        self.tree = Arc::clone(&context.candidate.state.tree);
        self.requests = Arc::clone(&context.candidate.requests);
        self.live.clear();
        self.live
            .extend(context.entities.iter().map(|&(entity, _)| entity));
        gather(&mut self.two, world, context);
        gather(&mut self.three, world, context);
    }

    pub(super) fn release(&mut self) {
        self.tree = Arc::clone(&EMPTY_TREE);
    }

    pub(super) fn len(&self) -> usize {
        self.live.len()
    }

    pub(super) fn has_retired_ancestor(&self, entity: Entity, retiring: &[Entity]) -> bool {
        std::iter::successors(self.tree.parent(entity), |&entity| self.tree.parent(entity))
            .any(|ancestor| retiring.contains(&ancestor))
    }

    pub(super) fn local<T: Component>(&self, entity: Entity) -> Option<T> {
        get(&self.two, entity)
            .and_then(|value| crate::prefab::Pose::Two(value).get())
            .or_else(|| {
                get(&self.three, entity).and_then(|value| crate::prefab::Pose::Three(value).get())
            })
    }

    pub(super) fn view(&self) -> Family<'_> {
        Family(self)
    }
}

fn gather<T: Component>(
    rows: &mut Vec<(Entity, T)>,
    world: &WorldRead<'_>,
    context: &ReadContext<'_>,
) {
    rows.clear();
    if let Some(permit) = context.bindings.try_read::<T>() {
        let column = world.column(permit).expect("family transform column");
        rows.extend(context.entities.iter().filter_map(|&(entity, handle)| {
            column.at_slot(handle.slot).map(|value| (entity, value.0))
        }));
    }
}

fn get<T: Copy>(rows: &[(Entity, T)], entity: Entity) -> Option<T> {
    rows.binary_search_by_key(&entity, |row| row.0)
        .ok()
        .map(|index| rows[index].1)
}

/// Read the hierarchy and transforms as they stood when the rule began.
pub struct Family<'a>(&'a FamilyRows);

impl Family<'_> {
    /// Whether the entity was alive when the rule began.
    pub fn alive(&self, entity: Entity) -> bool {
        self.0.live.binary_search(&entity).is_ok()
    }

    /// The entity's parent, if attached.
    pub fn parent(&self, child: Entity) -> Option<Entity> {
        self.0.tree.parent(child)
    }

    /// The direct children of this parent, in attachment order.
    pub fn children(&self, parent: Entity) -> impl ExactSizeIterator<Item = Entity> + use<> {
        self.0.tree.child_iter(parent)
    }

    /// Find a named child. A missing part refuses the tick.
    ///
    /// # Panics
    /// Only if the rule's request lock is poisoned.
    pub fn part(&self, parent: Entity, part: Part) -> Entity {
        if let Some(&entity) = self
            .0
            .tree
            .parts
            .get(&parent)
            .and_then(|parts| parts.get(&part))
        {
            return entity;
        }
        self.0
            .requests
            .lock()
            .expect("rule requests")
            .error
            .get_or_insert_with(|| {
                Error::refused(
                    "T0309",
                    format!("part {part:?} of {parent:?}"),
                    "the entity has no such part",
                    "Define the part with PrefabDef::part or use a part the prefab declares.",
                )
            });
        parent
    }

    /// Compose the 2D transform through its parents.
    pub fn world2d(&self, entity: Entity) -> Transform2d {
        self.0.tree.world(entity, |entity| get(&self.0.two, entity))
    }

    /// Compose the 3D transform through its parents.
    pub fn world3d(&self, entity: Entity) -> Transform3d {
        self.0
            .tree
            .world(entity, |entity| get(&self.0.three, entity))
    }
}

// SAFETY: The adapter lends only snapshots of the transforms and the hierarchy.
#[expect(unsafe_code, reason = "read-only family parameter adapter")]
unsafe impl Param for Family<'_> {
    type State = FamilyRows;
    type Item<'t> = Family<'t>;

    fn describe(_: &str, list: &mut ParameterList) {
        list.family();
    }

    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        state.read(context);
        Ok(())
    }

    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Self::Item<'t> {
        state.view()
    }

    fn journal(state: &mut Self::State, _: &mut crate::rule::params::Context<'_, '_>) -> Result {
        state.release();
        Ok(())
    }
}

impl ReadOnly for Family<'_> {}
