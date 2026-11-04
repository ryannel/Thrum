use std::num::NonZeroU32;

use crate::{ActorId, Handle, Origin, PeerId, StableId, WorldEpoch};

use super::{
    Access, AccessKey, AccessMode, ComponentKind, FrameError, InputFrame, Schedule, SimAction,
    Stage, SystemId, SystemSpec,
    controller::{ControllerRule, integrate_position, resolve_control},
    storage::Store,
};

/// Right-handed Y-up world position in f64 metres. A1 has no hierarchy/TRS solver.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position(pub [f64; 3]);

/// World-space metres per fixed-world second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Velocity(pub [f64; 3]);

/// Single authority control binding; reconnect changes its connection epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ownership {
    /// Controlling peer.
    pub peer: PeerId,
    /// Peer connection generation.
    pub connection_epoch: u64,
    /// Controlled actor.
    pub actor: ActorId,
}

/// Component values accepted by the bounded A1 adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComponentEntity {
    /// Stable identity, never a backend entity.
    pub id: StableId,
    /// Committed world position.
    pub position: Position,
    /// Constant-velocity fixture motion.
    pub velocity: Velocity,
    /// Fixture health; zero is a valid resolved state.
    pub health: u16,
    /// Optional peer control binding.
    pub ownership: Option<Ownership>,
    /// Optional opaque visual selector; no renderer-owned state is stored here.
    pub visual: Option<u64>,
}

/// Last committed component changes. Equality-preserving writes do not advance
/// ticks. A spawn marks every present column at its creation tick.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChangeTicks {
    /// Last position change.
    pub position: u64,
    /// Last velocity change.
    pub velocity: u64,
    /// Last health change.
    pub health: u64,
    /// Creation tick; ownership/visual changes currently require respawn.
    pub structural: u64,
}

/// Tick-bounded typed view. Values are copied from private storage, while the
/// immutable world borrow prevents retaining this view across a mutation.
pub struct ComponentView<'a> {
    /// Checked public runtime identity.
    pub handle: Handle,
    /// Stable identity.
    pub id: StableId,
    /// Position component at this tick.
    pub position: Position,
    /// Velocity component at this tick.
    pub velocity: Velocity,
    /// Health component.
    pub health: u16,
    /// Optional control component.
    pub ownership: Option<Ownership>,
    /// Optional visual component.
    pub visual: Option<u64>,
    /// Component change stamps.
    pub changes: ChangeTicks,
    pub(super) world_borrow: std::marker::PhantomData<&'a ()>,
}

/// One committed owned controller projection for a confirmed client. This is
/// a copied gameplay value, never a render packet or second world authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerState {
    /// Exact current peer, epoch and controlled actor binding.
    pub binding: Ownership,
    /// Committed world position.
    pub position: Position,
    /// Committed velocity after held control and this tick's actions.
    pub velocity: Velocity,
    /// Committed fixture health.
    pub health: u16,
    /// Held axes that the next input stage uses if no frame arrives.
    pub held_axes: [f32; 3],
}

/// Resolve-stage edits. A batch validates fully against the pre-tick identity
/// map before simulation. Conflicting writes/destruction reject the whole batch;
/// same-batch spawn references and stable-ID replacement are deliberately absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Spawn at resolve; its first simulation step occurs on the following tick.
    Spawn(ComponentEntity),
    /// Commit destruction after this tick's simulation.
    Despawn(Handle),
    /// Override the simulated position at resolve.
    SetPosition(Handle, Position),
    /// Set resolved velocity for subsequent simulation. A controlled entity's
    /// held axes derive a fresh velocity at the next input stage.
    SetVelocity(Handle, Velocity),
    /// Set resolved health.
    SetHealth(Handle, u16),
}

/// Explicit bounded fixture configuration, not an engine-wide memory policy.
#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// One immutable timestep. The host uses the same rate in `TickClock`.
    pub tick_hz: NonZeroU32,
    /// Maximum physical entity slots, preallocated at construction.
    pub entity_capacity: usize,
    /// Maximum distinct peer identities over this fixture world's lifetime.
    /// Retired peers retain a bounded connection-epoch tombstone.
    pub peer_capacity: usize,
    /// Maximum discrete actions in one frame.
    pub actions_per_frame: usize,
    /// Metres/second represented by a unit continuous movement axis.
    pub movement_speed: f64,
}

