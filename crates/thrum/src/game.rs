use std::{
    any::{Any, TypeId, type_name},
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
    ($description:literal, $($code:expr),+ $(,)?) => {
        concat!($description, "\n\n```\n# use thrum::prelude::*;\n",
                $($code, "\n",)+ "```\n")
    };
}

macro_rules! harness_example {
    ($description:literal, $($code:expr),+ $(,)?) => {
        example!($description,
            "# #[derive(Component)] struct Health(u32);",
            "# #[derive(Resource)] struct Score(u32);",
            "# #[derive(Event)] struct Point;",
            "# #[derive(Scene)] struct Finished(u32);",
            "# #[derive(Phase)] enum Stage { Play, Overtime }",
            "# fn play(scene: &mut Scene) {",
            "#     scene.phases(Stage::Play);",
            "#     scene.phase(Stage::Overtime);",
            "#     scene.rules(|mut emit: Emit<Point>| emit.send(Point));",
            "# }",
            "# let mut game = Game::new(\"Example\").tick_rate(1).resource(Score(0))",
            "#     .component::<Health>().scene(play).scene(|_: &mut Scene<Finished>| {})",
            "#     .test().unwrap();",
            $($code),+)
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

#[doc = example!("A game described in Rust, ready to start in a test.", "let game: Game = Game::new(\"Example\").scene(|_: &mut Scene| {});")]
#[must_use]
pub struct Game {
    description: Description,
    first: fn() -> ValueRef,
}

#[doc = example!("A game before its first scene is registered.", "let game: thrum::NewGame = Game::new(\"Example\");", "let _game = game.scene(|_: &mut Scene| {});")]
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

    #[doc = example!("Append another scene description.", "# #[derive(Scene)] struct Finished;", "let _game = Game::new(\"Example\").scene(|_: &mut Scene| {})", "    .scene(|_: &mut Scene<Finished>| {});")]
    pub fn scene<S: SceneType, M>(mut self, description: impl SceneDescription<S, M>) -> Self {
        self.description.scene(description);
        self
    }

    #[doc = example!("Start a headless game in the first scene's default value.", "# #[derive(Scene, Default)] struct Play;", "let game = Game::new(\"Example\").scene(|_: &mut Scene<Play>| {}).test().unwrap();", "assert!(game.in_scene::<Play>());")]
    /// # Errors
    /// The description or its initial scene is invalid.
    pub fn test(self) -> Result<TestGame> {
        {
            let first = (self.first)();
            self.start(first, 0)
        }
    }

    #[doc = example!("Start a headless game in a named scene and value.", "# #[derive(Scene)] struct Finished(u32);", "let game = Game::new(\"Example\").scene(|_: &mut Scene| {})", "    .scene(|_: &mut Scene<Finished>| {}).test_in(Finished(9)).unwrap();", "assert_eq!(game.scene::<Finished>().unwrap().0, 9);")]
    /// # Errors
    /// The requested scene or the game description is invalid.
    pub fn test_in(self, scene: impl SceneType) -> Result<TestGame> {
        self.start(Arc::new(scene), 0)
    }

    #[doc = example!("Set the fixtures of a windowless test before starting it.", "let setup = Game::new(\"Example\").scene(|_: &mut Scene| {}).test_setup();", "let _game = setup.seed(17).start().unwrap();")]
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
            crate::runtime::check_value(resource.as_ref())?;
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

#[doc = example!("A scene whose type can supply the game's starting value.", "# use thrum::__derive::FirstScene;", "# #[derive(Scene, Default)] struct Play;", "let _: Play = Play::first_value();")]
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "T0103: first scene `{Self}` cannot supply a default value",
    label = "the first scene needs a default value",
    note = "derive or implement Default on this scene, or register a defaultable scene first"
)]
pub trait FirstScene: SceneType {
    #[doc = example!("Get the starting value for the first scene.", "# use thrum::__derive::FirstScene;", "# #[derive(Scene, Default)] struct Play(u32);", "assert_eq!(Play::first_value().0, 0);")]
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
    ($game:ident, $description:tt, $builder:literal) => {
        impl $game {
            #[doc = example!("Provide a game resource that persists across scenes.", "# #[derive(Resource)] struct Score(u32);", concat!("# let builder = ", $builder, ";"), "let _builder = builder.resource(Score(0));")]
            pub fn resource(mut self, value: impl Resource) -> Self {
                self.$description.resources.push(Arc::new(value));
                self
            }

            #[doc = example!("Register a component absent from all rule signatures.", "# #[derive(Component)] struct Rock;", concat!("# let builder = ", $builder, ";"), "let _builder = builder.component::<Rock>();")]
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

            #[doc = example!("Set the maximum number of live entities.", concat!("# let builder = ", $builder, ";"), "let _builder = builder.max_entities(64);")]
            pub fn max_entities(mut self, capacity: u32) -> Self {
                self.$description.capacity = capacity;
                self
            }

            #[doc = example!("Set the number of ticks per simulated second.", concat!("# let builder = ", $builder, ";"), "let _builder = builder.tick_rate(30);")]
            pub fn tick_rate(mut self, hz: u32) -> Self {
                self.$description.rate = hz;
                self
            }
        }
    };
}

