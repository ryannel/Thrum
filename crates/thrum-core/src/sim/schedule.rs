/// Authoritative stage order. Every system in one stage finishes before the next.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// Map admitted controls and one-shot actions into tick work.
    Input,
    /// Advance physics, AI and timers even when no new action arrived.
    Simulate,
    /// Resolve ordered edits and validate the complete candidate tick.
    Resolve,
    /// Commit the validated tick; consumers can only observe this endpoint.
    Publish,
}

/// Stable registration identity within one compiled schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemId(pub u32);

/// Process-local identity of a registered component type. Stable game schema
/// identities for saves and wire data are separate from this scheduler key.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentKey(TypeId);

impl ComponentKey {
    /// Identify one game-owned component type for conflict detection.
    pub fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }
}

/// Stable game-assigned schema identity, distinct from process-local `TypeId`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentSchemaId(pub u128);

/// Setup-time component catalogue. A game registers its types before the
/// schedule is compiled; tick execution performs no registry lookup.
#[derive(Default)]
pub struct ComponentRegistry {
    pub(super) entries: Vec<ComponentEntry>,
    pub(super) activity_partition: Option<ComponentKey>,
}

/// Registration fails without changing the catalogue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationError {
    /// This Rust component type is already registered.
    DuplicateType,
    /// This stable schema identity belongs to another component type.
    DuplicateSchema,
    /// Publication selection names a component type absent from the catalogue.
    UnregisteredType,
    /// This component already has a publication policy.
    DuplicatePublication,
    /// This component is not backed by a maintained activity predicate.
    NotIndexed,
    /// A storage activity partition was already selected.
    DuplicateActivityPartition,
}