/// Expected simulation/admission failure. All failed ticks leave committed
/// columns, tick and consumer snapshots unchanged; admitted frames remain pending.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimError {
    /// Invalid capacity, nonfinite speed, or negative movement speed.
    InvalidConfig,
    /// Physical entity or peer capacity was exhausted.
    Capacity,
    /// Stable ID is active or repeated in the batch.
    DuplicateId,
    /// Runtime handle is stale or from another world.
    InvalidHandle,
    /// A component is nonfinite or integration overflowed.
    NonFinite,
    /// The same component is written twice, or destruction overlaps another edit.
    ConflictingCommands,
    /// Peer registration already exists or ownership does not match.
    InvalidOwnership,
    /// Initial entities can only be seeded before tick one and before peers.
    AlreadyStarted,
    /// World tick cannot advance without wrapping.
    TickExhausted,
}

/// Owned immutable-endpoint copy. Its private vector never aliases simulation
/// columns. Reuse with `copy_snapshot_into` to amortize consumer allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulationSnapshot {
    epoch: WorldEpoch,
    tick: u64,
    entities: Vec<ComponentEntity>,
}

impl SimulationSnapshot {
    /// Construct an independent consumer buffer with a declared capacity.
    pub fn with_capacity(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            epoch,
            tick: 0,
            entities: Vec::with_capacity(capacity),
        }
    }
    /// Source world identity.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Complete committed tick represented by this copy.
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Owned component records in unspecified order. Match records by stable ID;
    /// iteration order is not a replay, wire or save contract.
    pub fn entities(&self) -> &[ComponentEntity] {
        &self.entities
    }
}

/// Complete tick outcome; no consumer can observe a partial stage endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickOutcome<'a> {
    /// Tick that committed exactly once.
    pub tick: u64,
    /// Number of entities visited by the simulation stage, including idle ones.
    pub simulated_entities: usize,
    /// Position changes to pre-existing entities, excluding despawned records.
    pub changed_positions: usize,
    /// Exact origins whose pending frames were consumed by this committed tick.
    /// This borrowed page is valid until the next mutable world operation.
    pub committed_inputs: &'a [Origin],
}

struct PeerControl {
    binding: Ownership,
    handle: Handle,
    active: bool,
    last_sequence: Option<u64>,
    axes: [f32; 3],
    pending: Option<InputFrame<SimAction>>,
}

struct Work {
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<u16>,
    position_marks: Vec<u64>,
    velocity_marks: Vec<u64>,
    health_marks: Vec<u64>,
    touched_marks: Vec<u64>,
    touched: Vec<usize>,
}

impl Work {
    fn touch(&mut self, row: usize, tick: u64) {
        if self.touched_marks[row] != tick {
            self.touched_marks[row] = tick;
            self.touched.push(row);
        }
    }

    fn set_position(&mut self, row: usize, value: Position, tick: u64) {
        self.touch(row, tick);
        self.positions[row] = value;
        self.position_marks[row] = tick;
    }

    fn set_velocity(&mut self, row: usize, value: Velocity, tick: u64) {
        self.touch(row, tick);
        self.velocities[row] = value;
        self.velocity_marks[row] = tick;
    }