settings!(NewGame, 0, "Game::new(\"Example\")");
settings!(
    Game,
    description,
    "Game::new(\"Example\").scene(|_: &mut Scene| {})"
);

#[doc = example!("Fixtures for a headless game test.", "let setup: thrum::TestSetup = Game::new(\"Example\").scene(|_: &mut Scene| {}).test_setup();", "let _game = setup.seed(17).start().unwrap();")]
#[must_use]
pub struct TestSetup {
    game: Game,
    seed: u64,
}

impl TestSetup {
    #[doc = example!("Choose the seed used by Random in this test.", "let _game = Game::new(\"Example\").scene(|_: &mut Scene| {})", "    .test_setup().seed(17).start().unwrap();")]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    #[doc = example!("Start this test with the configured fixtures.", "let setup = Game::new(\"Example\").scene(|_: &mut Scene| {}).test_setup();", "let _game = setup.start().unwrap();")]
    /// # Errors
    /// The description or its initial scene is invalid.
    pub fn start(self) -> Result<TestGame> {
        let first = (self.game.first)();
        self.game.start(first, self.seed)
    }
}

#[doc = harness_example!("The headless game harness, stepped by a test without a window.", "let _: &mut thrum::TestGame = &mut game;", "game.step().unwrap();")]
pub struct TestGame {
    runtime: GameRuntime<Authority>,
    authority: Authority,
    candidate: Candidate,
    history: RuntimeHistory<State, ()>,
}

impl TestGame {
    #[doc = harness_example!("Advance the game by one tick.", "game.step().unwrap();", "assert_eq!(game.events::<Point>(), [Point]);")]
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

    #[doc = harness_example!("Advance the game by the requested number of ticks.", "game.step_ticks(3).unwrap();", "assert_eq!(game.events::<Point>().len(), 3);")]
    /// # Errors
    /// A rule or transition refused one of the ticks.
    pub fn step_ticks(&mut self, count: u64) -> Result {
        for _ in 0..count {
            self.step()?;
        }
        Ok(())
    }

    #[doc = harness_example!("Advance through the requested time without waiting on a clock.", "game.step_for(seconds(2)).unwrap();", "assert_eq!(game.events::<Point>().len(), 2);")]
    /// # Errors
    /// A rule or transition refused one of the ticks.
    pub fn step_for(&mut self, time: Seconds) -> Result {
        self.step_ticks(time.ticks(self.authority.program.rate()))
    }

    #[doc = harness_example!("Advance to the first state accepted by the predicate, within the time limit.", "let start = game.digest();", "game.step_until(seconds(2), |game| Ok(game.digest() != start)).unwrap();")]
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

