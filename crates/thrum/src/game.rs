use std::{
    any::{TypeId, type_name},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use thrum_core::{
    WorldEpoch,
    sim::{ExecutionMode, GameRuntime, HistoryError, HistoryGame, RuntimeHistory},
};

use crate::{
    Component, Data, Entities, Entity, Error, Event, Filter, PhaseType, ResMut, Resource, Result,
    Scene, SceneType, Seconds, Spawn,
    rule::{
        Invocation, ReadContextParts, RuleFunction, access::Column, params::Context,
        query::matching,
    },
    runtime::{
        Authority, Candidate, EventBuffers, Operation, State, Transition, ValueRef,
        collect_entities, operation_result,
    },
    scene::{Definition, Names, Program, SceneDescription, SceneNames},
};

macro_rules! example {
    ($description:literal) => {
        concat!($description, "\n```\nuse thrum::prelude::*;\nfn play(scene: &mut Scene) { scene.rules(|| {}); }\nlet mut game = Game::new(\"Example\").scene(play).test().unwrap();\ngame.step().unwrap();\n```\n")
    };
}

struct Description {
    name: String,
    scenes: Vec<Result<(SceneNames, Definition)>>,
    columns: Vec<Column>,
    resources: Vec<ValueRef>,
    capacity: u32,
    rate: u32,
}

#[doc = example!("A game described in Rust, ready to start in a test.")]
#[must_use]
pub struct Game {
    description: Description,
    first: fn() -> ValueRef,
}

#[doc = example!("A game before its first scene is registered.")]
#[must_use]
pub struct NewGame(Description);

impl Game {
    /// Describe a new game.
    /// ```
    /// use thrum::{Game, Scene};
    /// let _game = Game::new("Rockfall").scene(|_: &mut Scene| {});
    /// ```
    #[expect(
        clippy::new_ret_no_self,
        reason = "the first scene is statically required to have a default"
    )]
    pub fn new(name: impl Into<String>) -> NewGame {
        NewGame(Description {
            name: name.into(),
            scenes: Vec::new(),
            columns: Vec::new(),
            resources: Vec::new(),
            capacity: 256,
            rate: 60,
        })
    }

    #[doc = example!("Append another scene description.")]
    pub fn scene<S: SceneType, M>(mut self, description: impl SceneDescription<S, M>) -> Self {
        self.description.scene(description);
        self
    }

    #[doc = example!("Start a headless game in the first scene's default value.")]
    /// # Errors
    /// The description or its initial scene is invalid.
    pub fn test(self) -> Result<TestGame> {
        {
            let first = (self.first)();
            self.start(first, 0)
        }
    }

    #[doc = example!("Start a headless game in a named scene and value.")]
    /// # Errors
    /// The requested scene or the game description is invalid.
    pub fn test_in(self, scene: impl SceneType) -> Result<TestGame> {
        self.start(Arc::new(scene), 0)
    }

    #[doc = example!("Set the fixtures of a windowless test before starting it.")]
    pub fn test_setup(self) -> TestSetup {
        TestSetup {
            game: self,
            seed: 0,
        }
    }

    fn start(self, value: ValueRef, seed: u64) -> Result<TestGame> {
        let Description {
            name,
            scenes,
            columns,
            resources,
            capacity,
            rate,
        } = self.description;
        if rate == 0 || rate > 1_000_000_000 {
            return Err(Error::refused(
                "T0200",
                name,
                "invalid tick rate",
                "Use a positive tick rate of at most one billion ticks per second.",
            ));
        }
        let (names, scenes): (Vec<_>, Vec<_>) = scenes
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .unzip();
        let index = names
            .iter()
            .position(|known| known.id == value.id())
            .ok_or_else(|| {
                Error::refused(
                    "T0305",
                    value.name(),
                    "the requested initial scene was never registered",
                    "Add its description with Game::scene(..).",
                )
            })?;
        let global: Vec<TypeId> = resources.iter().map(|value| value.id()).collect();
        let (mut program, registry, systems) =
            Program::compile(Names(names), scenes, columns, &global, rate)?;
        let mut runtime = GameRuntime::new_published_with_rule_boundaries(
            WorldEpoch(0),
            capacity as usize,
            registry,
            systems,
        )
        .map_err(|error| {
            Error::refused(
                "T0200",
                name,
                error.to_string(),
                "Correct the game description and retry.",
            )
        })?;
        runtime.set_execution_mode(ExecutionMode::Rayon);
        program.bind(&runtime);
        let mut state = State {
            seed,
            ..State::default()
        };
        for resource in resources {
            state.resources.install(resource.as_ref(), false);
        }
        let mut authority = Authority {
            state,
            events: EventBuffers::default(),
            operation: Mutex::new(Operation::Start(index, value)),
            program,
        };
        let mut candidate = Candidate::default();
        runtime.publish_initial(&mut authority, &mut candidate);
        runtime
            .edit_current(&mut authority, &mut candidate)
            .map_err(operation_result)?;
        authority.events.collect(&mut candidate.events);
        *authority.operation.get_mut().expect("operation") = Operation::Tick;
        let history = new_history(&runtime, &authority);
        Ok(TestGame {
            runtime,
            authority,
            candidate,
            history,
        })
    }
}