    fn set_health(&mut self, row: usize, value: u16, tick: u64) {
        self.touch(row, tick);
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

/// One provisional A1 world authority. It is independent of the legacy I1
/// authority; a game must choose one, never mirror gameplay into both.
///
/// Private hecs components and reusable candidate columns allocate at setup. `step`
/// neither clones whole records nor allocates after warmup. Input
/// action vectors are caller-owned allocations moved into pending admission.
pub struct Simulation {
    epoch: WorldEpoch,
    config: SimConfig,
    tick: u64,
    store: Store,
    peers: Vec<PeerControl>,
    committed_inputs: Vec<Origin>,
    work: Work,
    schedule: Schedule,
}

impl Simulation {
    /// Allocate a bounded fixture world and compile its serial stage graph.
    ///
    /// # Errors
    ///
    /// Capacity or movement-speed configuration is invalid.
    pub fn new(epoch: WorldEpoch, config: SimConfig) -> Result<Self, SimError> {
        if config.entity_capacity > u32::MAX as usize
            || !config.movement_speed.is_finite()
            || config.movement_speed < 0.0
        {
            return Err(SimError::InvalidConfig);
        }
        Ok(Self {
            epoch,
            config,
            tick: 0,
            store: Store::new(epoch, config.entity_capacity),
            peers: Vec::with_capacity(config.peer_capacity),
            committed_inputs: Vec::with_capacity(config.peer_capacity),
            work: Work {
                positions: vec![Position::default(); config.entity_capacity],
                velocities: vec![Velocity::default(); config.entity_capacity],
                health: vec![0; config.entity_capacity],
                position_marks: vec![0; config.entity_capacity],
                velocity_marks: vec![0; config.entity_capacity],
                health_marks: vec![0; config.entity_capacity],
                touched_marks: vec![0; config.entity_capacity],
                touched: Vec::with_capacity(config.entity_capacity),
            },
            schedule: fixture_schedule(),
        })
    }

    /// Fixed rate selected at world construction; never driven by render time.
    pub fn tick_hz(&self) -> NonZeroU32 {
        self.config.tick_hz
    }
    /// Immutable controller recipe used by both this world and a separate
    /// speculative fixture branch. Capture it when the match starts.
    pub fn controller_rule(&self) -> ControllerRule {
        ControllerRule::from_config(self.config)
    }
    /// Last fully committed world tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// World generation shared by committed consumer snapshots.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Current live entity count.
    pub fn len(&self) -> usize {
        self.store.len()
    }
    /// Whether no live entities remain.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Compiled graph used by this fixture.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Seed initial committed tick-zero components before input registration.
    ///
    /// # Errors
    ///
    /// Seeding has already ended, the ID repeats, an entity component is nonfinite, or capacity is full.
    pub fn seed(&mut self, entity: ComponentEntity) -> Result<Handle, SimError> {
        if self.tick != 0 || !self.peers.is_empty() {
            return Err(SimError::AlreadyStarted);
        }
        validate_entity(&entity)?;
        if self.store.contains(entity.id) {
            return Err(SimError::DuplicateId);
        }
        if self.store.free_count() == 0 {
            return Err(SimError::Capacity);
        }
        Ok(self.store.spawn(entity, 0))
    }

    /// Resolve a stable identity to its current checked runtime handle.
    pub fn handle(&self, id: StableId) -> Option<Handle> {
        self.store.handle(id)
    }

    /// Read committed typed components through a checked handle.
    ///
    /// # Errors
    ///
    /// The handle is stale or belongs to another world.
    pub fn get(&self, handle: Handle) -> Result<ComponentView<'_>, SimError> {
        self.store.row(handle).map(|row| self.store.view(row))
    }

    /// Copy one active peer's complete controller value at the current
    /// committed world tick. A caller pairs it with that tick's result origins
    /// before publishing a client group.
    ///
    /// # Errors
    /// The peer is absent/inactive, its handle is stale, or ownership changed.
    pub fn controller_state(&self, peer_id: PeerId) -> Result<ControllerState, SimError> {
        let peer = self
            .peers
            .iter()
            .find(|peer| peer.binding.peer == peer_id && peer.active)
            .ok_or(SimError::InvalidOwnership)?;
        let row = self.store.row(peer.handle)?;
        if self.store.ownership(row) != Some(peer.binding) {
            return Err(SimError::InvalidOwnership);
        }
        Ok(ControllerState {
            binding: peer.binding,
            position: self.store.position(row),
            velocity: self.store.velocity(row),
            health: self.store.health(row),
            held_axes: peer.axes,
        })
    }

    /// Visit typed component columns without allocating temporary records.
    pub fn visit_components(&self, visit: impl FnMut(ComponentView<'_>)) {
        self.store.visit_components(visit);
    }

    /// Register one control binding. Local and remote callers use this same path.
    ///
    /// # Errors
    ///
    /// Peer capacity is full, the peer already exists, or the handle and ownership do not match.
    pub fn register_peer(&mut self, binding: Ownership, handle: Handle) -> Result<(), SimError> {
        let row = self.store.row(handle)?;
        if self.store.ownership(row) != Some(binding)
            || self.peers.iter().any(|p| p.active && p.handle == handle)
        {
            return Err(SimError::InvalidOwnership);
        }
        if let Some(peer) = self
            .peers
            .iter_mut()
            .find(|p| p.binding.peer == binding.peer)
        {
            if peer.active || binding.connection_epoch <= peer.binding.connection_epoch {
                return Err(SimError::InvalidOwnership);
            }
            *peer = PeerControl {
                binding,
                handle,
                active: true,
                last_sequence: None,
                axes: [0.0; 3],
                pending: None,
            };
            return Ok(());
        }
        if self.peers.len() == self.config.peer_capacity {
            return Err(SimError::Capacity);
        }
        self.peers.push(PeerControl {
            binding,
            handle,
            active: true,
            last_sequence: None,
            axes: [0.0; 3],
            pending: None,
        });
        // Stable peer order owns order-sensitive input traversal, never backend order.
        self.peers.sort_unstable_by_key(|p| p.binding.peer);
        Ok(())
    }

    /// Check game action count and numeric values before a future N1 frame
    /// reserves a timing slot. This is the same action gate used at W1 admission;
    /// it does not validate peer ownership or consume an input sequence.
    ///
    /// # Errors
    /// Returns `TooManyActions` or `InvalidAction` for an unsupported payload.
    pub fn validate_frame_actions(&self, frame: &InputFrame<SimAction>) -> Result<(), FrameError> {
        if frame.actions.len() > self.config.actions_per_frame {
            return Err(FrameError::TooManyActions);
        }
        if frame
            .actions
            .iter()
            .any(|action| matches!(action, SimAction::Impulse(v) if !finite(*v)))
        {
            return Err(FrameError::InvalidAction);
        }
        Ok(())
    }

    /// Admit exactly one frame for a peer's next tick. Complete validation comes
    /// before sequence mutation; admission does not mutate committed components.
    ///
    /// # Errors
    ///
    /// The peer, connection, actor, tick, sequence, control, axes, actions, or pending-frame slot fails admission.
    pub fn admit_frame(&mut self, frame: InputFrame<SimAction>) -> Result<(), FrameError> {
        frame.validate_axes()?;
        self.validate_frame_actions(&frame)?;
        let peer = self
            .peers
            .iter_mut()
            .find(|p| p.binding.peer == frame.origin.peer)
            .ok_or(FrameError::UnknownPeer)?;
        if peer.binding.connection_epoch != frame.origin.connection_epoch {
            return Err(FrameError::WrongConnection);
        }
        if peer.binding.actor != frame.actor {
            return Err(FrameError::WrongActor);
        }
        if !peer.active {
            return Err(FrameError::InvalidControl);
        }
        if self.tick.checked_add(1) != Some(frame.tick) {
            return Err(FrameError::WrongTick);
        }
        if peer
            .last_sequence
            .is_some_and(|last| frame.origin.sequence <= last)
        {
            return Err(FrameError::StaleSequence);
        }
        if peer.pending.is_some() {
            return Err(FrameError::PendingFrame);
        }
        let row = self
            .store
            .row(peer.handle)
            .map_err(|_| FrameError::InvalidControl)?;
        if self.store.ownership(row) != Some(peer.binding) {
            return Err(FrameError::InvalidControl);
        }
        peer.last_sequence = Some(frame.origin.sequence);
        peer.pending = Some(frame);
        Ok(())
    }

    /// Execute one fixed tick. Resolve commands apply after simulation; new
    /// entities begin moving next tick. All rejection paths precede publication.
    /// No wall time is read. Drive this from the existing bounded `TickClock`.
    ///
    /// # Errors
    ///
    /// The tick cannot advance, a command conflicts or exceeds capacity, or a component becomes nonfinite.
    ///
    /// # Panics
    ///
    /// Panics only if a checked internal storage invariant is broken.
    #[expect(
        clippy::float_cmp,
        reason = "change ticks track exact component value changes, not geometric proximity"
    )]
    pub fn step(&mut self, commands: &[Command]) -> Result<TickOutcome<'_>, SimError> {
        let next = self.tick.checked_add(1).ok_or(SimError::TickExhausted)?;
        self.validate_commands(commands)?;
        let simulated_entities = self.len();
        // A rejected candidate retries the same tick. Clear its stamps before
        // staging again so no stale row is omitted or later published.
        for &row in &self.work.touched {
            self.work.position_marks[row] = 0;
            self.work.velocity_marks[row] = 0;
            self.work.health_marks[row] = 0;
            self.work.touched_marks[row] = 0;
        }
        self.work.touched.clear();
        let Self {
            store,
            peers,
            committed_inputs,
            work,
            schedule,
            tick,
            config,
            ..
        } = self;
        let mut changed_positions = 0;
        schedule.run(|system| {
            match system.stage {
                Stage::Input => apply_input(store, peers, work, config.movement_speed, next),
                Stage::Simulate => {
                    let dt = 1.0 / f64::from(config.tick_hz.get());
                    store.for_each_motion(|row, mut position, base_velocity| {
                        let velocity = work.velocity(row, base_velocity, next);
                        if velocity.0 != [0.0; 3] {
                            position = integrate_position(position, velocity, dt);
                            work.set_position(row, position, next);
                        }
                    });
                }
                Stage::Resolve => {
                    for command in commands {
                        match command {
                            Command::SetPosition(h, value) => {
                                work.set_position(store.row(*h)?, *value, next);
                            }
                            Command::SetVelocity(h, value) => {
                                work.set_velocity(store.row(*h)?, *value, next);
                            }
                            Command::SetHealth(h, value) => {
                                work.set_health(store.row(*h)?, *value, next);
                            }
                            Command::Spawn(_) | Command::Despawn(_) => {}
                        }
                    }
                    if work.touched.iter().any(|&row| {
                        (work.position_marks[row] == next && !finite(work.positions[row].0))
                            || (work.velocity_marks[row] == next && !finite(work.velocities[row].0))
                    }) {
                        return Err(SimError::NonFinite);
                    }
                }
                Stage::Publish => {
                    committed_inputs.clear();
                    for peer in peers.iter() {
                        if let Some(frame) = &peer.pending {
                            committed_inputs.push(frame.origin);
                        }
                    }
                    changed_positions =
                        store.publish_positions(&work.positions, &work.position_marks, next);
                    for &row in &work.touched {
                        if work.velocity_marks[row] == next
                            && store.velocity(row) != work.velocities[row]
                        {
                            store.set_velocity(row, work.velocities[row]);
                            store.changes[row].velocity = next;
                        }
                        if work.health_marks[row] == next && store.health(row) != work.health[row] {
                            store.set_health(row, work.health[row]);
                            store.changes[row].health = next;
                        }
                    }
                    // All removals precede spawns so validated capacity is reusable.
                    for command in commands {
                        if let Command::Despawn(handle) = command {
                            let row = store.row(*handle).expect("validated unique destruction");
                            if store.changes[row].position == next {
                                changed_positions -= 1;
                            }
                            store.despawn(*handle);
                            // Retain the peer's connection generation so the same
                            // origin cannot replay into a later controlled body.
                            if let Some(peer) = peers.iter_mut().find(|p| p.handle == *handle) {
                                peer.active = false;
                                peer.axes = [0.0; 3];
                                peer.pending = None;
                            }
                        }
                    }
                    for command in commands {
                        if let Command::Spawn(entity) = command {
                            store.spawn(*entity, next);
                        }
                    }
                    for peer in peers.iter_mut() {
                        if let Some(frame) = peer.pending.take() {
                            peer.axes = frame.axes;
                        }
                    }
                    *tick = next;
                }
            }
            Ok(())
        })?;
        Ok(TickOutcome {
            tick: next,
            simulated_entities,
            changed_positions,
            committed_inputs,
        })
    }

    /// Copy only a committed endpoint into a consumer-owned snapshot. This is
    /// O(live entities); insufficient consumer capacity may allocate. Retained
    /// copies remain valid through all subsequent ticks and slot reuse.
    pub fn copy_snapshot_into(&self, output: &mut SimulationSnapshot) {
        self.store.copy_snapshot_into(&mut output.entities);
        output.epoch = self.epoch;
        output.tick = self.tick;
    }

    fn validate_commands(&self, commands: &[Command]) -> Result<(), SimError> {
        let mut free = self.store.free_count();
        let mut spawns = 0;
        for (i, command) in commands.iter().enumerate() {
            match command {
                Command::Spawn(entity) => {
                    validate_entity(entity)?;
                    if self.store.contains(entity.id)
                        || commands[..i]
                            .iter()
                            .any(|prior| matches!(prior, Command::Spawn(e) if e.id == entity.id))
                    {
                        return Err(SimError::DuplicateId);
                    }
                    spawns += 1;
                }
                Command::Despawn(handle) => {
                    self.store.row(*handle)?;
                    free += usize::from(self.store.reusable_after_despawn(*handle));
                }
                Command::SetPosition(handle, value) => {
                    self.store.row(*handle)?;
                    if !finite(value.0) {
                        return Err(SimError::NonFinite);
                    }
                }
                Command::SetVelocity(handle, value) => {
                    self.store.row(*handle)?;
                    if !finite(value.0) {
                        return Err(SimError::NonFinite);
                    }
                }
                Command::SetHealth(handle, _) => {
                    self.store.row(*handle)?;
                }
            }
            if commands[..i]
                .iter()
                .any(|prior| command_conflict(prior, command))
            {
                return Err(SimError::ConflictingCommands);
            }
        }
        if spawns > free {
            return Err(SimError::Capacity);
        }
        Ok(())
    }
}

