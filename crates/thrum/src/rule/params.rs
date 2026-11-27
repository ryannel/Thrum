use std::any::{TypeId, type_name};

use thrum_core::{
    Handle,
    sim::{Executor, WorldRead, WorldTxn},
};

use crate::{
    Data, Entity, Error, Event, Num, Result, Tick,
    runtime::{Candidate, Resources},
};

use super::access::{Bindings, Exclusive, Kind, Parameter, Signature};

/// The parameter declarations collected by the SDK for a rule.
/// ```
/// use thrum::Tick;
/// fn rule(_: Tick) {}
/// ```
#[doc(hidden)]
#[derive(Default)]
pub struct ParameterList(pub(crate) Signature);

impl ParameterList {
    pub(crate) fn resource<T: Data>(&mut self, name: &str, write: bool) {
        self.0.0.push(Parameter::Exclusive(
            name.to_owned(),
            Exclusive {
                kind: Kind::Resource,
                id: TypeId::of::<T>(),
                name: type_name::<T>(),
                write,
                column: None,
            },
        ));
    }

    pub(crate) fn writer<T: 'static>(&mut self, name: &str) {
        self.0.0.push(Parameter::Exclusive(
            name.to_owned(),
            Exclusive {
                kind: Kind::Writer,
                id: TypeId::of::<T>(),
                name: type_name::<T>(),
                write: true,
                column: None,
            },
        ));
    }
}

/// The view used to prepare rule parameters.
/// ```
/// use thrum::Tick;
/// fn rule(_: Tick) {}
/// ```
#[doc(hidden)]
pub struct ReadContext<'a> {
    pub(crate) world: &'a WorldRead<'a>,
    pub(crate) bindings: &'a Bindings,
    pub(crate) entities: &'a [(Entity, Handle)],
    pub(crate) executor: Executor,
    pub(crate) candidate: &'a Candidate,
    pub(crate) tick: Tick,
    pub(crate) name: &'a str,
    pub(crate) seed: u64,
    pub(crate) stage: crate::scene::Landing,
    pub(crate) names: &'a std::sync::Arc<crate::scene::Names>,
    pub(crate) stream: u32,
}

/// The view used to submit a rule's changes.
/// ```
/// use thrum::Tick;
/// fn rule(_: Tick) {}
/// ```
#[doc(hidden)]
pub struct Context<'a, 'w> {
    pub(crate) world: &'a mut WorldTxn<'w>,
    pub(crate) bindings: &'a Bindings,
    pub(crate) candidate: &'a mut Candidate,
    pub(crate) columns: &'a [super::access::Column],
    pub(crate) changes: &'a mut crate::entity::Changes,
    pub(crate) name: &'a str,
}

/// Implements one parameter or a struct of parameters accepted by a rule.
/// ```
/// use thrum::Tick;
/// fn rule(_: Tick) {}
/// ```
#[doc(hidden)]
pub trait Param {
    /// Reusable scratch for one rule parameter.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    type State: Default + Send + 'static;

    /// The parameter lent to one invocation.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    type Item<'t>;

    /// Declare the rule's accesses before compiling the schedule.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    fn describe(name: &str, list: &mut ParameterList);

    /// Prepare the parameter against the world at rule entry.
    ///
    /// # Errors
    /// The world cannot supply the parameter, such as an empty Single query.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result;

    /// Lend the prepared parameter to the rule.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    fn item<'t>(state: &'t mut Self::State, context: &ReadContext<'_>) -> Self::Item<'t>;

    /// Submit the parameter's effects at the end of the rule.
    ///
    /// # Errors
    /// A proposed change is invalid.
    /// ```
    /// use thrum::Tick;
    /// fn rule(_: Tick) {}
    /// ```
    fn write(_: &mut Self::State, _: &mut Context<'_, '_>) -> Result {
        Ok(())
    }
}

/// A parameter suitable for a read-only condition or duration.
/// ```
/// use thrum::Tick;
/// fn condition(_: Tick) -> bool { true }
/// ```
#[doc(hidden)]
pub trait ReadOnly: Param {}

