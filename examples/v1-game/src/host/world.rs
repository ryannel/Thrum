//! The game's single authoritative clock, rules and optional projection output.

use std::time::Duration;

pub(super) use thrum_core::sim::WorldAdvance;
use thrum_core::{
    ClockConfig,
    sim::{ClockedWorld, SnapshotId, WorldGame, WorldPublication, WorldStep},
};

use crate::{
    projection::{GameProjection, ProjectionGap, ProjectionOutbox},
    scene::{self, ENTITY_COUNT},
    simulation::{PublishedPage, SimError, Simulation},
};

#[cfg(not(target_arch = "wasm32"))]
use super::world_thread::WorldClient;

pub(super) enum WorldPort {
    Inline(Box<WorldDriver>),
    #[cfg(not(target_arch = "wasm32"))]
    Threaded(Box<WorldClient>),
    #[cfg(not(target_arch = "wasm32"))]
    Starting,
}

impl WorldPort {
    pub(super) fn is_threaded(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            matches!(self, Self::Threaded(_))
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    fn inline(&self) -> &WorldDriver {
        match self {
            Self::Inline(driver) => driver,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) | Self::Starting => {
                unreachable!("the threaded host does not expose inline world methods")
            }
        }
    }

    pub(super) fn inline_mut(&mut self) -> &mut WorldDriver {
        match self {
            Self::Inline(driver) => driver,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) | Self::Starting => {
                unreachable!("the threaded host does not expose inline world methods")
            }
        }
    }

    pub(super) fn tick(&self) -> u64 {
        match self {
            Self::Inline(driver) => driver.tick(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.tick(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn epoch(&self) -> u128 {
        match self {
            Self::Inline(driver) => driver.epoch(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.epoch(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn timeline(&self) -> u64 {
        match self {
            Self::Inline(driver) => driver.timeline(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.timeline(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn has_current_page(&self) -> bool {
        match self {
            Self::Inline(driver) => driver.has_current_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.has_current_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        match self {
            Self::Inline(driver) => driver.published_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.published_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        match self {
            Self::Inline(driver) => driver.advance(now),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.poll(now),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        match self {
            Self::Inline(driver) => driver.presentation_alpha_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.presentation_alpha_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn next_due_in(&self) -> Option<Duration> {
        match self {
            Self::Inline(driver) => driver.next_due_in(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) => None,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn pause(&mut self) {
        self.inline_mut().pause();
    }

    pub(super) fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.inline_mut().resume(now)
    }

    pub(super) fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        match self {
            Self::Inline(driver) => driver.set_controls(axes),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.set_controls(axes),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn controlled_position(&self) -> Option<[f64; 3]> {
        self.inline().controlled_position()
    }

    pub(super) fn enable_projection(&mut self) -> Result<(), String> {
        self.inline_mut().enable_projection()
    }

    pub(super) fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        self.inline_mut().poll_projection(consume)
    }

    pub(super) fn projection_reserved_bytes(&self) -> usize {
        self.inline().projection_reserved_bytes()
    }

    pub(super) fn reset_projection_baseline(&mut self) -> Result<(), String> {
        self.inline_mut().reset_projection_baseline()
    }
}

/// The game body stays here; scheduling and the fixed clock belong to core.
pub(super) struct WorldDriver {
    pub(super) world: ClockedWorld<GameWorld>,
}

pub(super) struct GameWorld {
    pub(super) sim: Simulation,
    projections: Option<ProjectionOutbox>,
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
}

impl WorldDriver {
    pub(super) fn new() -> Result<Self, String> {
        let game = GameWorld {
            sim: scene::simulation()?,
            projections: None,
            desired_axes: [0.0; 3],
            admitted_axes: [0.0; 3],
            input_sequence: 1,
        };
        Ok(Self {
            world: ClockedWorld::new(game, ClockConfig::default(), Duration::ZERO)
                .map_err(|error| error.to_string())?,
        })
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.world.presentation_alpha_at(lead)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn held_controls(&self) -> [f32; 3] {
        self.world.game().desired_axes
    }

    pub(super) fn tick(&self) -> u64 {
        self.world.game().sim.tick()
    }

    pub(super) fn epoch(&self) -> u128 {
        self.world.game().sim.epoch().0
    }

    pub(super) fn timeline(&self) -> u64 {
        self.world.game().sim.timeline()
    }

    pub(super) fn has_current_page(&self) -> bool {
        self.world.game().sim.has_current_page()
    }

    pub(super) fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        let game = self.world.game_mut();
        game.recover_page()?;
        if !game.sim.has_current_page() {
            return Err(SimError::PublicationBusy);
        }
        game.sim.published_page()
    }

    pub(super) fn next_due_in(&self) -> Option<Duration> {
        self.world.next_due_in()
    }

    pub(super) fn pause(&mut self) {
        self.world.pause();
    }

    pub(super) fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.world.resume(now).map_err(|error| error.to_string())
    }

    pub(super) fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        self.world
            .set_controls(axes)
            .map_err(|error| error.to_string())
    }

    pub(super) fn controlled_position(&self) -> Option<[f64; 3]> {
        let sim = &self.world.game().sim;
        sim.handle(scene::CONTROLLED_ID)
            .and_then(|handle| sim.get(handle).ok())
            .map(|entity| entity.position.0)
    }

    pub(super) fn enable_projection(&mut self) -> Result<(), String> {
        let game = self.world.game_mut();
        if game.projections.is_none() {
            let page = game
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            let mut outbox = ProjectionOutbox::new(ENTITY_COUNT);
            outbox.reset_full(&page);
            game.projections = Some(outbox);
        }
        Ok(())
    }

    pub(super) fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        let game = self.world.game_mut();
        let timeline = game.sim.timeline();
        game.projections
            .as_mut()
            .map(|outbox| outbox.poll(timeline, consume))
            .transpose()
    }

    pub(super) fn projection_reserved_bytes(&self) -> usize {
        self.world
            .game()
            .projections
            .as_ref()
            .map_or(0, ProjectionOutbox::reserved_bytes)
    }

    pub(super) fn reset_projection_baseline(&mut self) -> Result<(), String> {
        let game = self.world.game_mut();
        let outbox = game
            .projections
            .as_mut()
            .ok_or("projection output is disabled")?;
        let page = game
            .sim
            .published_page()
            .map_err(|error| format!("world page: {error:?}"))?;
        outbox.reset_full(&page);
        Ok(())
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        self.world.advance(now).map_err(|error| error.to_string())
    }
}

impl GameWorld {
    fn recover_page(&mut self) -> Result<Option<usize>, SimError> {
        if self.sim.has_current_page() {
            return Ok(None);
        }
        match self.sim.published_page() {
            Ok(page) => Ok(Some(
                self.projections
                    .as_mut()
                    .map_or(0, |outbox| outbox.capture(&page)),
            )),
            Err(SimError::PublicationBusy) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl WorldGame for GameWorld {
    type Control = [f32; 3];
    type Page = PublishedPage;
    type Error = String;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        scene::control_frame(scene::LOCAL_PEER, control, 0, 0)
            .validate_axes()
            .map_err(|error| format!("local controls: {error:?}"))
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        left.iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.desired_axes = control;
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: self.sim.epoch(),
                tick: self.sim.tick(),
            },
            self.sim.timeline(),
        )
    }

    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        if !Self::same_control(self.desired_axes, self.admitted_axes) {
            let tick = self
                .sim
                .tick()
                .checked_add(1)
                .ok_or("input tick exhausted")?;
            let next_sequence = self
                .input_sequence
                .checked_add(1)
                .ok_or("input sequence exhausted")?;
            let frame = scene::control_frame(
                scene::LOCAL_PEER,
                self.desired_axes,
                tick,
                self.input_sequence,
            );
            self.sim
                .admit_frame(frame)
                .map_err(|error| format!("local input admission: {error:?}"))?;
            self.admitted_axes = self.desired_axes;
            self.input_sequence = next_sequence;
        }
        self.sim
            .step(&[])
            .map_err(|error| format!("simulation step: {error:?}"))?;
        let skipped_publication = !self.sim.has_current_page();
        let projected_changes = if skipped_publication {
            0
        } else {
            let page = self
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            // capture owns baseline handling as well as ordinary deltas.
            self.projections
                .as_mut()
                .map_or(0, |outbox| outbox.capture(&page))
        };
        Ok(WorldStep {
            projected_changes,
            skipped_publication,
        })
    }

    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        self.recover_page()
            .map_err(|error| format!("world page: {error:?}"))
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        // Recovery belongs to the clock's recovery hook, which also captures
        // derived output. A lease arriving here must not create another page.
        if !self.sim.has_current_page() {
            return Ok(None);
        }
        match self.sim.published_page() {
            Ok(page) => Ok(Some(WorldPublication {
                id: page.id(),
                timeline: page.timeline(),
                page,
            })),
            Err(SimError::PublicationBusy) => Ok(None),
            Err(error) => Err(format!("world page: {error:?}")),
        }
    }

    fn projection_lost_ticks(&self) -> u64 {
        self.projections
            .as_ref()
            .map_or(0, ProjectionOutbox::lost_ticks)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
