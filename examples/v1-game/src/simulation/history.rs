//! Bounded sparse undo for the running game's prediction branch. The engine
//! owns moved component preimages; the game owns committed peer resources.

use std::{collections::VecDeque, mem::size_of, num::NonZeroUsize};

use thrum_core::sim::{InputFrame, RuntimeEndpoint, RuntimeUndo};

use super::{
    Command, PeerControl, SimAction, SimError, Simulation, map_runtime, peer_storage_bytes,
};

struct ReplayTick {
    tick: u64,
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

fn command_storage_bytes(commands: &Vec<Command>) -> usize {
    commands.capacity() * size_of::<Command>()
}

/// A caller-sized window of committed inverse ticks. Rewind consumes future
/// records; replay must commit through the same game schedule.
pub struct SimulationHistory {
    endpoint: RuntimeEndpoint,
    resource_revision: u64,
    oldest_tick: u64,
    undos: VecDeque<RuntimeUndo>,
    peers_before: VecDeque<Vec<PeerControl>>,
    inputs: VecDeque<Vec<InputFrame<SimAction>>>,
    commands: VecDeque<Vec<Command>>,
    spare_undos: Vec<RuntimeUndo>,
    spare_peers: Vec<Vec<PeerControl>>,
    spare_inputs: Vec<Vec<InputFrame<SimAction>>>,
    spare_commands: Vec<Vec<Command>>,
    future: VecDeque<ReplayTick>,
    committed_peers: Vec<PeerControl>,
    window: NonZeroUsize,
}

impl SimulationHistory {
    /// Newest committed tick retained in the history.
    pub fn latest_tick(&self) -> u64 {
        self.undos
            .back()
            .map_or(self.oldest_tick, RuntimeUndo::tick)
    }

    /// Oldest committed tick retained in the history.
    pub fn oldest_tick(&self) -> u64 {
        self.oldest_tick
    }

    /// Approximate retained sparse storage including reusable records.
    pub fn reserved_bytes(&self) -> usize {
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
            + self.peers_before.capacity() * size_of::<Vec<PeerControl>>()
            + self
                .peers_before
                .iter()
                .map(peer_storage_bytes)
                .sum::<usize>()
            + self.spare_peers.capacity() * size_of::<Vec<PeerControl>>()
            + self
                .spare_peers
                .iter()
                .map(peer_storage_bytes)
                .sum::<usize>()
            + peer_storage_bytes(&self.committed_peers)
            + self.inputs.capacity() * size_of::<Vec<InputFrame<SimAction>>>()
            + self.inputs.iter().map(input_storage_bytes).sum::<usize>()
            + self.commands.capacity() * size_of::<Vec<Command>>()
            + self
                .commands
                .iter()
                .map(command_storage_bytes)
                .sum::<usize>()
            + self.spare_inputs.capacity() * size_of::<Vec<InputFrame<SimAction>>>()
            + self
                .spare_inputs
                .iter()
                .map(input_storage_bytes)
                .sum::<usize>()
            + self.spare_commands.capacity() * size_of::<Vec<Command>>()
            + self
                .spare_commands
                .iter()
                .map(command_storage_bytes)
                .sum::<usize>()
            + self.future.capacity() * size_of::<ReplayTick>()
            + self
                .future
                .iter()
                .map(|record| {
                    input_storage_bytes(&record.inputs) + command_storage_bytes(&record.commands)
                })
                .sum::<usize>()
    }

    /// Commit one due game tick while moving its old values into sparse undo.
    /// A refused tick leaves this history and the authority endpoint unchanged.
    ///
    /// # Errors
    ///
    /// The simulation diverged from this branch or the game tick refused.
    ///
    /// # Panics
    ///
    /// Only if the internally paired bounded history loses a record.
    pub fn step(&mut self, sim: &mut Simulation, commands: &[Command]) -> Result<u64, SimError> {
        self.step_inner(sim, commands, false)
    }

