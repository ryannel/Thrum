//! Game policy for the engine-owned sparse rollback window.

use std::{mem::size_of, num::NonZeroUsize};

use thrum_core::sim::{
    GameRuntime, HistoryError, HistoryGame, InputFrame, RuntimeHistory, TickError,
};

use super::{
    Command, PeerControl, Rules, SimAction, SimError, Simulation, map_runtime, page::Page,
    peer_storage_bytes, validate_commands, validate_frame,
};

pub(super) struct HistoryRecord {
    inputs: Vec<InputFrame<SimAction>>,
    commands: Vec<Command>,
}

fn input_storage_bytes(frames: &Vec<InputFrame<SimAction>>) -> usize {
    frames.capacity() * size_of::<InputFrame<SimAction>>()
        + frames
            .iter()
            .filter(|frame| frame.actions.spilled())
            .map(|frame| frame.actions.capacity() * size_of::<SimAction>())
            .sum::<usize>()
}

fn record_storage_bytes(record: &HistoryRecord) -> usize {
    input_storage_bytes(&record.inputs) + record.commands.capacity() * size_of::<Command>()
}

impl HistoryGame<Page> for Rules {
    type HistoryState = Vec<PeerControl>;
    type HistoryRecord = HistoryRecord;
    type TickRequest = [Command];

    fn history_revision(&self) -> u64 {
        self.resource_revision
    }

    fn history_state(&self) -> &Self::HistoryState {
        &self.peers
    }

    fn restore_history_state(&mut self, state: &Self::HistoryState) {
        self.peers.clone_from(state);
    }

    fn new_history_record(&self) -> Self::HistoryRecord {
        HistoryRecord {
            inputs: Vec::with_capacity(self.config.peer_capacity),
            commands: Vec::with_capacity(128),
        }
    }

    fn record_history_tick(&self, request: &Self::TickRequest, record: &mut Self::HistoryRecord) {
        record.inputs.clear();
        record
            .inputs
            .extend(self.peers.iter().filter_map(|peer| peer.pending.clone()));
        record.commands.clear();
        record.commands.extend_from_slice(request);
    }

    fn prepare_history_tick(
        &mut self,
        runtime: &GameRuntime<Page, Self>,
        tick: u64,
        record: &Self::HistoryRecord,
    ) -> Result<(), Self::Error> {
        validate_commands(runtime, &record.commands)?;
        // Validate every input before replay admission changes any peer state.
        for frame in &record.inputs {
            if frame.tick != tick {
                return Err(SimError::InvalidSnapshot);
            }
            let already_pending = self
                .peers
                .iter()
                .any(|peer| peer.pending.as_ref() == Some(frame));
            if !already_pending && validate_frame(runtime, self, frame).is_err() {
                return Err(SimError::InvalidSnapshot);
            }
        }
        for frame in &record.inputs {
            let Some(peer) = self
                .peers
                .iter_mut()
                .find(|peer| peer.binding.peer == frame.origin.peer)
            else {
                return Err(SimError::InvalidSnapshot);
            };
            if peer.pending.as_ref() != Some(frame) {
                peer.last_sequence = Some(frame.origin.sequence);
                peer.pending = Some(frame.clone());
            }
        }
        self.commands.clear();
        self.commands.extend_from_slice(&record.commands);
        Ok(())
    }
}

fn map_history(error: HistoryError<SimError>) -> SimError {
    match error {
        HistoryError::Diverged | HistoryError::OutsideWindow => SimError::InvalidSnapshot,
        HistoryError::Runtime(error) => map_runtime(error),
        HistoryError::Tick(error) => match error {
            TickError::TickExhausted | TickError::RevisionExhausted => SimError::TickExhausted,
            TickError::BaselineRequired => SimError::PublicationBusy,
            TickError::Game(error) => error,
            TickError::Edit(_) => SimError::InvalidEdit,
        },
    }
}

/// The game's small adapter over a core-owned undo, rewind and replay window.
pub struct SimulationHistory {
    inner: RuntimeHistory<Vec<PeerControl>, HistoryRecord>,
}

impl SimulationHistory {
    /// Newest committed tick retained by this branch.
    pub fn latest_tick(&self) -> u64 {
        self.inner.latest_tick()
    }

