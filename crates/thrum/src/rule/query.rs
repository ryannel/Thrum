use std::{marker::PhantomData, slice};

use thrum_core::{
    Handle,
    sim::{ColumnRead, Executor, WorldRead, WorldTxn},
};

use crate::{Component, Entity, Error, Result, data::Stored};

use super::{
    access::{AccessColumn, Bindings, Column, Parameter, Projection},
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
        self.rows.iter().map(|row| D::read(&row.data))
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
        IterMut(self.rows.iter_mut(), PhantomData)
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
        Some(D::read(&self.rows[index].data))
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
        Some(D::item(&mut self.rows[index].data))
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
        Some((D::item(&mut a.data), D::item(&mut b.data)))
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
        self.executor
            .for_each_mut(self.rows, |row| run(D::item(&mut row.data)));
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
pub struct IterMut<'a, D: QueryData>(slice::IterMut<'a, Row<D::Owned>>, PhantomData<D>);

impl<'a, D: QueryData> Iterator for IterMut<'a, D> {
    type Item = D::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|row| D::item(&mut row.data))
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
pub struct Iter<'a, D: QueryData>(slice::Iter<'a, Row<D::Owned>>, PhantomData<D>);

impl<'a, D: QueryData> Iterator for Iter<'a, D> {
    type Item = D::Read<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|row| D::read(&row.data))
    }
}

impl<'b, D: QueryData, F: Filter> IntoIterator for &'b Query<'_, D, F> {
    type Item = D::Read<'b>;
    type IntoIter = Iter<'b, D>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.rows.iter(), PhantomData)
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
/// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
/// ```
#[expect(private_bounds, reason = "only the SDK can define a query projection")]
pub trait QueryData: sealed::Data<Owned = Self::RowData> {
    /// Reusable data held for one matching row.
    /// ```
    /// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
    /// ```
    type RowData: Send + 'static;
    /// A shared view of one row.
    /// ```
    /// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
    /// ```
    type Read<'a>;
    /// A view with the access declared by the query.
    /// ```
    /// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
    /// ```
    type Item<'a>;
    /// The same projection with references tied to the current rule.
    /// ```
    /// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
    /// ```
    type For<'a>: QueryData<RowData = Self::RowData>;
}

/// A component presence filter.
/// ```
/// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
/// ```
#[expect(private_bounds, reason = "only the SDK can define a query filter")]
pub trait Filter: sealed::Filter {}

mod sealed {
    use super::{Bindings, Entity, Handle, Projection, QueryData, Result, WorldRead, WorldTxn};

    pub(crate) trait Data: Sized {
        type Owned: Send + 'static;
        type Columns<'a>;

