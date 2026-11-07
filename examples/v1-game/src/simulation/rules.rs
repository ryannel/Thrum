//! Candidate work and V1 game rules. Every expected refusal happens before
//! the engine's infallible commit phase.

use std::any::Any;

use thrum_core::{
    Handle, Origin,
    sim::{
        Edit, EditWriter, EntityWriter, ExecutionMode, Executor, InputFrame, ReadPermit,
        RegisteredWorldError, StructuralPermit, TickGame, WorldRead, WorldTxn, WritePermit,
        WriteRead2,
    },
};

use super::{
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

/// Shared payload storage read by queued edit descriptors at final preflight.
/// Only the ordered Resolve merge writes it, from the Resolve stage.
pub(super) struct Candidate {
    pub(super) motion: WriteRead2<Position, Velocity>,
    position_edits: Vec<(Handle, Position)>,
    direct_positions: usize,
    velocities: Vec<Velocity>,
    health: Vec<Health>,
    spawns: Vec<super::types::ComponentEntity>,
}

impl Candidate {
    pub(super) fn new(capacity: usize, motion: WriteRead2<Position, Velocity>) -> Self {
        Self {
            motion,
            position_edits: Vec::with_capacity(capacity.min(2_048)),
            direct_positions: 0,
            velocities: vec![Velocity::default(); capacity],
            health: vec![Health(0); capacity],
            spawns: Vec::with_capacity(128),
        }
    }

    fn reset(&mut self) {
        self.position_edits.clear();
        self.spawns.clear();
        self.direct_positions = 0;
    }
}

#[derive(Clone, Copy)]
enum VelocityEdit {
    Insert,
    Replace,
    Remove,
}

#[derive(Clone, Copy)]
enum StagedEdit {
    Velocity(Handle, VelocityEdit, Velocity),
    Health(Handle, Health),
}

/// Resolve's private stage. The Resolve callback alone fills it from
/// committed state and this tick's commands; the ordered merge moves it into
/// shared payloads under Resolve's own permits.
#[derive(Default)]
pub(super) struct ResolveStage {
    positions: Vec<Position>,
    base_positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<Health>,
    base_velocity: Vec<Velocity>,
    velocity_present: Vec<bool>,
    handles: Vec<Option<Handle>>,
    position_marks: Vec<bool>,
    velocity_marks: Vec<bool>,
    health_marks: Vec<bool>,
    touched_marks: Vec<bool>,
    despawn_marks: Vec<bool>,
    despawned: Vec<usize>,
    touched: Vec<usize>,
    edits: Vec<StagedEdit>,
    position_edits: Vec<(Handle, Position)>,
}

impl ResolveStage {
    fn reset(&mut self, capacity: usize) {
        if self.touched_marks.len() < capacity {
            self.positions.resize(capacity, Position::default());
            self.base_positions.resize(capacity, Position::default());
            self.velocities.resize(capacity, Velocity::default());
            self.health.resize(capacity, Health(0));
            self.base_velocity.resize(capacity, Velocity::default());
            self.velocity_present.resize(capacity, false);
            self.handles.resize(capacity, None);
            self.position_marks.resize(capacity, false);
            self.velocity_marks.resize(capacity, false);
            self.health_marks.resize(capacity, false);
            self.touched_marks.resize(capacity, false);
            self.despawn_marks.resize(capacity, false);
            self.touched.reserve(capacity);
            self.edits.reserve(256);
            self.position_edits.reserve(capacity.min(2_048));
            self.despawned.reserve(128);
        }
        for &row in &self.touched {
            self.position_marks[row] = false;
            self.velocity_marks[row] = false;
            self.health_marks[row] = false;
            self.touched_marks[row] = false;
            self.handles[row] = None;
        }
        self.touched.clear();
        for &row in &self.despawned {
            self.despawn_marks[row] = false;
        }
        self.despawned.clear();
        self.edits.clear();
        self.position_edits.clear();
    }

    fn touch(&mut self, handle: Handle) -> usize {
        let row = handle.slot as usize;
        if !self.touched_marks[row] {
            self.touched_marks[row] = true;
            self.touched.push(row);
            self.handles[row] = Some(handle);
            self.velocity_present[row] = false;
        }
        row
    }

    fn set_position(&mut self, handle: Handle, value: Position) {
        let row = self.touch(handle);
        self.positions[row] = value;
        self.position_marks[row] = true;
    }

    fn set_velocity(&mut self, handle: Handle, value: Velocity) {
        let row = self.touch(handle);
        self.velocities[row] = value;
        self.velocity_marks[row] = true;
    }

    fn set_health(&mut self, handle: Handle, value: u16) {
        let row = self.touch(handle);
        self.health[row] = Health(value);
        self.health_marks[row] = true;
    }
}

/// Input velocity's declared access.
pub(super) struct InputVelocityPermits {
    pub(super) velocity_write: WritePermit<Velocity>,
}

/// Input health's declared access.
pub(super) struct InputHealthPermits {
    pub(super) health_read: ReadPermit<Health>,
    pub(super) health_write: WritePermit<Health>,
}

/// Resolve's declared access. Committed-state inspection outside a callback
/// also reads through these, since no callback is running then.
pub(super) struct ResolvePermits {
    pub(super) position_read: ReadPermit<Position>,
    pub(super) position_write: WritePermit<Position>,
    pub(super) velocity_read: ReadPermit<Velocity>,
    pub(super) velocity_write: WritePermit<Velocity>,
    pub(super) health_read: ReadPermit<Health>,
    pub(super) health_write: WritePermit<Health>,
    pub(super) ownership_read: ReadPermit<Ownership>,
    pub(super) visual_read: ReadPermit<Visual>,
    pub(super) structure: StructuralPermit,
}

/// Live entities from which requested Rayon dispatches the mover kernel.
/// `None` until a quiet measured win promotes it: shared-Mac rows on 7 Nov
/// (G2S parallel evidence) were slower at 10k–500k. `RayonForced` always
/// dispatches for correctness and crossover qualification.
pub(super) const PARALLEL_MOVER_CROSSOVER: Option<usize> = None;

pub(super) struct Rules {
    pub(super) config: SimConfig,
    pub(super) resource_revision: u64,
    pub(super) peers: Vec<PeerControl>,
    pub(super) commands: Vec<Command>,
    pub(super) committed_inputs: Vec<Origin>,
    pub(super) changed_positions: usize,
    pub(super) input_velocity: InputVelocityPermits,
    pub(super) input_health: InputHealthPermits,
    pub(super) resolve: ResolvePermits,
}

impl Rules {
    fn kernel_executor(executor: Executor, live: usize) -> Executor {
        if executor.mode() == ExecutionMode::Rayon
            && PARALLEL_MOVER_CROSSOVER.is_none_or(|crossover| live < crossover)
        {
            Executor::serial()
        } else {
            executor
        }
    }

    pub(super) fn input_velocity(
        &self,
        world: &mut WorldTxn<'_>,
        _executor: Executor,
        _tick: u64,
        _candidate: &mut Candidate,
        _edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), SimError> {
        for peer in &self.peers {
            if !peer.active || (peer.pending.is_none() && !peer.recompute_velocity) {
                continue;
            }
            let axes = peer.pending.as_ref().map_or(peer.axes, |frame| frame.axes);
            let actions = peer
                .pending
                .as_ref()
                .map_or(&[][..], |frame| frame.actions.as_slice());
            world
                .put(
                    &self.input_velocity.velocity_write,
                    peer.handle,
                    resolve_velocity(axes, actions, self.config.movement_speed),
                )
                .map_err(|_| SimError::InvalidHandle)?;
        }
        Ok(())
    }

    pub(super) fn input_health(
        &self,
        world: &mut WorldTxn<'_>,
        _executor: Executor,
        _tick: u64,
        _candidate: &mut Candidate,
        _edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), SimError> {
        let permits = &self.input_health;
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
                .with(&permits.health_read, peer.handle, |value| value.0)
                .map_err(|_| SimError::InvalidHandle)?;
            world
                .put(
                    &permits.health_write,
                    peer.handle,
                    Health(resolve_health(initial, actions)),
                )
                .map_err(|_| SimError::InvalidHandle)?;
        }
        Ok(())
    }

    /// Integrate every active mover in independent word-aligned chunks.
    /// Command targets that Resolve overrides or removes are excluded.
    pub(super) fn simulate(
        &self,
        world: &mut WorldTxn<'_>,
        executor: Executor,
        _tick: u64,
        candidate: &mut Candidate,
        _edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), SimError> {
        let dt = 1.0 / f64::from(self.config.tick_hz.get());
        let motion = candidate.motion;
        let changed = world.visit_chunked_indexed(
            &motion,
            Self::kernel_executor(executor, world.len()),
            &self.commands,
            |commands, mark| {
                for command in commands {
                    if let Command::Despawn(handle) | Command::SetPosition(handle, _) = command {
                        mark(handle.slot as usize);
                    }
                }
            },
            |_, _, _, _, _| true,
            |_, _, _, position, velocity| {
                let next = integrate_position(*position, *velocity, dt);
                if !finite(next.0) {
                    return Err(SimError::NonFinite);
                }
                if next != *position {
                    *position = next;
                    return Ok(true);
                }
                Ok(false)
            },
        )?;
        candidate.direct_positions = changed;
        Ok(())
    }

    /// Stage command overrides and structure privately from committed state.
    pub(super) fn resolve(
        &self,
        world: &WorldRead<'_>,
        _executor: Executor,
        _tick: u64,
        stage: &mut ResolveStage,
    ) -> Result<(), SimError> {
        let permits = &self.resolve;
        for command in &self.commands {
            if let Command::Despawn(handle) = command {
                let row = handle.slot as usize;
                stage.despawn_marks[row] = true;
                stage.despawned.push(row);
            } else if let Command::SetPosition(handle, position) = command {
                stage.set_position(*handle, *position);
            }
        }
        for command in &self.commands {
            match command {
                Command::SetVelocity(handle, value) => stage.set_velocity(*handle, *value),
                Command::SetHealth(handle, value) => stage.set_health(*handle, *value),
                Command::SetPosition(_, _) | Command::Spawn(_) | Command::Despawn(_) => {}
            }
        }
        for index in 0..stage.touched.len() {
            let row = stage.touched[index];
            if stage.despawn_marks[row] {
                continue;
            }
            let handle = stage.handles[row].expect("touched row has handle");
            if stage.position_marks[row] {
                stage.base_positions[row] = world
                    .with(&permits.position_read, handle, |value| *value)
                    .map_err(|_| SimError::InvalidHandle)?;
                if stage.base_positions[row] != stage.positions[row] {
                    stage.position_edits.push((handle, stage.positions[row]));
                }
            }
            if !stage.velocity_present[row] {
                if let Ok(value) = world.with(&permits.velocity_read, handle, |value| *value) {
                    stage.base_velocity[row] = value;
                    stage.velocity_present[row] = true;
                } else {
                    stage.base_velocity[row] = Velocity::default();
                }
            }
            let base_health = if stage.health_marks[row] {
                Some(
                    world
                        .with(&permits.health_read, handle, |value| value.0)
                        .map_err(|_| SimError::InvalidHandle)?,
                )
            } else {
                None
            };
            if (stage.position_marks[row] && !finite(stage.positions[row].0))
                || (stage.velocity_marks[row] && !finite(stage.velocities[row].0))
            {
                return Err(SimError::NonFinite);
            }
            if stage.velocity_marks[row] {
                let velocity = stage.velocities[row];
                if stage.base_velocity[row] != velocity {
                    let owned = velocity == Velocity::default()
                        && world.with(&permits.ownership_read, handle, |_| ()).is_ok();
                    let kind = if velocity == Velocity::default() && !owned {
                        VelocityEdit::Remove
                    } else if stage.velocity_present[row] {
                        VelocityEdit::Replace
                    } else {
                        VelocityEdit::Insert
                    };
                    stage
                        .edits
                        .push(StagedEdit::Velocity(handle, kind, velocity));
                }
            }
            if let Some(base) = base_health
                && base != stage.health[row].0
            {
                stage
                    .edits
                    .push(StagedEdit::Health(handle, stage.health[row]));
            }
        }
        Ok(())
    }

    /// Move Resolve's stage into shared payloads and append its descriptors,
    /// all under Resolve's permits, in the stage's canonical order.
    fn merge_resolve(
        &self,
        stage: &mut ResolveStage,
        candidate: &mut Candidate,
        edits: &mut EditWriter<'_, Candidate>,
    ) {
        let permits = &self.resolve;
        for staged in stage.edits.drain(..) {
            match staged {
                StagedEdit::Velocity(handle, kind, value) => {
                    let row = handle.slot as usize;
                    candidate.velocities[row] = value;
                    edits.push(match kind {
                        VelocityEdit::Remove => Edit::remove(&permits.velocity_write, handle),
                        VelocityEdit::Replace => {
                            Edit::replace(&permits.velocity_write, handle, row, velocity_at)
                        }
                        VelocityEdit::Insert => {
                            Edit::insert(&permits.velocity_write, handle, row, velocity_at)
                        }
                    });
                }
                StagedEdit::Health(handle, value) => {
                    let row = handle.slot as usize;
                    candidate.health[row] = value;
                    edits.push(Edit::replace(&permits.health_write, handle, row, health_at));
                }
            }
        }
        candidate.position_edits.append(&mut stage.position_edits);
        if !candidate.position_edits.is_empty() {
            edits.push(Edit::replace_batch(&permits.position_write, position_batch));
        }
        for command in &self.commands {
            if let Command::Despawn(handle) = command {
                edits.push(Edit::despawn(&permits.structure, *handle));
            }
        }
        for command in &self.commands {
            if let Command::Spawn(entity) = command {
                let index = candidate.spawns.len();
                candidate.spawns.push(*entity);
                edits.push(Edit::spawn(
                    &permits.structure,
                    entity.id,
                    index,
                    fill_spawn,
                ));
            }
        }
    }
}

impl TickGame<Page> for Rules {
    type Error = SimError;
    type Candidate = Candidate;
    type ParallelOutput = ResolveStage;

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.reset();
    }

    fn reset_parallel_output(&self, stage: &mut ResolveStage) {
        stage.reset(self.config.entity_capacity);
    }

    fn merge_parallel(
        &self,
        system: thrum_core::sim::SystemId,
        stage: &mut ResolveStage,
        candidate: &mut Candidate,
        _tick: u64,
        edits: &mut EditWriter<'_, Candidate>,
    ) {
        debug_assert_eq!(system, super::RESOLVE, "Resolve is the only staged system");
        self.merge_resolve(stage, candidate, edits);
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
