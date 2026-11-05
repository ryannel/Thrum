//! Candidate work and V1 game rules. Every expected refusal happens before
//! the engine's infallible commit phase.

use std::any::Any;

use thrum_core::{
    Handle, Origin,
    sim::{
        Edit, EntityWriter, Executor, InputFrame, Read2, ReadPermit, RegisteredWorldError,
        StructuralPermit, TickGame, WorldRead, WorldTxn, WritePermit, WriteRead2,
    },
};

use super::{
    INPUT, INPUT_HEALTH,
    page::Page,
    types::{
        Command, Health, Ownership, Position, SimAction, SimConfig, SimError, Velocity, Visual,
    },
};

#[derive(Clone)]
pub(super) struct PeerControl {
    pub(super) binding: Ownership,
    pub(super) handle: Handle,
    pub(super) active: bool,
    pub(super) last_sequence: Option<u64>,
    pub(super) axes: [f32; 3],
    pub(super) pending: Option<InputFrame<SimAction>>,
    pub(super) recompute_velocity: bool,
}

fn velocity_at(candidate: &Candidate, row: usize) -> &dyn Any {
    &candidate.velocities[row]
}

fn position_batch(candidate: &Candidate) -> &dyn Any {
    &candidate.position_edits
}

fn health_at(candidate: &Candidate, row: usize) -> &dyn Any {
    &candidate.health[row]
}

fn fill_spawn(
    candidate: &Candidate,
    index: usize,
    writer: &mut EntityWriter<'_>,
) -> Result<(), RegisteredWorldError> {
    let entity = candidate.spawns[index];
    writer.add(entity.position)?;
    writer.add(Health(entity.health))?;
    if entity.velocity != Velocity::default() || entity.ownership.is_some() {
        writer.add(entity.velocity)?;
    }
    if let Some(owner) = entity.ownership {
        writer.add(owner)?;
    }
    if let Some(visual) = entity.visual {
        writer.add(Visual(visual))?;
    }
    Ok(())
}

pub(super) struct Candidate {
    pub(super) motion: WriteRead2<Position, Velocity>,
    pub(super) capture_query: Read2<Position, Health>,
    positions: Vec<Position>,
    position_edits: Vec<(Handle, Position)>,
    base_positions: Vec<Position>,
    position_loaded: Vec<bool>,
    direct_positions: usize,
    velocities: Vec<Velocity>,
    health: Vec<Health>,
    base_velocity: Vec<Velocity>,
    base_health: Vec<u16>,
    velocity_present: Vec<bool>,
    owned: Vec<bool>,
    handles: Vec<Option<Handle>>,
    position_marks: Vec<u64>,
    velocity_marks: Vec<u64>,
    health_marks: Vec<u64>,
    touched_marks: Vec<u64>,
    despawn_marks: Vec<bool>,
    despawned: Vec<usize>,
    touched: Vec<usize>,
    spawns: Vec<super::types::ComponentEntity>,
    edits: Vec<Edit<Candidate>>,
}

#[derive(Default)]
pub(super) struct ParallelOutput {
    velocities: Vec<(Handle, Velocity)>,
    health: Vec<(Handle, u16)>,
}

impl Candidate {
    pub(super) fn new(
        capacity: usize,
        motion: WriteRead2<Position, Velocity>,
        capture_query: Read2<Position, Health>,
    ) -> Self {
        Self {
            motion,
            capture_query,
            positions: vec![Position::default(); capacity],
            position_edits: Vec::with_capacity(capacity.min(2_048)),
            base_positions: vec![Position::default(); capacity],
            position_loaded: vec![false; capacity],
            direct_positions: 0,
            velocities: vec![Velocity::default(); capacity],
            health: vec![Health(0); capacity],
            base_velocity: vec![Velocity::default(); capacity],
            base_health: vec![0; capacity],
            velocity_present: vec![false; capacity],
            owned: vec![false; capacity],
            handles: vec![None; capacity],
            position_marks: vec![0; capacity],
            velocity_marks: vec![0; capacity],
            health_marks: vec![0; capacity],
            touched_marks: vec![0; capacity],
            despawn_marks: vec![false; capacity],
            despawned: Vec::with_capacity(128),
            touched: Vec::with_capacity(capacity),
            spawns: Vec::with_capacity(128),
            edits: Vec::with_capacity(128),
        }
    }