        fn describe(projection: &mut Projection);
        fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a>;
        fn collect(
            columns: &Self::Columns<'_>,
            entity: Entity,
            handle: Handle,
        ) -> Option<Self::Owned>;
        fn read(data: &Self::Owned) -> <Self as QueryData>::Read<'_>
        where
            Self: QueryData;
        fn item(data: &mut Self::Owned) -> <Self as QueryData>::Item<'_>
        where
            Self: QueryData;
        fn save(
            data: &Self::Owned,
            world: &mut WorldTxn<'_>,
            bindings: &Bindings,
            handle: Handle,
        ) -> Result;
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

macro_rules! component_data {
    ([$($projection:tt)*]; $write:literal; $item:ty; $save:ident) => {
        impl<T: Component> QueryData for $($projection)* {
            type RowData = T;
            type Read<'a> = &'a T;
            type Item<'a> = $item;
            type For<'a> = $item;
        }
        impl<T: Component> sealed::Data for $($projection)* {
            type Owned = T;
            type Columns<'a> = ColumnRead<'a, Stored<T>>;

            fn describe(projection: &mut Projection) {
                projection.access.push(AccessColumn { column: Column::of::<T>(), write: $write, required: true });
            }
            fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a> {
                world.column(bindings.read::<T>()).expect("bound query column")
            }
            fn collect(column: &Self::Columns<'_>, _: Entity, handle: Handle) -> Option<T> {
                column.at_slot(handle.slot).map(|stored| stored.0.clone())
            }
            fn read(data: &Self::Owned) -> <Self as QueryData>::Read<'_> { data }
            fn item(data: &mut Self::Owned) -> <Self as QueryData>::Item<'_> { data }
            fn save(data: &T, world: &mut WorldTxn<'_>, bindings: &Bindings, handle: Handle) -> Result {
                $save(data, world, bindings, handle)
            }
        }
    };
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the same projection adapter also handles fallible writes"
)]
fn save_read<T>(_: &T, _: &mut WorldTxn<'_>, _: &Bindings, _: Handle) -> Result {
    Ok(())
}
fn save_write<T: Component>(
    data: &T,
    world: &mut WorldTxn<'_>,
    bindings: &Bindings,
    handle: Handle,
) -> Result {
    world
        .put(bindings.write::<T>(), handle, Stored(data.clone()))
        .map(|_| ())
        .map_err(|error| {
            Error::refused(
                "T0308",
                std::any::type_name::<T>(),
                error.to_string(),
                "Correct the rule's component value.",
            )
        })
}

component_data!([&T]; false; &'a T; save_read);
component_data!([&mut T]; true; &'a mut T; save_write);

macro_rules! optional_data {
    ([$($projection:tt)*], $item:ty, $write:literal, $item_method:ident, $save:ident) => {
        impl<T: Component> QueryData for Option<$($projection)*> {
            type RowData = Option<T>;
            type Read<'a> = Option<&'a T>;
            type Item<'a> = Option<$item>;
            type For<'a> = Option<$item>;
        }
        impl<T: Component> sealed::Data for Option<$($projection)*> {
            type Owned = Option<T>;
            type Columns<'a> = ColumnRead<'a, Stored<T>>;
            fn describe(projection: &mut Projection) {
                projection.access.push(AccessColumn { column: Column::of::<T>(), write: $write, required: false });
            }
            fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a> {
                world.column(bindings.read::<T>()).expect("bound optional column")
            }
            fn collect(column: &Self::Columns<'_>, _: Entity, handle: Handle) -> Option<Self::Owned> {
                Some(column.at_slot(handle.slot).map(|stored| stored.0.clone()))
            }
            fn read(data: &Self::Owned) -> <Self as QueryData>::Read<'_> { data.as_ref() }
            fn item(data: &mut Self::Owned) -> <Self as QueryData>::Item<'_> { data.$item_method() }
            fn save(data: &Self::Owned, world: &mut WorldTxn<'_>, bindings: &Bindings, handle: Handle) -> Result {
                if let Some(data) = data { $save(data, world, bindings, handle)?; }
                Ok(())
            }
        }
    };
}

optional_data!([&T], &'a T, false, as_ref, save_read);
optional_data!([&mut T], &'a mut T, true, as_mut, save_write);

impl QueryData for Entity {
    type RowData = Self;
    type Read<'a> = Self;
    type Item<'a> = Self;
    type For<'a> = Self;
}

impl sealed::Data for Entity {
    type Owned = Self;
    type Columns<'a> = ();
    fn describe(_: &mut Projection) {}
    fn bind<'a>(_: &'a WorldRead<'_>, _: &'a Bindings) {}
    fn collect((): &(), entity: Entity, _: Handle) -> Option<Self> {
        Some(entity)
    }
    fn read(data: &Self) -> Self {
        *data
    }
    fn item(data: &mut Self) -> Self {
        *data
    }
    fn save(_: &Self, _: &mut WorldTxn<'_>, _: &Bindings, _: Handle) -> Result {
        Ok(())
    }
}

impl QueryData for () {
    type RowData = ();
    type Read<'a> = ();
    type Item<'a> = ();
    type For<'a> = ();
}

impl sealed::Data for () {
    type Owned = ();
    type Columns<'a> = ();
    fn describe(_: &mut Projection) {}
    fn bind<'a>(_: &'a WorldRead<'_>, _: &'a Bindings) {}
    fn collect((): &(), _: Entity, _: Handle) -> Option<()> {
        Some(())
    }
    fn read((): &()) {}
    fn item((): &mut ()) {}
    fn save((): &(), _: &mut WorldTxn<'_>, _: &Bindings, _: Handle) -> Result {
        Ok(())
    }
}

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
            type Columns<'a> = ($($ty::Columns<'a>,)+);
            fn describe(projection: &mut Projection) { $($ty::describe(projection);)+ }
            fn bind<'a>(world: &'a WorldRead<'_>, bindings: &'a Bindings) -> Self::Columns<'a> {
                ($($ty::bind(world, bindings),)+)
            }
            fn collect(columns: &Self::Columns<'_>, entity: Entity, handle: Handle) -> Option<Self::Owned> {
                Some(($($ty::collect(&columns.$index, entity, handle)?,)+))
            }
            fn read(data: &Self::Owned) -> <Self as QueryData>::Read<'_> { ($($ty::read(&data.$index),)+) }
            fn item(data: &mut Self::Owned) -> <Self as QueryData>::Item<'_> { ($($ty::item(&mut data.$index),)+) }
            fn save(data: &Self::Owned, world: &mut WorldTxn<'_>, bindings: &Bindings, handle: Handle) -> Result {
                $($ty::save(&data.$index, world, bindings, handle)?;)+
                Ok(())
            }
        }
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

