//! One fixed-step clock around a game-owned authority and its optional outputs.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use std::{fmt, time::Duration};
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::{ClockConfig, ClockError, ClockSample, ClockState, TickClock, WorldEpoch};

use super::SnapshotId;

/// A publication's payload and its matching authority identity travel together.
pub struct WorldPublication<P> {
    /// Exact committed endpoint identity.
    pub id: SnapshotId,
    /// Replay branch; equal numeric ticks on different branches are distinct.
    pub timeline: u64,
    /// Immutable, cheaply cloned page lease.
    pub page: P,
}

/// One owner update. Counts are increments; the lane accumulates them for readers.
pub struct WorldUpdate<P> {
    /// Committed authority epoch.
    pub epoch: WorldEpoch,
    /// Committed authority tick.
    pub tick: u64,
    /// Replay branch generation.
    pub timeline: u64,
    /// Read-only phase sample from the owner's clock.
    pub clock: ClockSample,
    /// Updates committed by this call.
    pub completed_ticks: u32,
    /// Updates still due after this call.
    pub queued_ticks: u32,
    /// Authoritative tick callback CPU time.
    pub tick_cpu: Duration,
    /// Views omitted while authority continued.
    pub skipped_publications: u64,
    /// Optional derived output changed by this call.
    pub projected_changes: usize,
    /// Cumulative loss of optional derived output.
    pub projection_lost_ticks: u64,
    /// First refused tick count, if the clock overloaded.
    pub overload: Option<u64>,
    /// New page, or `None` when optional publication is busy or unchanged.
    pub publication: Option<WorldPublication<P>>,
    /// Delay to the next authority wake, measured from this call's input time.
    pub next_due: Option<Duration>,
}

/// Progress from one inline drive or since a resident reader's previous poll.
#[derive(Clone, Copy, Debug)]
pub struct WorldAdvance {
    /// Latest authority tick, even when publication is unavailable.
    pub tick: u64,
    /// Sticky first refused tick count after overload.
    pub overload: Option<u64>,
    /// Ticks committed during this drive or since the preceding poll.
    pub completed_ticks: u32,
    /// Due ticks remaining.
    pub queued_ticks: u32,
    /// Optional derived changes during this drive or since the preceding poll.
    pub projected_changes: usize,
    /// Cumulative optional output loss.
    pub projection_lost_ticks: u64,
    /// Views omitted during this drive or since the preceding poll.
    pub skipped_publications: u64,
    /// A page appeared without a new committed tick.
    pub recovered_publication: bool,
    /// Tick callback CPU during this drive or since the preceding poll.
    pub tick_cpu: Duration,
}

/// Output of one successfully committed game tick.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldStep {
    /// Optional derived values copied from this tick's publication.
    pub projected_changes: usize,
    /// The tick committed without an optional presentation publication.
    pub skipped_publication: bool,
}

/// Game authority without a competing wall-time clock or render scheduler.
/// Each successful step commits once. A failed step must leave authority
/// unchanged because its fixed-clock tick remains queued for retry.
pub trait WorldGame {
    /// Continuous controls; discrete actions keep their ordered admission path.
    type Control: Copy + Send + Sync;
    /// Immutable, cheaply cloned committed-page lease.
    type Page: Clone + Send + Sync;
    /// Authority or optional-output failure.
    type Error: fmt::Display + Send;

