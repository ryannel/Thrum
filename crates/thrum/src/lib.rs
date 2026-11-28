//! The game SDK's deterministic, windowless rules and scenes.
//!
//! ```
//! use thrum::prelude::*;
//! #[derive(Component)]
//! struct Health(u32);
//! assert_eq!(Health(2), Health(2));
//! ```

extern crate self as thrum;

mod data;
mod entity;
mod error;
mod game;
mod number;
mod prefab;
mod rule;
mod runtime;
mod scene;
mod time;
mod transform;

pub use data::{Component, Data, Event, PhaseType, Resource, SceneType};
pub use entity::family::{Family, Part};
pub use entity::{Attach, Detach, Entities, Entity, Reparenting, Spawn};
pub use error::{Diagnostic, Error, Result};
pub use game::{Game, NewGame, TestGame, TestSetup};
pub use number::{Num, Vec2, vec2};
pub use prefab::{Bundle, Place, Pose, Prefab, PrefabDef, PrefabSpawn, SpawnEdits};
pub use rule::params::{Emit, Events, Random, Res, ResMut};
pub use rule::query::{
    Disabled, Filter, Iter, IterMut, Query, QueryData, Single, With, WithDisabled, Without,
};
pub use rule::{ConfiguredRules, Rules};
pub use scene::{Phase, PhaseBlock, PhaseMut, Scene, Scenes};
pub use time::{Cooldown, PhaseClock, Seconds, Tick, TickNumber, Timer, seconds};
pub use transform::{Angle, Rot3, Transform2d, Transform3d, Vec3, vec3};

/// Declare data attached to an entity.
/// ```
/// use thrum::Component;
/// #[derive(Component)] struct Health(u32);
/// let health = Health(3);
/// let copy = health;
/// assert_eq!(health, copy);
/// ```
#[doc(inline)]
pub use thrum_derive::Component;

/// Declare portable world data.
/// ```
/// use thrum::Data;
/// #[derive(Data)] struct Points(u32);
/// assert_eq!(Points(3), Points(3).clone());
/// ```
#[doc(inline)]
pub use thrum_derive::Data;

/// Declare data sent to later rules of a tick.
/// ```
/// use thrum::Event;
/// #[derive(Event)] struct Scored(u32);
/// let mut game = thrum::Game::new("Example").scene(|scene: &mut thrum::Scene| {
///     scene.rules(|mut events: thrum::Emit<Scored>| events.send(Scored(3)));
/// }).test().unwrap();
/// game.step().unwrap();
/// assert_eq!(game.events::<Scored>(), [Scored(3)]);
/// ```
#[doc(inline)]
pub use thrum_derive::Event;

/// Bundle named rule parameters into one user-defined parameter.
/// ```
/// use thrum::Params;
/// #[derive(thrum::Resource)] struct Score(u32);
/// #[derive(Params)] struct Access<'a> { score: thrum::ResMut<'a, Score> }
/// fn score(mut access: Access<'_>) { access.score.0 += 1; }
/// let mut game = thrum::Game::new("Example").resource(Score(0))
///     .scene(|scene: &mut thrum::Scene| { scene.rules(score); }).test().unwrap();
/// game.step().unwrap();
/// assert_eq!(game.resource::<Score>().unwrap().0, 1);
/// ```
#[doc(inline)]
pub use thrum_derive::Params;

/// Declare the phases of a scene.
/// ```
/// use thrum::Phase;
/// #[derive(Phase)] enum Stage { Play }
/// let game = thrum::Game::new("Example").scene(|scene: &mut thrum::Scene| {
///     scene.phases(Stage::Play);
/// }).test().unwrap();
/// assert_eq!(game.phase::<Stage>().unwrap(), Stage::Play);
/// ```
#[doc(inline)]
pub use thrum_derive::Phase;

/// Declare data shared across rules.
/// ```
/// use thrum::Resource;
/// #[derive(Resource)] struct Score(u32);
/// let game = thrum::Game::new("Example").resource(Score(3))
///     .scene(|_: &mut thrum::Scene| {}).test().unwrap();
/// assert_eq!(game.resource::<Score>().unwrap().0, 3);
/// ```
#[doc(inline)]
pub use thrum_derive::Resource;

/// Declare the data a scene is entered with.
/// ```
/// use thrum::Scene;
/// #[derive(Scene, Default)] struct Play;
/// let game = thrum::Game::new("Example")
///     .scene(|_: &mut thrum::Scene<Play>| {}).test().unwrap();
/// assert!(game.in_scene::<Play>());
/// ```
#[doc(inline)]
pub use thrum_derive::Scene;

/// Game data derives and their marker traits.
/// ```
/// use thrum::prelude::*;
/// #[derive(Resource)]
/// struct Score(u32);
/// assert_eq!(Score(3).0, 3);
/// ```
pub mod prelude {
    pub use crate::{
        Angle, Bundle, Component, Cooldown, Data, Disabled, Emit, Entities, Entity, Error, Event,
        Events, Family, Game, Num, Params, Part, Phase, PhaseClock, PhaseMut, Prefab, PrefabDef,
        Query, Random, Res, ResMut, Resource, Result, Rot3, Rules, Scene, Scenes, Seconds, Single,
        Spawn, Tick, TickNumber, Timer, Transform2d, Transform3d, Vec2, Vec3, With, WithDisabled,
        Without, prefabs, seconds, vec2, vec3,
    };
}

/// Implementation details used by the game SDK's derives.
/// ```
/// use thrum::Data;
/// #[derive(Data)]
/// struct Amount(u32);
/// assert_eq!(Amount(2), Amount(2));
/// ```
#[doc(hidden)]
pub mod __derive {
    pub use crate::data::{DataReader, DataWriter, InvalidData, schema_id};
    pub use crate::entity::{Changes, family::FamilyRows};
    pub use crate::game::FirstScene;
    pub use crate::prefab::{Declaration, LinkedDeclaration};
    pub use crate::rule::params::{Context, Param, ParameterList, ReadContext, ReadOnly};
    pub use crate::rule::query::Rows;
    pub use crate::rule::{
        Function, Group, Invocation, ReadFunction, Rule, RuleFunction, RuleOutput,
        ScheduledFunction,
    };
    pub use crate::scene::{Duration, PhaseDuration, SceneDescription};
    pub use crate::scene::{PhaseRequests, SceneRequests};
    pub use inventory::submit as submit_prefab;
}
