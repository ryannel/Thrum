use std::{
    any::{TypeId, type_name},
    marker::PhantomData,
    sync::Arc,
};

use crate::{
    Error, PhaseType, Resource, Result, SceneType, Seconds,
    rule::{Invocation, ReadFunction, Rule, Rules},
    runtime::ValueRef,
};

mod phase;
mod program;
pub use phase::{Phase, PhaseMut, PhaseRequests, SceneRequests, Scenes};
pub(crate) use program::Program;

macro_rules! example {
    ($description:literal) => {
        concat!($description, "\n```\nuse thrum::prelude::*;\n#[derive(Scene, Default)] struct Play;\nfn play(scene: &mut Scene<Play>) { scene.rules(|| {}); }\nlet _game = Game::new(\"Example\").scene(play);\n```\n")
    };
}

#[derive(Default)]
pub(crate) struct Names(pub(crate) Vec<SceneNames>);

pub(crate) struct SceneNames {
    pub(crate) id: TypeId,
    pub(crate) phase_values: Vec<ValueRef>,
}

impl Names {
    pub(crate) fn scene<S: SceneType>(&self) -> Option<usize> {
        self.0
            .iter()
            .position(|definition| definition.id == TypeId::of::<S>())
    }

    pub(crate) fn phase<P: PhaseType>(&self, scene: usize, value: &P) -> Option<usize> {
        self.0
            .get(scene)?
            .phase_values
            .iter()
            .position(|known| known.any().downcast_ref() == Some(value))
    }

    pub(crate) fn phase_value<P: PhaseType>(&self, scene: usize, phase: usize) -> Option<&P> {
        self.0
            .get(scene)?
            .phase_values
            .get(phase)?
            .any()
            .downcast_ref()
    }
}

#[derive(Default)]
pub(crate) struct Definition {
    pub(crate) label: &'static str,
    pub(crate) resources: Vec<ValueRef>,
    pub(crate) phase_values: Vec<ValueRef>,
    pub(crate) first_phase: Option<usize>,
    pub(crate) phases: Vec<PhaseDefinition>,
    pub(crate) main: Vec<(Option<usize>, Rule)>,
    pub(crate) enter: Vec<Rule>,
    pub(crate) exit: Vec<Rule>,
    pub(crate) invalid: Option<Error>,
}

#[derive(Default)]
pub(crate) struct PhaseDefinition {
    pub(crate) enter: Vec<Rule>,
    pub(crate) exit: Vec<Rule>,
    pub(crate) duration: Option<Duration>,
    pub(crate) expiry: Option<Expiry>,
}

#[doc = example!("Prepared phase duration.") ]
#[doc(hidden)]
pub enum Duration {
    /// A fixed phase duration.
    /// ```
    /// let _ = thrum::__derive::Duration::Fixed(thrum::seconds(2));
    /// ```
    Fixed(
        /// The fixed interval.
        /// ```
        /// let _ = thrum::__derive::Duration::Fixed(thrum::seconds(2));
        /// ```
        Seconds,
    ),
    /// A duration calculated from the world on phase entry.
    /// ```
    /// use thrum::{__derive::{Duration, RuleFunction}, seconds};
    /// let _ = Duration::Read((|| seconds(2)).compile());
    /// ```
    Read(
        /// The function prepared for entry.
        /// ```
        /// use thrum::{__derive::{Duration, RuleFunction}, seconds};
        /// let _ = Duration::Read((|| seconds(2)).compile());
        /// ```
        Invocation<Seconds>,
    ),
}

pub(crate) enum Expiry {
    Phase(usize),
    Run(Vec<Rule>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Landing {
    Rules,
    LeavingScene,
    LeavingPhase,
    EnteringScene,
    EnteringPhase,
}

#[doc = example!("Describe a scene with its own rules and phase blocks.")]
pub struct Scene<S: SceneType = ()> {
    definition: Definition,
    marker: PhantomData<S>,
}

impl<S: SceneType> Default for Scene<S> {
    fn default() -> Self {
        Self {
            definition: Definition {
                label: type_name::<S>(),
                ..Definition::default()
            },
            marker: PhantomData,
        }
    }
}

impl<S: SceneType> Scene<S> {
    #[doc = example!("Provide a resource when this scene is entered.")]
    pub fn resource(&mut self, value: impl Resource) {
        self.definition.resources.push(Arc::new(value));
    }

    #[doc = example!("Append rules that run as the scene is entered.")]
    pub fn on_enter<M>(&mut self, rules: impl Rules<M>) {
        rules.into_rules(&mut self.definition.enter);
    }

    #[doc = example!("Append rules that run before this scene is left.")]
    pub fn on_exit<M>(&mut self, rules: impl Rules<M>) {
        rules.into_rules(&mut self.definition.exit);
    }

    #[doc = example!("Append rules in written order.")]
    pub fn rules<M>(&mut self, rules: impl Rules<M>) {
        let mut list = Vec::new();
        rules.into_rules(&mut list);
        self.definition
            .main
            .extend(list.into_iter().map(|rule| (None, rule)));
    }

    #[doc = example!("Append a description of part of this scene.")]
    pub fn add(&mut self, part: impl FnOnce(&mut Self)) {
        part(self);
    }

    #[doc = example!("Declare the first phase of this scene.")]
    pub fn phases<P: PhaseType>(&mut self, first: P) {
        if self.definition.first_phase.is_some() {
            self.definition.invalid = Some(Error::refused(
                "T0205",
                self.definition.label,
                "scene.phases(..) was called twice",
                "Declare the starting phase once.",
            ));
        } else {
            self.definition.first_phase = Some(phase_index(&mut self.definition, first));
        }
    }

    #[doc = example!("Describe a phase block in this position in the scene.")]
    pub fn phase<P: PhaseType>(&mut self, phase: P) -> PhaseBlock<'_, P> {
        let index = phase_index(&mut self.definition, phase);
        PhaseBlock {
            definition: &mut self.definition,
            index,
            marker: PhantomData,
        }
    }

    pub(crate) fn build(mut self) -> Result<(SceneNames, Definition)> {
        if let Some(error) = self.definition.invalid.take() {
            return Err(error);
        }
        let names = SceneNames {
            id: TypeId::of::<S>(),
            phase_values: std::mem::take(&mut self.definition.phase_values),
        };
        Ok((names, self.definition))
    }
}

fn phase_index<P: PhaseType>(definition: &mut Definition, phase: P) -> usize {
    if let Some(index) = definition
        .phase_values
        .iter()
        .position(|known| known.any().downcast_ref() == Some(&phase))
    {
        return index;
    }
    let index = definition.phase_values.len();
    definition.phase_values.push(Arc::new(phase));
    definition.phases.push(PhaseDefinition::default());
    index
}

#[doc = example!("Describe the rules, entry, exit and length of one phase.")]
pub struct PhaseBlock<'a, P> {
    definition: &'a mut Definition,
    index: usize,
    marker: PhantomData<P>,
}