    /// Validate controls before changing held game input.
    /// # Errors
    /// Refuses invalid controls without changing the game.
    fn validate_control(control: Self::Control) -> Result<(), Self::Error>;
    /// Preserve the game's equality rule for continuous controls.
    fn same_control(left: Self::Control, right: Self::Control) -> bool;
    /// Replace already validated controls before the next game tick.
    /// # Errors
    /// Returns an authority failure without partially applying controls.
    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error>;
    /// Current committed authority endpoint and replay timeline.
    fn endpoint(&self) -> (SnapshotId, u64);
    /// Commit one tick and capture its optional derived outputs.
    /// # Errors
    /// Returns a failure with the attempted tick uncommitted.
    fn step(&mut self) -> Result<WorldStep, Self::Error>;
    /// Recover a previously busy publication without committing another tick.
    /// `Some(0)` still identifies a newly recovered publication. `None` means
    /// publication is still busy or already current. Capture derived output
    /// exactly once when a publication recovers.
    /// # Errors
    /// Returns the game's publication or derived-output failure.
    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error>;
    /// Lease the publication with its actual identity, which may lag authority.
    /// Creating or recovering it belongs to `step`/`recover_publication`, so
    /// derived output is captured with the same publication.
    /// # Errors
    /// Returns a publication fault; `None` denotes optional publication busy.
    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error>;
    /// Cumulative optional derived-output loss.
    fn projection_lost_ticks(&self) -> u64 {
        0
    }
}

/// Failure at the fixed-clock/game boundary.
#[derive(Debug)]
pub enum ClockedWorldError<E> {
    /// The supplied clock configuration or time operation was refused.
    Clock(ClockError),
    /// The game's control, tick or publication operation failed.
    Game(E),
}

impl<E: fmt::Display> fmt::Display for ClockedWorldError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clock(error) => write!(formatter, "world clock: {error:?}"),
            Self::Game(error) => write!(formatter, "world game: {error}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ClockedWorldError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Clock(_) => None,
            Self::Game(error) => Some(error),
        }
    }
}

/// The sole wall-time clock for a game authority, usable inline or in WorldLane.
/// Orchestration retains the clock and overload status; the game owns its storage.
pub struct ClockedWorld<G: WorldGame> {
    clock: TickClock,
    game: G,
    overload: Option<u64>,
}

impl<G: WorldGame> ClockedWorld<G> {
    /// Begin the game's fixed clock at the platform's monotonic origin.
    /// # Errors
    /// Refuses an invalid clock configuration without stepping the game.
    pub fn new(
        game: G,
        config: ClockConfig,
        origin: Duration,
    ) -> Result<Self, ClockedWorldError<G::Error>> {
        Ok(Self {
            clock: TickClock::new(config, origin).map_err(ClockedWorldError::Clock)?,
            game,
            overload: None,
        })
    }

    /// Inspect the owned game without transferring clock authority.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// Exclusively access the game between clock operations.
    pub fn game_mut(&mut self) -> &mut G {
        &mut self.game
    }

    /// Read-only phase sample for a renderer sharing the monotonic origin.
    pub fn presentation_sample(&self) -> ClockSample {
        self.clock.presentation_sample()
    }