struct Row<T> {
    entity: Entity,
    handle: Handle,
    data: T,
}

/// Scratch for a query's prepared rows.
/// ```
/// fn count(query: thrum::Query<thrum::Entity>) -> u32 { query.count() }
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
    let world = context.world;
    let columns = D::bind(world, context.bindings);
    let filters = F::bind(world, context.bindings);
    for &(entity, handle) in context.entities {
        if F::matches(&filters, handle.slot)
            && let Some(data) = D::collect(&columns, entity, handle)
        {
            rows.rows.push(Row {
                entity,
                handle,
                data,
            });
        }
    }
}

fn write_rows<D: QueryData>(rows: &Rows<D::Owned>, context: &mut Context<'_, '_>) -> Result {
    for row in &rows.rows {
        D::save(&row.data, context.world, context.bindings, row.handle)?;
    }
    Ok(())
}

impl<D: QueryData, F: Filter + 'static> Param for Query<'_, D, F> {
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
            executor: context.executor,
            filter: PhantomData,
        }
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        write_rows::<D>(state, context)
    }
}

impl<D: QueryData, F: Filter + 'static> Param for Single<'_, D, F> {
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
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Self::Item<'t> {
        Single {
            item: <D::For<'t> as sealed::Data>::item(&mut state.rows[0].data),
            filter: PhantomData,
        }
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        write_rows::<D>(state, context)
    }
}

impl<D: QueryData, F: Filter + 'static> Param for Option<Single<'_, D, F>> {
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
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Self::Item<'t> {
        state.rows.first_mut().map(|row| Single {
            item: <D::For<'t> as sealed::Data>::item(&mut row.data),
            filter: PhantomData,
        })
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        write_rows::<D>(state, context)
    }
}

trait ReadProjection: QueryData {}
impl<T: Component> ReadProjection for &T {}
impl<T: Component> ReadProjection for Option<&T> {}
impl ReadProjection for Entity {}
impl ReadProjection for () {}
impl<A: ReadProjection, B: ReadProjection> ReadProjection for (A, B) {}
impl<A: ReadProjection, B: ReadProjection, C: ReadProjection> ReadProjection for (A, B, C) {}
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
