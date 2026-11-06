//! One resident world owner and one latest immutable publication for rendering.
//! The game supplies its clock and rules; this lane only schedules that owner.

use std::{
    cell::Cell,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Condvar, Mutex, TryLockError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{ClockSample, WorldEpoch};

use super::{SnapshotId, WorldAdvance, WorldPublication, WorldUpdate};

/// A game-owned clock and rules task moved into the resident engine lane.
/// Continuous controls replace one sampled state; discrete actions use the
/// game's admitted input path and must never be coalesced here.
pub trait WorldTask: Send + 'static {
    /// Already validated continuous control state.
    type Control: Copy + Send + Sync + 'static;
    /// Immutable publication lease.
    type Page: Clone + Send + Sync + 'static;
    /// Game's authority failure; the lane retains its display text in a fault.
    type Error: fmt::Display + Send + 'static;

    /// Validate a control before it enters the mailbox.
    /// # Errors
    /// Refuses invalid game-authored input without changing held controls.
    fn validate_control(control: Self::Control) -> Result<(), Self::Error>;
    /// Preserve the game's authoritative control equality rule.
    fn same_control(left: Self::Control, right: Self::Control) -> bool;
    /// Capture the initial endpoint and clock without advancing the world.
    /// # Errors
    /// Returns setup or initial-publication failure.
    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error>;
    /// Apply one validated control replacement before the next world update.
    /// # Errors
    /// Returns an authority failure without mutating the shared mailbox.
    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error>;
    /// Advance the same authoritative clock and rules at monotonic `now`.
    /// # Errors
    /// Returns the task's authoritative failure.
    fn advance(&mut self, now: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error>;
    /// Stop new time admission at the last observed sample, retaining accepted
    /// ticks for ordinary bounded advances. Repeated pause is idempotent.
    fn pause(&mut self);
    /// Resume a drained task at worker-observed time. Already running is an
    /// idempotent success without changing its origin or phase.
    /// # Errors
    /// Refuses an invalid transition without changing the task's clock.
    fn resume(&mut self, now: Duration) -> Result<(), Self::Error>;
}

/// Explicit authority policy, independent of surface availability or occlusion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldLifecycle {
    /// Stop admission and drain already accepted ticks before acknowledgement.
    Pause,
    /// Restart from the worker's observed time, excluding the paused interval.
    Resume,
}

/// Immutable summary from the exact update acknowledging one lifecycle request.
/// Later world progress cannot change this reply's identity or clock sample.
#[derive(Clone, Copy, Debug)]
pub struct WorldLifecycleReply {
    /// The completed request.
    pub request: WorldLifecycle,
    /// Committed authority identity after the operation.
    pub id: SnapshotId,
    /// Replay timeline at acknowledgement.
    pub timeline: u64,
    /// Read-only presentation phase at acknowledgement.
    pub clock: ClockSample,
}

enum LifecycleSlot {
    Empty,
    Queued(WorldLifecycle),
    Processing(WorldLifecycle),
    Ready(Result<WorldLifecycleReply, WorldLaneFault>),
}

/// The caller must retry input, stop, or rebuild; a render fault never owns the world.
#[derive(Clone, Debug)]
pub enum WorldLaneFault {
    /// Native thread could not start.
    Start(Arc<str>),
    /// The authoritative task failed.
    Task(Arc<str>),
    /// A control was rejected before entering the mailbox.
    InvalidControl(Arc<str>),
    /// A lifecycle request or its unconsumed reply still occupies the slot.
    Busy,
    /// A lifecycle operation was refused; authority remains available.
    Lifecycle(Arc<str>),
    /// The fixed clock refused elapsed work.
    Overload(u64),
    /// The world owner stopped.
    Stopped,
    /// Synchronization was poisoned by a bug.
    Poisoned,
}

impl fmt::Display for WorldLaneFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(error) => write!(formatter, "world lane start: {error}"),
            Self::Task(error) => write!(formatter, "world owner: {error}"),
            Self::InvalidControl(error) => write!(formatter, "world control: {error}"),
            Self::Busy => formatter.write_str("world lifecycle request or reply is pending"),
            Self::Lifecycle(error) => write!(formatter, "world lifecycle: {error}"),
            Self::Overload(refused) => {
                write!(formatter, "fixed-clock overload: {refused} ticks refused")
            }
            Self::Stopped => formatter.write_str("world owner has stopped"),
            Self::Poisoned => formatter.write_str("world lane synchronization poisoned"),
        }
    }
}

impl std::error::Error for WorldLaneFault {}