impl ComponentRegistry {
    /// Register one game component type and its durable schema identity.
    ///
    /// # Errors
    ///
    /// The type or schema identity was already registered.
    pub fn register<T: Copy + Send + Sync + 'static>(
        &mut self,
        schema: ComponentSchemaId,
    ) -> Result<ComponentKey, RegistrationError> {
        self.register_with_snapshot(schema, |value: &T| *value, |_| 0)
    }

    /// Register a component with a maintained predicate index. The predicate
    /// must be pure and depend only on its component value; it selects present
    /// values for indexed queries without changing component membership.
    ///
    /// # Errors
    ///
    /// The type or schema identity was already registered.
    pub fn register_indexed<T: Copy + PartialEq + Send + Sync + 'static>(
        &mut self,
        schema: ComponentSchemaId,
        predicate: fn(&T) -> bool,
    ) -> Result<ComponentKey, RegistrationError> {
        self.register_component(
            schema,
            |value: &T| *value,
            |_| 0,
            true,
            Box::new(move |capacity| Box::new(TypedStore::<T>::new_indexed(capacity, predicate))),
        )
    }

    /// Register a non-Copy value with an explicit independent snapshot copy.
    /// The operation must not retain shared mutable state from its input.
    /// `nested_bytes` reports heap storage retained by one copied value,
    /// excluding the inline `T` bytes already counted by the column.
    ///
    /// # Errors
    ///
    /// The type or schema identity was already registered.
    pub fn register_with_snapshot<T: Send + Sync + 'static>(
        &mut self,
        schema: ComponentSchemaId,
        snapshot: fn(&T) -> T,
        nested_bytes: fn(&T) -> usize,
    ) -> Result<ComponentKey, RegistrationError> {
        self.register_component(
            schema,
            snapshot,
            nested_bytes,
            false,
            Box::new(|capacity| Box::new(TypedStore::<T>::new(capacity))),
        )
    }

    fn register_component<T: Send + Sync + 'static>(
        &mut self,
        schema: ComponentSchemaId,
        snapshot: fn(&T) -> T,
        nested_bytes: fn(&T) -> usize,
        indexed: bool,
        storage: CreateStorage,
    ) -> Result<ComponentKey, RegistrationError> {
        let key = ComponentKey::of::<T>();
        if self.entries.iter().any(|entry| entry.key == key) {
            return Err(RegistrationError::DuplicateType);
        }
        if self.entries.iter().any(|entry| entry.schema == schema) {
            return Err(RegistrationError::DuplicateSchema);
        }
        self.entries.push(ComponentEntry {
            key,
            schema,
            indexed,
            publication_copy: Box::new(super::publication::PublicationCopy {
                snapshot,
                nested_bytes,
            }),
            publication: None,
            capture: Box::new(move |world| capture_component(world, snapshot, nested_bytes)),
            storage,
            pending: Box::new(|| Box::new(Pending::<T> { value: None })),
            undo: Box::new(move || Box::new(super::undo::TypedUndo::<T>::new(snapshot))),
            empty: Box::new(move || {
                Box::new(TypedColumn::<T> {
                    rows: Vec::new(),
                    snapshot,
                    nested_bytes,
                })
            }),
        });
        Ok(key)
    }

    /// Select the one indexed component that partitions hot rows by activity.
    /// Logical handles and game component types remain unchanged.
    ///
    /// # Errors
    ///
    /// The type is absent, lacks an index, or a partition was already selected.
    pub fn select_activity_partition<T: Send + Sync + 'static>(
        &mut self,
    ) -> Result<(), RegistrationError> {
        if self.activity_partition.is_some() {
            return Err(RegistrationError::DuplicateActivityPartition);
        }
        let key = ComponentKey::of::<T>();
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.key == key)
            .ok_or(RegistrationError::UnregisteredType)?;
        if !entry.indexed {
            return Err(RegistrationError::NotIndexed);
        }
        self.activity_partition = Some(key);
        Ok(())
    }

    /// Select an engine-owned publication column for a registered game type.
    /// `temporal` reserves a previous-value column for adjacent-tick sampling.
    /// Equality compares complete typed values; snapshot copying reuses the
    /// callback supplied at component registration.
    ///
    /// # Errors
    ///
    /// The type is unregistered or already selected for publication.
    ///
    /// # Panics
    ///
    /// Panics only if the internal registered-type invariant is violated.
    pub fn select_publication<T: PartialEq + Send + Sync + 'static>(
        &mut self,
        temporal: bool,
    ) -> Result<(), RegistrationError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.key == ComponentKey::of::<T>())
            .ok_or(RegistrationError::UnregisteredType)?;
        if entry.publication.is_some() {
            return Err(RegistrationError::DuplicatePublication);
        }
        let copy = entry
            .publication_copy
            .downcast_ref::<super::publication::PublicationCopy<T>>()
            .expect("registered component snapshot policy");
        let snapshot = copy.snapshot;
        let nested_bytes = copy.nested_bytes;
        entry.publication = Some(Box::new(move |capacity| {
            Box::new(super::publication::PublicationColumn::<T>::new(
                capacity,
                temporal,
                snapshot,
                nested_bytes,
            ))
        }));
        Ok(())
    }

    /// Stable schema identity for a registered process-local component key.
    pub fn schema(&self, key: ComponentKey) -> Option<ComponentSchemaId> {
        self.entries
            .iter()
            .find_map(|entry| (entry.key == key).then_some(entry.schema))
    }
}

/// Shared access identity. External IDs must name one owner-defined resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessKey {
    /// One W1 component column.
    Component(ComponentKey),
    /// A named external resource, distinct from any component.
    External(u64),
}

/// Declared system access; a write conflicts with any other access to its key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Read-only access.
    Read,
    /// Exclusive access.
    Write,
}

/// A system's declared component or external-resource access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Access {
    /// Shared identity of the accessed state.
    pub key: AccessKey,
    /// Read or exclusive write.
    pub mode: AccessMode,
}

/// Setup-time system declaration. No closures or backend storage escape here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemSpec {
    /// Unique identity in this schedule.
    pub id: SystemId,
    /// Mandatory authoritative stage.
    pub stage: Stage,
    /// Component/resource reads and writes.
    pub accesses: Vec<Access>,
    /// Explicit predecessors. They must not come from a later stage.
    pub after: Vec<SystemId>,
}

/// Graph construction error; a failed compile produces no executable schedule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    /// A system names a component not registered for this game.
    UnknownComponent,
    /// Registration identities must be unique.
    DuplicateSystem,
    /// Explicit predecessor is absent.
    UnknownDependency,
    /// An explicit edge contradicts input → simulate → resolve → publish.
    BackwardStage,
    /// Explicit edges or conflict ordering form a cycle.
    Cycle,
}