    #[doc = harness_example!("Return to an earlier tick in the test's retained history.", "let start = game.digest();", "game.step().unwrap();", "game.rewind(1).unwrap();", "assert_eq!(game.digest(), start);")]
    /// # Errors
    /// The requested tick is outside the retained history.
    pub fn rewind(&mut self, ticks: usize) -> Result {
        self.history
            .rewind(&mut self.runtime, &mut self.authority, ticks)
            .map_err(history_error)?;
        self.authority.events.clear();
        Ok(())
    }

    #[doc = harness_example!("Digest the committed world and its resources.", "let start = game.digest();", "game.set_resource(Score(2)).unwrap();", "assert_ne!(game.digest(), start);")]
    /// # Panics
    /// Only if an engine registration invariant is broken.
    pub fn digest(&self) -> u64 {
        self.runtime
            .world_digest(&self.authority.state, |_, _| {})
            .expect("all SDK columns have canonical encoding")
    }

    #[doc = harness_example!("Spawn a tuple of components at the current tick.", "let entity = game.spawn((Health(3),)).unwrap();", "assert!(game.alive(entity));")]
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

    #[doc = harness_example!("Despawn this entity at the current tick.", "let entity = game.spawn((Health(3),)).unwrap();", "assert!(game.despawn(entity).unwrap());", "assert!(!game.alive(entity));")]
    /// # Errors
    /// The edit was refused.
    pub fn despawn(&mut self, entity: Entity) -> Result<bool> {
        self.run(move |mut entities: Entities<'_>| entities.despawn(entity))
    }

    #[doc = harness_example!("Set a component at the current tick.", "let entity = game.spawn((Health(3),)).unwrap();", "game.set(entity, Health(5)).unwrap();", "assert_eq!(game.get::<Health>(entity).unwrap().0, 5);")]
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

    #[doc = harness_example!("Read a copy of this entity's component.", "let entity = game.spawn((Health(3),)).unwrap();", "assert_eq!(game.get::<Health>(entity).unwrap().0, 3);")]
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
                    |stored| stored.0,
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

    #[doc = harness_example!("Read a copy of a resource in the current world.", "assert_eq!(game.resource::<Score>().unwrap().0, 0);")]
    /// # Errors
    /// The resource is absent.
    pub fn resource<T: Data>(&self) -> Result<T> {
        self.authority
            .state
            .resource::<T>()
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

    #[doc = harness_example!("Change a registered resource at the current tick.", "game.set_resource(Score(2)).unwrap();", "assert_eq!(game.resource::<Score>().unwrap().0, 2);")]
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

    #[doc = harness_example!("Whether this entity is still alive.", "let entity = game.spawn((Health(3),)).unwrap();", "assert!(game.alive(entity));")]
    pub fn alive(&self, entity: Entity) -> bool {
        self.runtime.handle(entity.stable()).is_some()
    }

    #[doc = harness_example!("Find all matching live entities in spawn order.", "let a = game.spawn((Health(3),)).unwrap();", "let b = game.spawn((Health(5),)).unwrap();", "assert_eq!(game.find::<With<Health>>(), [a, b]);")]
    pub fn find<F: Filter>(&self) -> Vec<Entity> {
        let world = self.runtime.read();
        let mut entities = Vec::new();
        collect_entities(&world, &mut entities);
        matching::<F>(&world, &entities)
    }

    #[doc = harness_example!("Count live entities that match a filter.", "game.spawn((Health(3),)).unwrap();", "assert_eq!(game.count::<With<Health>>(), 1);")]
    pub fn count<F: Filter>(&self) -> u32 {
        u32::try_from(self.find::<F>().len()).unwrap_or(u32::MAX)
    }

    #[doc = harness_example!("Find the sole entity that matches a filter.", "let entity = game.spawn((Health(3),)).unwrap();", "assert_eq!(game.single::<With<Health>>().unwrap(), entity);")]
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

    #[doc = harness_example!("Read the current phase's typed value.", "assert_eq!(game.phase::<Stage>().unwrap(), Stage::Play);")]
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

    #[doc = harness_example!("Read the value with which the running scene was entered.", "game.go_to(Finished(8)).unwrap();", "assert_eq!(game.scene::<Finished>().unwrap().0, 8);")]
    /// # Errors
    /// The requested scene is not the running scene.
    pub fn scene<S: SceneType>(&self) -> Result<S> {
        self.authority
            .state
            .scene_value
            .as_ref()
            .and_then(|value| value.0.any().downcast_ref::<S>())
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

    #[doc = harness_example!("Whether a scene of this type is running.", "assert!(game.in_scene::<()>());", "assert!(!game.in_scene::<Finished>());")]
    pub fn in_scene<S: SceneType>(&self) -> bool {
        self.authority.program.names().scene::<S>() == Some(self.authority.state.scene)
    }

    #[doc = harness_example!("Enter the named scene immediately, without advancing a tick.", "game.go_to(Finished(8)).unwrap();", "assert_eq!(game.scene::<Finished>().unwrap().0, 8);")]
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

    #[doc = harness_example!("Enter another phase of the running scene without advancing a tick.", "game.go_to_phase(Stage::Overtime).unwrap();", "assert_eq!(game.phase::<Stage>().unwrap(), Stage::Overtime);")]
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

    #[doc = harness_example!("Read events of this type since the preceding read.", "game.step().unwrap();", "assert_eq!(game.events::<Point>(), [Point]);", "assert!(game.events::<Point>().is_empty());")]
    pub fn events<E: Event>(&mut self) -> Vec<E> {
        self.authority.events.take::<E>()
    }

    #[doc = harness_example!("Run one rule or closure immediately at the current tick.", "game.run(|mut score: ResMut<Score>| score.0 += 1).unwrap();", "assert_eq!(game.resource::<Score>().unwrap().0, 1);")]
    /// # Errors
    /// The rule cannot acquire its parameters, returns an error as a rule
    /// result, or proposes an invalid edit.
    ///
    /// Function pointers get their random stream when first registered or
    /// run. Repeat manual calls in the same order to reproduce a run in a
    /// freshly built game.
    /// # Panics
    /// Only if an internal harness invariant is broken.
    pub fn run<M, O: Send + 'static>(&mut self, function: impl RuleFunction<M, O>) -> Result<O> {
        let mut invocation = function.compile();
        self.authority.program.validate_manual(
            &invocation.signature,
            invocation.name,
            &self.authority.state,
        )?;
        let output = Arc::new(Mutex::new(None));
        let result = Arc::clone(&output);
        let name = invocation.name;
        let signature = invocation.signature.clone();
        let callable = invocation.callable;
        let pointer = invocation.pointer;
        if pointer.is_some() || invocation.anonymous() {
            self.authority.program.streams.identify(&mut invocation);
        }
        let stream = invocation.stream;
        let invoke = move |context: &mut Context<'_, '_>, parts: &ReadContextParts<'_>| {
            let mut value = invocation.call(context, parts)?;
            let outcome: &mut dyn Any = &mut value;
            if let Some(outcome) = outcome.downcast_mut::<Result>() {
                std::mem::replace(outcome, Ok(()))?;
            }
            *result.lock().expect("manual result") = Some(value);
            Ok(())
        };
        self.edit(Operation::Manual(Invocation {
            name,
            signature,
            stream,
            callable,
            pointer,
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
        let result = self.runtime.edit_current_if_changed(
            &mut self.authority,
            &mut self.candidate,
            |authority, candidate| authority.state != candidate.state,
        );
        *self.authority.operation.get_mut().expect("operation") = Operation::Tick;
        let changed = result.map_err(operation_result)?;
        self.authority.events.collect(&mut self.candidate.events);
        if changed {
            self.history = new_history(&self.runtime, &self.authority);
        }
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
