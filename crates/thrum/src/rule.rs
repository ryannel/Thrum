use std::{
    any::type_name,
    sync::{Arc, Mutex},
};

use crate::{Result, Seconds};

pub(crate) mod access;
pub(crate) mod params;
pub(crate) mod query;

use access::Signature;
use params::{Context, Param, ParameterList, ReadContext, ReadOnly};

macro_rules! internal_doc {
    ($doc:literal) => {
        concat!($doc, "\n```\nfn rule(_: thrum::Tick) {}\n```\n")
    };
}

type Invoke<O> = dyn FnMut(&mut Context<'_, '_>, &ReadContextParts<'_>) -> Result<O> + Send;

#[doc = internal_doc!("A compiled callback prepared from a Rust rule.")]
#[doc(hidden)]
pub struct Invocation<O> {
    pub(crate) name: &'static str,
    pub(crate) signature: Signature,
    pub(crate) invoke: Box<Invoke<O>>,
}

pub(crate) struct ReadContextParts<'a> {
    pub(crate) entities: &'a [(crate::Entity, thrum_core::Handle)],
    pub(crate) executor: thrum_core::sim::Executor,
    pub(crate) tick: crate::Tick,
    pub(crate) seed: u64,
    pub(crate) stream: u32,
    pub(crate) stage: crate::scene::Landing,
    pub(crate) names: &'a Arc<crate::scene::Names>,
}

impl<O> Invocation<O> {
    pub(crate) fn call(
        &mut self,
        context: &mut Context<'_, '_>,
        parts: &ReadContextParts<'_>,
    ) -> Result<O> {
        (self.invoke)(context, parts)
    }
}

#[doc = internal_doc!("Marker used to infer rule parameter types.")]
#[doc(hidden)]
pub struct Function<M>(std::marker::PhantomData<M>);

#[doc = internal_doc!("Marker used to infer one rule or a tuple of rules.")]
#[doc(hidden)]
pub struct Group<M>(std::marker::PhantomData<M>);

#[doc = internal_doc!("Parameter and return-type information inferred from a plain Rust function.")]
#[doc(hidden)]
pub trait RuleFunction<M, O> {
    #[doc = internal_doc!("Compile the function's parameter adapters.")]
    fn compile(self) -> Invocation<O>;
}

#[doc = internal_doc!("A function whose parameters permit reads alone.")]
#[doc(hidden)]
pub trait ReadFunction<M, O>: RuleFunction<M, O> {}

impl<F, O> RuleFunction<(), O> for F
where
    F: FnMut() -> O + Send + 'static,
{
    fn compile(mut self) -> Invocation<O> {
        Invocation {
            name: type_name::<F>(),
            signature: Signature::default(),
            invoke: Box::new(move |_, _| Ok(self())),
        }
    }
}

impl<F, O> ReadFunction<(), O> for F where F: RuleFunction<(), O> {}

macro_rules! functions {
    ($(($($parameter:ident $value:ident $index:tt),*)),+ $(,)?) => {$(
        impl<F, O, $($parameter,)*> RuleFunction<($($parameter,)*), O> for F
        where
            F: FnMut($($parameter),*) -> O + for<'t> FnMut($($parameter::Item<'t>),*) -> O + Send + 'static,
            $($parameter: Param + 'static,)*
        {
            fn compile(mut self) -> Invocation<O> {
                let mut list = ParameterList::default();
                $($parameter::describe(&format!("parameter {} ({})", $index + 1, type_name::<$parameter>()), &mut list);)*
                let name = type_name::<F>();
                let mut state: ($($parameter::State,)*) = Default::default();
                Invocation {
                    name,
                    signature: list.0,
                    invoke: Box::new(move |context, parts| {
                        let output = {
                            let read = ReadContext {
                                world: &context.world.read(),
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
                            $($parameter::read(&mut state.$index, &read)?;)*
                            $(let $value = $parameter::item(&mut state.$index, &read);)*
                            self($($value),*)
                        };
                        $($parameter::write(&mut state.$index, context)?;)*
                        Ok(output)
                    }),
                }
            }
        }

        impl<F, O, $($parameter,)*> ReadFunction<($($parameter,)*), O> for F
        where
            F: RuleFunction<($($parameter,)*), O>,
            $($parameter: ReadOnly,)*
        {}
    )+};
}

functions!( (A a 0), (A a 0, B b 1), (A a 0, B b 1, C c 2), (A a 0, B b 1, C c 2, D d 3), (A a 0, B b 1, C c 2, D d 3, E e 4), (A a 0, B b 1, C c 2, D d 3, E e 4, F0 f 5));

#[doc = internal_doc!("An ordinary rule returns nothing or a Thrum result.")]
#[doc(hidden)]
pub trait RuleOutput: 'static {
    #[doc = internal_doc!("Resolve the outcome of the rule.")]
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

#[doc = internal_doc!("One registered rule and its activation condition.")]
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
}

impl Condition {
    fn new(invocation: Invocation<bool>) -> Self {
        Self {
            name: invocation.name,
            signature: invocation.signature.clone(),
            state: Mutex::new(ConditionState {
                invocation,
                value: None,
            }),
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
    pub(crate) fn new<M, O: RuleOutput>(function: impl RuleFunction<M, O>) -> Self {
        let Invocation {
            name,
            signature,
            mut invoke,
        } = function.compile();
        Self {
            invoke: Invocation {
                name,
                signature,
                invoke: Box::new(move |ctx, parts| invoke(ctx, parts)?.result()),
            },
            cadence: None,
            conditions: Vec::new(),
        }
    }
}

#[doc = internal_doc!("A plain function, configured rule, or nested tuple of rules.")]
pub trait Rules<M> {
    #[doc = internal_doc!("Build the ordered list of registered rules.")]
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

#[doc = internal_doc!("An ordered rule or group with a cadence or condition.")]
pub struct ConfiguredRules(Vec<Rule>);

impl Rules<()> for ConfiguredRules {
    fn into_rules(self, out: &mut Vec<Rule>) {
        out.extend(self.0);
    }
}

impl<F, M, O> Rules<Function<(M, O)>> for F
where
    F: RuleFunction<M, O>,
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
