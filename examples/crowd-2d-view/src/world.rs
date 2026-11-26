use std::{num::NonZeroU32, sync::Arc, time::Duration};

use crowd_2d::{Simulation, frames};
use thrum_core::{
    ClockConfig, ClockError, ClockState, TickClock,
    sim::{PublicationPage, WorldAdvance, WorldPublication},
};
use thrum_render::{
    extract::SourceTick,
    runtime::{FrameError, FrameWorld},
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Inline owner of the reference replay and its public tick pages.
pub struct CrowdWorld {
    simulation: Simulation,
    exact_steps: bool,
    end_tick: Option<u64>,
    clock: TickClock,
    overload: Option<u64>,
}

impl CrowdWorld {
    /// Construct the retained replay at a count validated by the caller.
    ///
    /// # Panics
    /// Panics if the built-in rule or clock configuration cannot be initialized.
    #[must_use]
    pub fn new(count: u32, exact_steps: bool) -> Self {
        Self {
            simulation: Simulation::for_presentation(count, 1),
            exact_steps,
            end_tick: None,
            clock: TickClock::new(
                ClockConfig {
                    // Realtime capture and a contended renderer can briefly fall behind.
                    // Keep accepted authority ticks queued; render still limits per-call work.
                    max_queued_ticks: NonZeroU32::new(32).expect("nonzero queue"),
                    ..ClockConfig::default()
                },
                Duration::ZERO,
            )
            .expect("valid Crowd clock"),
            overload: None,
        }
    }

    /// Hold the last publication from this tick onwards.
    #[must_use]
    pub fn through(mut self, tick: u64) -> Self {
        self.end_tick = Some(tick);
        self
    }

    /// Extend a held replay to a later tick. An unbounded replay stays unbounded.
    pub fn extend_through(&mut self, tick: u64) {
        if let Some(end) = &mut self.end_tick {
            *end = (*end).max(tick);
        }
    }

    /// Most recently committed tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.simulation.runtime.tick()
    }

    /// Read the unmodified authority digest.
    #[must_use]
    pub fn digest(&self) -> u64 {
        self.simulation.digest()
    }

    pub(crate) fn pause_clock(&mut self) {
        self.clock.pause();
    }

    pub(crate) fn resume_clock(&mut self, now: Duration) -> Result<(), ClockError> {
        self.clock.resume(now)?;
        self.overload = None;
        Ok(())
    }
}

fn step(simulation: &mut Simulation) -> Result<(), FrameError> {
    let input = frames(simulation.runtime.tick() + 1, true);
    simulation
        .history
        .step(
            &mut simulation.runtime,
            &mut simulation.game,
            &mut simulation.candidate,
            &input,
        )
        .map_err(|error| FrameError::World(format!("Crowd trace: {error:?}")))?;
    Ok(())
}

impl FrameWorld for CrowdWorld {
    type Page = Arc<PublicationPage>;

    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            timeline: 0,
            tick: self.tick(),
        }
    }

    fn has_current_page(&self) -> bool {
        self.simulation.runtime.has_current_publication()
    }

    fn is_threaded(&self) -> bool {
        false
    }

    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        if self.end_tick.is_some_and(|end| self.tick() >= end) {
            return Ok(WorldAdvance {
                tick: self.tick(),
                overload: None,
                completed_ticks: 0,
                queued_ticks: 0,
                projected_changes: 0,
                projection_lost_ticks: 0,
                skipped_publications: 0,
                recovered_publication: false,
                tick_cpu: Duration::ZERO,
            });
        }
        let steps = self.end_tick.map_or(u32::MAX, |end| {
            u32::try_from(end.saturating_sub(self.tick())).unwrap_or(u32::MAX)
        });
        let mut tick_cpu = Duration::ZERO;
        let mut tick = || {
            let start = Instant::now();
            let result = step(&mut self.simulation);
            tick_cpu += start.elapsed();
            result
        };
        let (completed_ticks, queued_ticks) =
            if self.exact_steps && self.clock.state() == ClockState::Active {
                tick()?;
                (1, 0)
            } else {
                let report = self
                    .clock
                    .advance(now, steps, tick)
                    .map_err(|error| FrameError::World(error.to_string()))?;
                if let Some(error) = report.step_error {
                    return Err(error);
                }
                if report.state == ClockState::Overloaded {
                    self.overload.get_or_insert(report.refused_on_overload);
                }
                (report.completed, report.queued)
            };
        Ok(WorldAdvance {
            tick: self.tick(),
            completed_ticks,
            queued_ticks,
            overload: if self.clock.state() == ClockState::Paused {
                None
            } else {
                self.overload
            },
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu,
        })
    }

    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        let page = self
            .simulation
            .runtime
            .latest()
            .ok_or(FrameError::InvalidState("missing Crowd publication"))?;
        Ok(WorldPublication {
            id: page.id(),
            timeline: page.timeline(),
            page,
        })
    }

    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        if self.exact_steps {
            1.0
        } else {
            self.clock.presentation_alpha_at(lead)
        }
    }

    fn display_phase_at(&self, lead: Duration) -> f32 {
        if self.exact_steps {
            1.0
        } else {
            self.clock.display_phase_at(lead)
        }
    }

    fn is_interpolating(&self) -> bool {
        !self.exact_steps && self.clock.presentation_sample().is_interpolating()
    }

    fn tick_due_time(&self) -> Option<Duration> {
        if self.exact_steps {
            None
        } else {
            self.clock.presentation_sample().tick_due_time()
        }
    }
}
