use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::{Arc, Mutex},
};

use crate::{Result, Seconds};

pub(crate) mod access;
pub(crate) mod params;
pub(crate) mod query;

use access::{Loans, Signature};
use params::{Context, Param, ParameterList, ReadContext, ReadOnly};

type Invoke<O> = dyn FnMut(&mut Context<'_, '_>, &ReadContextParts<'_>) -> Result<O> + Send;

/// A compiled callback prepared from a Rust rule.
/// ```
/// use thrum::__derive::{Invocation, RuleFunction};
/// let _: Invocation<u32> = (|| 7_u32).compile();
/// ```
#[doc(hidden)]
pub struct Invocation<O> {
    pub(crate) name: &'static str,
    pub(crate) signature: Signature,
    pub(crate) stream: u64,
    pub(crate) callable: TypeId,
    pub(crate) pointer: Option<usize>,
    pub(crate) invoke: Box<Invoke<O>>,
}

#[derive(Default)]
pub(crate) struct RuleStreams(HashMap<(TypeId, Option<usize>), u64>);

impl RuleStreams {
    pub(crate) fn identify<O>(&mut self, invocation: &mut Invocation<O>) {
        // Addresses only identify a pointer within this process. Registration
        // order supplies its seed, so ASLR never enters the draw.
        let next = self.0.len() as u64;
        let key = (invocation.callable, invocation.pointer);
        let registration = *self.0.entry(key).or_insert(next);
        invocation.distinguish(0x6d61_6e75_616c_0000 ^ registration);
    }
}

pub(crate) struct ReadContextParts<'a> {
    pub(crate) entities: &'a [(crate::Entity, thrum_core::Handle)],
    pub(crate) executor: thrum_core::sim::Executor,
    pub(crate) tick: crate::Tick,
    pub(crate) seed: u64,
    pub(crate) stream: u64,
    pub(crate) stage: crate::scene::Landing,
    pub(crate) names: &'a Arc<crate::scene::Names>,
}

impl<O> Invocation<O> {
    pub(crate) fn anonymous(&self) -> bool {
        self.name.ends_with("::{{closure}}")
    }

    pub(crate) fn distinguish(&mut self, registration: u64) {
        self.stream = (self.stream ^ registration).wrapping_mul(0x0100_0000_01b3);
    }

    pub(crate) fn identify_scheduled(&mut self, registration: u64, streams: &mut RuleStreams) {
        if self.pointer.is_some() {
            streams.identify(self);
        } else if self.anonymous() {
            self.distinguish(registration);
        }
    }

    pub(crate) fn call(
        &mut self,
        context: &mut Context<'_, '_>,
        parts: &ReadContextParts<'_>,
    ) -> Result<O> {
        (self.invoke)(context, parts)
    }
}

/// Marker used to infer rule parameter types.
/// ```
/// use thrum::{Rules, Tick, __derive::Function};
/// fn require_rule(_: impl Rules<Function<((Tick,), ())>>) {}
/// require_rule(|_: Tick| {});
/// ```
#[doc(hidden)]
pub struct Function<M>(std::marker::PhantomData<M>);

/// Marker used to infer a tuple of rules.
/// ```
/// use thrum::{Rules, __derive::{Function, Group}};
/// type NoArgs = Function<((), ())>;
/// fn require_pair(_: impl Rules<Group<(NoArgs, NoArgs)>>) {}
/// require_pair((|| {}, || {}));
/// ```
#[doc(hidden)]
pub struct Group<M>(std::marker::PhantomData<M>);

/// Parameter and return-type information inferred from a plain Rust function.
/// ```
/// use thrum::{Tick, __derive::RuleFunction};
/// fn require_tick(_: impl RuleFunction<(Tick,), ()>) {}
/// require_tick(|_: Tick| {});
/// ```
#[doc(hidden)]
pub trait RuleFunction<M, O> {
    /// Compile the function's parameter adapters.
    /// ```
    /// use thrum::{Tick, __derive::RuleFunction};
    /// fn tick_number(tick: Tick) -> thrum::TickNumber { tick.number() }
    /// let _compiled = tick_number.compile();
    /// ```
    fn compile(self) -> Invocation<O>;
}