/// Validated deterministic schedule. Setup allocates O(systems²) edges and
/// retains conflict-free dependency batches; serial execution walks a flat
/// order without allocation. Same-stage access conflicts follow registration
/// order; explicit reverse edges therefore reject. Declarations are a
/// scheduler contract, not dynamic borrow enforcement.
#[derive(Clone, Debug)]
pub struct Schedule {
    systems: Vec<SystemSpec>,
    order: Vec<usize>,
    batches: Vec<Vec<SystemId>>,
    edges: Vec<(SystemId, SystemId)>,
}

impl Schedule {
    pub(super) fn permits(&self, system: SystemId, key: ComponentKey, mode: AccessMode) -> bool {
        self.systems.iter().any(|spec| {
            spec.id == system
                && spec.accesses.iter().any(|access| {
                    access.key == AccessKey::Component(key)
                        && (access.mode == AccessMode::Write || mode == AccessMode::Read)
                })
        })
    }

    pub(super) fn permits_external(
        &self,
        system: SystemId,
        resource: u64,
        mode: AccessMode,
    ) -> bool {
        self.systems.iter().any(|spec| {
            spec.id == system
                && spec.accesses.iter().any(|access| {
                    access.key == AccessKey::External(resource)
                        && (access.mode == AccessMode::Write || mode == AccessMode::Read)
                })
        })
    }

    /// Compile stage, explicit-dependency and conflicting-access edges once.
    ///
    /// # Errors
    ///
    /// A system ID repeats, a dependency is missing or backwards across stages, or the graph has a cycle.
    pub fn new(
        registry: &ComponentRegistry,
        systems: Vec<SystemSpec>,
    ) -> Result<Self, ScheduleError> {
        let n = systems.len();
        for (i, system) in systems.iter().enumerate() {
            if system.accesses.iter().any(|access| {
                matches!(access.key, AccessKey::Component(key) if registry.schema(key).is_none())
            }) {
                return Err(ScheduleError::UnknownComponent);
            }
            if systems[..i].iter().any(|prior| prior.id == system.id) {
                return Err(ScheduleError::DuplicateSystem);
            }
        }
        let mut matrix = vec![false; n.checked_mul(n).ok_or(ScheduleError::Cycle)?];
        for (i, a) in systems.iter().enumerate() {
            for (j, b) in systems.iter().enumerate() {
                if a.stage < b.stage || (i < j && a.stage == b.stage && conflicts(a, b)) {
                    matrix[i * n + j] = true;
                }
            }
            for dependency in &a.after {
                let j = systems
                    .iter()
                    .position(|s| s.id == *dependency)
                    .ok_or(ScheduleError::UnknownDependency)?;
                if systems[j].stage > a.stage {
                    return Err(ScheduleError::BackwardStage);
                }
                matrix[j * n + i] = true;
            }
        }
        let mut indegree = vec![0usize; n];
        let mut edges = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if matrix[i * n + j] {
                    indegree[j] += 1;
                    edges.push((systems[i].id, systems[j].id));
                }
            }
        }
        let mut order = Vec::with_capacity(n);
        let mut batches = Vec::new();
        let mut visited = vec![false; n];
        while order.len() < n {
            let ready: Vec<usize> = (0..n)
                .filter(|&i| !visited[i] && indegree[i] == 0)
                .collect();
            if ready.is_empty() {
                return Err(ScheduleError::Cycle);
            }
            batches.push(ready.iter().map(|&i| systems[i].id).collect());
            for &i in &ready {
                visited[i] = true;
                order.push(i);
            }
            for &i in &ready {
                for j in 0..n {
                    if matrix[i * n + j] {
                        indegree[j] -= 1;
                    }
                }
            }
        }
        Ok(Self {
            systems,
            order,
            batches,
            edges,
        })
    }

    /// Compiled execution order, stable for the same declarations.
    pub fn order(&self) -> impl ExactSizeIterator<Item = &SystemSpec> {
        self.order.iter().map(|&i| &self.systems[i])
    }

    /// Deterministic dependency waves. IDs within a wave have no declared
    /// conflicts or dependency edge, but A1 executes them serially.
    pub fn batches(&self) -> impl ExactSizeIterator<Item = &[SystemId]> {
        self.batches.iter().map(Vec::as_slice)
    }

    /// Retained dependency edges for graph inspection and a future executor.
    pub fn edges(&self) -> &[(SystemId, SystemId)] {
        &self.edges
    }

    /// Run the same graph serially. Failure prevents all later systems, including
    /// publication. The runner owns staged writes and its recovery/fault policy.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by the caller-supplied system callback.
    pub fn run<E>(&self, mut run: impl FnMut(&SystemSpec) -> Result<(), E>) -> Result<(), E> {
        self.order().try_for_each(&mut run)
    }
}