fn finite(values: [f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn validate_entity(entity: &ComponentEntity) -> Result<(), SimError> {
    if finite(entity.position.0) && finite(entity.velocity.0) {
        Ok(())
    } else {
        Err(SimError::NonFinite)
    }
}

fn command_target(command: &Command) -> Option<(Handle, u8)> {
    match command {
        Command::Spawn(_) => None,
        Command::Despawn(handle) => Some((*handle, 0)),
        Command::SetPosition(handle, _) => Some((*handle, 1)),
        Command::SetVelocity(handle, _) => Some((*handle, 2)),
        Command::SetHealth(handle, _) => Some((*handle, 3)),
    }
}

fn command_conflict(a: &Command, b: &Command) -> bool {
    match (command_target(a), command_target(b)) {
        (Some((a, field_a)), Some((b, field_b))) => {
            a == b && (field_a == 0 || field_b == 0 || field_a == field_b)
        }
        _ => false,
    }
}

fn apply_input(
    store: &Store,
    peers: &[PeerControl],
    work: &mut Work,
    movement_speed: f64,
    tick: u64,
) {
    for peer in peers {
        if !peer.active {
            continue;
        }
        let Ok(row) = store.row(peer.handle) else {
            continue;
        };
        let axes = peer.pending.as_ref().map_or(peer.axes, |frame| frame.axes);
        let actions = peer
            .pending
            .as_ref()
            .map_or(&[][..], |frame| frame.actions.as_slice());
        let (velocity, health, damaged) =
            resolve_control(axes, actions, store.health(row), movement_speed);
        work.set_velocity(row, velocity, tick);
        if damaged {
            work.set_health(row, health, tick);
        }
    }
}

fn fixture_schedule() -> Schedule {
    use AccessMode::{Read, Write};
    use ComponentKind::{Health, Ownership, Position, Velocity};
    let component = |kind, mode| Access {
        key: AccessKey::Component(kind),
        mode,
    };
    let external = |key, mode| Access {
        key: AccessKey::External(key),
        mode,
    };
    Schedule::new(vec![
        SystemSpec {
            id: SystemId(0),
            stage: Stage::Input,
            accesses: vec![
                component(Ownership, Read),
                component(Velocity, Write),
                component(Health, Write),
                external(0, Read),
            ],
            after: vec![],
        },
        SystemSpec {
            id: SystemId(1),
            stage: Stage::Simulate,
            accesses: vec![component(Velocity, Read), component(Position, Write)],
            after: vec![],
        },
        SystemSpec {
            id: SystemId(2),
            stage: Stage::Resolve,
            accesses: vec![
                component(Position, Write),
                component(Velocity, Write),
                component(Health, Write),
                external(1, Read),
            ],
            after: vec![],
        },
        SystemSpec {
            id: SystemId(3),
            stage: Stage::Publish,
            accesses: vec![
                component(Position, Read),
                component(Velocity, Read),
                component(Health, Read),
                external(2, Write),
            ],
            after: vec![],
        },
    ])
    .expect("fixed fixture graph is acyclic")
}
