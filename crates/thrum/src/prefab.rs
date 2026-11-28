pub(crate) mod staging;

use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    fmt,
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

use thrum_core::{
    profile::StateEncode,
    sim::{EntityWriter, RegisteredWorldError},
};

use crate::{
    Angle, Component, Data, Entity, Error, Num, Result, Spawn, Transform2d, Transform3d, Vec2,
    Vec3,
    data::{DataReader, DataWriter, InvalidData, Stored, schema_id},
    entity::SpawnValue,
    rule::access::{Column, Signature},
};

/// A typed prefab name declared by the game.
pub struct Prefab<N = ()> {
    declaration: &'static Declaration,
    need: PhantomData<fn() -> N>,
}

impl<N> Clone for Prefab<N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<N> Copy for Prefab<N> {}
impl<N> PartialEq for Prefab<N> {
    fn eq(&self, other: &Self) -> bool {
        self.declaration.key == other.declaration.key
    }
}
impl<N> fmt::Debug for Prefab<N> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_tuple("Prefab")
            .field(&self.declaration.name)
            .finish()
    }
}
impl<N: Data> Data for Prefab<N> {
    const SCHEMA: &'static str = "thrum::Prefab";
    const SCHEMA_ID: u128 = schema_id(Self::SCHEMA, &[N::SCHEMA_ID]);

    fn same_encoding(&self, other: &Self) -> bool {
        self == other
    }
    fn write(&self, writer: &mut DataWriter<'_, '_>) {
        self.declaration.key.write(writer);
    }
    fn read(reader: &mut DataReader<'_, '_>) -> std::result::Result<Self, InvalidData> {
        let key = u128::read(reader)?;
        let declaration = inventory::iter::<LinkedDeclaration>
            .into_iter()
            .map(|item| item.0)
            .find(|item| item.key == key && item.requirement == N::SCHEMA_ID)
            .ok_or(InvalidData)?;
        Ok(Self {
            declaration,
            need: PhantomData,
        })
    }
}

#[doc(hidden)]
pub struct Declaration {
    key: u128,
    name: &'static str,
    block: &'static str,
    constant: &'static str,
    index: OnceLock<usize>,
    requirement: u128,
}

/// The link-time registration of a prefab declaration.
#[doc(hidden)]
pub struct LinkedDeclaration(
    /// The typed constant's backing declaration.
    pub &'static Declaration,
);

inventory::collect!(LinkedDeclaration);

impl Declaration {
    /// Describe one declaration for startup validation.
    #[doc(hidden)]
    pub const fn new(
        name: &'static str,
        block: &'static str,
        constant: &'static str,
        requirement: u128,
    ) -> Self {
        Self {
            key: schema_id(name, &[]),
            name,
            block,
            constant,
            requirement,
            index: OnceLock::new(),
        }
    }
}

/// Declare the typed names a game defines before play.
///
/// ```standalone_crate
/// use thrum::prelude::*;
/// prefabs! { const BIRD: Prefab = "bird"; }
/// let mut game = Game::new("Birds").prefab(BIRD, ()).scene(|_: &mut Scene| {}).test().unwrap();
/// let bird = game.spawn(BIRD.at(2, 3)).unwrap();
/// assert!(game.alive(bird));
/// ```
#[macro_export]
macro_rules! prefabs {
    ($($visibility:vis const $constant:ident: $ty:ty = $name:literal;)+) => {$(
        $visibility const $constant: $ty = {
            static DECLARATION: $crate::__derive::Declaration = $crate::__derive::Declaration::new(
                $name,
                concat!(module_path!(), " at ", file!(), ":", line!()),
                stringify!($constant),
                <$ty>::__requirement(),
            );
            $crate::__derive::submit_prefab! { $crate::__derive::LinkedDeclaration(&DECLARATION) }
            <$ty>::__declared(&DECLARATION)
        };
    )+};
}

