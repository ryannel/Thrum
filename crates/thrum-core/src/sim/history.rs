//! Bounded rollback scheduling over opaque game resources and replay records.

use std::{collections::VecDeque, mem::size_of, num::NonZeroUsize};

use super::{
    GameRuntime, PublicationMode, RuntimeEndpoint, RuntimeError, RuntimeUndo, TickError, TickGame,
    WorldRead,
};

/// Game policy paired with the engine's bounded rollback schedule.
pub trait HistoryGame<P>: TickGame<P> + Sized {
    /// Complete game-owned authority resources needed to restore a tick.
    type HistoryState: Clone;
    /// Inputs and commands needed to re-admit and replay one tick.
    type HistoryRecord;
    /// A caller's request for one new tick, such as a command slice.
    type TickRequest: ?Sized;

    /// Revision of resource configuration that invalidates existing history.
    fn history_revision(&self) -> u64;
    /// Current resources, including any inputs admitted since the last commit.
    fn history_state(&self) -> &Self::HistoryState;
    /// Restore resources without changing transient tick outputs.
    fn restore_history_state(&mut self, state: &Self::HistoryState);
    /// Allocate a reusable replay record with game-selected capacities.
    fn new_history_record(&self) -> Self::HistoryRecord;
    /// Replace a reusable record with the current admitted inputs and request.
    fn record_history_tick(&self, request: &Self::TickRequest, record: &mut Self::HistoryRecord);
    /// Validate commands and inputs, re-admit absent inputs, and stage the tick.
    /// A fresh step's record comes from the current game and must leave
    /// `history_state` unchanged before commit. Replay may re-admit retained
    /// input; on refusal the engine restores `history_state`. Other authority
    /// state must remain unchanged. An identical pending input is accepted once.
    ///
    /// # Errors
    ///
    /// The record is invalid for the current game authority.
    fn prepare_history_tick(
        &mut self,
        runtime: &GameRuntime<P, Self>,
        tick: u64,
        record: &Self::HistoryRecord,
    ) -> Result<(), Self::Error>;
}

/// A rejected history operation leaves authority and retained ticks unchanged.
#[derive(Debug, Eq, PartialEq)]
pub enum HistoryError<E> {
    /// The runtime endpoint or game resource configuration changed externally.
    Diverged,
    /// The requested number of ticks exceeds the retained undo window.
    OutsideWindow,
    /// Runtime rewind validation refused the operation.
    Runtime(RuntimeError),
    /// Game preparation or the ordinary tick refused the operation.
    Tick(TickError<E>),
}

struct ReplayTick<R> {
    tick: u64,
    record: R,
}

/// Caller-sized sparse undo and replay retention. Game state and records stay
/// opaque; the runtime remains the sole owner of world and branch identity.
pub struct RuntimeHistory<S, R> {
    endpoint: RuntimeEndpoint,
    resource_revision: u64,
    oldest_tick: u64,
    undos: VecDeque<RuntimeUndo>,
    states: VecDeque<S>,
    records: VecDeque<R>,
    spare_undos: Vec<RuntimeUndo>,
    spare_states: Vec<S>,
    spare_records: Vec<R>,
    future: VecDeque<ReplayTick<R>>,
    committed_state: S,
    attempt_state: S,
    window: NonZeroUsize,
}

impl<S: Clone, R> RuntimeHistory<S, R> {
    /// Start at an already published committed endpoint. The game must reject
    /// creation while uncommitted input is pending before calling this method.
    pub fn new<P, G>(runtime: &GameRuntime<P, G>, game: &G, window: NonZeroUsize) -> Self
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R>,
    {
        Self {
            endpoint: runtime.endpoint(),
            resource_revision: game.history_revision(),
            oldest_tick: runtime.tick(),
            undos: VecDeque::with_capacity(window.get()),
            states: VecDeque::with_capacity(window.get()),
            records: VecDeque::with_capacity(window.get()),
            spare_undos: Vec::with_capacity(window.get()),
            spare_states: Vec::with_capacity(window.get()),
            spare_records: Vec::with_capacity(window.get()),
            future: VecDeque::with_capacity(window.get()),
            committed_state: game.history_state().clone(),
            attempt_state: game.history_state().clone(),
            window,
        }
    }

    /// Latest committed tick in this branch.
    pub fn latest_tick(&self) -> u64 {
        self.undos
            .back()
            .map_or(self.oldest_tick, RuntimeUndo::tick)
    }

    /// Oldest endpoint that can still be restored.
    pub fn oldest_tick(&self) -> u64 {
        self.oldest_tick
    }

