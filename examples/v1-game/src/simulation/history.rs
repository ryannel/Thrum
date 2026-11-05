//! Bounded prediction history over one full registered baseline and reusable
//! changed-component deltas. The running game owns its small peer resources.

use std::{collections::VecDeque, mem::size_of, num::NonZeroUsize};

use thrum_core::sim::{RuntimeCapture, RuntimeDelta};

use super::{
    Page, PeerControl, SimError, Simulation, SimulationCapture, map_runtime, peer_storage_bytes,
};

struct SimulationDelta {
    runtime: RuntimeDelta,
    peers: Vec<PeerControl>,
}

impl SimulationDelta {
    fn tick(&self) -> u64 {
        self.runtime.tick()
    }

    fn reserved_bytes(&self) -> usize {
        self.runtime.reserved_bytes() + peer_storage_bytes(&self.peers)
    }
}

/// A bounded sequence of committed game endpoints for restore and replay.
/// One full baseline advances as old deltas leave the window.
pub struct SimulationHistory {
    base: SimulationCapture,
    restore_scratch: RuntimeCapture,
    deltas: VecDeque<SimulationDelta>,
    spare: Vec<SimulationDelta>,
    window: NonZeroUsize,
}

impl SimulationHistory {
    /// Newest committed tick retained in the history.
    pub fn latest_tick(&self) -> u64 {
        self.deltas
            .back()
            .map_or(self.base.runtime.tick(), SimulationDelta::tick)
    }

    /// Oldest committed tick retained in the history.
    pub fn oldest_tick(&self) -> u64 {
        self.base.runtime.tick()
    }

    /// Approximate retained owned storage, including the full baseline.
    pub fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.base.reserved_bytes()
            + self.restore_scratch.reserved_bytes()
            + self.deltas.capacity() * size_of::<SimulationDelta>()
            + self
                .deltas
                .iter()
                .map(SimulationDelta::reserved_bytes)
                .sum::<usize>()
            + self.spare.capacity() * size_of::<SimulationDelta>()
            + self
                .spare
                .iter()
                .map(SimulationDelta::reserved_bytes)
                .sum::<usize>()
    }

    /// Record the immediately following committed tick. Reuses delta storage
    /// after the window fills; refusal leaves the retained history unchanged.
    ///
    /// # Errors
    ///
    /// The simulation is foreign or its tick does not follow this history.
    pub fn record(&mut self, sim: &Simulation) -> Result<(), SimError> {
        if !sim.runtime.matches_capture(&self.base.runtime)
            || sim.tick()
                != self
                    .latest_tick()
                    .checked_add(1)
                    .ok_or(SimError::InvalidSnapshot)?
        {
            return Err(SimError::InvalidSnapshot);
        }
        let mut next = self.spare.pop().unwrap_or_else(|| SimulationDelta {
            runtime: sim.runtime.new_delta(),
            peers: Vec::with_capacity(sim.rules.config.peer_capacity),
        });
        if let Err(error) = sim.runtime.capture_delta_into(&mut next.runtime) {
            self.spare.push(next);
            return Err(map_runtime(error));
        }
        let follows = self.deltas.back().map_or_else(
            || self.base.runtime.accepts_delta(&next.runtime),
            |previous| next.runtime.follows(&previous.runtime),
        );
        if !follows {
            self.spare.push(next);
            return Err(SimError::InvalidSnapshot);
        }
        next.peers.clone_from(&sim.rules.peers);
        if self.deltas.len() == self.window.get() {
            let Some(oldest) = self.deltas.front() else {
                return Err(SimError::InvalidSnapshot);
            };
            self.base
                .runtime
                .advance(&oldest.runtime)
                .map_err(map_runtime)?;
            self.base.peers.clone_from(&oldest.peers);
            let Some(retired) = self.deltas.pop_front() else {
                return Err(SimError::InvalidSnapshot);
            };
            self.spare.push(retired);
        }
        self.deltas.push_back(next);
        Ok(())
    }

    /// Restore one retained tick and discard its superseded future deltas.
    /// The next game tick uses the same registered callbacks and input gate.
    ///
    /// # Errors
    ///
    /// The target is outside this history or a delta chain is incompatible.
    pub fn restore_to(&mut self, sim: &mut Simulation, target: u64) -> Result<(), SimError> {
        if target < self.oldest_tick() || target > self.latest_tick() {
            return Err(SimError::InvalidSnapshot);
        }
        self.restore_scratch
            .copy_from(&self.base.runtime)
            .map_err(map_runtime)?;
        let mut peers = &self.base.peers;
        for delta in &self.deltas {
            if delta.tick() > target {
                break;
            }
            self.restore_scratch
                .advance(&delta.runtime)
                .map_err(map_runtime)?;
            peers = &delta.peers;
        }
        let epoch = sim.runtime.epoch();
        let capacity = sim.rules.config.entity_capacity;
        sim.runtime
            .restore(&self.restore_scratch, || Page::new(epoch, capacity))
            .map_err(map_runtime)?;
        sim.rules.peers.clone_from(peers);
        sim.rules.commands.clear();
        sim.rules.committed_inputs.clear();
        sim.rules.changed_positions = 0;
        sim.runtime
            .publish_initial(&mut sim.rules, &mut sim.candidate);
        while self
            .deltas
            .back()
            .is_some_and(|delta| delta.tick() > target)
        {
            if let Some(delta) = self.deltas.pop_back() {
                self.spare.push(delta);
            }
        }
        Ok(())
    }
}

impl Simulation {
    /// Start an opt-in rolling history at this committed game endpoint.
    pub fn begin_history(&self, window: NonZeroUsize) -> SimulationHistory {
        let base = self.capture();
        let restore_scratch = base.runtime.clone_independent();
        SimulationHistory {
            base,
            restore_scratch,
            deltas: VecDeque::with_capacity(window.get()),
            spare: Vec::with_capacity(window.get()),
            window,
        }
    }
}
