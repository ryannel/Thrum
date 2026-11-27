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
mod rule;
mod runtime;
mod scene;
mod time;

pub use data::{Component, Data, Event, PhaseType, Resource, SceneType};
pub use entity::{Entities, Entity, Spawn};
pub use error::{Diagnostic, Error, Result};
pub use game::{Game, NewGame, TestGame, TestSetup};
pub use number::{Num, Vec2, vec2};
pub use rule::params::{Emit, Events, Random, Res, ResMut};
pub use rule::query::{Filter, Iter, IterMut, Query, QueryData, Single, With, Without};
pub use rule::{ConfiguredRules, Rules};
pub use scene::{Phase, PhaseBlock, PhaseMut, Scene, Scenes};
pub use thrum_derive::{Component, Data, Event, Params, Phase, Resource, Scene};
pub use time::{Cooldown, PhaseClock, Seconds, Tick, TickNumber, Timer, seconds};

/// Game data derives and their marker traits.
/// ```
/// use thrum::prelude::*;
/// #[derive(Resource)]
/// struct Score(u32);
/// assert_eq!(Score(3).0, 3);
/// ```
pub mod prelude {
    pub use crate::{
        Component, Cooldown, Data, Emit, Entities, Entity, Error, Event, Events, Game, Num, Params,
        Phase, PhaseClock, PhaseMut, Query, Random, Res, ResMut, Resource, Result, Rules, Scene,
        Scenes, Seconds, Single, Spawn, Tick, TickNumber, Timer, Vec2, With, Without, seconds,
        vec2,
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
    pub use crate::data::{DataReader, DataWriter, InvalidData};
    pub use crate::entity::Changes;
    pub use crate::game::FirstScene;
    pub use crate::rule::params::{Context, Param, ParameterList, ReadContext, ReadOnly};
    pub use crate::rule::query::Rows;
    pub use crate::rule::{
        Function, Group, Invocation, ReadFunction, Rule, RuleFunction, RuleOutput,
    };
    pub use crate::scene::{Duration, PhaseDuration, SceneDescription};
    pub use crate::scene::{PhaseRequests, SceneRequests};
}