    /// Sample a display target without advancing the world.
    pub fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.clock.presentation_alpha_at(lead)
    }

    /// Time until another authoritative update is due.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.clock.next_due_in()
    }

    /// Stop wall-time admission while retaining accepted pending ticks.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume with a new monotonic origin after accepted ticks drain.
    /// # Errors
    /// Refuses an active clock, pending ticks or backwards time.
    pub fn resume(&mut self, now: Duration) -> Result<(), ClockedWorldError<G::Error>> {
        self.clock.resume(now).map_err(ClockedWorldError::Clock)?;
        self.overload = None;
        Ok(())
    }

    /// Validate and replace the game's held continuous controls.
    /// # Errors
    /// Refuses invalid input before calling the game's control replacement.
    pub fn set_controls(&mut self, control: G::Control) -> Result<(), ClockedWorldError<G::Error>> {
        G::validate_control(control).map_err(ClockedWorldError::Game)?;
        self.game
            .apply_control(control)
            .map_err(ClockedWorldError::Game)
    }

    /// Admit observed monotonic time and commit only the configured tick cap.
    /// Optional publication pressure never consumes a tick without committing
    /// it. Recovery can publish a page even when no new tick is due.
    /// # Errors
    /// Returns a clock refusal or the first game error. A failed tick remains
    /// queued; earlier successful ticks from this call remain committed.
    pub fn advance(&mut self, now: Duration) -> Result<WorldAdvance, ClockedWorldError<G::Error>> {
        let mut tick_cpu = Duration::ZERO;
        let mut projected_changes = 0_usize;
        let mut skipped_publications = 0_u64;
        let game = &mut self.game;
        let report = self
            .clock
            .advance(now, u32::MAX, || {
                let start = Instant::now();
                let step = game.step();
                tick_cpu += start.elapsed();
                let step = step?;
                projected_changes = projected_changes.saturating_add(step.projected_changes);
                skipped_publications =
                    skipped_publications.saturating_add(u64::from(step.skipped_publication));
                Ok(())
            })
            .map_err(ClockedWorldError::Clock)?;
        if report.state == ClockState::Overloaded && self.overload.is_none() {
            self.overload = Some(report.refused_on_overload);
        }
        if let Some(error) = report.step_error {
            return Err(ClockedWorldError::Game(error));
        }
        let recovered = self
            .game
            .recover_publication()
            .map_err(ClockedWorldError::Game)?;
        projected_changes = projected_changes.saturating_add(recovered.unwrap_or(0));
        Ok(WorldAdvance {
            tick: self.game.endpoint().0.tick,
            overload: self.overload,
            completed_ticks: report.completed,
            queued_ticks: report.queued,
            projected_changes,
            projection_lost_ticks: self.game.projection_lost_ticks(),
            skipped_publications,
            recovered_publication: recovered.is_some(),
            tick_cpu,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<G> super::WorldTask for ClockedWorld<G>
where
    G: WorldGame + Send + 'static,
    G::Control: 'static,
    G::Page: 'static,
    G::Error: 'static,
{
    type Control = G::Control;
    type Page = G::Page;
    type Error = ClockedWorldError<G::Error>;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        G::validate_control(control).map_err(ClockedWorldError::Game)
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        G::same_control(left, right)
    }

    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        // Sealing the initial page uses the same recovery hook as later
        // pressure recovery; leasing itself must never publish behind it.
        let recovered = self
            .game
            .recover_publication()
            .map_err(ClockedWorldError::Game)?;
        let (id, timeline) = self.game.endpoint();
        Ok(WorldUpdate {
            epoch: id.epoch,
            tick: id.tick,
            timeline,
            clock: self.presentation_sample(),
            completed_ticks: 0,
            queued_ticks: self.clock.queued_ticks(),
            tick_cpu: Duration::ZERO,
            skipped_publications: 0,
            projected_changes: recovered.unwrap_or(0),
            projection_lost_ticks: self.game.projection_lost_ticks(),
            overload: self.overload,
            publication: self.game.publication().map_err(ClockedWorldError::Game)?,
            next_due: self.next_due_in(),
        })
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.game
            .apply_control(control)
            .map_err(ClockedWorldError::Game)
    }

    fn pause(&mut self) {
        if self.clock.state() != ClockState::Paused {
            Self::pause(self);
        }
    }

    fn resume(&mut self, now: Duration) -> Result<(), Self::Error> {
        if self.clock.state() == ClockState::Active {
            return Ok(());
        }
        Self::resume(self, now)
    }

    fn advance(&mut self, now: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let progress = Self::advance(self, now)?;
        let (id, timeline) = self.game.endpoint();
        let publication = if progress.completed_ticks != 0 || progress.recovered_publication {
            self.game.publication().map_err(ClockedWorldError::Game)?
        } else {
            None
        };
        Ok(WorldUpdate {
            epoch: id.epoch,
            tick: id.tick,
            timeline,
            clock: self.presentation_sample(),
            completed_ticks: progress.completed_ticks,
            queued_ticks: progress.queued_ticks,
            tick_cpu: progress.tick_cpu,
            skipped_publications: progress.skipped_publications,
            projected_changes: progress.projected_changes,
            projection_lost_ticks: progress.projection_lost_ticks,
            overload: progress.overload,
            publication,
            next_due: self.next_due_in(),
        })
    }
}

#[cfg(test)]
mod tests;
