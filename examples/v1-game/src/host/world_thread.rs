//! Autonomous native world lane. The render lane observes immutable pages.

use std::{
    sync::{Arc, Condvar, Mutex, TryLockError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use thrum_core::ClockSample;

use crate::{
    scene,
    simulation::{PublishedPage, SimError},
};

use super::world::{WorldAdvance, WorldDriver};

#[derive(Clone, Copy)]
struct WorldSnapshot {
    epoch: u128,
    timeline: u64,
    tick: u64,
    clock: ClockSample,
    completed_total: u64,
    tick_cpu_total: Duration,
    skipped_total: u64,
    projected_total: usize,
    projection_lost_ticks: u64,
    queued_ticks: u32,
    overload: Option<u64>,
    page_sequence: u64,
}

struct SharedState {
    snapshot: WorldSnapshot,
    page: Option<PublishedPage>,
    controls: [f32; 3],
    control_sequence: u64,
    shutdown: bool,
    fault: Option<String>,
}

struct Shared {
    state: Mutex<SharedState>,
    wake: Condvar,
}

/// A normalized input mailbox that does not wait for display or GPU work.
#[derive(Clone)]
pub struct WorldControls {
    shared: Arc<Shared>,
}

impl WorldControls {
    /// Replace continuous local axes for the next world update.
    /// # Errors
    /// Refuses invalid axes or a stopped simulation owner.
    pub fn set_controls(&self, axes: [f32; 3]) -> Result<(), String> {
        scene::control_frame(scene::LOCAL_PEER, axes, 0, 0)
            .validate_axes()
            .map_err(|error| format!("local controls: {error:?}"))?;
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| "world control mailbox poisoned".to_owned())?;
        if state.shutdown || state.fault.is_some() {
            return Err("world owner has stopped".to_owned());
        }
        if state
            .controls
            .iter()
            .zip(axes)
            .all(|(old, new)| old.to_bits() == new.to_bits())
        {
            return Ok(());
        }
        state.controls = axes;
        state.control_sequence = state.control_sequence.wrapping_add(1);
        drop(state);
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Observe a world fault without scheduling a render callback.
    /// # Errors
    /// Reports a failed owner, a fixed-clock overload or a poisoned mailbox.
    pub fn check_health(&self) -> Result<u64, String> {
        let state = self
            .shared
            .state
            .lock()
            .map_err(|_| "world state mailbox poisoned".to_owned())?;
        if let Some(error) = &state.fault {
            return Err(format!("world owner: {error}"));
        }
        if let Some(refused) = state.snapshot.overload {
            return Err(format!("fixed-clock overload: {refused} ticks refused"));
        }
        if state.shutdown {
            return Err("world owner has stopped".to_owned());
        }
        Ok(state.snapshot.tick)
    }
}

pub(super) struct WorldClient {
    shared: Arc<Shared>,
    controls: WorldControls,
    join: Option<JoinHandle<()>>,
    snapshot: WorldSnapshot,
    page: Option<PublishedPage>,
    last_completed: u64,
    last_tick_cpu: Duration,
    last_skipped: u64,
    last_projected: usize,
    last_page_sequence: u64,
    sample_now: Duration,
}

impl WorldClient {
    pub(super) fn start(
        mut driver: WorldDriver,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Self, WorldControls), String> {
        let page = driver
            .published_page()
            .map_err(|error| format!("initial world page: {error:?}"))?;
        let snapshot = WorldSnapshot {
            epoch: driver.epoch(),
            timeline: driver.timeline(),
            tick: driver.tick(),
            clock: driver.clock_sample(),
            completed_total: 0,
            tick_cpu_total: Duration::ZERO,
            skipped_total: 0,
            projected_total: 0,
            projection_lost_ticks: 0,
            queued_ticks: 0,
            overload: None,
            page_sequence: 1,
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(SharedState {
                snapshot,
                page: Some(page.clone()),
                controls: driver.held_controls(),
                control_sequence: 0,
                shutdown: false,
                fault: None,
            }),
            wake: Condvar::new(),
        });
        let controls = WorldControls {
            shared: Arc::clone(&shared),
        };
        let worker_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("thrum-v1-world".to_owned())
            .spawn(move || {
                if let Err(error) = run_world(&mut driver, origin, &worker_shared, &wake) {
                    if let Ok(mut state) = worker_shared.state.lock() {
                        state.fault = Some(error);
                    }
                    wake();
                }
            })
            .map_err(|error| format!("world worker: {error}"))?;
        let client = Self {
            shared,
            controls: controls.clone(),
            join: Some(join),
            snapshot,
            page: Some(page),
            last_completed: 0,
            last_tick_cpu: Duration::ZERO,
            last_skipped: 0,
            last_projected: 0,
            last_page_sequence: 1,
            sample_now: Duration::ZERO,
        };
        Ok((client, controls))
    }

    pub(super) fn poll(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        self.sample_now = now;
        match self.shared.state.try_lock() {
            Ok(state) => {
                if let Some(error) = &state.fault {
                    return Err(format!("world owner: {error}"));
                }
                if state.snapshot.page_sequence != self.snapshot.page_sequence {
                    self.page.clone_from(&state.page);
                }
                self.snapshot = state.snapshot;
            }
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Poisoned(_)) => {
                return Err("world state mailbox poisoned".to_owned());
            }
        }
        if self.join.as_ref().is_some_and(JoinHandle::is_finished) {
            return Err("world owner stopped without shutdown".to_owned());
        }
        let completed = self
            .snapshot
            .completed_total
            .saturating_sub(self.last_completed);
        let skipped = self
            .snapshot
            .skipped_total
            .saturating_sub(self.last_skipped);
        let projected = self
            .snapshot
            .projected_total
            .saturating_sub(self.last_projected);
        let tick_cpu = self
            .snapshot
            .tick_cpu_total
            .saturating_sub(self.last_tick_cpu);
        let recovered_publication =
            self.snapshot.page_sequence != self.last_page_sequence && completed == 0;
        self.last_completed = self.snapshot.completed_total;
        self.last_skipped = self.snapshot.skipped_total;
        self.last_projected = self.snapshot.projected_total;
        self.last_tick_cpu = self.snapshot.tick_cpu_total;
        self.last_page_sequence = self.snapshot.page_sequence;
        Ok(WorldAdvance {
            tick: self.snapshot.tick,
            overload: self.snapshot.overload,
            completed_ticks: u32::try_from(completed).unwrap_or(u32::MAX),
            queued_ticks: self.snapshot.queued_ticks,
            projected_changes: projected,
            projection_lost_ticks: self.snapshot.projection_lost_ticks,
            skipped_publications: skipped,
            recovered_publication,
            tick_cpu,
        })
    }

    pub(super) fn tick(&self) -> u64 {
        self.snapshot.tick
    }

    pub(super) fn epoch(&self) -> u128 {
        self.snapshot.epoch
    }

    pub(super) fn timeline(&self) -> u64 {
        self.snapshot.timeline
    }

    pub(super) fn has_current_page(&self) -> bool {
        self.page.as_ref().is_some_and(|page| {
            page.id().epoch.0 == self.snapshot.epoch
                && page.id().tick == self.snapshot.tick
                && page.timeline() == self.snapshot.timeline
        })
    }

    pub(super) fn published_page(&self) -> Result<PublishedPage, SimError> {
        if !self.has_current_page() {
            return Err(SimError::PublicationBusy);
        }
        self.page.clone().ok_or(SimError::PublicationBusy)
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.snapshot
            .clock
            .alpha_at(self.sample_now.saturating_add(lead))
    }

    pub(super) fn set_controls(&self, axes: [f32; 3]) -> Result<(), String> {
        self.controls.set_controls(axes)
    }
}