    /// Approximate reserved storage, including recycled records. Callbacks
    /// return each value's nested allocation bytes, excluding its inline size.
    pub fn reserved_bytes(
        &self,
        state_bytes: impl Fn(&S) -> usize,
        record_bytes: impl Fn(&R) -> usize,
    ) -> usize {
        size_of::<Self>()
            + self.undos.capacity() * size_of::<RuntimeUndo>()
            + self
                .undos
                .iter()
                .map(RuntimeUndo::reserved_bytes)
                .sum::<usize>()
            + self.spare_undos.capacity() * size_of::<RuntimeUndo>()
            + self
                .spare_undos
                .iter()
                .map(RuntimeUndo::reserved_bytes)
                .sum::<usize>()
            + self.states.capacity() * size_of::<S>()
            + self.states.iter().map(&state_bytes).sum::<usize>()
            + self.spare_states.capacity() * size_of::<S>()
            + self.spare_states.iter().map(&state_bytes).sum::<usize>()
            + state_bytes(&self.committed_state)
            + state_bytes(&self.attempt_state)
            + self.records.capacity() * size_of::<R>()
            + self.records.iter().map(&record_bytes).sum::<usize>()
            + self.spare_records.capacity() * size_of::<R>()
            + self.spare_records.iter().map(&record_bytes).sum::<usize>()
            + self.future.capacity() * size_of::<ReplayTick<R>>()
            + self
                .future
                .iter()
                .map(|entry| record_bytes(&entry.record))
                .sum::<usize>()
    }

    fn check<P, G>(
        &self,
        runtime: &GameRuntime<P, G>,
        game: &G,
    ) -> Result<(), HistoryError<G::Error>>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R>,
    {
        if runtime.tick() != self.latest_tick()
            || runtime.endpoint() != self.endpoint
            || game.history_revision() != self.resource_revision
        {
            return Err(HistoryError::Diverged);
        }
        Ok(())
    }

    /// Record and commit one new tick; only success abandons retained future.
    ///
    /// # Errors
    ///
    /// The branch diverged, or game preparation or the tick refused.
    pub fn step<P, G>(
        &mut self,
        runtime: &mut GameRuntime<P, G>,
        game: &mut G,
        candidate: &mut G::Candidate,
        request: &G::TickRequest,
    ) -> Result<u64, HistoryError<G::Error>>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R> + Sync,
    {
        self.check(runtime, game)?;
        let mut record = self
            .spare_records
            .pop()
            .unwrap_or_else(|| game.new_history_record());
        game.record_history_tick(request, &mut record);
        match self.commit(
            runtime,
            game,
            candidate,
            record,
            PublicationMode::Capture,
            false,
        ) {
            Ok(tick) => {
                self.discard_future();
                Ok(tick)
            }
            Err((error, record)) => {
                self.spare_records.push(record);
                Err(error)
            }
        }
    }

    fn commit<P, G>(
        &mut self,
        runtime: &mut GameRuntime<P, G>,
        game: &mut G,
        candidate: &mut G::Candidate,
        record: R,
        publication: PublicationMode,
        replaying: bool,
    ) -> Result<u64, (HistoryError<G::Error>, R)>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R> + Sync,
    {
        let Some(next) = runtime.tick().checked_add(1) else {
            return Err((HistoryError::Tick(TickError::TickExhausted), record));
        };
        if replaying {
            self.attempt_state.clone_from(game.history_state());
        }
        if let Err(error) = game.prepare_history_tick(runtime, next, &record) {
            if replaying {
                game.restore_history_state(&self.attempt_state);
            }
            return Err((HistoryError::Tick(TickError::Game(error)), record));
        }
        let mut undo = self.spare_undos.pop().unwrap_or_else(|| runtime.new_undo());
        let tick = match runtime.step_recording(game, candidate, &mut undo, publication) {
            Ok(tick) => tick,
            Err(error) => {
                if replaying {
                    game.restore_history_state(&self.attempt_state);
                }
                self.spare_undos.push(undo);
                return Err((HistoryError::Tick(error), record));
            }
        };
        let mut state = self
            .spare_states
            .pop()
            .unwrap_or_else(|| self.committed_state.clone());
        state.clone_from(&self.committed_state);
        self.committed_state.clone_from(game.history_state());
        if self.undos.len() == self.window.get() {
            self.spare_undos
                .push(self.undos.pop_front().expect("full undo window"));
            self.spare_states
                .push(self.states.pop_front().expect("paired state window"));
            self.spare_records
                .push(self.records.pop_front().expect("paired record window"));
            self.oldest_tick += 1;
        }
        self.undos.push_back(undo);
        self.states.push_back(state);
        self.records.push_back(record);
        self.endpoint = runtime.endpoint();
        Ok(tick)
    }