impl<N> Prefab<N> {
    /// Construct a constant emitted by `prefabs!`.
    #[doc(hidden)]
    pub const fn __declared(declaration: &'static Declaration) -> Self {
        Self {
            declaration,
            need: PhantomData,
        }
    }

    /// The required component schema emitted by `prefabs!`.
    #[doc(hidden)]
    pub const fn __requirement() -> u128
    where
        N: Data,
    {
        N::SCHEMA_ID
    }

    /// Use this prefab as a definition's base.
    pub fn variant(self) -> PrefabDef {
        PrefabDef {
            base: Some(self.declaration.key),
            ..PrefabDef::default()
        }
    }

    /// Supply the spawn's components, including the ones required by the prefab.
    pub fn with<I>(self, components: I) -> PrefabSpawn<N, I> {
        PrefabSpawn {
            spec: SpawnSpec {
                declaration: self.declaration,
                pose: None,
                parent: None,
                disabled: false,
            },
            input: components,
            additions: (),
            need: PhantomData,
        }
    }

    /// Start a two-dimensional spawn at the given coordinates.
    pub fn at(self, x: impl Into<Num>, y: impl Into<Num>) -> PrefabSpawn<N> {
        self.with(()).at(x, y)
    }

    /// Start a spawn at a point.
    pub fn at_point(self, point: impl Into<Place>) -> PrefabSpawn<N> {
        self.with(()).at_point(point)
    }

    /// Start a two-dimensional spawn at coordinates local to its parent.
    pub fn at_local(self, x: impl Into<Num>, y: impl Into<Num>) -> PrefabSpawn<N> {
        self.at(x, y)
    }
}

/// A 2D or 3D position supplied to a prefab spawn.
#[doc(hidden)]
pub enum Place {
    /// A 2D point.
    Two(Vec2),
    /// A 3D point.
    Three(Vec3),
}

impl From<Vec2> for Place {
    fn from(value: Vec2) -> Self {
        Self::Two(value)
    }
}
impl From<Vec3> for Place {
    fn from(value: Vec3) -> Self {
        Self::Three(value)
    }
}

#[derive(Component)]
pub(crate) struct Origin {
    pub(crate) prefab: u128,
    pub(crate) child: bool,
    pub(crate) part: Option<crate::Part>,
}

pub(crate) trait Field: Send + Sync {
    fn column(&self) -> Column;
    fn any(&mut self) -> &mut dyn Any;
    fn read(&self) -> &dyn Any;
    fn copy_field(&self) -> Box<dyn Field>;
    fn is_finite(&self) -> bool;
    fn fill(&self, writer: &mut EntityWriter<'_>) -> std::result::Result<(), RegisteredWorldError>;
}

impl<T: Component> Field for Stored<T> {
    fn column(&self) -> Column {
        Column::of::<T>()
    }
    fn any(&mut self) -> &mut dyn Any {
        &mut self.0
    }
    fn read(&self) -> &dyn Any {
        &self.0
    }
    fn is_finite(&self) -> bool {
        StateEncode::is_finite(self)
    }
    fn copy_field(&self) -> Box<dyn Field> {
        Box::new(Stored(self.0))
    }
    fn fill(&self, writer: &mut EntityWriter<'_>) -> std::result::Result<(), RegisteredWorldError> {
        writer.add(Stored(self.0)).map(|_| ())
    }
}

#[expect(
    private_bounds,
    reason = "only SDK bundles and components supply prefab fields"
)]
/// The tuple of components used as a prefab definition.
pub trait Bundle: Fields {}
impl<T: Spawn + Fields> Bundle for T {}

