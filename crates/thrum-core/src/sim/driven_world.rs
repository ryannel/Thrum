//! A world whose driver owns tick admission and presentation time.

use std::{error::Error, fmt, num::NonZeroU32, time::Duration};

use crate::ClockSample;

#[cfg(not(target_arch = "wasm32"))]
use super::WorldUpdate;
use super::{SnapshotId, WorldAdvance, WorldPublication};

/// Result of a driver's bounded advance.
pub struct DrivenAdvance {
    /// Number of ticks committed by this call, excluding restored endpoints.
    pub completed_ticks: u32,
    /// Fraction between the resulting endpoint pair and its rate.
    pub presentation: ClockSample,
    /// Time until the driver should next be polled on a resident lane.
    pub next_due: Duration,
}

/// Owns authority and decides how many ticks to commit during each advance.
pub trait WorldDriver {
    /// Continuous controls handed to the owner at the advance barrier.
    type Control: Copy + Send + Sync;
    /// Immutable committed-page lease.
    type Page: Clone + Send + Sync;
    /// A failure in control admission, advancement, or publication.
    type Error: Error + 'static;

    /// Validate controls before admission.
    /// # Errors
    /// Refuses invalid controls.
    fn validate_control(control: Self::Control) -> Result<(), Self::Error>;
    /// The authored equality rule for held controls.
    fn same_control(left: Self::Control, right: Self::Control) -> bool;
    /// Replace already validated held controls without committing a tick.
    /// # Errors
    /// Returns a driver refusal.
    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error>;
    /// Current authority identity and replay timeline.
    fn endpoint(&self) -> (SnapshotId, u64);
    /// Commit exactly the ticks the driver admits, up to `cap`. A restore may
    /// establish a new endpoint independently of the number of ticks committed.
    /// Return a sample anchored to `now` for the resulting pair.
    /// # Errors
    /// Returns the driver's authority failure.
    fn advance(&mut self, now: Duration, cap: NonZeroU32) -> Result<DrivenAdvance, Self::Error>;
    /// Lease the actual committed page, or report that publication is busy.
    /// # Errors
    /// Returns the publication failure.
    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error>;
}

/// A failure while driving a world.
#[derive(Debug)]
pub enum DrivenWorldError<E> {
    /// A driver operation failed.
    Driver(E),
    /// The driver reported more committed ticks than the catch-up cap.
    CatchUpCap,
    /// The driver received a timestamp before the prior observation.
    NonMonotonicTime,
    /// The presentation sample was not anchored to this advance.
    PresentationSample,
}

impl<E: Error> fmt::Display for DrivenWorldError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(error) => write!(formatter, "world driver: {error}"),
            Self::CatchUpCap => formatter.write_str("world driver exceeded its catch-up cap"),
            Self::NonMonotonicTime => formatter.write_str("driven world time went backwards"),
            Self::PresentationSample => {
                formatter.write_str("driver sample must be anchored to the current advance")
            }
        }
    }
}

impl<E: Error + 'static> Error for DrivenWorldError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::CatchUpCap | Self::NonMonotonicTime | Self::PresentationSample => None,
        }
    }
}

/// Driver-owned authority for either an inline observer or the resident world lane.
pub struct DrivenWorld<D: WorldDriver> {
    driver: D,
    cap: NonZeroU32,
    sample: ClockSample,
    now: Duration,
    paused: bool,
    next_due: Option<Duration>,
    page: Option<WorldPublication<D::Page>>,
}

impl<D: WorldDriver> DrivenWorld<D> {
    /// Start at the existing endpoint, holding there until the first driver sample.
    pub fn new(driver: D, cap: NonZeroU32) -> Self {
        Self {
            driver,
            cap,
            sample: ClockSample::held(Duration::ZERO, 1.0),
            now: Duration::ZERO,
            paused: false,
            next_due: Some(Duration::ZERO),
            page: None,
        }
    }

    /// Read the game-owned driver.
    pub fn driver(&self) -> &D {
        &self.driver
    }

    /// Access the inline driver's inputs or game-owned observations.
    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    /// The current committed identity and replay timeline.
    pub fn endpoint(&self) -> (SnapshotId, u64) {
        self.driver.endpoint()
    }