    fn step_inner(
        &mut self,
        sim: &mut Simulation,
        commands: &[Command],
        replaying: bool,
    ) -> Result<u64, SimError> {
        if sim.tick() != self.latest_tick()
            || sim.runtime.endpoint() != self.endpoint
            || sim.resource_revision != self.resource_revision
        {
            return Err(SimError::InvalidSnapshot);
        }
        let mut inputs = self
            .spare_inputs
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(sim.rules.config.peer_capacity));
        inputs.clear();
        inputs.extend(
            sim.rules
                .peers
                .iter()
                .filter_map(|peer| peer.pending.clone()),
        );
        let mut saved_commands = self
            .spare_commands
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(commands.len()));
        saved_commands.clear();
        saved_commands.extend_from_slice(commands);
        let mut undo = self
            .spare_undos
            .pop()
            .unwrap_or_else(|| sim.runtime.new_undo());
        let tick = match sim.step_recording(commands, &mut undo) {
            Ok(outcome) => outcome.tick,
            Err(error) => {
                self.spare_undos.push(undo);
                self.spare_inputs.push(inputs);
                self.spare_commands.push(saved_commands);
                return Err(error);
            }
        };
        if !replaying {
            self.discard_future();
        }
        let mut peers = self
            .spare_peers
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(sim.rules.config.peer_capacity));
        peers.clone_from(&self.committed_peers);
        self.committed_peers.clone_from(&sim.rules.peers);
        if self.undos.len() == self.window.get() {
            self.spare_undos
                .push(self.undos.pop_front().expect("full undo window"));
            self.spare_peers
                .push(self.peers_before.pop_front().expect("paired peer window"));
            self.spare_inputs
                .push(self.inputs.pop_front().expect("paired input window"));
            self.spare_commands
                .push(self.commands.pop_front().expect("paired command window"));
            self.oldest_tick += 1;
        }
        self.undos.push_back(undo);
        self.peers_before.push_back(peers);
        self.inputs.push_back(inputs);
        self.commands.push_back(saved_commands);
        self.endpoint = sim.runtime.endpoint();
        Ok(tick)
    }

    fn discard_future(&mut self) {
        while let Some(record) = self.future.pop_front() {
            self.spare_inputs.push(record.inputs);
            self.spare_commands.push(record.commands);
        }
    }

    /// Re-admit and commit the next retained speculative tick once. A changed
    /// branch may reject a frame or command; the retained tick stays queued.
    ///
    /// # Errors
    ///
    /// Retained input is invalid for the new branch, or its tick refuses.
    ///
    /// # Panics
    ///
    /// Only if the internally paired history loses a record.
    pub fn replay_next(&mut self, sim: &mut Simulation) -> Result<bool, SimError> {
        let Some(record) = self.future.pop_front() else {
            return Ok(false);
        };
        if sim.tick().checked_add(1) != Some(record.tick) {
            self.future.push_front(record);
            return Err(SimError::InvalidSnapshot);
        }
        // Preflight every peer before mutating any admission state.
        if record.inputs.iter().any(|frame| {
            !sim.rules
                .peers
                .iter()
                .any(|peer| peer.pending.as_ref() == Some(frame))
                && sim.validate_frame(frame).is_err()
        }) {
            self.future.push_front(record);
            return Err(SimError::InvalidSnapshot);
        }
        for frame in &record.inputs {
            if sim
                .rules
                .peers
                .iter()
                .any(|peer| peer.pending.as_ref() == Some(frame))
            {
                continue;
            }
            sim.admit_frame(frame.clone())
                .expect("all distinct retained peer frames prevalidated");
        }
        let result = self.step_inner(sim, &record.commands, true);
        match result {
            Ok(_) => {
                self.spare_inputs.push(record.inputs);
                self.spare_commands.push(record.commands);
                Ok(true)
            }
            Err(error) => {
                self.future.push_front(record);
                Err(error)
            }
        }
    }

    /// Reverse retained ticks in the selected branch. Publication is invalid
    /// until a free page is patched or a later full baseline is captured.
    ///
    /// # Errors
    ///
    /// The target is outside this window or the runtime revision diverged.
    ///
    /// # Panics
    ///
    /// Only if the internally bounded window exceeds the process address range.
    pub fn restore_to(&mut self, sim: &mut Simulation, target: u64) -> Result<(), SimError> {
        let latest = self.latest_tick();
        if target < self.oldest_tick
            || target > latest
            || sim.tick() != latest
            || sim.runtime.endpoint() != self.endpoint
            || sim.resource_revision != self.resource_revision
        {
            return Err(SimError::InvalidSnapshot);
        }
        if target == latest {
            return Ok(());
        }
        let count = usize::try_from(latest - target).expect("window fits usize");
        let contiguous = self.undos.make_contiguous();
        let start = contiguous.len() - count;
        let old_timeline = sim.runtime.timeline();
        sim.runtime
            .rewind_many(&mut contiguous[start..])
            .map_err(map_runtime)?;
        let rules = &sim.rules;
        sim.runtime.patch_rewound_page(
            &contiguous[start..],
            |page| page.source_tick(old_timeline, target, latest),
            |world, page, source, target, timeline, steps| {
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
        );
        self.discard_future();
        let offset = usize::try_from(target - self.oldest_tick).expect("window fits usize");
        let peers = &self.peers_before[offset];
        sim.rules.peers.clone_from(peers);
        self.committed_peers.clone_from(peers);
        self.endpoint = sim.runtime.endpoint();
        sim.rules.commands.clear();
        sim.rules.committed_inputs.clear();
        sim.rules.changed_positions = 0;
        while self.undos.back().is_some_and(|undo| undo.tick() > target) {
            let tick = self.undos.back().expect("future undo").tick();
            self.future.push_front(ReplayTick {
                tick,
                inputs: self.inputs.pop_back().expect("future inputs"),
                commands: self.commands.pop_back().expect("future commands"),
            });
            self.spare_undos
                .push(self.undos.pop_back().expect("future undo"));
            self.spare_peers
                .push(self.peers_before.pop_back().expect("future peers"));
        }
        Ok(())
    }
}

impl Simulation {
    /// Start a sparse history at a committed endpoint with no pending inputs.
    ///
    /// # Errors
    ///
    /// An admitted input has not committed yet.
    pub fn begin_history(&mut self, window: NonZeroUsize) -> Result<SimulationHistory, SimError> {
        if self.rules.peers.iter().any(|peer| peer.pending.is_some()) {
            return Err(SimError::InvalidSnapshot);
        }
        self.runtime
            .publish_initial(&mut self.rules, &mut self.candidate);
        Ok(SimulationHistory {
            endpoint: self.runtime.endpoint(),
            resource_revision: self.resource_revision,
            oldest_tick: self.tick(),
            undos: VecDeque::with_capacity(window.get()),
            peers_before: VecDeque::with_capacity(window.get()),
            inputs: VecDeque::with_capacity(window.get()),
            commands: VecDeque::with_capacity(window.get()),
            spare_undos: Vec::with_capacity(window.get()),
            spare_peers: Vec::with_capacity(window.get()),
            spare_inputs: Vec::with_capacity(window.get()),
            spare_commands: Vec::with_capacity(window.get()),
            future: VecDeque::with_capacity(window.get()),
            committed_peers: self.rules.peers.clone(),
            window,
        })
    }
}