pub(crate) trait Fields: Send + Sync + 'static {
    fn fields(&self, add: &mut dyn FnMut(&dyn Field));
    fn field_columns(add: &mut dyn FnMut(Column));
}
impl Fields for () {
    fn fields(&self, _: &mut dyn FnMut(&dyn Field)) {}
    fn field_columns(_: &mut dyn FnMut(Column)) {}
}
impl<T: Component> Fields for T {
    fn fields(&self, add: &mut dyn FnMut(&dyn Field)) {
        add(&Stored(*self));
    }
    fn field_columns(add: &mut dyn FnMut(Column)) {
        add(Column::of::<T>());
    }
}
macro_rules! fields {
    ($(($($ty:ident $index:tt),+)),+ $(,)?) => {$(
        impl<$($ty: Component),+> Fields for ($($ty,)+) {
            fn fields(&self, add: &mut dyn FnMut(&dyn Field)) {
                $(self.$index.fields(add);)+
            }
            fn field_columns(add: &mut dyn FnMut(Column)) {
                $(<$ty as Fields>::field_columns(add);)+
            }
        }
    )+};
}
fields!((A 0), (A 0, B 1), (A 0, B 1, C 2), (A 0, B 1, C 2, D 3), (A 0, B 1, C 2, D 3, E 4), (A 0, B 1, C 2, D 3, E 4, F 5));

pub(crate) struct Required<N, I>(PhantomData<(N, I)>);