    fn discard_future(&mut self) {
        while let Some(entry) = self.future.pop_front() {
            self.spare_records.push(entry.record);
        }
    }

    /// Replace validated data in one retained future record without moving its
    /// tick or consuming it. `validate` sees the old record first; `apply` runs
    /// only after validation succeeds and must be infallible.
    ///
    /// # Errors
    ///
    /// The branch diverged, the tick is not queued for replay, or validation
    /// rejected the proposed replacement. Every error leaves history unchanged.
    pub fn amend_future<P, G, V>(
        &mut self,
        runtime: &GameRuntime<P, G>,
        game: &G,
        tick: u64,
        validate: impl FnOnce(&R) -> Result<V, G::Error>,
        apply: impl FnOnce(&mut R, V),
    ) -> Result<(), HistoryError<G::Error>>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R>,
    {
        self.check(runtime, game)?;
        let entry = self
            .future
            .iter_mut()
            .find(|entry| entry.tick == tick)
            .ok_or(HistoryError::OutsideWindow)?;
        let checked =
            validate(&entry.record).map_err(|error| HistoryError::Tick(TickError::Game(error)))?;
        apply(&mut entry.record, checked);
        Ok(())
    }

    /// Re-admit and commit the next retained tick, consuming it only on success.
    ///
    /// # Errors
    ///
    /// The branch diverged, retained input is invalid, or the tick refused.
    pub fn replay_next<P, G>(
        &mut self,
        runtime: &mut GameRuntime<P, G>,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> Result<bool, HistoryError<G::Error>>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R> + Sync,
    {
        self.check(runtime, game)?;
        let Some(entry) = self.future.pop_front() else {
            return Ok(false);
        };
        if runtime.tick().checked_add(1) != Some(entry.tick) {
            self.future.push_front(entry);
            return Err(HistoryError::Diverged);
        }
        let publication = if self.future.is_empty() {
            PublicationMode::Capture
        } else {
            PublicationMode::Defer
        };
        match self.commit(runtime, game, candidate, entry.record, publication, true) {
            Ok(_) => Ok(true),
            Err((error, record)) => {
                self.future.push_front(ReplayTick {
                    tick: entry.tick,
                    record,
                });
                Err(error)
            }
        }
    }

    /// Reverse retained ticks and queue their records for replay. Source-page
    /// selection receives old timeline, target and latest ticks. The patch
    /// receives source tick, target tick, new timeline and reversed records.
    /// Page pressure defers publication without refusing authority rewind.
    ///
    /// # Errors
    ///
    /// The branch diverged, the window is too short, or runtime validation fails.
    ///
    /// # Panics
    ///
    /// Only if internally paired queues lose a retained record.
    pub fn rewind<P, G>(
        &mut self,
        runtime: &mut GameRuntime<P, G>,
        game: &mut G,
        count: usize,
        source_tick: impl Fn(&G, &P, u64, u64, u64) -> Option<u64>,
        patch: impl FnOnce(&G, &WorldRead<'_>, &mut P, u64, u64, u64, &[RuntimeUndo]),
    ) -> Result<(), HistoryError<G::Error>>
    where
        G: HistoryGame<P, HistoryState = S, HistoryRecord = R>,
    {
        self.check(runtime, game)?;
        if count > self.undos.len() {
            return Err(HistoryError::OutsideWindow);
        }
        if count == 0 {
            return Ok(());
        }
        let latest = self.latest_tick();
        let start = self.undos.len() - count;
        let target = latest - u64::try_from(count).expect("retained tick count fits u64");
        let old_timeline = runtime.timeline();
        let contiguous = self.undos.make_contiguous();
        runtime
            .rewind_many(&mut contiguous[start..])
            .map_err(HistoryError::Runtime)?;
        game.restore_history_state(&self.states[start]);
        self.committed_state.clone_from(&self.states[start]);
        self.endpoint = runtime.endpoint();
        runtime.patch_rewound_page(
            &contiguous[start..],
            |page| source_tick(game, page, old_timeline, target, latest),
            |world, page, source, target, timeline, steps| {
                patch(game, world, page, source, target, timeline, steps);
            },
        );
        for _ in 0..count {
            let undo = self.undos.pop_back().expect("retained undo");
            self.future.push_front(ReplayTick {
                tick: undo.tick(),
                record: self.records.pop_back().expect("paired replay record"),
            });
            self.spare_undos.push(undo);
            self.spare_states
                .push(self.states.pop_back().expect("paired history state"));
        }
        Ok(())
    }
}
