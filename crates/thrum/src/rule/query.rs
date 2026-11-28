use std::{marker::PhantomData, slice};

use thrum_core::{
    Handle,
    profile::StateEncode,
    sim::{ColumnRead, Executor, RuleColumn, WorldRead},
};

use crate::{Component, Entity, Error, Result, data::Stored};

use super::{
    access::{AccessColumn, Bindings, Column, Loans, Parameter, Projection},
    params::{Context, Param, ParameterList, ReadContext, ReadOnly},
};

/// Select entities that carry a component.
/// ```
/// use thrum::{Component, Query, With};
/// #[derive(Component)] struct Hero;
/// fn one(query: Query<(), With<Hero>>) -> bool { query.count() == 1 }
/// ```
pub struct With<T: Component>(PhantomData<T>);

/// Select entities that lack a component.
/// ```
/// use thrum::{Component, Query, Without};
/// #[derive(Component)] struct Exhausted;
/// fn can_run(query: Query<(), Without<Exhausted>>) -> bool { !query.is_empty() }
/// ```
pub struct Without<T: Component>(PhantomData<T>);

/// The rows that match one rule parameter, in spawn order.
/// ```
/// use thrum::{Component, Query};
/// #[derive(Component)] struct Health(u32);
/// fn heal(mut health: Query<&mut Health>) {
///     for health in &mut health { health.0 += 1; }
/// }
/// ```
pub struct Query<'a, D: QueryData, F: Filter = ()> {
    rows: &'a mut [Row<D::Owned>],
    columns: D::Columns,
    executor: Executor,
    filter: PhantomData<F>,
}

impl<D: QueryData, F: Filter> Query<'_, D, F> {
    /// Iterate through shared views of the matching rows.
    /// ```
    /// use thrum::{Component, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn total(health: Query<&Health>) -> u32 { health.iter().map(|health| health.0).sum() }
    /// ```
    pub fn iter(&self) -> impl ExactSizeIterator<Item = D::Read<'_>> {
        self.rows
            .iter()
            .map(|row| D::read(&self.columns, row.entity, row.physical, &row.data))
    }

    /// Iterate through the matching rows with the access declared by this query.
    /// ```
    /// use thrum::{Component, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn heal(mut health: Query<&mut Health>) {
    ///     health.iter_mut().for_each(|health| health.0 += 1);
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, D> {
        IterMut(self.rows.iter_mut(), &self.columns, PhantomData)
    }

    /// Read the matching item for this entity.
    /// ```
    /// use thrum::{Component, Entity, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn living(query: Query<&Health>, entity: Entity) -> bool {
    ///     query.get(entity).is_some_and(|health| health.0 > 0)
    /// }
    /// ```
    pub fn get(&self, entity: impl Into<Option<Entity>>) -> Option<D::Read<'_>> {
        let index = self.index(entity.into()?)?;
        let row = &self.rows[index];
        Some(D::read(&self.columns, row.entity, row.physical, &row.data))
    }

    /// Access the matching item for this entity.
    /// ```
    /// use thrum::{Component, Entity, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn heal(mut query: Query<&mut Health>, entity: Entity) {
    ///     if let Some(health) = query.get_mut(entity) { health.0 += 1; }
    /// }
    /// ```
    pub fn get_mut(&mut self, entity: impl Into<Option<Entity>>) -> Option<D::Item<'_>> {
        let index = self.index(entity.into()?)?;
        let row = &mut self.rows[index];
        Some(D::item(
            &self.columns,
            row.entity,
            row.physical,
            &mut row.data,
        ))
    }

    /// Access two distinct matching rows at once, in argument order.
    /// ```
    /// use thrum::{Component, Entity, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn copy(mut query: Query<&mut Health>, from: Entity, to: Entity) {
    ///     if let Some((from, to)) = query.pair_mut(from, to) { to.0 = from.0; }
    /// }
    /// ```
    pub fn pair_mut(&mut self, a: Entity, b: Entity) -> Option<(D::Item<'_>, D::Item<'_>)> {
        let [a, b] = [self.index(a)?, self.index(b)?];
        let [a, b] = self.rows.get_disjoint_mut([a, b]).ok()?;
        Some((
            D::item(&self.columns, a.entity, a.physical, &mut a.data),
            D::item(&self.columns, b.entity, b.physical, &mut b.data),
        ))
    }

    /// The number of matching entities.
    /// ```
    /// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
    /// ```
    pub fn count(&self) -> u32 {
        u32::try_from(self.rows.len()).unwrap_or(u32::MAX)
    }

    /// Whether there are zero matching entities.
    /// ```
    /// fn empty(query: thrum::Query<thrum::Entity>) -> bool { query.is_empty() }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Run independent work over the matching rows using the engine executor.
    /// ```
    /// use thrum::{Component, Query};
    /// #[derive(Component)] struct Health(u32);
    /// fn heal(mut query: Query<&mut Health>) { query.par_each(|health| health.0 += 1); }
    /// ```
    pub fn par_each(&mut self, run: impl Fn(D::Item<'_>) + Send + Sync) {
        let columns = &self.columns;
        self.executor.for_each_query_row(self.rows, |row| {
            run(D::item(columns, row.entity, row.physical, &mut row.data));
        });
    }

    fn index(&self, entity: Entity) -> Option<usize> {
        self.rows
            .binary_search_by_key(&entity, |row| row.entity)
            .ok()
    }
}

