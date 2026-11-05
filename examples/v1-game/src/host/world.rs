//! The game's single authoritative clock, rules and optional projection output.

use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use thrum_core::{ClockConfig, ClockState, TickClock};

use crate::{
    projection::{GameProjection, ProjectionGap, ProjectionOutbox},
    scene::{self, ENTITY_COUNT},
    simulation::{SimError, Simulation},
};

pub(super) struct WorldDriver {
    pub(super) clock: TickClock,
    pub(super) sim: Simulation,
    projections: Option<ProjectionOutbox>,
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
}

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