#[derive(Clone, Copy)]
struct Snapshot {
    epoch: WorldEpoch,
    tick: u64,
    timeline: u64,
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

struct SharedState<P, C> {
    snapshot: Snapshot,
    page: Option<WorldPublication<P>>,
    control: C,
    control_sequence: u64,
    lifecycle: LifecycleSlot,
    shutdown: bool,
    fault: Option<WorldLaneFault>,
}

struct Shared<P, C> {
    state: Mutex<SharedState<P, C>>,
    wake: Condvar,
}

struct ExitNotify<'a, P, C, W: Fn()> {
    shared: &'a Arc<Shared<P, C>>,
    wake: &'a W,
    wake_in_progress: Cell<bool>,
}

impl<P, C, W: Fn()> ExitNotify<'_, P, C, W> {
    fn notify(&self) {
        self.wake_in_progress.set(true);
        (self.wake)();
        self.wake_in_progress.set(false);
    }
}

impl<P, C, W: Fn()> Drop for ExitNotify<'_, P, C, W> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock()
            && !state.shutdown
            && state.fault.is_none()
        {
            state.fault = Some(WorldLaneFault::Stopped);
        }
        if !self.wake_in_progress.get() {
            // A failing observer cannot abort the process during owner unwinding.
            drop(catch_unwind(AssertUnwindSafe(|| (self.wake)())));
        }
    }
}

/// Cloneable validated continuous-control handle; it never owns the world.
pub struct WorldLaneControls<T: WorldTask> {
    shared: Arc<Shared<T::Page, T::Control>>,
}

impl<T: WorldTask> Clone for WorldLaneControls<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: WorldTask> WorldLaneControls<T> {
    /// Queue one explicit lifecycle operation without waiting for execution.
    /// Exactly one designated lifecycle consumer must request and consume replies,
    /// even when handles are cloned for continuous controls. Accepted operations
    /// are never coalesced. The slot stays busy until its reply is consumed.
    /// Pause cuts off admission at the task's last observed sample; it does not
    /// admit the unsampled interval before the worker processes the request.
    /// # Errors
    /// Returns `Busy` for an occupied slot or a retained owner fault.
    pub fn request_lifecycle(&self, request: WorldLifecycle) -> Result<(), WorldLaneFault> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorldLaneFault::Poisoned)?;
        if let Some(error) = &state.fault {
            return Err(error.clone());
        }
        if state.shutdown {
            return Err(WorldLaneFault::Stopped);
        }
        if !matches!(state.lifecycle, LifecycleSlot::Empty) {
            return Err(WorldLaneFault::Busy);
        }
        state.lifecycle = LifecycleSlot::Queued(request);
        drop(state);
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Consume the one ready lifecycle reply without waiting for execution.
    /// `None` means no reply is ready. The designated lifecycle consumer must
    /// also consume a refused-operation error before submitting another request.
    /// # Errors
    /// Returns a transition refusal or retained owner fault; a dead owner never
    /// leaves its pending operation permanently busy.
    pub fn poll_lifecycle_reply(&self) -> Result<Option<WorldLifecycleReply>, WorldLaneFault> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorldLaneFault::Poisoned)?;
        if let Some(error) = &state.fault {
            return Err(error.clone());
        }
        if state.shutdown {
            return Err(WorldLaneFault::Stopped);
        }
        if !matches!(state.lifecycle, LifecycleSlot::Ready(_)) {
            return Ok(None);
        }
        let LifecycleSlot::Ready(reply) =
            std::mem::replace(&mut state.lifecycle, LifecycleSlot::Empty)
        else {
            unreachable!("ready lifecycle slot checked under its lock");
        };
        reply.map(Some)
    }

    /// Replace the sampled continuous state without waiting for a render job.
    /// # Errors
    /// Refuses invalid input or a stopped owner.
    pub fn set(&self, control: T::Control) -> Result<(), WorldLaneFault> {
        T::validate_control(control)
            .map_err(|error| WorldLaneFault::InvalidControl(Arc::from(error.to_string())))?;
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorldLaneFault::Poisoned)?;
        if state.shutdown {
            return Err(WorldLaneFault::Stopped);
        }
        if let Some(error) = &state.fault {
            return Err(error.clone());
        }
        if T::same_control(state.control, control) {
            return Ok(());
        }
        state.control = control;
        state.control_sequence = state.control_sequence.wrapping_add(1);
        drop(state);
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Observe an authority fault even when no drawable exists.
    /// # Errors
    /// Returns the retained task, overload or synchronization fault.
    pub fn check_health(&self) -> Result<u64, WorldLaneFault> {
        let state = self
            .shared
            .state
            .lock()
            .map_err(|_| WorldLaneFault::Poisoned)?;
        if let Some(error) = &state.fault {
            return Err(error.clone());
        }
        if let Some(refused) = state.snapshot.overload {
            return Err(WorldLaneFault::Overload(refused));
        }
        if state.shutdown {
            return Err(WorldLaneFault::Stopped);
        }
        Ok(state.snapshot.tick)
    }
}