/// Mutable traversal of a query's matching rows.
/// ```
/// fn visit(mut query: thrum::Query<thrum::Entity>) {
///     for entity in &mut query { let _ = entity; }
/// }
/// ```
pub struct IterMut<'a, D: QueryData>(
    slice::IterMut<'a, Row<D::Owned>>,
    &'a D::Columns,
    PhantomData<D>,
);

impl<'a, D: QueryData> Iterator for IterMut<'a, D> {
    type Item = D::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|row| D::item(self.1, row.entity, row.physical, &mut row.data))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<D: QueryData> ExactSizeIterator for IterMut<'_, D> {}

impl<'b, D: QueryData, F: Filter> IntoIterator for &'b mut Query<'_, D, F> {
    type Item = D::Item<'b>;
    type IntoIter = IterMut<'b, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Shared traversal of a query's matching rows.
/// ```
/// fn visit(query: thrum::Query<thrum::Entity>) {
///     for entity in &query { let _ = entity; }
/// }
/// ```
pub struct Iter<'a, D: QueryData>(
    slice::Iter<'a, Row<D::Owned>>,
    &'a D::Columns,
    PhantomData<D>,
);

impl<'a, D: QueryData> Iterator for Iter<'a, D> {
    type Item = D::Read<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|row| D::read(self.1, row.entity, row.physical, &row.data))
    }
}

impl<'b, D: QueryData, F: Filter> IntoIterator for &'b Query<'_, D, F> {
    type Item = D::Read<'b>;
    type IntoIter = Iter<'b, D>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.rows.iter(), &self.columns, PhantomData)
    }
}

/// Exactly one query match, checked before the rule is called.
/// ```
/// use thrum::{Component, Single};
/// #[derive(Component)] struct Hero(u32);
/// fn hero_health(hero: Single<&Hero>) -> u32 { hero.0 }
/// ```
pub struct Single<'a, D: QueryData, F: Filter = ()> {
    item: D::Item<'a>,
    filter: PhantomData<F>,
}

impl<'a, D: QueryData, F: Filter> Single<'a, D, F> {
    /// Take the projection out of the single match.
    /// ```
    /// use thrum::{Component, Entity, Single};
    /// #[derive(Component)] struct Hero;
    /// fn find(hero: Single<(Entity, &Hero)>) -> Entity { hero.into_inner().0 }
    /// ```
    pub fn into_inner(self) -> D::Item<'a> {
        self.item
    }
}