#[diagnostic::on_unimplemented(
    message = "T0110: prefab spawn is missing required components `{N}`",
    label = "this prefab spawn does not supply its required components",
    note = "Call Prefab::with with the required component or tuple before spawning."
)]
pub(crate) trait Supplies<N, I> {}
impl<I: Fields> Supplies<(), I> for Required<(), I> {}
impl<T: Component> Supplies<T, T> for Required<T, T> {}
macro_rules! needs {
    ($(($($ty:ident),+)),+ $(,)?) => {$(
        impl<$($ty: Component),+> Supplies<($($ty,)+), ($($ty,)+)> for Required<($($ty,)+), ($($ty,)+)> {}
    )+};
}
needs!(
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

type ApplyTweak = dyn Fn(&mut dyn Any) + Send + Sync;

enum Change {
    Set(Box<dyn Field>),
    Tweak(Column, Box<ApplyTweak>),
}

impl Change {
    fn tweak<T: Component>(apply: impl Fn(&mut T) + Send + Sync + 'static) -> Self {
        Self::Tweak(
            Column::of::<T>(),
            Box::new(move |field| {
                apply(field.downcast_mut().expect("type checked prefab tweak"));
            }),
        )
    }
}

#[derive(Clone, Copy)]
#[doc(hidden)]
/// A dimensional transform supplied to a hierarchy edit.
pub enum Pose {
    /// A 2D transform.
    Two(Transform2d),
    /// A 3D transform.
    Three(Transform3d),
}

impl From<Transform2d> for Pose {
    fn from(value: Transform2d) -> Self {
        Self::Two(value)
    }
}
impl From<Transform3d> for Pose {
    fn from(value: Transform3d) -> Self {
        Self::Three(value)
    }
}

impl Pose {
    pub(crate) fn column(self) -> Column {
        match self {
            Self::Two(_) => Column::of::<Transform2d>(),
            Self::Three(_) => Column::of::<Transform3d>(),
        }
    }

    pub(crate) fn fill(
        self,
        writer: &mut EntityWriter<'_>,
    ) -> std::result::Result<(), RegisteredWorldError> {
        match self {
            Self::Two(pose) => writer.add(Stored(pose)).map(|_| ()),
            Self::Three(pose) => writer.add(Stored(pose)).map(|_| ()),
        }
    }

    pub(crate) fn get<T: Component>(self) -> Option<T> {
        let pose: &dyn Any = match &self {
            Self::Two(pose) => pose,
            Self::Three(pose) => pose,
        };
        pose.downcast_ref::<T>().copied()
    }
}

pub(crate) struct SpawnSpec {
    declaration: &'static Declaration,
    pub(crate) pose: Option<Pose>,
    pub(crate) parent: Option<Entity>,
    pub(crate) disabled: bool,
}

/// A spawn made from a prefab, with the components supplied by its caller.
#[must_use]
pub struct PrefabSpawn<N = (), I = (), A = ()> {
    spec: SpawnSpec,
    input: I,
    additions: A,
    need: PhantomData<fn() -> N>,
}

impl<N, I, A: SpawnEdits> PrefabSpawn<N, I, A> {
    /// Set a two-dimensional spawn's place.
    pub fn at(self, x: impl Into<Num>, y: impl Into<Num>) -> Self {
        self.at_point(crate::vec2(x, y))
    }

    /// Set the spawn's point, in the point's dimensionality.
    pub fn at_point(mut self, point: impl Into<Place>) -> Self {
        self.spec.pose = Some(match point.into() {
            Place::Two(position) => Pose::Two(Transform2d {
                position,
                ..Transform2d::ORIGIN
            }),
            Place::Three(position) => Pose::Three(Transform3d {
                position,
                ..Transform3d::ORIGIN
            }),
        });
        self
    }

    /// Set coordinates local to the owning entity.
    pub fn at_local(self, x: impl Into<Num>, y: impl Into<Num>) -> Self {
        self.at(x, y)
    }

    /// Set the turn at the spawn's place.
    pub fn turned(mut self, angle: Angle) -> Self {
        self.spec.pose = Some(match self.spec.pose {
            Some(Pose::Three(pose)) => Pose::Three(pose.turned_y(angle)),
            Some(Pose::Two(pose)) => Pose::Two(pose.turned(angle)),
            None => Pose::Two(Transform2d::ORIGIN.turned(angle)),
        });
        self
    }

    /// Set the scale at the spawn's place.
    pub fn scaled(mut self, scale: impl Into<Num>) -> Self {
        self.spec.pose = Some(match self.spec.pose {
            Some(Pose::Three(pose)) => Pose::Three(pose.scaled(scale)),
            Some(Pose::Two(pose)) => Pose::Two(pose.scaled(scale)),
            None => Pose::Two(Transform2d::ORIGIN.scaled(scale)),
        });
        self
    }

    /// Add or replace a component on this spawn.
    pub fn with(self, value: impl Component) -> PrefabSpawn<N, I, impl SpawnEdits> {
        self.add(Set(value))
    }

    /// Modify one of the spawn's components.
    pub fn tweak<T: Component>(
        self,
        apply: impl Fn(&mut T) + Send + Sync + 'static,
    ) -> PrefabSpawn<N, I, impl SpawnEdits> {
        self.add(Tweak {
            apply,
            component: PhantomData,
        })
    }

    /// Make this spawn a child of an existing or reserved entity.
    pub fn under(mut self, parent: Entity) -> Self {
        self.spec.parent = Some(parent);
        self
    }

    /// Add a child after the definition's own children.
    pub fn child(self, spawn: impl Spawn) -> PrefabSpawn<N, I, impl SpawnEdits> {
        self.add(OwnChild(spawn))
    }

    fn add<E: Decorations>(self, edit: E) -> PrefabSpawn<N, I, impl SpawnEdits> {
        PrefabSpawn {
            spec: self.spec,
            input: self.input,
            additions: Then(self.additions, edit),
            need: self.need,
        }
    }

    /// Spawn disabled, also hiding its children from ordinary queries.
    pub fn disabled(mut self) -> Self {
        self.spec.disabled = true;
        self
    }
}

impl<N: 'static, I: Fields, A: SpawnEdits> Spawn for PrefabSpawn<N, I, A> where
    Required<N, I>: Supplies<N, I>
{
}

impl<N: 'static, I: Fields, A: SpawnEdits> SpawnValue for PrefabSpawn<N, I, A> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn fill(&self, _: &mut EntityWriter<'_>) -> std::result::Result<(), RegisteredWorldError> {
        unreachable!("prefab spawn is resolved before landing")
    }
    fn columns(&self, expect: &mut dyn FnMut(Column)) {
        I::field_columns(expect);
    }
    fn fields(&self, add: &mut dyn FnMut(&dyn Field)) {
        self.input.fields(add);
    }
    fn prefab(&self) -> Option<&SpawnSpec> {
        Some(&self.spec)
    }

    fn patch(&self, builder: &mut dyn SpawnPatches) -> Result {
        self.additions.patch(builder)
    }

    fn children(&self, add: &mut dyn FnMut(&dyn SpawnValue) -> Result) -> Result {
        self.additions.children(add)
    }
}