    /// Oldest endpoint retained by this branch.
    pub fn oldest_tick(&self) -> u64 {
        self.inner.oldest_tick()
    }

    /// Approximate allocated bytes retained by engine and game history.
    pub fn reserved_bytes(&self) -> usize {
        self.inner
            .reserved_bytes(peer_storage_bytes, record_storage_bytes)
    }

    /// Amend one peer's retained input after rewind, preserving its origin,
    /// actor, sequence and tick. A rejected replacement leaves replay intact.
    ///
    /// # Errors
    ///
    /// The branch diverged, the tick or peer was not retained, or the new
    /// controls fail V1 admission.
    pub fn amend_input(
        &mut self,
        sim: &Simulation,
        frame: InputFrame<SimAction>,
    ) -> Result<(), SimError> {
        frame.validate_axes().map_err(|_| SimError::InvalidInput)?;
        super::validate_frame_actions(&sim.rules.config, &frame)
            .map_err(|_| SimError::InvalidInput)?;
        let origin = frame.origin;
        let actor = frame.actor;
        let tick = frame.tick;
        self.inner
            .amend_future(
                &sim.runtime,
                &sim.rules,
                tick,
                |record| {
                    record
                        .inputs
                        .iter()
                        .position(|old| old.origin.peer == origin.peer)
                        .filter(|&index| {
                            let old = &record.inputs[index];
                            old.origin == origin && old.actor == actor && old.tick == tick
                        })
                        .ok_or(SimError::InvalidInput)
                },
                |record, index| record.inputs[index] = frame,
            )
            .map_err(map_history)
    }

    /// Record and commit one complete game tick.
    ///
    /// # Errors
    /// Returns branch divergence, game validation or engine tick refusal.
    pub fn step(&mut self, sim: &mut Simulation, commands: &[Command]) -> Result<u64, SimError> {
        self.inner
            .step(
                &mut sim.runtime,
                &mut sim.rules,
                &mut sim.candidate,
                commands,
            )
            .map_err(map_history)
    }

    /// Re-admit and commit the next retained speculative tick, if any.
    ///
    /// # Errors
    /// Returns branch divergence, rejected replay input or engine tick refusal.
    pub fn replay_next(&mut self, sim: &mut Simulation) -> Result<bool, SimError> {
        self.inner
            .replay_next(&mut sim.runtime, &mut sim.rules, &mut sim.candidate)
            .map_err(map_history)
    }

    /// Reverse `count` committed ticks. Held pages remain immutable, and an
    /// unavailable page defers only publication of the restored endpoint.
    ///
    /// # Errors
    /// Returns divergence or a request outside the retained window.
    pub fn rewind(&mut self, sim: &mut Simulation, count: usize) -> Result<(), SimError> {
        self.inner
            .rewind(
                &mut sim.runtime,
                &mut sim.rules,
                count,
                |_, page, old_timeline, target, latest| {
                    page.source_tick(old_timeline, target, latest)
                },
                |rules, world, page, source, target, timeline, steps| {
                    page.patch_rewind(
                        world,
                        &rules.position_read,
                        &rules.velocity_read,
                        &rules.health_read,
                        &rules.ownership_read,
                        &rules.visual_read,
                        steps,
                        source,
                        target,
                        timeline,
                    );
                },
            )
            .map_err(map_history)?;
        if count != 0 {
            sim.rules.commands.clear();
            sim.rules.committed_inputs.clear();
            sim.rules.changed_positions = 0;
        }
        Ok(())
    }
}

impl Simulation {
    /// Begin a core-owned sparse history at a committed, input-drained endpoint.
    ///
    /// # Errors
    /// An admitted input has not committed yet.
    pub fn begin_history(&mut self, window: NonZeroUsize) -> Result<SimulationHistory, SimError> {
        if self.rules.peers.iter().any(|peer| peer.pending.is_some()) {
            return Err(SimError::InvalidSnapshot);
        }
        self.runtime
            .publish_initial(&mut self.rules, &mut self.candidate);
        Ok(SimulationHistory {
            inner: RuntimeHistory::new(&self.runtime, &self.rules, window),
        })
    }
}