impl<F: Filter> std::ops::Deref for Single<'_, Entity, F> {
    type Target = Entity;
    fn deref(&self) -> &Entity {
        &self.item
    }
}

impl<T: Component, F: Filter> std::ops::Deref for Single<'_, &T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        self.item
    }
}

impl<T: Component, F: Filter> std::ops::Deref for Single<'_, &mut T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        self.item
    }
}

impl<T: Component, F: Filter> std::ops::DerefMut for Single<'_, &mut T, F> {
    fn deref_mut(&mut self) -> &mut T {
        self.item
    }
}

/// Data projected for one matching query row.
/// ```
/// use thrum::{Query, QueryData};
/// fn count<D: QueryData>(query: Query<'_, D>) -> u32 { query.count() }
/// ```
#[expect(private_bounds, reason = "only the SDK can define a query projection")]
pub trait QueryData: sealed::Data<Owned = Self::RowData> {
    /// Reusable data held for one matching row.
    /// ```
    /// use thrum::{Entity, QueryData};
    /// let _: <Entity as QueryData>::RowData = Default::default();
    /// ```
    type RowData: Default + Send + 'static;
    /// A shared view of one row.
    /// ```
    /// use thrum::{Entity, Query, QueryData};
    /// fn first<'a>(query: &'a Query<'_, Entity>) -> Option<<Entity as QueryData>::Read<'a>> {
    ///     query.iter().next()
    /// }
    /// ```
    type Read<'a>;
    /// A view with the access declared by the query.
    /// ```
    /// use thrum::{Component, Query, QueryData};
    /// #[derive(Component)] struct Health(u32);
    /// fn heal(mut query: Query<'_, &mut Health>) {
    ///     for health in &mut query {
    ///         let health: <&mut Health as QueryData>::Item<'_> = health;
    ///         health.0 += 1;
    ///     }
    /// }
    /// ```
    type Item<'a>;
    /// The same projection with references tied to the current rule.
    /// ```
    /// use thrum::{Component, Query, QueryData};
    /// #[derive(Component)] struct Health(u32);
    /// fn count<'a>(query: Query<'a, <&'static Health as QueryData>::For<'a>>) -> u32 {
    ///     query.count()
    /// }
    /// ```
    type For<'a>: QueryData<RowData = Self::RowData>;
}

/// A component presence filter.
/// ```
/// use thrum::{Component, Entity, Filter, Query, With};
/// #[derive(Component)] struct Player;
/// fn count<F: Filter>(query: Query<'_, Entity, F>) -> u32 { query.count() }
/// fn players(query: Query<'_, Entity, With<Player>>) -> u32 { count(query) }
/// ```
#[expect(private_bounds, reason = "only the SDK can define a query filter")]
pub trait Filter: sealed::Filter {}

mod sealed {
    use super::{Bindings, Context, Entity, Loans, Projection, QueryData, Result, Row, WorldRead};

    pub(crate) trait Data: Sized {
        type Owned: Default + Send + 'static;
        type Presence<'a>;
        type Columns: Sync;

        fn describe(projection: &mut Projection);
        fn presence<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Presence<'a>;
        fn matches(presence: &Self::Presence<'_>, slot: u32) -> bool;
        fn bind(loans: &Loans) -> Self::Columns;
        fn read<'a>(
            columns: &Self::Columns,
            entity: Entity,
            physical: usize,
            data: &'a Self::Owned,
        ) -> <Self as QueryData>::Read<'a>
        where
            Self: QueryData;
        fn item<'a>(
            columns: &Self::Columns,
            entity: Entity,
            physical: usize,
            data: &'a mut Self::Owned,
        ) -> <Self as QueryData>::Item<'a>
        where
            Self: QueryData;
        fn journal<R>(
            _: &mut [Row<R>],
            _: impl Fn(&mut R) -> &mut Self::Owned,
            _: &mut Context<'_, '_>,
        ) -> Result {
            Ok(())
        }
    }

    pub(crate) trait Filter {
        type Columns<'a>;
        fn describe(projection: &mut Projection);
        fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a>;
        fn matches(columns: &Self::Columns<'_>, slot: u32) -> bool;
    }
}