impl Drop for WorldClient {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.shutdown = true;
        }
        self.shared.wake.notify_one();
        if let Some(join) = self.join.take() {
            drop(join.join());
        }
    }
}

fn run_world(
    driver: &mut WorldDriver,
    origin: Instant,
    shared: &Arc<Shared>,
    wake: &impl Fn(),
) -> Result<(), String> {
    let mut applied_controls = 0;
    loop {
        let (controls, sequence) = {
            let state = shared
                .state
                .lock()
                .map_err(|_| "world state mailbox poisoned".to_owned())?;
            if state.shutdown {
                return Ok(());
            }
            (state.controls, state.control_sequence)
        };
        if sequence != applied_controls {
            driver.set_controls(controls)?;
            applied_controls = sequence;
        }
        let advanced = driver.advance(origin.elapsed())?;
        let page = if advanced.completed_ticks != 0 || advanced.recovered_publication {
            driver.published_page().ok()
        } else {
            None
        };
        let mut state = shared
            .state
            .lock()
            .map_err(|_| "world state mailbox poisoned".to_owned())?;
        let status = &mut state.snapshot;
        status.tick = advanced.tick;
        status.epoch = driver.epoch();
        status.timeline = driver.timeline();
        status.clock = driver.clock_sample();
        status.completed_total = status
            .completed_total
            .saturating_add(u64::from(advanced.completed_ticks));
        status.tick_cpu_total = status.tick_cpu_total.saturating_add(advanced.tick_cpu);
        status.skipped_total = status
            .skipped_total
            .saturating_add(advanced.skipped_publications);
        status.projected_total = status
            .projected_total
            .saturating_add(advanced.projected_changes);
        status.projection_lost_ticks = advanced.projection_lost_ticks;
        status.queued_ticks = advanced.queued_ticks;
        status.overload = match (status.overload, advanced.overload) {
            (Some(first), Some(0)) => Some(first),
            (_, current) => current,
        };
        if let Some(page) = page {
            state.page = Some(page);
            state.snapshot.page_sequence = state.snapshot.page_sequence.wrapping_add(1);
        }
        let sequence = state.control_sequence;
        drop(state);
        if advanced.completed_ticks != 0 || advanced.overload.is_some() {
            wake();
        }
        let delay = driver.next_due_in().unwrap_or(Duration::from_millis(100));
        let state = shared
            .state
            .lock()
            .map_err(|_| "world state mailbox poisoned".to_owned())?;
        drop(
            shared
                .wake
                .wait_timeout_while(state, delay, |state| {
                    !state.shutdown && state.control_sequence == sequence
                })
                .map_err(|_| "world state mailbox poisoned".to_owned())?,
        );
    }
}

#[cfg(test)]
mod tests;