pub(crate) trait SpawnPatches {
    fn set(&mut self, field: &dyn Field) -> Result;
    fn tweak(&mut self, tweak: &dyn FieldTweak) -> Result;
}

pub(crate) trait FieldTweak {
    fn column(&self) -> Column;
    fn apply(&self, field: &mut dyn Any);
    fn stage(&self, patch: &mut staging::SpawnPatch<'_>) -> Result;
}

/// The statically typed edits attached to a prefab spawn.
#[doc(hidden)]
#[expect(
    private_bounds,
    reason = "only SDK spawn edits may implement this trait"
)]
pub trait SpawnEdits: Decorations {}
impl<T: Decorations> SpawnEdits for T {}

pub(crate) trait Decorations: Send + Sync + 'static {
    fn patch(&self, _: &mut dyn SpawnPatches) -> Result {
        Ok(())
    }
    fn children(&self, _: &mut dyn FnMut(&dyn SpawnValue) -> Result) -> Result {
        Ok(())
    }
}

impl Decorations for () {}
struct Then<P, E>(P, E);
struct Set<T>(T);
struct Tweak<T, F> {
    apply: F,
    component: PhantomData<fn() -> T>,
}
struct OwnChild<S>(S);

impl<P: Decorations, E: Decorations> Decorations for Then<P, E> {
    fn patch(&self, builder: &mut dyn SpawnPatches) -> Result {
        self.0.patch(builder)?;
        self.1.patch(builder)
    }

    fn children(&self, add: &mut dyn FnMut(&dyn SpawnValue) -> Result) -> Result {
        self.0.children(add)?;
        self.1.children(add)
    }
}

impl<T: Component> Decorations for Set<T> {
    fn patch(&self, builder: &mut dyn SpawnPatches) -> Result {
        builder.set(&Stored(self.0))
    }
}

impl<T: Component, F: Fn(&mut T) + Send + Sync + 'static> Decorations for Tweak<T, F> {
    fn patch(&self, builder: &mut dyn SpawnPatches) -> Result {
        builder.tweak(self)
    }
}
impl<T: Component, F: Fn(&mut T)> FieldTweak for Tweak<T, F> {
    fn column(&self) -> Column {
        Column::of::<T>()
    }
    fn apply(&self, field: &mut dyn Any) {
        (self.apply)(field.downcast_mut().expect("type checked prefab tweak"));
    }

    fn stage(&self, patch: &mut staging::SpawnPatch<'_>) -> Result {
        let mut value = patch.baseline::<T>()?;
        // User code runs after releasing the scratch lock. A panic only
        // refuses this tick and leaves the staging usable on retry.
        (self.apply)(&mut value);
        patch.set(&Stored(value))
    }
}

impl<S: Spawn> Decorations for OwnChild<S> {
    fn children(&self, add: &mut dyn FnMut(&dyn SpawnValue) -> Result) -> Result {
        add(&self.0)
    }
}

pub(crate) struct Child {
    pub(crate) part: Option<crate::Part>,
    pub(crate) spawn: Box<dyn SpawnValue>,
}

impl Child {
    fn new(part: Option<crate::Part>, spawn: impl Spawn) -> Self {
        Self {
            part,
            spawn: Box::new(spawn),
        }
    }
}