impl<T: Component> Filter for With<T> {}
impl<T: Component> Filter for Without<T> {}
impl Filter for () {}

macro_rules! presence_filter {
    ($filter:ident, $field:ident, $expected:literal) => {
        impl<T: Component> sealed::Filter for $filter<T> {
            type Columns<'a> = ColumnRead<'a, Stored<T>>;
            fn describe(projection: &mut Projection) {
                projection.$field.push(Column::of::<T>());
            }
            fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a> {
                world
                    .column(bindings.read::<T>())
                    .expect("bound filter column")
            }
            fn matches(column: &Self::Columns<'_>, slot: u32) -> bool {
                column.at_slot(slot).is_some() == $expected
            }
        }
    };
}
presence_filter!(With, present, true);
presence_filter!(Without, absent, false);

impl sealed::Filter for () {
    type Columns<'a> = ();
    fn describe(_: &mut Projection) {}
    fn bind<'a>(_: &'a WorldRead<'_>, _: &'a Bindings) {}
    fn matches((): &(), _: u32) -> bool {
        true
    }
}

// These adapters are private to monomorphised QueryData. Rows are prepared
// against the live component set, with no conflicting access across parameters.
// Query iteration lends at most one write to a row, and the rule owns the loan
// scope until it has journaled all touched rows.
#[expect(
    unsafe_code,
    reason = "mutate a row admitted exclusively by the scoped query projection"
)]
fn write_component<'a, T: Component>(
    column: &RuleColumn<Stored<T>>,
    row: usize,
    prior: &'a mut Option<T>,
) -> Option<&'a mut T> {
    // SAFETY: The row belongs exclusively to this projection. The preimage is
    // journaled after every invocation, including a panicking rule.
    unsafe { column.write(row, prior, |stored| stored.0) }.map(|stored| &mut stored.0)
}

#[expect(
    unsafe_code,
    reason = "journal preimages captured from valid, now-released query loans"
)]
fn journal_component<T: Component, R>(
    rows: &mut [Row<R>],
    project: impl Fn(&mut R) -> &mut Option<T>,
    context: &mut Context<'_, '_>,
) -> Result {
    let preimages = rows.iter_mut().filter_map(|row| {
        project(&mut row.data)
            .take()
            .map(|prior| (row.handle, Stored(prior)))
    });
    // SAFETY: The callback has released its query loans, and structure cannot
    // change until after this journal is submitted.
    unsafe {
        context.world.journal_rule_column(
            context.bindings.write::<T>(),
            preimages,
            StateEncode::is_finite,
        )
    }
    .map_err(|error| {
        Error::refused(
            "T0308",
            std::any::type_name::<T>(),
            error.to_string(),
            "Correct the rule's component value.",
        )
    })
}

macro_rules! component_data {
    ([$($projection:tt)*]; $write:literal; $required:literal; $owned:ty;
        $read:ty; $item:ty; $read_expr:expr; $item_expr:expr; $($journal:item)?) => {
        impl<T: Component> QueryData for $($projection)* {
            type RowData = $owned;
            type Read<'a> = $read;
            type Item<'a> = $item;
            type For<'a> = $item;
        }
        impl<T: Component> sealed::Data for $($projection)* {
            type Owned = $owned;
            type Presence<'a> = ColumnRead<'a, Stored<T>>;
            type Columns = RuleColumn<Stored<T>>;

            fn describe(projection: &mut Projection) {
                projection.access.push(AccessColumn { column: Column::of::<T>(), write: $write, required: $required });
            }
            fn presence<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Presence<'a> {
                world.column(bindings.read::<T>()).expect("bound query column")
            }
            fn matches(column: &Self::Presence<'_>, slot: u32) -> bool {
                !$required || column.at_slot(slot).is_some()
            }
            fn bind(loans: &Loans) -> Self::Columns {
                loans.column::<T>()
            }
            fn read<'a>(column: &Self::Columns, _: Entity, row: usize, _: &'a Self::Owned) -> <Self as QueryData>::Read<'a> {
                $read_expr(column, row)
            }
            fn item<'a>(column: &Self::Columns, _: Entity, row: usize, prior: &'a mut Self::Owned) -> <Self as QueryData>::Item<'a> {
                $item_expr(column, row, prior)
            }
            $($journal)?
        }
    };
}