    fn reset(&mut self) {
        for &row in &self.touched {
            self.position_marks[row] = 0;
            self.velocity_marks[row] = 0;
            self.health_marks[row] = 0;
            self.touched_marks[row] = 0;
            self.handles[row] = None;
            self.position_loaded[row] = false;
        }
        self.touched.clear();
        for &row in &self.despawned {
            self.despawn_marks[row] = false;
        }
        self.despawned.clear();
        self.position_edits.clear();
        self.spawns.clear();
        self.edits.clear();
        self.direct_positions = 0;
    }

    fn touch(&mut self, handle: Handle, tick: u64) -> usize {
        let row = handle.slot as usize;
        if self.touched_marks[row] != tick {
            self.touched_marks[row] = tick;
            self.touched.push(row);
            self.handles[row] = Some(handle);
            self.velocity_present[row] = false;
        }
        row
    }

    fn set_position(&mut self, handle: Handle, value: Position, tick: u64) {
        let row = self.touch(handle, tick);
        self.positions[row] = value;
        self.position_marks[row] = tick;
    }

    fn set_velocity(&mut self, handle: Handle, value: Velocity, tick: u64) {
        let row = self.touch(handle, tick);
        self.velocities[row] = value;
        self.velocity_marks[row] = tick;
    }

    fn set_health(&mut self, handle: Handle, value: u16, tick: u64) {
        let row = self.touch(handle, tick);
        self.health[row] = Health(value);
        self.health_marks[row] = tick;
    }

    fn velocity(&self, row: usize, base: Velocity, tick: u64) -> Velocity {
        if self.velocity_marks[row] == tick {
            self.velocities[row]
        } else {
            base
        }
    }
}

pub(super) struct Rules {
    pub(super) config: SimConfig,
    pub(super) resource_revision: u64,
    pub(super) peers: Vec<PeerControl>,
    pub(super) commands: Vec<Command>,
    pub(super) committed_inputs: Vec<Origin>,
    pub(super) changed_positions: usize,
    pub(super) position_write: WritePermit<Position>,
    pub(super) resolve_position_read: ReadPermit<Position>,
    pub(super) velocity_write: WritePermit<Velocity>,
    pub(super) health_write: WritePermit<Health>,
    pub(super) structure: StructuralPermit,
    pub(super) position_read: ReadPermit<Position>,
    pub(super) velocity_read: ReadPermit<Velocity>,
    pub(super) health_read: ReadPermit<Health>,
    pub(super) input_health_read: ReadPermit<Health>,
    pub(super) resolve_health_read: ReadPermit<Health>,
    pub(super) resolve_velocity_read: ReadPermit<Velocity>,
    pub(super) resolve_ownership_read: ReadPermit<Ownership>,
    pub(super) ownership_read: ReadPermit<Ownership>,
    pub(super) visual_read: ReadPermit<Visual>,
}

impl Rules {
    fn append_structural_edits(&self, candidate: &mut Candidate) {
        for command in &self.commands {
            if let Command::Despawn(handle) = command {
                candidate
                    .edits
                    .push(Edit::despawn(&self.structure, *handle));
            }
        }
        for command in &self.commands {
            if let Command::Spawn(entity) = command {
                let index = candidate.spawns.len();
                candidate.spawns.push(*entity);
                candidate
                    .edits
                    .push(Edit::spawn(&self.structure, entity.id, index, fill_spawn));
            }
        }
    }

    pub(super) fn input_velocity(
        &self,
        _world: &WorldRead<'_>,
        _executor: Executor,
        _tick: u64,
        output: &mut ParallelOutput,
    ) {
        output.velocities.clear();
        for peer in &self.peers {
            if !peer.active || (peer.pending.is_none() && !peer.recompute_velocity) {
                continue;
            }
            let axes = peer.pending.as_ref().map_or(peer.axes, |frame| frame.axes);
            let actions = peer
                .pending
                .as_ref()
                .map_or(&[][..], |frame| frame.actions.as_slice());
            output.velocities.push((
                peer.handle,
                resolve_velocity(axes, actions, self.config.movement_speed),
            ));
        }
    }