/// A function whose parameters permit reads alone.
/// ```
/// use thrum::{Tick, __derive::ReadFunction};
/// fn require_condition(_: impl ReadFunction<(Tick,), bool>) {}
/// require_condition(|_: Tick| true);
/// ```
#[doc(hidden)]
pub trait ReadFunction<M, O>: ScheduledFunction<M, O> {}

/// A scheduled rule cannot mutate captured state.
/// ```
/// use thrum::{Tick, __derive::ScheduledFunction};
/// fn require_scheduled(_: impl ScheduledFunction<(Tick,), ()>) {}
/// let limit = 5;
/// require_scheduled(move |_: Tick| assert_eq!(limit, 5));
/// ```
#[doc(hidden)]
pub trait ScheduledFunction<M, O>: RuleFunction<M, O> {}

impl<F, O: 'static> RuleFunction<(), O> for F
where
    F: FnMut() -> O + Send + 'static,
{
    fn compile(mut self) -> Invocation<O> {
        let function: &dyn Any = &self;
        Invocation {
            name: type_name::<F>(),
            callable: TypeId::of::<F>(),
            pointer: function
                .downcast_ref::<fn() -> O>()
                .map(|function| *function as usize),
            stream: stream::<F>(),
            signature: Signature::default(),
            invoke: Box::new(move |_, _| Ok(self())),
        }
    }
}

impl<F: Fn() -> O + Copy + Send + 'static, O: 'static> ScheduledFunction<(), O> for F {}
impl<F: Fn() -> O + Copy + Send + 'static, O: 'static> ReadFunction<(), O> for F {}

fn stream<F>() -> u64 {
    type_name::<F>()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

macro_rules! functions {
    ($(($($parameter:ident $value:ident $index:tt $borrow:lifetime),*)),+ $(,)?) => {$(
        impl<F, O: 'static, $($parameter,)*> RuleFunction<($($parameter,)*), O> for F
        where
            F: FnMut($($parameter),*) -> O + for<'t> FnMut($($parameter::Item<'t>),*) -> O + Send + 'static,
            $($parameter: Param + 'static,)*
        {
            fn compile(mut self) -> Invocation<O> {
                let mut list = ParameterList::default();
                $($parameter::describe(&format!("parameter {} ({})", $index + 1, type_name::<$parameter>()), &mut list);)*
                let name = type_name::<F>();
                let loan_types = list.0.loan_types();
                let mut loans = Loans::default();
                let mut state: ($($parameter::State,)*) = Default::default();
                let function: &dyn Any = &self;
                let pointer = function
                    .downcast_ref::<fn($($parameter),*) -> O>()
                    .map(|function| *function as usize)
                    .or_else(|| {
                        // A function type can quantify each parameter's loan
                        // separately or use a single lifetime for the loans.
                        function
                            .downcast_ref::<for<$($borrow),*> fn($($parameter::Item<$borrow>),*) -> O>()
                            .map(|function| *function as usize)
                    })
                    .or_else(|| {
                        function
                            .downcast_ref::<for<'t> fn($($parameter::Item<'t>),*) -> O>()
                            .map(|function| *function as usize)
                    });
                Invocation {
                    name,
                    callable: TypeId::of::<F>(),
                    pointer,
                    stream: stream::<F>(),
                    signature: list.0,
                    invoke: Box::new(move |context, parts| {
                        let read = ReadContext {
                            world: None,
                            loans: None,
                            bindings: context.bindings,
                            candidate: context.candidate,
                            entities: parts.entities,
                            executor: parts.executor,
                            tick: parts.tick,
                            name,
                            seed: parts.seed,
                            stream: parts.stream,
                            stage: parts.stage,
                            names: parts.names,
                        };
                        {
                            let world = context.world.read();
                            let preparing = ReadContext { world: Some(&world), ..read };
                            $($parameter::read(&mut state.$index, &preparing)?;)*
                        }
                        context.bindings.prepare_loans(context.world, &loan_types, &mut loans);
                        let read = ReadContext { loans: Some(&loans), ..read };
                        let output = catch_unwind(AssertUnwindSafe(|| {
                            $(let $value = $parameter::item(&mut state.$index, &read);)*
                            self($($value),*)
                        }));
                        loans.clear();
                        let journal = Ok(());
                        $(let journal = journal.and($parameter::journal(&mut state.$index, context));)*
                        let output = output.unwrap_or_else(|panic| resume_unwind(panic));
                        journal?;
                        $($parameter::write(&mut state.$index, context)?;)*
                        Ok(output)
                    }),
                }
            }
        }

        impl<F, O, $($parameter,)*> ScheduledFunction<($($parameter,)*), O> for F
        where
            F: Fn($($parameter),*) -> O + Copy + RuleFunction<($($parameter,)*), O>,
        {}

        impl<F, O, $($parameter,)*> ReadFunction<($($parameter,)*), O> for F
        where
            F: ScheduledFunction<($($parameter,)*), O>,
            $($parameter: ReadOnly,)*
        {}
    )+};
}