component_data!([&T]; false; true; (); &'a T; &'a T;
    read_required; |column, row, &mut ()| read_required(column, row); );
component_data!([Option<&T>]; false; false; (); Option<&'a T>; Option<&'a T>;
    read_optional; |column, row, &mut ()| read_optional(column, row); );
component_data!([&mut T]; true; true; Option<T>; &'a T; &'a mut T;
    read_required; |column, row, prior| write_component(column, row, prior).expect("matched component");
    fn journal<R>(rows: &mut [Row<R>], project: impl Fn(&mut R) -> &mut Self::Owned, context: &mut Context<'_, '_>) -> Result {
        journal_component(rows, project, context)
    }
);
component_data!([Option<&mut T>]; true; false; Option<T>; Option<&'a T>; Option<&'a mut T>;
    read_optional; write_component;
    fn journal<R>(rows: &mut [Row<R>], project: impl Fn(&mut R) -> &mut Self::Owned, context: &mut Context<'_, '_>) -> Result {
        journal_component(rows, project, context)
    }
);

#[expect(
    unsafe_code,
    reason = "read stays inside the rule's admitted projection"
)]
fn read_optional<'a, T: Component>(column: &RuleColumn<Stored<T>>, row: usize) -> Option<&'a T> {
    // SAFETY: Only the rule's prepared QueryData can use this helper, and only
    // for a row admitted for this projection and the lifetime of that query.
    let result = unsafe { column.read(row) };
    result.map(|stored| &stored.0)
}

fn read_required<'a, T: Component>(column: &RuleColumn<Stored<T>>, row: usize) -> &'a T {
    read_optional(column, row).expect("matched component")
}

macro_rules! row_value {
    ($ty:ty, $value:expr) => {
        impl QueryData for $ty {
            type RowData = ();
            type Read<'a> = Self;
            type Item<'a> = Self;
            type For<'a> = Self;
        }

        impl sealed::Data for $ty {
            type Owned = ();
            type Presence<'a> = ();
            type Columns = ();
            fn describe(_: &mut Projection) {}
            fn presence<'a>(_: &'a WorldRead<'_>, _: &'a Bindings) {}
            fn matches((): &(), _: u32) -> bool {
                true
            }
            fn bind(_: &Loans) {}
            fn read((): &(), entity: Entity, _: usize, (): &()) -> Self {
                $value(entity)
            }
            fn item((): &(), entity: Entity, _: usize, (): &mut ()) -> Self {
                $value(entity)
            }
        }
    };
}

row_value!(Entity, |entity| entity);
row_value!((), |_| ());