/// One non-cloneable render observation of the autonomous world owner.
pub struct WorldLane<T: WorldTask> {
    shared: Arc<Shared<T::Page, T::Control>>,
    controls: WorldLaneControls<T>,
    join: Option<JoinHandle<()>>,
    snapshot: Snapshot,
    page: Option<WorldPublication<T::Page>>,
    last_completed: u64,
    last_tick_cpu: Duration,
    last_skipped: u64,
    last_projected: usize,
    last_page_sequence: u64,
    sample_now: Duration,
}

impl<T: WorldTask> WorldLane<T> {
    /// Consume one task and start its autonomous deadline-driven world owner.
    /// # Errors
    /// Returns a setup, initial-publication or OS thread failure.
    pub fn start(
        mut task: T,
        initial_control: T::Control,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Self, WorldLaneControls<T>), WorldLaneFault> {
        T::validate_control(initial_control)
            .map_err(|error| WorldLaneFault::InvalidControl(Arc::from(error.to_string())))?;
        task.apply_control(initial_control)
            .map_err(|error| WorldLaneFault::Task(Arc::from(error.to_string())))?;
        let initial = task
            .initial()
            .map_err(|error| WorldLaneFault::Task(Arc::from(error.to_string())))?;
        let page = initial
            .publication
            .ok_or_else(|| WorldLaneFault::Task(Arc::from("initial publication unavailable")))?;
        let snapshot = Snapshot {
            epoch: initial.epoch,
            tick: initial.tick,
            timeline: initial.timeline,
            clock: initial.clock,
            completed_total: 0,
            tick_cpu_total: Duration::ZERO,
            skipped_total: 0,
            projected_total: 0,
            projection_lost_ticks: initial.projection_lost_ticks,
            queued_ticks: initial.queued_ticks,
            overload: initial.overload,
            page_sequence: 1,
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(SharedState {
                snapshot,
                page: Some(WorldPublication {
                    id: page.id,
                    timeline: page.timeline,
                    page: page.page.clone(),
                }),
                control: initial_control,
                control_sequence: 0,
                lifecycle: LifecycleSlot::Empty,
                shutdown: false,
                fault: None,
            }),
            wake: Condvar::new(),
        });
        let controls = WorldLaneControls {
            shared: Arc::clone(&shared),
        };
        let worker_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("thrum-world".into())
            .spawn(move || {
                let notify = ExitNotify {
                    shared: &worker_shared,
                    wake: &wake,
                    wake_in_progress: Cell::new(false),
                };
                if let Err(error) = run_world(&mut task, origin, &worker_shared, &notify)
                    && let Ok(mut state) = worker_shared.state.lock()
                {
                    state.fault = Some(error);
                }
            })
            .map_err(|error| WorldLaneFault::Start(Arc::from(error.to_string())))?;
        let lane = Self {
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
        Ok((lane, controls))
    }

    /// Poll metadata and the matching latest page without waiting for the owner.
    /// # Errors
    /// Returns a retained authority or synchronization fault.
    pub fn poll(&mut self, now: Duration) -> Result<WorldAdvance, WorldLaneFault> {
        self.sample_now = now;
        match self.shared.state.try_lock() {
            Ok(state) => {
                if let Some(error) = &state.fault {
                    return Err(error.clone());
                }
                if state.snapshot.page_sequence != self.snapshot.page_sequence {
                    self.page = state.page.as_ref().map(|page| WorldPublication {
                        id: page.id,
                        timeline: page.timeline,
                        page: page.page.clone(),
                    });
                }
                self.snapshot = state.snapshot;
            }
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Poisoned(_)) => return Err(WorldLaneFault::Poisoned),
        }
        if self.join.as_ref().is_some_and(JoinHandle::is_finished) {
            return Err(WorldLaneFault::Stopped);
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

    /// Latest authority tick.
    pub fn tick(&self) -> u64 {
        self.snapshot.tick
    }
    /// Latest authority epoch.
    pub fn epoch(&self) -> WorldEpoch {
        self.snapshot.epoch
    }
    /// Latest replay branch.
    pub fn timeline(&self) -> u64 {
        self.snapshot.timeline
    }
    /// Whether the retained page matches the latest authority identity.
    pub fn has_current_page(&self) -> bool {
        self.page.as_ref().is_some_and(|page| {
            page.id
                == SnapshotId {
                    epoch: self.snapshot.epoch,
                    tick: self.snapshot.tick,
                }
                && page.timeline == self.snapshot.timeline
        })
    }
    /// Cheap immutable lease of the current page, absent across a publication gap.
    pub fn current_page(&self) -> Option<T::Page> {
        self.has_current_page()
            .then(|| self.page.as_ref().map(|page| page.page.clone()))
            .flatten()
    }
    /// Read-only interpolation fraction evaluated at the current display target.
    pub fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.snapshot
            .clock
            .alpha_at(self.sample_now.saturating_add(lead))
    }
    /// Replace validated continuous controls.
    /// # Errors
    /// Refuses invalid input or a stopped owner.
    pub fn set_controls(&self, control: T::Control) -> Result<(), WorldLaneFault> {
        self.controls.set(control)
    }
}