functions!(
    (A a 0 'a0),
    (A a 0 'a0, B b 1 'a1),
    (A a 0 'a0, B b 1 'a1, C c 2 'a2),
    (A a 0 'a0, B b 1 'a1, C c 2 'a2, D d 3 'a3),
    (A a 0 'a0, B b 1 'a1, C c 2 'a2, D d 3 'a3, E e 4 'a4),
    (A a 0 'a0, B b 1 'a1, C c 2 'a2, D d 3 'a3, E e 4 'a4, F0 f 5 'a5),
);

/// An ordinary rule returns nothing or a Thrum result.
/// ```
/// use thrum::__derive::RuleOutput;
/// fn require_output(_: impl RuleOutput) {}
/// require_output(());
/// require_output(thrum::Result::Ok(()));
/// ```
#[doc(hidden)]
pub trait RuleOutput: 'static {
    /// Resolve the outcome of the rule.
    /// ```
    /// use thrum::__derive::RuleOutput;
    /// assert!(().result().is_ok());
    /// let refused: thrum::Result = Err(thrum::Error::invalid("score", "overflow"));
    /// assert_eq!(refused.result().unwrap_err().code(), "T0401");
    /// ```
    /// # Errors
    /// A rule can refuse a tick.
    fn result(self) -> Result;
}

impl RuleOutput for () {
    fn result(self) -> Result {
        Ok(())
    }
}

impl RuleOutput for Result {
    fn result(self) -> Result {
        self
    }
}

/// One registered rule and its activation condition.
/// ```
/// use thrum::{Rules, __derive::Rule};
/// let mut rules: Vec<Rule> = Vec::new();
/// (|| {}).into_rules(&mut rules);
/// assert_eq!(rules.len(), 1);
/// ```
#[doc(hidden)]
pub struct Rule {
    pub(crate) invoke: Invocation<()>,
    pub(crate) cadence: Option<Seconds>,
    pub(crate) conditions: Vec<Arc<Condition>>,
}

pub(crate) struct Condition {
    pub(crate) name: &'static str,
    pub(crate) signature: Signature,
    state: Mutex<ConditionState>,
}

struct ConditionState {
    invocation: Invocation<bool>,
    value: Option<(u64, bool)>,
    identified: bool,
}

impl Condition {
    fn new(invocation: Invocation<bool>) -> Self {
        Self {
            name: invocation.name,
            signature: invocation.signature.clone(),
            state: Mutex::new(ConditionState {
                invocation,
                value: None,
                identified: false,
            }),
        }
    }

    pub(crate) fn identify(&self, registration: u64, streams: &mut RuleStreams) {
        let mut state = self.state.lock().expect("condition scratch");
        if !state.identified {
            state.invocation.identify_scheduled(registration, streams);
            state.identified = true;
        }
    }

    pub(crate) fn evaluate(
        &self,
        attempt: u64,
        invoke: impl FnOnce(&mut Invocation<bool>) -> Result<bool>,
    ) -> Result<bool> {
        let mut state = self.state.lock().expect("condition scratch");
        if let Some((current, value)) = state.value
            && current == attempt
        {
            return Ok(value);
        }
        let value = invoke(&mut state.invocation)?;
        state.value = Some((attempt, value));
        Ok(value)
    }
}

impl Rule {
    pub(crate) fn new<M, O: RuleOutput>(function: impl ScheduledFunction<M, O>) -> Self {
        let Invocation {
            name,
            signature,
            stream,
            callable,
            pointer,
            mut invoke,
        } = function.compile();
        Self {
            invoke: Invocation {
                name,
                signature,
                stream,
                callable,
                pointer,
                invoke: Box::new(move |ctx, parts| invoke(ctx, parts)?.result()),
            },
            cadence: None,
            conditions: Vec::new(),
        }
    }
}

/// A plain function, configured rule, or nested tuple of rules.
/// ```
/// use thrum::{Rules, Scene};
/// fn add<M>(scene: &mut Scene, rules: impl Rules<M>) {
///     scene.rules(rules);
/// }
/// add(&mut Scene::default(), (|| {}, || {}));
/// ```
pub trait Rules<M> {
    /// Build the ordered list of registered rules.
    /// ```
    /// use thrum::Rules;
    /// let mut rules = Vec::new();
    /// (|| {}, || {}).into_rules(&mut rules);
    /// assert_eq!(rules.len(), 2);
    /// ```
    #[doc(hidden)]
    fn into_rules(self, out: &mut Vec<Rule>);

    /// Run this rule or group once at the end of each interval.
    /// ```
    /// use thrum::{Rules, Tick, seconds};
    /// fn advance(_: Tick) {}
    /// let _ = advance.every(seconds(2));
    /// ```
    fn every(self, interval: Seconds) -> ConfiguredRules
    where
        Self: Sized,
    {
        let mut rules = Vec::new();
        self.into_rules(&mut rules);
        for rule in &mut rules {
            rule.cadence = Some(interval);
        }
        ConfiguredRules(rules)
    }

    /// Run this rule or group in ticks where a read-only condition holds.
    /// ```
    /// use thrum::{Rules, Tick};
    /// fn advance(_: Tick) {}
    /// fn active(_: Tick) -> bool { true }
    /// let _ = (advance, advance).when(active);
    /// ```
    fn when<C, CM>(self, condition: C) -> ConfiguredRules
    where
        Self: Sized,
        C: ReadFunction<CM, bool>,
    {
        let mut rules = Vec::new();
        self.into_rules(&mut rules);
        let condition = Arc::new(Condition::new(condition.compile()));
        for rule in &mut rules {
            rule.conditions.push(Arc::clone(&condition));
        }
        ConfiguredRules(rules)
    }
}

/// An ordered rule or group with a cadence or condition.
/// ```
/// use thrum::{ConfiguredRules, Rules, Scene, seconds};
/// let updates: ConfiguredRules = (|| {}).every(seconds(2)).when(|| true);
/// Scene::<()>::default().rules(updates);
/// ```
pub struct ConfiguredRules(Vec<Rule>);

impl Rules<()> for ConfiguredRules {
    fn into_rules(self, out: &mut Vec<Rule>) {
        out.extend(self.0);
    }
}

impl<F, M, O> Rules<Function<(M, O)>> for F
where
    F: ScheduledFunction<M, O>,
    O: RuleOutput,
{
    fn into_rules(self, out: &mut Vec<Rule>) {
        out.push(Rule::new(self));
    }
}

macro_rules! rule_tuples {
    ($(($($ty:ident $marker:ident $binding:ident),+)),+ $(,)?) => {$(
        impl<$($ty: Rules<$marker>, $marker,)+> Rules<Group<($($marker,)+)>> for ($($ty,)+) {
            fn into_rules(self, out: &mut Vec<Rule>) {
                let ($($binding,)+) = self;
                $($binding.into_rules(out);)+
            }
        }
    )+};
}

rule_tuples!((A MA a), (A MA a, B MB b), (A MA a, B MB b, C MC c), (A MA a, B MB b, C MC c, D MD d), (A MA a, B MB b, C MC c, D MD d, E ME e), (A MA a, B MB b, C MC c, D MD d, E ME e, F MF f), (A MA a, B MB b, C MC c, D MD d, E ME e, F MF f, G MG g));