macro_rules! tuples {
    ($(($($ty:ident $index:tt),+)),+ $(,)?) => {$(
        impl<$($ty: QueryData),+> QueryData for ($($ty,)+) {
            type RowData = ($($ty::RowData,)+);
            type Read<'a> = ($($ty::Read<'a>,)+);
            type Item<'a> = ($($ty::Item<'a>,)+);
            type For<'a> = ($($ty::For<'a>,)+);
        }
        impl<$($ty: QueryData),+> sealed::Data for ($($ty,)+) {
            type Owned = ($($ty::Owned,)+);
            type Presence<'a> = ($($ty::Presence<'a>,)+);
            type Columns = ($($ty::Columns,)+);
            fn describe(projection: &mut Projection) { $($ty::describe(projection);)+ }
            fn presence<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Presence<'a> {
                ($($ty::presence(world, bindings),)+)
            }
            fn matches(columns: &Self::Presence<'_>, slot: u32) -> bool {
                true $(&& $ty::matches(&columns.$index, slot))+
            }
            fn bind(loans: &Loans) -> Self::Columns { ($($ty::bind(loans),)+) }
            fn read<'a>(columns: &Self::Columns, entity: Entity, physical: usize, data: &'a Self::Owned) -> <Self as QueryData>::Read<'a> {
                ($($ty::read(&columns.$index, entity, physical, &data.$index),)+)
            }
            fn item<'a>(columns: &Self::Columns, entity: Entity, physical: usize, data: &'a mut Self::Owned) -> <Self as QueryData>::Item<'a> {
                ($($ty::item(&columns.$index, entity, physical, &mut data.$index),)+)
            }
            fn journal<R>(rows: &mut [Row<R>], project: impl Fn(&mut R) -> &mut Self::Owned, context: &mut Context<'_, '_>) -> Result {
                let result = Ok(());
                $(let result = result.and($ty::journal(rows, |data| &mut project(data).$index, context));)+
                result
            }
        }
        impl<$($ty: ReadProjection),+> ReadProjection for ($($ty,)+) {}
        impl<$($ty: Filter),+> Filter for ($($ty,)+) {}
        impl<$($ty: Filter),+> sealed::Filter for ($($ty,)+) {
            type Columns<'a> = ($($ty::Columns<'a>,)+);
            fn describe(projection: &mut Projection) { $($ty::describe(projection);)+ }
            fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a> {
                ($($ty::bind(world, bindings),)+)
            }
            fn matches(columns: &Self::Columns<'_>, slot: u32) -> bool {
                true $(&& $ty::matches(&columns.$index, slot))+
            }
        }
    )+};
}

tuples!((A 0, B 1), (A 0, B 1, C 2), (A 0, B 1, C 2, D 3), (A 0, B 1, C 2, D 3, E 4), (A 0, B 1, C 2, D 3, E 4, F 5));

pub(crate) struct Row<T> {
    entity: Entity,
    handle: Handle,
    physical: usize,
    data: T,
}

/// Scratch for a query's prepared rows.
/// ```
/// use thrum::{Entity, Query, __derive::{Param, Rows}};
/// let _: <Query<'static, Entity> as Param>::State = Rows::default();
/// ```
#[doc(hidden)]
pub struct Rows<T> {
    rows: Vec<Row<T>>,
}

impl<T> Default for Rows<T> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

fn describe<D: QueryData, F: Filter>(name: &str, list: &mut ParameterList) {
    let mut projection = Projection::default();
    D::describe(&mut projection);
    F::describe(&mut projection);
    list.0.0.push(Parameter::Query(name.to_owned(), projection));
}

fn read_rows<D: QueryData, F: Filter>(rows: &mut Rows<D::Owned>, context: &ReadContext<'_>) {
    rows.rows.clear();
    let world = context.world.expect("preparing query matches");
    let columns = D::presence(world, context.bindings);
    let filters = F::bind(world, context.bindings);
    for &(entity, handle) in context.entities {
        if F::matches(&filters, handle.slot) && D::matches(&columns, handle.slot) {
            rows.rows.push(Row {
                entity,
                handle,
                physical: world.physical_row(handle).expect("matching live handle"),
                data: D::Owned::default(),
            });
        }
    }
}

// SAFETY: This adapter prepares, lends and journals exactly its declared rows.
#[expect(unsafe_code, reason = "trusted scoped query projection")]
unsafe impl<D: QueryData, F: Filter + 'static> Param for Query<'_, D, F> {
    type State = Rows<D::RowData>;
    type Item<'t> = Query<'t, D::For<'t>, F>;
    fn describe(name: &str, list: &mut ParameterList) {
        describe::<D, F>(name, list);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        read_rows::<D, F>(state, context);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, context: &ReadContext<'_>) -> Self::Item<'t> {
        Query {
            rows: &mut state.rows,
            columns: <D::For<'t> as sealed::Data>::bind(
                context.loans.expect("lending query columns"),
            ),
            executor: context.executor,
            filter: PhantomData,
        }
    }
    fn journal(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        D::journal(&mut state.rows, |data| data, context)
    }
}