/// Read a game resource, a scene resource, or the current scene's value.
/// ```
/// use thrum::{Res, Resource};
/// #[derive(Resource)] struct Score(u32);
/// fn score(score: Res<Score>) -> u32 { score.0 }
/// ```
pub struct Res<'t, T: Data>(&'t T);

impl<T: Data> std::ops::Deref for Res<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

/// Read and change a game or scene resource.
/// ```
/// use thrum::{ResMut, Resource};
/// #[derive(Resource)] struct Score(u32);
/// fn point(mut score: ResMut<Score>) { score.0 += 1; }
/// ```
pub struct ResMut<'t, T: crate::Resource>(&'t mut T);

impl<T: crate::Resource> std::ops::Deref for ResMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: crate::Resource> std::ops::DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}

fn get_resource<T: Data>(resources: &Resources, name: &str) -> Result<T> {
    resources.get::<T>().cloned().ok_or_else(|| {
        Error::refused(
            "T0201",
            name,
            format!("resource {} has no value in this scene", type_name::<T>()),
            "Register it with scene.resource(..) or Game::resource(..).",
        )
    })
}

impl<T: Data> Param for Res<'_, T> {
    type State = Option<T>;
    type Item<'t> = Res<'t, T>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.resource::<T>(name, false);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        *state = Some(get_resource(
            &context.candidate.state.resources,
            context.name,
        )?);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Res<'t, T> {
        Res(state.as_ref().expect("resource was prepared"))
    }
}

impl<T: Data> ReadOnly for Res<'_, T> {}

impl<T: crate::Resource> Param for ResMut<'_, T> {
    type State = Option<T>;
    type Item<'t> = ResMut<'t, T>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.resource::<T>(name, true);
    }
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        *state = Some(get_resource(
            &context.candidate.state.resources,
            context.name,
        )?);
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> ResMut<'t, T> {
        ResMut(state.as_mut().expect("resource was prepared"))
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        let value = state.take().expect("resource was prepared");
        crate::runtime::check_value(&value, type_name::<T>())?;
        context.candidate.state.resources.replace(value);
        Ok(())
    }
}

/// Events sent by rules that finished earlier in this tick.
/// ```
/// use thrum::{Event, Events};
/// #[derive(Event)] struct Scored;
/// fn scored(events: Events<Scored>) -> bool { events.read().next().is_some() }
/// ```
pub struct Events<'t, E: Event>(&'t [E]);

impl<E: Event> Events<'_, E> {
    /// Traverse events in send order.
    /// ```
    /// use thrum::{Event, Events};
    /// #[derive(Event)] struct Scored;
    /// fn count(events: Events<Scored>) -> usize { events.read().len() }
    /// ```
    pub fn read(&self) -> std::slice::Iter<'_, E> {
        self.0.iter()
    }
}

/// The events a rule sends to later rules of this tick.
/// ```
/// use thrum::{Emit, Event};
/// #[derive(Event)] struct Scored;
/// fn score(mut emit: Emit<Scored>) { emit.send(Scored); }
/// ```
pub struct Emit<'t, E: Event>(&'t mut Vec<E>);

impl<E: Event> Emit<'_, E> {
    /// Send an event which becomes visible when this rule finishes.
    /// ```
    /// use thrum::{Emit, Event};
    /// #[derive(Event)] struct Scored;
    /// fn score(mut emit: Emit<Scored>) { emit.send(Scored); }
    /// ```
    pub fn send(&mut self, event: E) {
        self.0.push(event);
    }
}

impl<E: Event> Param for Events<'_, E> {
    type State = Vec<E>;
    type Item<'t> = Events<'t, E>;
    fn describe(_: &str, _: &mut ParameterList) {}
    fn read(state: &mut Self::State, context: &ReadContext<'_>) -> Result {
        state.clear();
        state.extend_from_slice(context.candidate.events.read::<E>());
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Events<'t, E> {
        Events(state)
    }
}
impl<E: Event> ReadOnly for Events<'_, E> {}