#[doc = example!("A scene whose type can supply the game's starting value.")]
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "T0103: first scene `{Self}` cannot supply a default value",
    label = "the first scene needs a default value",
    note = "derive or implement Default on this scene, or register a defaultable scene first"
)]
pub trait FirstScene: SceneType {
    #[doc = example!("Get the starting value for the first scene.")]
    fn first_value() -> Self;
}

impl<S: SceneType + Default> FirstScene for S {
    fn first_value() -> Self {
        Self::default()
    }
}

impl NewGame {
    /// Register the first scene, whose type supplies the starting value.
    ///
    /// ```
    /// use thrum::{Game, Scene};
    /// #[derive(Scene, Default)] struct Play;
    /// let _game = Game::new("Rockfall").scene(|_: &mut Scene<Play>| {});
    /// ```
    pub fn scene<S: FirstScene, M>(mut self, description: impl SceneDescription<S, M>) -> Game {
        self.0.scene(description);
        Game {
            description: self.0,
            first: || Arc::new(S::first_value()),
        }
    }
}

impl Description {
    fn scene<S: SceneType, M>(&mut self, description: impl SceneDescription<S, M>) {
        let mut scene = Scene::default();
        description.describe(&mut scene);
        self.scenes.push(scene.build());
    }
}

macro_rules! settings {
    ($game:ident, $description:tt) => {
        impl $game {
            #[doc = example!("Provide a game resource that persists across scenes.")]
            pub fn resource(mut self, value: impl Resource) -> Self {
                self.$description.resources.push(Arc::new(value));
                self
            }

            #[doc = example!("Register a component absent from all rule signatures.")]
            pub fn component<T: Component>(mut self) -> Self {
                let column = Column::of::<T>();
                if !self
                    .$description
                    .columns
                    .iter()
                    .any(|known| known.key == column.key)
                {
                    self.$description.columns.push(column);
                }
                self
            }

            #[doc = example!("Set the maximum number of live entities.")]
            pub fn max_entities(mut self, capacity: u32) -> Self {
                self.$description.capacity = capacity;
                self
            }

            #[doc = example!("Set the number of ticks per simulated second.")]
            pub fn tick_rate(mut self, hz: u32) -> Self {
                self.$description.rate = hz;
                self
            }
        }
    };
}

settings!(NewGame, 0);
settings!(Game, description);

#[doc = example!("Fixtures for a headless game test.")]
#[must_use]
pub struct TestSetup {
    game: Game,
    seed: u64,
}

impl TestSetup {
    #[doc = example!("Choose the seed used by Random in this test.")]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    #[doc = example!("Start this test with the configured fixtures.")]
    /// # Errors
    /// The description or its initial scene is invalid.
    pub fn start(self) -> Result<TestGame> {
        let first = (self.game.first)();
        self.game.start(first, self.seed)
    }
}