// SAFETY: This adapter prepares, lends and journals exactly its declared rows.
#[expect(unsafe_code, reason = "trusted scoped query projection")]
unsafe impl<D: QueryData, F: Filter + 'static> Param for Single<'_, D, F> {
    type State = Rows<D::RowData>;
    type Item<'t> = Single<'t, D::For<'t>, F>;
    fn describe(name: &str, list: &mut ParameterList) {
        describe::<D, F>(name, list);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        read_rows::<D, F>(state, context);
        if state.rows.len() != 1 {
            return Err(Error::refused(
                "T0304",
                context.name,
                format!("Single matched {} entities", state.rows.len()),
                "Ensure that exactly one entity matches, or use Query.",
            ));
        }
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, context: &ReadContext<'_>) -> Self::Item<'t> {
        let row = &mut state.rows[0];
        let columns =
            <D::For<'t> as sealed::Data>::bind(context.loans.expect("lending single columns"));
        Single {
            item: <D::For<'t> as sealed::Data>::item(
                &columns,
                row.entity,
                row.physical,
                &mut row.data,
            ),
            filter: PhantomData,
        }
    }
    fn journal(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        D::journal(&mut state.rows, |data| data, context)
    }
}

// SAFETY: This adapter prepares, lends and journals exactly its declared rows.
#[expect(unsafe_code, reason = "trusted scoped query projection")]
unsafe impl<D: QueryData, F: Filter + 'static> Param for Option<Single<'_, D, F>> {
    type State = Rows<D::RowData>;
    type Item<'t> = Option<Single<'t, D::For<'t>, F>>;

    fn describe(name: &str, list: &mut ParameterList) {
        describe::<D, F>(name, list);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        read_rows::<D, F>(state, context);
        if state.rows.len() > 1 {
            return Err(Error::refused(
                "T0304",
                context.name,
                format!("optional Single matched {} entities", state.rows.len()),
                "Ensure that at most one entity matches, or use Query.",
            ));
        }
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, context: &ReadContext<'_>) -> Self::Item<'t> {
        let columns = <D::For<'t> as sealed::Data>::bind(
            context.loans.expect("lending optional single columns"),
        );
        state.rows.first_mut().map(|row| Single {
            item: <D::For<'t> as sealed::Data>::item(
                &columns,
                row.entity,
                row.physical,
                &mut row.data,
            ),
            filter: PhantomData,
        })
    }
    fn journal(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        D::journal(&mut state.rows, |data| data, context)
    }
}

trait ReadProjection: QueryData {}
impl<T: Component> ReadProjection for &T {}
impl<T: Component> ReadProjection for Option<&T> {}
impl ReadProjection for Entity {}
impl ReadProjection for () {}
impl<D: ReadProjection, F: Filter + 'static> ReadOnly for Query<'_, D, F> {}
impl<D: ReadProjection, F: Filter + 'static> ReadOnly for Single<'_, D, F> {}
impl<D: ReadProjection, F: Filter + 'static> ReadOnly for Option<Single<'_, D, F>> {}

pub(crate) fn matching<F: Filter>(
    world: &WorldRead<'_>,
    entities: &[(Entity, Handle)],
) -> Vec<Entity> {
    let mut projection = Projection::default();
    F::describe(&mut projection);
    entities
        .iter()
        .filter_map(|&(entity, handle)| {
            (projection
                .present
                .iter()
                .all(|column| world.has_component(handle, column.key))
                && projection
                    .absent
                    .iter()
                    .all(|column| !world.has_component(handle, column.key)))
            .then_some(entity)
        })
        .collect()
}
