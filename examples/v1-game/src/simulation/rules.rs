//! Candidate work and V1 game rules. Every expected refusal happens before
//! the engine's infallible commit phase.

use thrum_core::{
    Handle, Origin,
    sim::{
        ExecutionMode, Executor, InputFrame, Read2, ReadPermit, Schedule, StructuralPermit,
        TickGame, WorldCommit, WorldRead, Write1, WritePermit,
    },
};

use super::{
    INPUT, PUBLISH, RESOLVE, SIMULATE,
    page::Page,
    types::{
        ChangeTicks, Command, Health, Ownership, Position, SimAction, SimConfig, SimError,
        Velocity, Visual,
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
}

pub(super) struct Candidate {
    pub(super) motion: Option<Read2<Position, Velocity>>,
    motion_jobs: Vec<MotionJob>,
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<u16>,
    base_velocity: Vec<Velocity>,
    base_health: Vec<u16>,
    velocity_present: Vec<bool>,
    owned: Vec<bool>,
    handles: Vec<Option<Handle>>,
    position_marks: Vec<u64>,
    velocity_marks: Vec<u64>,
    health_marks: Vec<u64>,
    touched_marks: Vec<u64>,
    touched: Vec<usize>,
}

struct MotionJob {
    handle: Handle,
    position: Position,
    velocity: Velocity,
}

impl Candidate {
    pub(super) fn new(capacity: usize, motion: Read2<Position, Velocity>) -> Self {
        Self {
            motion: Some(motion),
            motion_jobs: Vec::with_capacity(capacity.min(2_048)),
            positions: vec![Position::default(); capacity],
            velocities: vec![Velocity::default(); capacity],
            health: vec![0; capacity],
            base_velocity: vec![Velocity::default(); capacity],
            base_health: vec![0; capacity],
            velocity_present: vec![false; capacity],
            owned: vec![false; capacity],
            handles: vec![None; capacity],
            position_marks: vec![0; capacity],
            velocity_marks: vec![0; capacity],
            health_marks: vec![0; capacity],
            touched_marks: vec![0; capacity],
            touched: Vec::with_capacity(capacity),
        }
    }

    fn reset(&mut self) {
        for &row in &self.touched {
            self.position_marks[row] = 0;
            self.velocity_marks[row] = 0;
            self.health_marks[row] = 0;
            self.touched_marks[row] = 0;
            self.handles[row] = None;
        }
        self.touched.clear();
        self.motion_jobs.clear();
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
        self.health[row] = value;
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
    pub(super) peers: Vec<PeerControl>,
    pub(super) commands: Vec<Command>,
    pub(super) committed_inputs: Vec<Origin>,
    pub(super) changes: Vec<ChangeTicks>,
    pub(super) changed_positions: usize,
    pub(super) position_write: Write1<Position>,
    pub(super) velocity_write: WritePermit<Velocity>,
    pub(super) health_write: WritePermit<Health>,
    pub(super) structure: StructuralPermit,
    pub(super) capture_query: Read2<Position, Health>,
    pub(super) position_read: ReadPermit<Position>,
    pub(super) velocity_read: ReadPermit<Velocity>,
    pub(super) health_read: ReadPermit<Health>,
    pub(super) ownership_read: ReadPermit<Ownership>,
    pub(super) visual_read: ReadPermit<Visual>,
}

impl TickGame<Page> for Rules {
    type Error = SimError;
    type Candidate = Candidate;

    fn prepare(
        &self,
        world: &WorldRead<'_>,
        schedule: &Schedule,
        executor: Executor,
        tick: u64,
        candidate: &mut Candidate,
    ) -> Result<(), SimError> {
        candidate.reset();
        schedule.run(|system| {
            if system.id == INPUT {
                for peer in &self.peers {
                    if !peer.active {
                        continue;
                    }
                    let health = world
                        .with(&self.health_read, peer.handle, |value| value.0)
                        .map_err(|_| SimError::InvalidHandle)?;
                    let axes = peer.pending.as_ref().map_or(peer.axes, |frame| frame.axes);
                    let actions = peer
                        .pending
                        .as_ref()
                        .map_or(&[][..], |frame| frame.actions.as_slice());
                    let (velocity, health, damaged) =
                        resolve_control(axes, actions, health, self.config.movement_speed);
                    candidate.set_velocity(peer.handle, velocity, tick);
                    if damaged {
                        candidate.set_health(peer.handle, health, tick);
                    }
                }
            } else if system.id == SIMULATE {
                let dt = 1.0 / f64::from(self.config.tick_hz.get());
                let mut motion = candidate.motion.take().expect("prepared motion query");
                motion.visit(world, |handle, _, position, base_velocity| {
                    let row = candidate.touch(handle, tick);
                    candidate.base_velocity[row] = *base_velocity;
                    candidate.velocity_present[row] = true;
                    let velocity = candidate.velocity(row, *base_velocity, tick);
                    if velocity != Velocity::default() {
                        if executor.mode() == ExecutionMode::Serial {
                            candidate.set_position(
                                handle,
                                integrate_position(*position, velocity, dt),
                                tick,
                            );
                        } else {
                            candidate.motion_jobs.push(MotionJob {
                                handle,
                                position: *position,
                                velocity,
                            });
                        }
                    }
                });
                candidate.motion = Some(motion);
                executor.for_each_mut(&mut candidate.motion_jobs, |job| {
                    job.position = integrate_position(job.position, job.velocity, dt);
                });
                for index in 0..candidate.motion_jobs.len() {
                    let job = &candidate.motion_jobs[index];
                    candidate.set_position(job.handle, job.position, tick);
                }
            } else if system.id == RESOLVE {
                for command in &self.commands {
                    match command {
                        Command::SetPosition(handle, value) => {
                            candidate.set_position(*handle, *value, tick);
                        }
                        Command::SetVelocity(handle, value) => {
                            candidate.set_velocity(*handle, *value, tick);
                        }
                        Command::SetHealth(handle, value) => {
                            candidate.set_health(*handle, *value, tick);
                        }
                        Command::Spawn(_) | Command::Despawn(_) => {}
                    }
                }
                for &row in &candidate.touched {
                    let handle = candidate.handles[row].expect("touched row has handle");
                    if !candidate.velocity_present[row] {
                        if let Ok(value) = world.with(&self.velocity_read, handle, |value| *value) {
                            candidate.base_velocity[row] = value;
                            candidate.velocity_present[row] = true;
                        } else {
                            candidate.base_velocity[row] = Velocity::default();
                        }
                    }
                    if candidate.health_marks[row] == tick {
                        candidate.base_health[row] = world
                            .with(&self.health_read, handle, |value| value.0)
                            .map_err(|_| SimError::InvalidHandle)?;
                    }
                    if candidate.velocity_marks[row] == tick
                        && candidate.velocities[row] == Velocity::default()
                    {
                        candidate.owned[row] =
                            world.with(&self.ownership_read, handle, |_| ()).is_ok();
                    }
                    if (candidate.position_marks[row] == tick
                        && !finite(candidate.positions[row].0))
                        || (candidate.velocity_marks[row] == tick
                            && !finite(candidate.velocities[row].0))
                    {
                        return Err(SimError::NonFinite);
                    }
                }
            } else if system.id != PUBLISH {
                return Err(SimError::InvalidConfig);
            }
            Ok(())
        })
    }

    fn apply(&mut self, world: &mut WorldCommit<'_>, tick: u64, candidate: &Candidate) {
        self.committed_inputs.clear();
        for peer in &self.peers {
            if let Some(frame) = &peer.pending {
                self.committed_inputs.push(frame.origin);
            }
        }
        self.changed_positions = 0;
        self.position_write.visit(world, |handle, _, position| {
            let row = handle.slot as usize;
            if candidate.position_marks[row] == tick && *position != candidate.positions[row] {
                *position = candidate.positions[row];
                self.changes[row].position = tick;
                self.changed_positions += 1;
            }
        });
        for &row in &candidate.touched {
            let handle = candidate.handles[row].expect("touched row has handle");
            if candidate.velocity_marks[row] == tick
                && candidate.base_velocity[row] != candidate.velocities[row]
            {
                let value = candidate.velocities[row];
                if value == Velocity::default() && !candidate.owned[row] {
                    if candidate.velocity_present[row] {
                        world
                            .remove(&self.velocity_write, handle)
                            .expect("validated velocity removal");
                    }
                } else if candidate.velocity_present[row] {
                    world
                        .replace(&self.velocity_write, handle, value)
                        .expect("validated velocity replacement");
                } else {
                    world
                        .insert(&self.velocity_write, handle, value)
                        .expect("validated velocity insertion");
                }
                self.changes[row].velocity = tick;
            }
            if candidate.health_marks[row] == tick
                && candidate.base_health[row] != candidate.health[row]
            {
                world
                    .replace(&self.health_write, handle, Health(candidate.health[row]))
                    .expect("validated health replacement");
                self.changes[row].health = tick;
            }
        }
        for command in &self.commands {
            if let Command::Despawn(handle) = command {
                if self.changes[handle.slot as usize].position == tick {
                    self.changed_positions -= 1;
                }
                world
                    .despawn(&self.structure, *handle)
                    .expect("validated destruction");
                self.changes[handle.slot as usize] = ChangeTicks::default();
                if let Some(peer) = self.peers.iter_mut().find(|peer| peer.handle == *handle) {
                    peer.active = false;
                    peer.axes = [0.0; 3];
                    peer.pending = None;
                }
            }
        }
        for command in &self.commands {
            if let Command::Spawn(entity) = command {
                let handle = world
                    .spawn(&self.structure, entity.id, |writer| {
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
                    })
                    .expect("validated spawn");
                self.changes[handle.slot as usize] = ChangeTicks {
                    position: tick,
                    velocity: tick,
                    health: tick,
                    structural: tick,
                };
            }
        }
        for peer in &mut self.peers {
            if let Some(frame) = peer.pending.take() {
                peer.axes = frame.axes;
            }
        }
    }

    fn capture(&mut self, world: &WorldRead<'_>, page: &mut Page, first: bool, tick: u64) {
        page.capture(
            world,
            &mut self.capture_query,
            &self.velocity_read,
            &self.ownership_read,
            &self.visual_read,
            &self.changes,
            &self.commands,
            tick,
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

fn resolve_control(
    axes: [f32; 3],
    actions: &[SimAction],
    initial_health: u16,
    speed: f64,
) -> (Velocity, u16, bool) {
    let mut velocity = Velocity(axes.map(|axis| f64::from(axis) * speed));
    let mut health = initial_health;
    let mut damaged = false;
    for action in actions {
        match action {
            SimAction::Impulse(impulse) => {
                for (value, delta) in velocity.0.iter_mut().zip(impulse) {
                    *value += delta;
                }
            }
            SimAction::Damage(damage) => {
                health = health.saturating_sub(*damage);
                damaged = true;
            }
        }
    }
    (velocity, health, damaged)
}
