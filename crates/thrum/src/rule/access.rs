use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
};

use thrum_core::sim::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId, GameRuntime,
    ReadPermit, RuleColumn, RuleStorage, SystemId, TickGame, WorldTxn, WritePermit,
};

use crate::{Data, Error, Result, data::Stored};

type ErasedPermit = Box<dyn Any + Send + Sync>;

#[derive(Clone, Copy)]
pub(crate) struct Column {
    pub(crate) key: ComponentKey,
    pub(crate) type_id: TypeId,
    pub(crate) name: &'static str,
    schema: fn() -> ComponentSchemaId,
    register: fn(&mut ComponentRegistry, ComponentSchemaId) -> Result,
    bind: fn(&GameRuntime<crate::runtime::Authority>, SystemId, AccessMode) -> Binding,
}

impl Column {
    pub(crate) fn of<T: Data>() -> Self {
        Self {
            key: ComponentKey::of::<Stored<T>>(),
            type_id: TypeId::of::<T>(),
            name: type_name::<T>(),
            schema: || {
                ComponentSchemaId(crate::data::schema_hash(
                    T::SCHEMA_ID,
                    type_name::<T>().as_bytes(),
                ))
            },
            register: |registry, schema| {
                registry
                    .register_cloned_profile::<Stored<T>>(schema, |_| 0)
                    .map(|_| ())
                    .map_err(|error| {
                        Error::refused(
                            "T0200",
                            type_name::<T>(),
                            error.to_string(),
                            "Give each world type a distinct name and schema.",
                        )
                    })
            },
            bind: bind::<T, crate::runtime::Authority>,
        }
    }

    pub(crate) fn register(self, registry: &mut ComponentRegistry) -> Result {
        if registry.schema(self.key).is_none() {
            (self.register)(registry, (self.schema)())?;
        }
        Ok(())
    }
}

pub(crate) struct Binding {
    read: ErasedPermit,
    write: Option<ErasedPermit>,
    loan: fn(&mut WorldTxn<'_>, &ErasedPermit) -> RuleStorage,
}

fn bind<T: Data, G: TickGame>(runtime: &GameRuntime<G>, id: SystemId, mode: AccessMode) -> Binding {
    Binding {
        loan: |world, permit| {
            world
                .rule_storage(
                    permit
                        .downcast_ref::<ReadPermit<Stored<T>>>()
                        .expect("typed loan binding"),
                )
                .expect("declared component loan")
        },
        read: Box::new(runtime.read_permit::<Stored<T>>(id).expect("declared read")),
        write: (mode == AccessMode::Write).then(|| -> ErasedPermit {
            Box::new(
                runtime
                    .write_permit::<Stored<T>>(id)
                    .expect("declared write"),
            )
        }),
    }
}

#[derive(Default)]
pub(crate) struct Bindings(HashMap<TypeId, Binding>);

impl Bindings {
    pub(crate) fn new(
        runtime: &GameRuntime<crate::runtime::Authority>,
        id: SystemId,
        columns: &[(Column, AccessMode)],
    ) -> Self {
        Self(
            columns
                .iter()
                .map(|&(column, mode)| (column.type_id, (column.bind)(runtime, id, mode)))
                .collect(),
        )
    }

    pub(crate) fn prepare_loans(
        &self,
        world: &mut WorldTxn<'_>,
        columns: &[TypeId],
        loans: &mut Loans,
    ) {
        loans.0.extend(columns.iter().map(|id| {
            let binding = &self.0[id];
            (*id, (binding.loan)(world, &binding.read))
        }));
    }

    pub(crate) fn read<T: Data>(&self) -> &ReadPermit<Stored<T>> {
        self.0[&TypeId::of::<T>()]
            .read
            .downcast_ref()
            .expect("typed read binding")
    }