/// The defaults, children and named parts of one prefab.
#[derive(Default)]
#[must_use]
pub struct PrefabDef {
    base: Option<u128>,
    changes: Vec<Change>,
    children: Vec<Child>,
}

impl PrefabDef {
    /// Define a prefab from a component tuple.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "prefab definitions take the authored component tuple"
    )]
    pub fn new(bundle: impl Bundle) -> Self {
        let mut changes = Vec::new();
        bundle.fields(&mut |field| changes.push(Change::Set(field.copy_field())));
        Self {
            changes,
            ..Self::default()
        }
    }

    /// Add or replace a default component.
    pub fn with(mut self, value: impl Component) -> Self {
        self.changes.push(Change::Set(Box::new(Stored(value))));
        self
    }

    /// Modify one of the definition's components.
    pub fn tweak<T: Component>(mut self, apply: impl Fn(&mut T) + Send + Sync + 'static) -> Self {
        self.changes.push(Change::tweak(apply));
        self
    }

    /// Define a child in its local place.
    pub fn child(mut self, spawn: impl Spawn) -> Self {
        self.children.push(Child::new(None, spawn));
        self
    }

    /// Define a named child, or a socket from an empty tuple.
    pub fn part(mut self, part: crate::Part, spawn: impl Spawn) -> Self {
        self.children.push(Child::new(Some(part), spawn));
        self
    }
}

impl<B: Bundle> From<B> for PrefabDef {
    fn from(bundle: B) -> Self {
        Self::new(bundle)
    }
}

pub(crate) struct Authored {
    key: u128,
    definition: PrefabDef,
    required: fn(&mut dyn FnMut(Column)),
}