    pub(super) fn input_health(
        &self,
        world: &WorldRead<'_>,
        _executor: Executor,
        _tick: u64,
        output: &mut ParallelOutput,
    ) -> Result<(), SimError> {
        output.health.clear();
        for peer in &self.peers {
            if !peer.active {
                continue;
            }
            let actions = peer
                .pending
                .as_ref()
                .map_or(&[][..], |frame| frame.actions.as_slice());
            if !actions
                .iter()
                .any(|action| matches!(action, SimAction::Damage(_)))
            {
                continue;
            }
            let initial = world
                .with(&self.input_health_read, peer.handle, |value| value.0)
                .map_err(|_| SimError::InvalidHandle)?;
            output
                .health
                .push((peer.handle, resolve_health(initial, actions)));
        }
        Ok(())
    }

    pub(super) fn simulate(
        &self,
        world: &mut WorldTxn<'_>,
        _executor: Executor,
        tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), SimError> {
        let dt = 1.0 / f64::from(self.config.tick_hz.get());
        for command in &self.commands {
            if let Command::Despawn(handle) = command {
                let row = handle.slot as usize;
                candidate.despawn_marks[row] = true;
                candidate.despawned.push(row);
            } else if let Command::SetPosition(handle, position) = command {
                candidate.set_position(*handle, *position, tick);
            }
        }
        let motion = candidate.motion;
        world.visit(&motion, |handle, _, position, base_velocity| {
            let row = handle.slot as usize;
            if candidate.despawn_marks[row] {
                return Ok(false);
            }
            if candidate.position_marks[row] == tick {
                candidate.position_marks[row] = 0;
                let next = candidate.positions[row];
                if next != *position {
                    *position = next;
                    candidate.direct_positions += 1;
                    return Ok(true);
                }
                return Ok(false);
            }
            let effective = candidate.velocity(row, *base_velocity, tick);
            if candidate.velocity_marks[row] == tick {
                candidate.base_velocity[row] = *base_velocity;
                candidate.velocity_present[row] = true;
            }
            if effective != Velocity::default() {
                let next = integrate_position(*position, effective, dt);
                if !finite(next.0) {
                    return Err(SimError::NonFinite);
                }
                if next != *position {
                    *position = next;
                    candidate.direct_positions += 1;
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

    pub(super) fn resolve(
        &self,
        world: &WorldRead<'_>,
        _executor: Executor,
        tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), SimError> {
        for command in &self.commands {
            match command {
                Command::SetVelocity(handle, value) => {
                    candidate.set_velocity(*handle, *value, tick);
                }
                Command::SetHealth(handle, value) => {
                    candidate.set_health(*handle, *value, tick);
                }
                Command::SetPosition(_, _) | Command::Spawn(_) | Command::Despawn(_) => {}
            }
        }
        for &row in &candidate.touched {
            if candidate.despawn_marks[row] {
                continue;
            }
            let handle = candidate.handles[row].expect("touched row has handle");
            if candidate.position_marks[row] == tick {
                if !candidate.position_loaded[row] {
                    candidate.base_positions[row] = world
                        .with(&self.resolve_position_read, handle, |value| *value)
                        .map_err(|_| SimError::InvalidHandle)?;
                    candidate.position_loaded[row] = true;
                }
                if candidate.base_positions[row] != candidate.positions[row] {
                    candidate
                        .position_edits
                        .push((handle, candidate.positions[row]));
                }
            }
            if !candidate.velocity_present[row] {
                if let Ok(value) = world.with(&self.resolve_velocity_read, handle, |value| *value) {
                    candidate.base_velocity[row] = value;
                    candidate.velocity_present[row] = true;
                } else {
                    candidate.base_velocity[row] = Velocity::default();
                }
            }
            if candidate.health_marks[row] == tick {
                candidate.base_health[row] = world
                    .with(&self.resolve_health_read, handle, |value| value.0)
                    .map_err(|_| SimError::InvalidHandle)?;
            }
            if candidate.velocity_marks[row] == tick
                && candidate.velocities[row] == Velocity::default()
            {
                candidate.owned[row] = world
                    .with(&self.resolve_ownership_read, handle, |_| ())
                    .is_ok();
            }
            if (candidate.position_marks[row] == tick && !finite(candidate.positions[row].0))
                || (candidate.velocity_marks[row] == tick && !finite(candidate.velocities[row].0))
            {
                return Err(SimError::NonFinite);
            }
            if candidate.velocity_marks[row] == tick
                && candidate.base_velocity[row] != candidate.velocities[row]
            {
                let edit =
                    if candidate.velocities[row] == Velocity::default() && !candidate.owned[row] {
                        Edit::remove(&self.velocity_write, handle)
                    } else if candidate.velocity_present[row] {
                        Edit::replace(&self.velocity_write, handle, row, velocity_at)
                    } else {
                        Edit::insert(&self.velocity_write, handle, row, velocity_at)
                    };
                candidate.edits.push(edit);
            }
            if candidate.health_marks[row] == tick
                && candidate.base_health[row] != candidate.health[row].0
            {
                candidate
                    .edits
                    .push(Edit::replace(&self.health_write, handle, row, health_at));
            }
        }
        if !candidate.position_edits.is_empty() {
            candidate
                .edits
                .push(Edit::replace_batch(&self.position_write, position_batch));
        }
        self.append_structural_edits(candidate);
        Ok(())
    }
}

impl TickGame<Page> for Rules {
    type Error = SimError;
    type Candidate = Candidate;
    type ParallelOutput = ParallelOutput;

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.reset();
    }

    fn reset_parallel_output(&self, output: &mut ParallelOutput) {
        output.velocities.clear();
        output.health.clear();
    }

    fn merge_parallel(
        &self,
        system: thrum_core::sim::SystemId,
        output: &mut ParallelOutput,
        candidate: &mut Candidate,
        tick: u64,
    ) {
        match system {
            INPUT => {
                for (handle, velocity) in output.velocities.drain(..) {
                    candidate.set_velocity(handle, velocity, tick);
                }
            }
            INPUT_HEALTH => {
                for (handle, health) in output.health.drain(..) {
                    candidate.set_health(handle, health, tick);
                }
            }
            _ => unreachable!("only two V1 input systems use parallel output"),
        }
    }

    fn edits<'a>(&self, candidate: &'a Candidate) -> &'a [Edit<Candidate>] {
        &candidate.edits
    }

    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, candidate: &Candidate) {
        self.committed_inputs.clear();
        for peer in &self.peers {
            if let Some(frame) = &peer.pending {
                self.committed_inputs.push(frame.origin);
            }
        }
        self.changed_positions = candidate.direct_positions + candidate.position_edits.len();
        for command in &self.commands {
            if let Command::Despawn(handle) = command
                && let Some(peer) = self.peers.iter_mut().find(|peer| peer.handle == *handle)
            {
                peer.active = false;
                peer.axes = [0.0; 3];
                peer.pending = None;
            }
        }
        for peer in &mut self.peers {
            let command_overrode_velocity = self.commands.iter().any(
                |command| matches!(command, Command::SetVelocity(handle, _) if *handle == peer.handle),
            );
            if let Some(frame) = peer.pending.take() {
                peer.axes = frame.axes;
                peer.recompute_velocity = frame
                    .actions
                    .iter()
                    .any(|action| matches!(action, SimAction::Impulse(_)))
                    || command_overrode_velocity;
            } else {
                peer.recompute_velocity = command_overrode_velocity;
            }
        }
    }

    fn capture(
        &mut self,
        world: &WorldRead<'_>,
        candidate: &mut Candidate,
        events: &[thrum_core::sim::StructuralChange],
        page: &mut Page,
        first: bool,
        tick: u64,
        timeline: u64,
    ) {
        page.capture(
            world,
            &mut candidate.capture_query,
            &self.position_read,
            &self.velocity_read,
            &self.health_read,
            &self.ownership_read,
            &self.visual_read,
            events,
            tick,
            timeline,
            first,
        );
    }
}

pub(super) fn finite(values: [f64; 3]) -> bool {
    values.into_iter().all(f64::is_finite)
}

fn integrate_position(mut position: Position, velocity: Velocity, dt: f64) -> Position {
    for (value, speed) in position.0.iter_mut().zip(velocity.0) {
        *value += speed * dt;
    }
    position
}

fn resolve_velocity(axes: [f32; 3], actions: &[SimAction], speed: f64) -> Velocity {
    let mut velocity = Velocity(axes.map(|axis| f64::from(axis) * speed));
    for action in actions {
        if let SimAction::Impulse(impulse) = action {
            for (value, delta) in velocity.0.iter_mut().zip(impulse) {
                *value += delta;
            }
        }
    }
    velocity
}

fn resolve_health(mut health: u16, actions: &[SimAction]) -> u16 {
    for action in actions {
        if let SimAction::Damage(damage) = action {
            health = health.saturating_sub(*damage);
        }
    }
    health
}