fn conflicts(a: &SystemSpec, b: &SystemSpec) -> bool {
    a.accesses.iter().any(|a| {
        b.accesses
            .iter()
            .any(|b| a.key == b.key && (a.mode == AccessMode::Write || b.mode == AccessMode::Write))
    })
}
use std::{
    any::{Any, TypeId},
    mem::size_of,
};

use super::{
    column_store::{Pending, PendingValue, StorageColumn, TypedStore},
    dense_directory::DenseDirectory,
    registered_world::RegisteredWorld,
};

pub(super) trait SnapshotColumn: Send + Sync {
    fn restore_into(&self, column: &mut dyn StorageColumn, directory: Option<&DenseDirectory>);
    fn clone_box(&self) -> Box<dyn SnapshotColumn>;
    fn copy_from(&mut self, source: &dyn SnapshotColumn);
    fn clear(&mut self);
    fn capture_slot(&mut self, world: &RegisteredWorld, slot: u32);
    fn overlay(&mut self, delta: &dyn SnapshotColumn, changed: &[(u32, u64)]);
    fn as_any(&self) -> &dyn Any;
    fn reserved_bytes(&self) -> usize;
}

type CaptureColumn = Box<dyn Fn(&RegisteredWorld) -> Box<dyn SnapshotColumn> + Send + Sync>;
type CreateStorage = Box<dyn Fn(usize) -> Box<dyn StorageColumn> + Send + Sync>;

struct TypedColumn<T> {
    rows: Vec<(u32, T)>,
    snapshot: fn(&T) -> T,
    nested_bytes: fn(&T) -> usize,
}