impl<E: Event> Param for Emit<'_, E> {
    type State = Vec<E>;
    type Item<'t> = Emit<'t, E>;
    fn describe(name: &str, list: &mut ParameterList) {
        list.writer::<E>(name);
    }
    fn read(state: &mut Self::State, _: &ReadContext<'_>) -> Result {
        state.clear();
        Ok(())
    }
    fn item<'t>(state: &'t mut Self::State, _: &ReadContext<'_>) -> Emit<'t, E> {
        Emit(state)
    }
    fn write(state: &mut Self::State, context: &mut Context<'_, '_>) -> Result {
        context.candidate.events.extend(state.drain(..));
        Ok(())
    }
}

impl Param for Tick {
    type State = ();
    type Item<'t> = Tick;
    fn describe(_: &str, _: &mut ParameterList) {}
    fn read((): &mut (), _: &ReadContext<'_>) -> Result {
        Ok(())
    }
    fn item((): &mut (), context: &ReadContext<'_>) -> Tick {
        context.tick
    }
}

impl ReadOnly for Tick {}

/// A reproducible stream of draws for the current rule and tick.
/// ```
/// use thrum::{Num, Random};
/// fn roll(mut random: Random) -> Num { random.unit() }
/// ```
pub struct Random {
    seed: u64,
    tick: u64,
    stream: u32,
    draw: u128,
    entity: u128,
}

impl Random {
    fn word(&mut self) -> u64 {
        let key = self.draw ^ self.entity.rotate_left(64);
        let value = thrum_core::profile::random_u64(self.seed, self.tick, key, self.stream);
        self.draw = self.draw.wrapping_add(1);
        value
    }

    fn unit(&mut self) -> Num {
        Num(thrum_core::profile::random_fraction(self.word()))
    }

    /// A draw between the supplied portable numbers.
    /// ```
    /// use thrum::{Num, Random};
    /// fn rock_speed(mut random: Random) -> Num { random.between(Num::from(4), Num::from(9)) }
    /// ```
    pub fn between(&mut self, low: Num, high: Num) -> Num {
        low + (high - low) * self.unit()
    }

    /// An integer below the exclusive upper bound, or zero for an empty range.
    /// ```
    /// fn die(mut random: thrum::Random) -> u32 { 1 + random.below(6) }
    /// ```
    pub fn below(&mut self, high: u32) -> u32 {
        let bits = self.word();
        u32::try_from((u128::from(bits) * u128::from(high)) >> 64).unwrap_or(high)
    }

    /// Draw whether an event happens at the supplied probability.
    /// ```
    /// fn coin(mut random: thrum::Random) -> bool { random.chance(thrum::Num::ONE / thrum::Num::from(2)) }
    /// ```
    pub fn chance(&mut self, probability: Num) -> bool {
        self.unit() < probability
    }

    /// Choose an item uniformly from this slice, or return none when it is empty.
    /// ```
    /// fn choose(mut random: thrum::Random) -> Option<i32> { random.pick(&[2, 3, 5]).copied() }
    /// ```
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let index = (u128::from(self.word()) * items.len() as u128) >> 64;
        items.get(usize::try_from(index).ok()?)
    }

    /// Start an independent stream for an entity, suitable for a parallel rule.
    /// ```
    /// fn choose(random: thrum::Random, entity: thrum::Entity) -> u32 {
    ///     random.for_entity(entity).below(10)
    /// }
    /// ```
    #[must_use]
    pub fn for_entity(&self, entity: Entity) -> Self {
        Self {
            seed: self.seed,
            tick: self.tick,
            stream: self.stream,
            draw: 0,
            entity: entity.0,
        }
    }
}

impl Param for Random {
    type State = ();
    type Item<'t> = Self;
    fn describe(_: &str, _: &mut ParameterList) {}
    fn read((): &mut (), _: &ReadContext<'_>) -> Result {
        Ok(())
    }
    fn item((): &mut (), context: &ReadContext<'_>) -> Self {
        Self {
            seed: context.seed,
            tick: context.tick.number().0,
            stream: context.stream,
            draw: 0,
            entity: 0,
        }
    }
}
impl ReadOnly for Random {}