#[doc = example!("The headless game harness, stepped by a test without a window.")]
pub struct TestGame {
    runtime: GameRuntime<Authority>,
    authority: Authority,
    candidate: Candidate,
    history: RuntimeHistory<State, ()>,
}

impl TestGame {
    #[doc = example!("Advance the game by one tick.")]
    /// # Errors
    /// A rule or transition refused the tick.
    pub fn step(&mut self) -> Result {
        self.history
            .step(
                &mut self.runtime,
                &mut self.authority,
                &mut self.candidate,
                &(),
            )
            .map_err(history_error)?;
        self.authority.events.collect(&mut self.candidate.events);
        Ok(())
    }

    #[doc = example!("Advance the game by the requested number of ticks.")]
    /// # Errors
    /// A rule or transition refused one of the ticks.
    pub fn step_ticks(&mut self, count: u64) -> Result {
        for _ in 0..count {
            self.step()?;
        }
        Ok(())
    }

    #[doc = example!("Advance through the requested time without waiting on a clock.")]
    /// # Errors
    /// A rule or transition refused one of the ticks.
    pub fn step_for(&mut self, time: Seconds) -> Result {
        self.step_ticks(time.ticks(self.authority.program.rate()))
    }

    #[doc = example!("Advance to the first state accepted by the predicate, within the time limit.")]
    /// # Errors
    /// The predicate was unsatisfied before the limit, or a rule or the predicate refused.
    pub fn step_until(
        &mut self,
        limit: Seconds,
        mut condition: impl FnMut(&Self) -> Result<bool>,
    ) -> Result {
        if condition(self)? {
            return Ok(());
        }
        for _ in 0..limit.ticks(self.authority.program.rate()) {
            self.step()?;
            if condition(self)? {
                return Ok(());
            }
        }
        Err(Error::refused(
            "T0501",
            format!("tick {}", self.runtime.tick()),
            "the step_until predicate was false at the time limit",
            "Check the predicate or allow more simulated time.",
        ))
    }

    #[doc = example!("Return to an earlier tick in the test's retained history.")]
    /// # Errors
    /// The requested tick is outside the retained history.
    pub fn rewind(&mut self, ticks: usize) -> Result {
        self.history
            .rewind(&mut self.runtime, &mut self.authority, ticks)
            .map_err(history_error)?;
        self.authority.events = EventBuffers::default();
        Ok(())
    }

    #[doc = example!("Digest the committed world and its resources.")]
    /// # Panics
    /// Only if an engine registration invariant is broken.
    pub fn digest(&self) -> u64 {
        self.runtime
            .world_digest(&self.authority.state, |_, _| {})
            .expect("all SDK columns have canonical encoding")
    }