    /// Whether a page is available for the current endpoint.
    pub fn has_current_page(&self) -> bool {
        self.page
            .as_ref()
            .is_some_and(|page| (page.id, page.timeline) == self.endpoint())
    }

    /// Lease the current available publication.
    /// # Errors
    /// Returns the driver's publication failure.
    pub fn publication(
        &mut self,
    ) -> Result<Option<WorldPublication<D::Page>>, DrivenWorldError<D::Error>> {
        self.refresh_page()?;
        Ok(self.page.as_ref().map(|page| WorldPublication {
            id: page.id,
            timeline: page.timeline,
            page: page.page.clone(),
        }))
    }

    fn refresh_page(&mut self) -> Result<bool, DrivenWorldError<D::Error>> {
        if self.has_current_page() {
            return Ok(false);
        }
        let endpoint = self.endpoint();
        self.page = self
            .driver
            .publication()
            .map_err(DrivenWorldError::Driver)?
            .filter(|page| (page.id, page.timeline) == endpoint);
        Ok(self.page.is_some())
    }

    /// Sample just the presentation side at the most recent observation plus lead.
    pub fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.sample.alpha_at(self.now.saturating_add(lead))
    }

    /// Stop calling the driver. The next observation freezes the current phase.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Allow the driver to run again at the next observation.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Advance using only the driver. Elapsed time never queues authority work.
    /// # Errors
    /// Returns a backwards timestamp, exceeded cap, invalid sample, or driver failure.
    pub fn advance(&mut self, now: Duration) -> Result<WorldAdvance, DrivenWorldError<D::Error>> {
        if now < self.now {
            return Err(DrivenWorldError::NonMonotonicTime);
        }
        self.now = now;
        let completed_ticks = if self.paused {
            self.sample = ClockSample::held(now, self.sample.alpha_at(now));
            self.next_due = None;
            0
        } else {
            let advance = self
                .driver
                .advance(now, self.cap)
                .map_err(DrivenWorldError::Driver)?;
            if advance.completed_ticks > self.cap.get() {
                return Err(DrivenWorldError::CatchUpCap);
            }
            if !advance.presentation.is_driven_at(now) {
                return Err(DrivenWorldError::PresentationSample);
            }
            self.sample = advance.presentation;
            self.next_due = Some(advance.next_due);
            advance.completed_ticks
        };
        let published = self.refresh_page()?;
        Ok(WorldAdvance {
            tick: self.endpoint().0.tick,
            overload: None,
            completed_ticks,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: published && completed_ticks == 0,
            tick_cpu: Duration::ZERO,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn update(
        &mut self,
        completed_ticks: u32,
        publish: bool,
    ) -> Result<WorldUpdate<D::Page>, DrivenWorldError<D::Error>> {
        let (id, timeline) = self.endpoint();
        Ok(WorldUpdate {
            epoch: id.epoch,
            tick: id.tick,
            timeline,
            clock: self.sample,
            completed_ticks,
            queued_ticks: 0,
            tick_cpu: Duration::ZERO,
            skipped_publications: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            overload: None,
            publication: if publish { self.publication()? } else { None },
            next_due: self.next_due,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<D> super::WorldTask for DrivenWorld<D>
where
    D: WorldDriver + Send + 'static,
    D::Control: 'static,
    D::Page: 'static,
    D::Error: Send + Sync,
{
    type Control = D::Control;
    type Page = D::Page;
    type Error = DrivenWorldError<D::Error>;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        D::validate_control(control).map_err(DrivenWorldError::Driver)
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        D::same_control(left, right)
    }

    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        self.update(0, true)
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.driver
            .apply_control(control)
            .map_err(DrivenWorldError::Driver)
    }

    fn advance(&mut self, now: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let progress = Self::advance(self, now)?;
        self.update(
            progress.completed_ticks,
            progress.completed_ticks != 0 || progress.recovered_publication,
        )
    }

    fn pause(&mut self) {
        Self::pause(self);
    }

    fn resume(&mut self, _now: Duration) -> Result<(), Self::Error> {
        Self::resume(self);
        Ok(())
    }
}
