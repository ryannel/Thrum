//! The game's single authoritative clock, rules and optional projection output.

use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use thrum_core::ClockSample;
use thrum_core::{ClockConfig, ClockState, TickClock};
#[cfg(not(target_arch = "wasm32"))]
use thrum_core::{
    WorldEpoch,
    sim::{WorldPublication, WorldTask, WorldUpdate},
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

pub(super) struct WorldDriver {
    pub(super) clock: TickClock,
    pub(super) sim: Simulation,
    projections: Option<ProjectionOutbox>,
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
}

#[derive(Clone, Copy)]
pub(super) struct WorldAdvance {
    pub(super) tick: u64,
    pub(super) overload: Option<u64>,
    pub(super) completed_ticks: u32,
    pub(super) queued_ticks: u32,
    pub(super) projected_changes: usize,
    pub(super) projection_lost_ticks: u64,
    pub(super) skipped_publications: u64,
    pub(super) recovered_publication: bool,
    pub(super) tick_cpu: Duration,
}

impl WorldDriver {
    pub(super) fn new() -> Result<Self, String> {
        Ok(Self {
            clock: TickClock::new(ClockConfig::default(), Duration::ZERO)
                .map_err(|error| format!("clock: {error:?}"))?,
            sim: scene::simulation()?,
            projections: None,
            desired_axes: [0.0; 3],
            admitted_axes: [0.0; 3],
            input_sequence: 1,
        })
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.clock.presentation_alpha_at(lead)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn clock_sample(&self) -> ClockSample {
        self.clock.presentation_sample()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn held_controls(&self) -> [f32; 3] {
        self.desired_axes
    }

    pub(super) fn tick(&self) -> u64 {
        self.sim.tick()
    }

    pub(super) fn epoch(&self) -> u128 {
        self.sim.epoch().0
    }

    pub(super) fn timeline(&self) -> u64 {
        self.sim.timeline()
    }

    pub(super) fn has_current_page(&self) -> bool {
        self.sim.has_current_page()
    }

    pub(super) fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        self.sim.published_page()
    }

    pub(super) fn next_due_in(&self) -> Option<Duration> {
        self.clock.next_due_in()
    }

    pub(super) fn pause(&mut self) {
        self.clock.pause();
    }

    pub(super) fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.clock
            .resume(now)
            .map_err(|error| format!("clock resume: {error:?}"))
    }

    pub(super) fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        scene::control_frame(scene::LOCAL_PEER, axes, 0, 0)
            .validate_axes()
            .map_err(|error| format!("local controls: {error:?}"))?;
        self.desired_axes = axes;
        Ok(())
    }

    pub(super) fn controlled_position(&self) -> Option<[f64; 3]> {
        self.sim
            .handle(scene::CONTROLLED_ID)
            .and_then(|handle| self.sim.get(handle).ok())
            .map(|entity| entity.position.0)
    }

    pub(super) fn enable_projection(&mut self) -> Result<(), String> {
        if self.projections.is_none() {
            let page = self
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            let mut outbox = ProjectionOutbox::new(ENTITY_COUNT);
            outbox.reset_full(&page);
            self.projections = Some(outbox);
        }
        Ok(())
    }

    pub(super) fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        self.projections
            .as_mut()
            .map(|outbox| outbox.poll(self.sim.timeline(), consume))
            .transpose()
    }

    pub(super) fn projection_reserved_bytes(&self) -> usize {
        self.projections
            .as_ref()
            .map_or(0, ProjectionOutbox::reserved_bytes)
    }

    pub(super) fn reset_projection_baseline(&mut self) -> Result<(), String> {
        let outbox = self
            .projections
            .as_mut()
            .ok_or("projection output is disabled")?;
        let page = self
            .sim
            .published_page()
            .map_err(|error| format!("world page: {error:?}"))?;
        outbox.reset_full(&page);
        Ok(())
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        let mut tick_cpu = Duration::ZERO;
        let desired_axes = self.desired_axes;
        let admitted_axes = &mut self.admitted_axes;
        let input_sequence = &mut self.input_sequence;
        let sim = &mut self.sim;
        let projections = &mut self.projections;
        let mut projected_changes = 0;
        let mut skipped_publications = 0_u64;
        let clock = self
            .clock
            .advance(now, 4, || {
                let start = Instant::now();
                let result = (|| {
                    if desired_axes
                        .iter()
                        .zip(admitted_axes.iter())
                        .any(|(desired, admitted)| desired.to_bits() != admitted.to_bits())
                    {
                        let tick = sim.tick().checked_add(1).ok_or("input tick exhausted")?;
                        let frame = scene::control_frame(
                            scene::LOCAL_PEER,
                            desired_axes,
                            tick,
                            *input_sequence,
                        );
                        sim.admit_frame(frame)
                            .map_err(|error| format!("local input admission: {error:?}"))?;
                        *admitted_axes = desired_axes;
                        *input_sequence = input_sequence
                            .checked_add(1)
                            .ok_or("input sequence exhausted")?;
                    }
                    sim.step(&[])
                        .map_err(|error| format!("simulation step: {error:?}"))?;
                    if sim.has_current_page() {
                        let page = sim
                            .published_page()
                            .map_err(|error| format!("world page: {error:?}"))?;
                        if let Some(outbox) = projections.as_mut() {
                            if page.is_baseline() {
                                outbox.reset_full(&page);
                                projected_changes += page.len();
                            } else {
                                projected_changes += outbox.capture(&page);
                            }
                        }
                    } else {
                        skipped_publications = skipped_publications.saturating_add(1);
                    }
                    Ok::<(), String>(())
                })();
                tick_cpu += start.elapsed();
                result
            })
            .map_err(|error| format!("clock advance: {error:?}"))?;
        if let Some(error) = clock.step_error {
            return Err(format!("simulation tick failed: {error:?}"));
        }
        let recovered_publication = if self.sim.has_current_page() {
            false
        } else {
            match self.sim.published_page() {
                Ok(page) => {
                    if let Some(outbox) = self.projections.as_mut() {
                        projected_changes += outbox.capture(&page);
                    }
                    true
                }
                Err(SimError::PublicationBusy) => false,
                Err(error) => return Err(format!("world page: {error:?}")),
            }
        };
        Ok(WorldAdvance {
            tick: self.sim.tick(),
            overload: (clock.state == ClockState::Overloaded).then_some(clock.refused_on_overload),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            projected_changes,
            projection_lost_ticks: self
                .projections
                .as_ref()
                .map_or(0, ProjectionOutbox::lost_ticks),
            skipped_publications,
            recovered_publication,
            tick_cpu,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WorldTask for WorldDriver {
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

    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let page = self
            .published_page()
            .map_err(|error| format!("initial world page: {error:?}"))?;
        Ok(WorldUpdate {
            epoch: WorldEpoch(self.epoch()),
            tick: self.tick(),
            timeline: self.timeline(),
            clock: self.clock_sample(),
            completed_ticks: 0,
            queued_ticks: 0,
            tick_cpu: Duration::ZERO,
            skipped_publications: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            overload: None,
            publication: Some(WorldPublication {
                id: page.id(),
                timeline: page.timeline(),
                page,
            }),
            next_due: self.next_due_in(),
        })
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.set_controls(control)
    }

    fn advance(&mut self, now: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let report = Self::advance(self, now)?;
        let publication = if report.completed_ticks != 0 || report.recovered_publication {
            match self.published_page() {
                Ok(page) => Some(WorldPublication {
                    id: page.id(),
                    timeline: page.timeline(),
                    page,
                }),
                Err(SimError::PublicationBusy) => None,
                Err(error) => return Err(format!("world page: {error:?}")),
            }
        } else {
            None
        };
        Ok(WorldUpdate {
            epoch: WorldEpoch(self.epoch()),
            tick: report.tick,
            timeline: self.timeline(),
            clock: self.clock_sample(),
            completed_ticks: report.completed_ticks,
            queued_ticks: report.queued_ticks,
            tick_cpu: report.tick_cpu,
            skipped_publications: report.skipped_publications,
            projected_changes: report.projected_changes,
            projection_lost_ticks: report.projection_lost_ticks,
            overload: report.overload,
            publication,
            next_due: self.next_due_in(),
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