    pub(crate) fn write<T: Data>(&self) -> &WritePermit<Stored<T>> {
        self.0[&TypeId::of::<T>()]
            .write
            .as_ref()
            .expect("declared mutable binding")
            .downcast_ref()
            .expect("typed write binding")
    }
}

#[derive(Default)]
pub(crate) struct Loans(HashMap<TypeId, RuleStorage>);

impl Loans {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn column<T: Data>(&self) -> RuleColumn<Stored<T>> {
        self.0[&TypeId::of::<T>()].typed()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AccessColumn {
    pub(crate) column: Column,
    pub(crate) write: bool,
    pub(crate) required: bool,
}

#[derive(Clone, Default)]
pub(crate) struct Projection {
    pub(crate) access: Vec<AccessColumn>,
    pub(crate) present: Vec<Column>,
    pub(crate) absent: Vec<Column>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Resource,
    Writer,
}

#[derive(Clone, Copy)]
pub(crate) struct Exclusive {
    pub(crate) kind: Kind,
    pub(crate) id: TypeId,
    pub(crate) name: &'static str,
    pub(crate) write: bool,
    pub(crate) column: Option<Column>,
}

#[derive(Clone)]
pub(crate) enum Parameter {
    Query(String, Projection),
    Exclusive(String, Exclusive),
}

impl Parameter {
    fn name(&self) -> &str {
        match self {
            Self::Query(name, _) | Self::Exclusive(name, _) => name,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Signature(pub(crate) Vec<Parameter>);

impl Signature {
    pub(crate) fn loan_types(&self) -> Vec<TypeId> {
        let mut types = Vec::new();
        for parameter in &self.0 {
            if let Parameter::Query(_, projection) = parameter {
                for item in &projection.access {
                    if !types.contains(&item.column.type_id) {
                        types.push(item.column.type_id);
                    }
                }
            }
        }
        types
    }

    pub(crate) fn needs_entities(&self) -> bool {
        self.0
            .iter()
            .any(|parameter| matches!(parameter, Parameter::Query(..)))
            || crate::entity::contains_entities(self)
    }

    pub(crate) fn checks_entity_overlap(&self) -> bool {
        let mut queries = self.0.iter().filter_map(|parameter| {
            if let Parameter::Query(_, projection) = parameter {
                Some(projection)
            } else {
                None
            }
        });
        while let Some(left) = queries.next() {
            if queries
                .clone()
                .any(|right| overlapping(&left.access, &right.access, false, |_| true).is_some())
            {
                return true;
            }
        }
        false
    }

    pub(crate) fn validate(
        &self,
        rule: &str,
        resources: &[TypeId],
        scene_value: Option<TypeId>,
    ) -> Result {
        for (index, param) in self.0.iter().enumerate() {
            match param {
                Parameter::Query(name, projection) => {
                    if let Some(column) =
                        overlapping(&projection.access, &projection.access, true, |_| true)
                    {
                        return Err(Error::refused(
                            "T0202",
                            format!("{rule}, {name}"),
                            format!("{} is accessed twice and one access writes", column.name),
                            "Drop the duplicate component access.",
                        ));
                    }
                }
                Parameter::Exclusive(name, access) => {
                    if access.kind == Kind::Resource
                        && !(resources.contains(&access.id)
                            || !access.write && scene_value == Some(access.id))
                    {
                        return Err(Error::refused(
                            "T0201",
                            format!("{rule}, {name}"),
                            format!("resource {} has no provider", access.name),
                            "Register it with scene.resource(..) or Game::resource(..).",
                        ));
                    }
                    for other in &self.0[..index] {
                        if let Parameter::Exclusive(_, earlier) = other
                            && earlier.kind == access.kind
                            && earlier.id == access.id
                            && (earlier.write || access.write)
                        {
                            return Err(Error::refused(
                                "T0203",
                                format!("{rule}, {} and {name}", other.name()),
                                format!("both parameters reach {} and one writes", access.name),
                                "Drop one access or use one parameter for both operations.",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn columns(&self) -> Vec<(Column, AccessMode)> {
        let mut columns: Vec<(Column, AccessMode)> = Vec::new();
        let mut insert = |column: Column, write: bool| {
            let mode = if write {
                AccessMode::Write
            } else {
                AccessMode::Read
            };
            if let Some((_, current)) = columns
                .iter_mut()
                .find(|(existing, _)| existing.key == column.key)
            {
                if write {
                    *current = mode;
                }
            } else {
                columns.push((column, mode));
            }
        };
        for parameter in &self.0 {
            match parameter {
                Parameter::Exclusive(_, access) => {
                    if let Some(column) = access.column {
                        insert(column, access.write);
                    }
                }
                Parameter::Query(_, data) => {
                    for item in &data.access {
                        insert(item.column, item.write);
                    }
                    for &column in data.present.iter().chain(&data.absent) {
                        insert(column, false);
                    }
                }
            }
        }
        columns
    }

    pub(crate) fn validate_entity(
        &self,
        rule: &str,
        entity: crate::Entity,
        present: impl Fn(ComponentKey) -> bool,
    ) -> Result {
        let matches = |data: &Projection| {
            data.access
                .iter()
                .all(|item| !item.required || present(item.column.key))
                && data.present.iter().all(|column| present(column.key))
                && data.absent.iter().all(|column| !present(column.key))
        };
        for (index, left) in self.0.iter().enumerate() {
            let Parameter::Query(left_name, left) = left else {
                continue;
            };
            for right in &self.0[..index] {
                let Parameter::Query(right_name, right) = right else {
                    continue;
                };
                if let Some(column) = overlapping(&left.access, &right.access, false, |column| {
                    present(column.key)
                }) && matches(left)
                    && matches(right)
                {
                    return Err(Error::refused(
                        "T0303",
                        format!("{rule}, {right_name} and {left_name}"),
                        format!("both reach {} on {entity:?} and one writes", column.name),
                        "Change the query projections or match disjoint sets of entities.",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn accesses(columns: &[(Column, AccessMode)]) -> Vec<Access> {
        columns
            .iter()
            .map(|&(column, mode)| Access {
                key: AccessKey::Component(column.key),
                mode,
            })
            .collect()
    }
}

fn overlapping(
    left: &[AccessColumn],
    right: &[AccessColumn],
    same: bool,
    matches: impl Fn(Column) -> bool,
) -> Option<Column> {
    for (index, left) in left.iter().enumerate() {
        let right = if same { &right[..index] } else { right };
        for right in right {
            if left.column.key == right.column.key
                && (left.write || right.write)
                && matches(left.column)
            {
                return Some(left.column);
            }
        }
    }
    None
}