impl<T: WorldTask> Drop for WorldLane<T> {
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

fn run_world<T: WorldTask>(
    task: &mut T,
    origin: Instant,
    shared: &Arc<Shared<T::Page, T::Control>>,
    notify: &ExitNotify<'_, T::Page, T::Control, impl Fn()>,
) -> Result<(), WorldLaneFault> {
    let mut applied_controls = 0;
    loop {
        let (control, sequence, request) = {
            let mut state = shared.state.lock().map_err(|_| WorldLaneFault::Poisoned)?;
            if state.shutdown {
                return Ok(());
            }
            let request = if let LifecycleSlot::Queued(request) = state.lifecycle {
                state.lifecycle = LifecycleSlot::Processing(request);
                Some(request)
            } else {
                None
            };
            (state.control, state.control_sequence, request)
        };
        if sequence != applied_controls {
            task.apply_control(control)
                .map_err(|error| WorldLaneFault::Task(Arc::from(error.to_string())))?;
            applied_controls = sequence;
        }
        let sampled_now = origin.elapsed();
        let refusal = match request {
            Some(WorldLifecycle::Pause) => {
                task.pause();
                None
            }
            Some(WorldLifecycle::Resume) => task
                .resume(sampled_now)
                .err()
                .map(|error| WorldLaneFault::Lifecycle(Arc::from(error.to_string()))),
            None => None,
        };
        let update = task
            .advance(sampled_now)
            .map_err(|error| WorldLaneFault::Task(Arc::from(error.to_string())))?;
        let deadline =
            sampled_now.saturating_add(update.next_due.unwrap_or(Duration::from_millis(100)));
        let mut state = shared.state.lock().map_err(|_| WorldLaneFault::Poisoned)?;
        if state.shutdown {
            return Ok(());
        }
        let status = &mut state.snapshot;
        status.epoch = update.epoch;
        status.tick = update.tick;
        status.timeline = update.timeline;
        status.clock = update.clock;
        status.completed_total = status
            .completed_total
            .saturating_add(u64::from(update.completed_ticks));
        status.tick_cpu_total = status.tick_cpu_total.saturating_add(update.tick_cpu);
        status.skipped_total = status
            .skipped_total
            .saturating_add(update.skipped_publications);
        status.projected_total = status
            .projected_total
            .saturating_add(update.projected_changes);
        status.projection_lost_ticks = update.projection_lost_ticks;
        status.queued_ticks = update.queued_ticks;
        status.overload = match (status.overload, update.overload) {
            (Some(first), Some(0)) => Some(first),
            (_, current) => current,
        };
        let published = update.publication.is_some();
        if let Some(page) = update.publication {
            state.page = Some(page);
            state.snapshot.page_sequence = state.snapshot.page_sequence.wrapping_add(1);
        }
        let acknowledged = if let LifecycleSlot::Processing(request) = state.lifecycle {
            if let Some(error) = refusal {
                state.lifecycle = LifecycleSlot::Ready(Err(error));
                true
            } else if request == WorldLifecycle::Resume || update.queued_ticks == 0 {
                state.lifecycle = LifecycleSlot::Ready(Ok(WorldLifecycleReply {
                    request,
                    id: SnapshotId {
                        epoch: update.epoch,
                        tick: update.tick,
                    },
                    timeline: update.timeline,
                    clock: update.clock,
                }));
                true
            } else {
                false
            }
        } else {
            false
        };
        let completed = update.completed_ticks;
        let overloaded = update.overload.is_some();
        drop(state);
        if completed != 0 || overloaded || published || acknowledged {
            notify.notify();
        }
        let delay = deadline.saturating_sub(origin.elapsed());
        let state = shared.state.lock().map_err(|_| WorldLaneFault::Poisoned)?;
        drop(
            shared
                .wake
                .wait_timeout_while(state, delay, |state| {
                    !state.shutdown
                        && state.control_sequence == applied_controls
                        && !matches!(state.lifecycle, LifecycleSlot::Queued(_))
                })
                .map_err(|_| WorldLaneFault::Poisoned)?,
        );
    }
}

#[cfg(test)]
mod tests;