impl<T: Send + Sync + 'static> SnapshotColumn for TypedColumn<T> {
    fn restore_into(&self, column: &mut dyn StorageColumn, directory: Option<&DenseDirectory>) {
        let typed = column
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered capture column type");
        for (slot, value) in &self.rows {
            let row =
                directory.map_or(*slot as usize, |directory| directory.row_of(*slot as usize));
            assert!(
                typed.insert(row, (self.snapshot)(value)).is_none(),
                "one captured value per slot"
            );
        }
    }

    fn clone_box(&self) -> Box<dyn SnapshotColumn> {
        Box::new(Self {
            rows: self
                .rows
                .iter()
                .map(|(slot, value)| (*slot, (self.snapshot)(value)))
                .collect(),
            snapshot: self.snapshot,
            nested_bytes: self.nested_bytes,
        })
    }

    fn copy_from(&mut self, source: &dyn SnapshotColumn) {
        let source = source
            .as_any()
            .downcast_ref::<Self>()
            .expect("same registered snapshot schema");
        self.rows.clear();
        self.rows.extend(
            source
                .rows
                .iter()
                .map(|(slot, value)| (*slot, (self.snapshot)(value))),
        );
    }

    fn clear(&mut self) {
        self.rows.clear();
    }

    fn capture_slot(&mut self, world: &RegisteredWorld, slot: u32) {
        let handle = world.handle_at(slot);
        if let Ok(value) = world.with::<T, _>(handle, |value| (self.snapshot)(value)) {
            self.rows.push((slot, value));
        }
    }

    fn overlay(&mut self, delta: &dyn SnapshotColumn, changed: &[(u32, u64)]) {
        let delta = delta
            .as_any()
            .downcast_ref::<Self>()
            .expect("same registered schema");
        for &(slot, _) in changed {
            let current = self.rows.binary_search_by_key(&slot, |row| row.0);
            let next = delta.rows.binary_search_by_key(&slot, |row| row.0);
            match (current, next) {
                (Ok(at), Ok(from)) => self.rows[at].1 = (self.snapshot)(&delta.rows[from].1),
                (Err(at), Ok(from)) => self
                    .rows
                    .insert(at, (slot, (self.snapshot)(&delta.rows[from].1))),
                (Ok(at), Err(_)) => {
                    self.rows.remove(at);
                }
                (Err(_), Err(_)) => {}
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.rows.capacity() * size_of::<(u32, T)>()
            + self
                .rows
                .iter()
                .map(|(_, value)| (self.nested_bytes)(value))
                .sum::<usize>()
    }
}

pub(super) struct ComponentEntry {
    pub(super) key: ComponentKey,
    pub(super) schema: ComponentSchemaId,
    indexed: bool,
    publication_copy: Box<dyn Any + Send + Sync>,
    pub(super) publication: Option<super::publication::PublicationFactory>,
    pub(super) capture: CaptureColumn,
    pub(super) storage: CreateStorage,
    pub(super) pending: Box<dyn Fn() -> Box<dyn PendingValue> + Send + Sync>,
    pub(super) undo: Box<dyn Fn() -> Box<dyn super::undo::UndoColumn> + Send + Sync>,
    pub(super) empty: Box<dyn Fn() -> Box<dyn SnapshotColumn> + Send + Sync>,
}

fn capture_component<T: Send + Sync + 'static>(
    world: &RegisteredWorld,
    snapshot: fn(&T) -> T,
    nested_bytes: fn(&T) -> usize,
) -> Box<dyn SnapshotColumn> {
    let mut rows = world
        .column::<T>()
        .values()
        .iter()
        .enumerate()
        .filter_map(|(row, value)| {
            value.as_ref().map(|value| {
                (
                    u32::try_from(world.slot_at_row(row)).expect("validated slot capacity"),
                    snapshot(value),
                )
            })
        })
        .collect::<Vec<_>>();
    rows.sort_unstable_by_key(|(slot, _)| *slot);
    Box::new(TypedColumn {
        rows,
        snapshot,
        nested_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ComponentRegistry, ComponentSchemaId, RegistrationError, SnapshotColumn, TypedColumn,
        TypedStore,
    };

    #[test]
    fn indexed_registration_selects_its_factory_and_rejection_preserves_it() {
        let mut registry = ComponentRegistry::default();
        registry
            .register_indexed::<u32>(ComponentSchemaId(1), |value| *value > 0)
            .expect("indexed type");
        assert_eq!(
            registry.register_indexed::<u32>(ComponentSchemaId(2), |_| true),
            Err(RegistrationError::DuplicateType)
        );
        assert_eq!(
            registry.register_indexed::<u64>(ComponentSchemaId(1), |_| true),
            Err(RegistrationError::DuplicateSchema)
        );
        registry
            .register::<u64>(ComponentSchemaId(2))
            .expect("ordinary type after refused schema");
        let mut indexed = (registry.entries[0].storage)(65);
        let typed = indexed
            .as_any_mut()
            .downcast_mut::<TypedStore<u32>>()
            .expect("registered type");
        typed.insert(0, 0);
        typed.insert(64, 1);
        assert_eq!(indexed.active_presence(), Some(&[0, 1][..]));
        let ordinary = (registry.entries[1].storage)(65);
        assert_eq!(ordinary.active_presence(), None);
    }

    #[test]
    fn noncopy_column_delta_overlays_values_membership_and_independent_copy() {
        let mut baseline = TypedColumn {
            rows: vec![(0, vec![1_u8, 2]), (2, vec![3_u8])],
            snapshot: Vec::clone,
            nested_bytes: Vec::capacity,
        };
        let retained = baseline.clone_box();
        let delta = TypedColumn {
            rows: vec![(0, vec![9_u8]), (1, vec![7_u8, 8])],
            snapshot: Vec::clone,
            nested_bytes: Vec::capacity,
        };
        baseline.overlay(&delta, &[(0, 1), (1, 1), (2, 1)]);
        assert_eq!(baseline.rows, [(0, vec![9]), (1, vec![7, 8])]);
        let original = retained
            .as_any()
            .downcast_ref::<TypedColumn<Vec<u8>>>()
            .expect("independent typed column");
        assert_eq!(original.rows, [(0, vec![1, 2]), (2, vec![3])]);
        assert!(baseline.reserved_bytes() >= 3);
    }
}