    #[doc = example!("Spawn a tuple of components at the current tick.")]
    /// # Errors
    /// The bundle is invalid or the world is full.
    /// # Panics
    /// Only if an internal harness rule is invoked more than once.
    pub fn spawn(&mut self, bundle: impl Spawn) -> Result<Entity> {
        let mut bundle = Some(bundle);
        self.run(move |mut entities: Entities<'_>| {
            entities.spawn(bundle.take().expect("rule runs once"))
        })
    }

    #[doc = example!("Despawn this entity at the current tick.")]
    /// # Errors
    /// The edit was refused.
    pub fn despawn(&mut self, entity: Entity) -> Result<bool> {
        self.run(move |mut entities: Entities<'_>| entities.despawn(entity))
    }

    #[doc = example!("Set a component at the current tick.")]
    /// # Errors
    /// The entity is gone or the component value was refused.
    /// # Panics
    /// Only if an internal harness rule is invoked more than once.
    pub fn set<T: Component>(&mut self, entity: Entity, value: T) -> Result {
        self.get::<T>(entity)?;
        let mut value = Some(value);
        self.run(move |mut entities: Entities<'_>| {
            entities.insert(entity, value.take().expect("rule runs once"));
        })
    }

    #[doc = example!("Read a copy of this entity's component.")]
    /// # Errors
    /// The entity is gone or the component is absent.
    pub fn get<T: Component>(&self, entity: Entity) -> Result<T> {
        let world = self.runtime.read();
        let column = Column::of::<T>();
        if let Some(handle) = world.handle(entity.stable())
            && world.has_component(handle, column.key)
        {
            return world
                .with(
                    self.authority.program.bindings().read::<T>(),
                    handle,
                    |stored| stored.0.clone(),
                )
                .map_err(|error| Error::invalid(format!("{entity:?}"), error.to_string()));
        }
        Err(Error::refused(
            "T0502",
            format!("{entity:?} and {}", type_name::<T>()),
            "entity is gone or lacks the component",
            "Use a live entity that holds the requested component.",
        ))
    }

    #[doc = example!("Read a copy of a resource in the current world.")]
    /// # Errors
    /// The resource is absent.
    pub fn resource<T: Data>(&self) -> Result<T> {
        self.authority
            .state
            .resources
            .get::<T>()
            .cloned()
            .ok_or_else(|| {
                Error::refused(
                    "T0201",
                    type_name::<T>(),
                    "this world does not provide the resource",
                    "Register the value with Game::resource or scene.resource.",
                )
            })
    }

    #[doc = example!("Change a registered resource at the current tick.")]
    /// # Errors
    /// The resource is missing or the value is invalid.
    pub fn set_resource<T: Resource>(&mut self, value: T) -> Result {
        let mut value = Some(value);
        self.run(move |mut resource: ResMut<'_, T>| {
            if let Some(value) = value.take() {
                *resource = value;
            }
        })
    }

    #[doc = example!("Whether this entity is still alive.")]
    pub fn alive(&self, entity: Entity) -> bool {
        self.runtime.handle(entity.stable()).is_some()
    }

    #[doc = example!("Find all matching live entities in spawn order.")]
    pub fn find<F: Filter>(&self) -> Vec<Entity> {
        let world = self.runtime.read();
        let mut entities = Vec::new();
        collect_entities(&world, &mut entities);
        matching::<F>(&world, &entities)
    }

    #[doc = example!("Count live entities that match a filter.")]
    pub fn count<F: Filter>(&self) -> u32 {
        u32::try_from(self.find::<F>().len()).unwrap_or(u32::MAX)
    }

    #[doc = example!("Find the sole entity that matches a filter.")]
    /// # Errors
    /// The filter matches zero entities or more than one.
    pub fn single<F: Filter>(&self) -> Result<Entity> {
        let entities = self.find::<F>();
        if let [entity] = entities.as_slice() {
            return Ok(*entity);
        }
        Err(Error::refused(
            "T0304",
            type_name::<F>(),
            format!("Single matched {} entities", entities.len()),
            "Ensure that exactly one entity matches, or use find.",
        ))
    }

    #[doc = example!("Read the current phase's typed value.")]
    /// # Errors
    /// The running scene has no phase of this type.
    pub fn phase<P: PhaseType>(&self) -> Result<P> {
        let state = &self.authority.state;
        state
            .phase
            .and_then(|phase| {
                self.authority
                    .program
                    .names()
                    .phase_value(state.scene, phase)
            })
            .cloned()
            .ok_or_else(|| {
                Error::refused(
                    "T0503",
                    type_name::<P>(),
                    "no phase of that type is running",
                    "Enter a scene with the requested phase type.",
                )
            })
    }

    #[doc = example!("Read the value with which the running scene was entered.")]
    /// # Errors
    /// The requested scene is not the running scene.
    pub fn scene<S: SceneType>(&self) -> Result<S> {
        self.authority
            .state
            .scene_value
            .as_ref()
            .and_then(|value| value.any().downcast_ref::<S>())
            .cloned()
            .ok_or_else(|| {
                Error::refused(
                    "T0503",
                    type_name::<S>(),
                    "this scene is not running",
                    "Check in_scene or enter the requested scene.",
                )
            })
    }

    #[doc = example!("Whether a scene of this type is running.")]
    pub fn in_scene<S: SceneType>(&self) -> bool {
        self.authority.program.names().scene::<S>() == Some(self.authority.state.scene)
    }

    #[doc = example!("Enter the named scene immediately, without advancing a tick.")]
    /// # Errors
    /// The scene is not registered or its transition was refused.
    pub fn go_to<S: SceneType>(&mut self, value: S) -> Result {
        let index = self.authority.program.names().scene::<S>().ok_or_else(|| {
            Error::refused(
                "T0305",
                type_name::<S>(),
                "the requested scene was never registered",
                "Add its description with Game::scene(..).",
            )
        })?;
        self.edit(Operation::Transition(Transition::Scene(
            index,
            Arc::new(value),
        )))
    }

    #[doc = example!("Enter another phase of the running scene without advancing a tick.")]
    /// # Errors
    /// The phase does not belong to the running scene or the transition was refused.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "phase requests take a phase value like scene requests"
    )]
    pub fn go_to_phase<P: PhaseType>(&mut self, phase: P) -> Result {
        let index = self
            .authority
            .program
            .names()
            .phase(self.authority.state.scene, &phase)
            .ok_or_else(|| {
                Error::refused(
                    "T0503",
                    type_name::<P>(),
                    "the running scene has no such phase",
                    "Add a phase block to the scene description.",
                )
            })?;
        self.edit(Operation::Transition(Transition::Phase(index)))
    }

    #[doc = example!("Read events of this type since the preceding read.")]
    pub fn events<E: Event>(&mut self) -> Vec<E> {
        self.authority.events.take::<E>()
    }

    #[doc = example!("Run one rule or closure immediately at the current tick.")]
    /// # Errors
    /// The rule cannot acquire its parameters or its edits are refused.
    /// # Panics
    /// Only if an internal harness invariant is broken.
    pub fn run<M, O: Send + 'static>(&mut self, function: impl RuleFunction<M, O>) -> Result<O> {
        let mut invocation = function.compile();
        self.authority.program.validate_manual(
            &invocation.signature,
            invocation.name,
            &self.authority.state.resources,
        )?;
        let output = Arc::new(Mutex::new(None));
        let result = Arc::clone(&output);
        let name = invocation.name;
        let signature = invocation.signature.clone();
        let invoke = move |context: &mut Context<'_, '_>, parts: &ReadContextParts<'_>| {
            let value = invocation.call(context, parts)?;
            *result.lock().expect("manual result") = Some(value);
            Ok(())
        };
        self.edit(Operation::Manual(Invocation {
            name,
            signature,
            invoke: Box::new(invoke),
        }))?;
        Ok(output
            .lock()
            .expect("manual result")
            .take()
            .expect("manual rule ran"))
    }

    fn edit(&mut self, operation: Operation) -> Result {
        *self.authority.operation.get_mut().expect("operation") = operation;
        let result = self
            .runtime
            .edit_current(&mut self.authority, &mut self.candidate);
        *self.authority.operation.get_mut().expect("operation") = Operation::Tick;
        result.map_err(operation_result)?;
        self.authority.events.collect(&mut self.candidate.events);
        self.history = new_history(&self.runtime, &self.authority);
        Ok(())
    }
}

fn new_history(
    runtime: &GameRuntime<Authority>,
    authority: &Authority,
) -> RuntimeHistory<State, ()> {
    RuntimeHistory::new(
        runtime,
        authority,
        NonZeroUsize::new(512).expect("positive window"),
    )
}

fn history_error(error: HistoryError<Error>) -> Error {
    match error {
        HistoryError::Tick(error) => operation_result(error),
        error => Error::refused(
            "T0505",
            "test history",
            error.to_string(),
            "Rewind within the retained interval of the current run.",
        ),
    }
}

impl HistoryGame for Authority {
    type HistoryState = State;
    type HistoryRecord = ();
    type TickRequest = ();
    fn history_revision(&self) -> u64 {
        0
    }
    fn history_state(&self) -> &State {
        &self.state
    }
    fn restore_history_state(&mut self, state: &State) {
        self.state.clone_from(state);
    }
    fn new_history_record(&self) {}
    fn record_history_tick(&self, (): &(), (): &mut ()) {}
    fn prepare_history_tick(&mut self, _: &GameRuntime<Self>, _: u64, (): &()) -> Result {
        Ok(())
    }
}