impl Authored {
    pub(crate) fn new<N: Fields>(name: Prefab<N>, definition: PrefabDef) -> Self {
        Self {
            key: name.declaration.key,
            definition,
            required: N::field_columns,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Prepared {
    key: u128,
    name: &'static str,
    fields: Vec<Arc<dyn Field>>,
    required: Vec<Column>,
    pub(crate) children: Vec<PreparedChild>,
    pub(crate) disabled: bool,
}

#[derive(Clone)]
pub(crate) struct PreparedChild {
    pub(crate) value: Arc<Prepared>,
    pub(crate) part: Option<crate::Part>,
}

impl Prepared {
    pub(crate) fn fill(
        &self,
        writer: &mut EntityWriter<'_>,
        child: bool,
        part: Option<crate::Part>,
        omitted: impl Fn(Column) -> bool,
    ) -> std::result::Result<(), RegisteredWorldError> {
        for field in &self.fields {
            if !omitted(field.column()) {
                field.fill(writer)?;
            }
        }
        if self.key != 0 || child {
            writer.add(Stored(Origin {
                prefab: self.key,
                child,
                part,
            }))?;
        }
        Ok(())
    }

    pub(crate) fn field(&self, column: Column) -> Option<&dyn Field> {
        self.fields
            .iter()
            .find(|value| value.column().key == column.key)
            .map(Arc::as_ref)
    }

    pub(crate) fn get<T: Component>(&self) -> Option<T> {
        self.field(Column::of::<T>())?
            .read()
            .downcast_ref::<T>()
            .copied()
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn columns(&self, add: &mut impl FnMut(Column)) {
        for field in &self.fields {
            add(field.column());
        }
        for &column in &self.required {
            add(column);
        }
        for child in &self.children {
            child.value.columns(add);
        }
    }

    fn validate_fields(&self) -> Result {
        if let Some(field) = self.fields.iter().find(|field| !field.is_finite()) {
            return Err(Error::refused(
                "T0213",
                self.name,
                format!("non-finite component {}", field.column().name),
                "Use finite components in the prefab definition.",
            ));
        }
        Ok(())
    }

    pub(crate) fn check_access(&self, name: &str, signature: &Signature) -> Result {
        signature.validate_projection(
            name,
            || format!("prefab {}", self.name),
            self.disabled,
            |key| {
                self.fields.iter().any(|field| field.column().key == key)
                    || self.required.iter().any(|column| column.key == key)
                    || key == Column::of::<Origin>().key
            },
        )?;
        for child in &self.children {
            child.value.check_access(name, signature)?;
        }
        Ok(())
    }

    fn set(&mut self, field: Arc<dyn Field>) {
        if let Some(slot) = self
            .fields
            .iter_mut()
            .find(|slot| slot.column().key == field.column().key)
        {
            *slot = field;
        } else {
            self.fields.push(field);
        }
    }

    fn change(&mut self, change: &Change) -> Result {
        match change {
            Change::Set(field) => self.set(field.copy_field().into()),
            Change::Tweak(column, apply) => {
                let field = self
                    .fields
                    .iter_mut()
                    .find(|slot| slot.column().key == column.key)
                    .ok_or_else(|| {
                        Error::refused(
                            "T0213",
                            self.name,
                            format!("tweak has no component {}", column.name),
                            "Supply the component with .with before tweaking it.",
                        )
                    })?;
                let mut value = field.copy_field();
                apply(value.any());
                *field = value.into();
            }
        }
        Ok(())
    }

    fn apply(&mut self, spawn: &dyn SpawnValue, spec: &SpawnSpec) -> Result {
        spawn.fields(&mut |field| self.set(field.copy_field().into()));
        spawn.patch(self)?;
        match spec.pose {
            Some(Pose::Two(value)) => self.set(Arc::new(Stored(value))),
            Some(Pose::Three(value)) => self.set(Arc::new(Stored(value))),
            None => {}
        }
        self.disabled = spec.disabled;
        Ok(())
    }
}

impl SpawnPatches for Prepared {
    fn set(&mut self, field: &dyn Field) -> Result {
        self.set(field.copy_field().into());
        Ok(())
    }

    fn tweak(&mut self, tweak: &dyn FieldTweak) -> Result {
        let column = tweak.column();
        let field = self
            .field(column)
            .ok_or_else(|| missing_tweak(self.name, column))?;
        let mut value = field.copy_field();
        tweak.apply(value.any());
        self.set(value.into());
        Ok(())
    }
}

pub(crate) fn missing_tweak(name: &str, column: Column) -> Error {
    Error::refused(
        "T0213",
        name,
        format!("tweak has no component {}", column.name),
        "Supply the component with .with before tweaking it.",
    )
}

#[derive(Default)]
pub(crate) struct Catalog(Vec<Arc<Prepared>>);

impl Catalog {
    pub(crate) fn compile(authored: Vec<Authored>) -> Result<Self> {
        let mut declarations: Vec<_> = inventory::iter::<LinkedDeclaration>
            .into_iter()
            .map(|item| item.0)
            .collect();
        declarations.sort_unstable_by_key(|item| (item.name, item.block, item.constant));
        let mut names = BTreeMap::new();
        for (index, &declaration) in declarations.iter().enumerate() {
            if names.insert(declaration.key, declaration).is_some() {
                return Err(Error::refused(
                    "T0211",
                    declaration.name,
                    "prefab name is declared twice",
                    "Give each prefab a unique name in the game.",
                ));
            }
            declaration.index.get_or_init(|| index);
        }
        let mut definitions = BTreeMap::new();
        for entry in authored {
            if definitions.contains_key(&entry.key) {
                return Err(Error::refused(
                    "T0211",
                    names.get(&entry.key).map_or("prefab", |d| d.name),
                    "two Game::prefab definitions",
                    "Define each prefab once with Game::prefab.",
                ));
            }
            definitions.insert(entry.key, entry);
        }

        for declaration in names.values() {
            if !definitions.contains_key(&declaration.key) {
                return Err(Error::refused(
                    "T0210",
                    format!(
                        "prefab {} ({}) in prefabs! block {}",
                        declaration.name, declaration.constant, declaration.block
                    ),
                    "declared without a definition",
                    "Define it with Game::prefab before starting the game.",
                ));
            }
        }
        let mut resolver = Resolver {
            names,
            definitions,
            resolved: BTreeMap::new(),
            visiting: BTreeSet::new(),
        };
        for &key in &resolver.definitions.keys().copied().collect::<Vec<_>>() {
            resolver.resolve(key)?;
        }
        Ok(Self(
            declarations
                .iter()
                .map(|item| Arc::clone(&resolver.resolved[&item.key]))
                .collect(),
        ))
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &Prepared> {
        self.0.iter().map(Arc::as_ref)
    }

    pub(crate) fn spawn(&self, spec: &SpawnSpec) -> Result<Arc<Prepared>> {
        spec.declaration
            .index
            .get()
            .and_then(|&index| self.0.get(index))
            .map(Arc::clone)
            .ok_or_else(|| unknown(spec.declaration.key))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

struct Resolver {
    names: BTreeMap<u128, &'static Declaration>,
    definitions: BTreeMap<u128, Authored>,
    resolved: BTreeMap<u128, Arc<Prepared>>,
    visiting: BTreeSet<u128>,
}

impl Resolver {
    fn resolve(&mut self, key: u128) -> Result<Arc<Prepared>> {
        if let Some(value) = self.resolved.get(&key) {
            return Ok(Arc::clone(value));
        }
        let declaration = self.names.get(&key).copied().ok_or_else(|| unknown(key))?;
        if !self.visiting.insert(key) {
            return Err(Error::refused(
                "T0212",
                declaration.name,
                "prefab base or child contains a cycle",
                "Remove the circular prefab reference.",
            ));
        }
        let entry = self
            .definitions
            .remove(&key)
            .expect("every declared prefab has a definition");
        let mut prepared = if let Some(base) = entry.definition.base {
            self.resolve(base)?.as_ref().clone()
        } else {
            Prepared::default()
        };
        prepared.key = key;
        prepared.name = declaration.name;
        prepared.required.clear();
        (entry.required)(&mut |column| prepared.required.push(column));
        for change in &entry.definition.changes {
            prepared.change(change)?;
        }
        for child in &entry.definition.children {
            if let Some(part) = child.part
                && prepared
                    .children
                    .iter()
                    .any(|known| known.part == child.part)
            {
                return Err(Error::refused(
                    "T0213",
                    declaration.name,
                    format!("duplicate part {part:?}"),
                    "Give each part a unique name within the prefab.",
                ));
            }
            prepared.children.push(PreparedChild {
                value: self.child(child.spawn.as_ref())?,
                part: child.part,
            });
        }

        prepared.validate_fields()?;
        self.visiting.remove(&key);
        let prepared = Arc::new(prepared);
        self.resolved.insert(key, Arc::clone(&prepared));
        Ok(prepared)
    }

    fn child(&mut self, child: &dyn SpawnValue) -> Result<Arc<Prepared>> {
        let mut value = Prepared::default();
        if let Some(spec) = child.prefab() {
            value = self.resolve(spec.declaration.key)?.as_ref().clone();
            value.apply(child, spec)?;
            child.children(&mut |child| {
                value.children.push(PreparedChild {
                    value: self.child(child)?,
                    part: None,
                });
                Ok(())
            })?;
        } else {
            child.fields(&mut |field| value.set(field.copy_field().into()));
        }
        value.validate_fields()?;
        Ok(Arc::new(value))
    }
}

fn unknown(key: u128) -> Error {
    Error::refused(
        "T0210",
        format!("prefab {key:032x}"),
        "no linked prefab declaration with that identity",
        "Declare the name in prefabs! and define it with Game::prefab.",
    )
}