impl<P: PhaseType> PhaseBlock<'_, P> {
    #[doc = example!("Append rules that run as this phase is entered.")]
    #[expect(
        clippy::return_self_not_must_use,
        reason = "each phase declaration takes effect even when it ends a chain"
    )]
    pub fn on_enter<M>(self, rules: impl Rules<M>) -> Self {
        rules.into_rules(&mut self.definition.phases[self.index].enter);
        self
    }

    #[doc = example!("Append rules that run as this phase is left.")]
    #[expect(
        clippy::return_self_not_must_use,
        reason = "each phase declaration takes effect even when it ends a chain"
    )]
    pub fn on_exit<M>(self, rules: impl Rules<M>) -> Self {
        rules.into_rules(&mut self.definition.phases[self.index].exit);
        self
    }

    #[doc = example!("Append rules in the phase block's written position.")]
    #[expect(
        clippy::return_self_not_must_use,
        reason = "each phase declaration takes effect even when it ends a chain"
    )]
    pub fn rules<M>(self, rules: impl Rules<M>) -> Self {
        let mut list = Vec::new();
        rules.into_rules(&mut list);
        self.definition
            .main
            .extend(list.into_iter().map(|rule| (Some(self.index), rule)));
        self
    }

    #[doc = example!("Set this phase's length to a duration or a read-only function.")]
    #[expect(
        clippy::return_self_not_must_use,
        reason = "each phase declaration takes effect even when it ends a chain"
    )]
    pub fn lasts<M>(self, length: impl PhaseDuration<M>) -> Self {
        self.definition.phases[self.index].duration = Some(length.duration());
        self
    }

    #[doc = example!("Request another phase when this phase reaches its deadline.")]
    #[expect(
        clippy::return_self_not_must_use,
        reason = "the expiry is registered even when it ends a chain"
    )]
    pub fn then(self, phase: P) -> Self {
        let next = phase_index(self.definition, phase);
        self.definition.phases[self.index].expiry = Some(Expiry::Phase(next));
        self
    }

    #[doc = example!("Run a rule once after the deadline tick's other rules.")]
    pub fn then_run<M>(self, rule: impl Rules<M>) {
        let mut list = Vec::new();
        rule.into_rules(&mut list);
        self.definition.phases[self.index].expiry = Some(Expiry::Run(list));
    }
}

#[doc = example!("A duration expression or read-only function used by lasts.")]
#[doc(hidden)]
pub trait PhaseDuration<M> {
    #[doc = example!("Prepare the duration for phase entry.")]
    fn duration(self) -> Duration;
}

impl PhaseDuration<()> for Seconds {
    fn duration(self) -> Duration {
        Duration::Fixed(self)
    }
}

impl<M, F: ReadFunction<M, Seconds>> PhaseDuration<crate::rule::Function<M>> for F {
    fn duration(self) -> Duration {
        Duration::Read(self.compile())
    }
}

#[doc = example!("One scene description function or a tuple of functions in written order.")]
#[doc(hidden)]
pub trait SceneDescription<S: SceneType, M> {
    #[doc = example!("Write the entries of this scene.")]
    fn describe(self, scene: &mut Scene<S>);
}

impl<S: SceneType, F: FnOnce(&mut Scene<S>)> SceneDescription<S, ()> for F {
    fn describe(self, scene: &mut Scene<S>) {
        self(scene);
    }
}

macro_rules! descriptions {
    ($(($($ty:ident $marker:ident $index:tt),+)),+ $(,)?) => {$(
        impl<S: SceneType, $($ty: SceneDescription<S, $marker>, $marker),+> SceneDescription<S, crate::rule::Group<($($marker,)+)>> for ($($ty,)+) {
            fn describe(self, scene: &mut Scene<S>) { $(self.$index.describe(scene);)+ }
        }
    )+};
}

descriptions!((A MA 0, B MB 1), (A MA 0, B MB 1, C MC 2), (A MA 0, B MB 1, C MC 2, D MD 3));
