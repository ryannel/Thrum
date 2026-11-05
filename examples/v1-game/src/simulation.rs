//! V1 game's authority composed from generic engine storage, schedule and page
//! leases. The game owns all schema, rule, input and projection values.

mod history;
mod page;
mod rules;
mod types;

use std::mem::size_of;

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{
        Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
        ExecutionMode, FrameError, GameRuntime, InputFrame, RegisteredSystem, RegisteredWorldError,
        RuntimeCapture, RuntimeError, Stage, SystemId, SystemSpec, TickError,
    },
};

pub use history::SimulationHistory;
use page::Page;
pub use page::{DirtySlots, PublishedPage};
use rules::{Candidate, PeerControl, Rules, finite};
pub use types::{
    Command, ComponentEntity, ComponentView, Ownership, Position, SimAction, SimConfig, SimError,
    TickOutcome, Velocity,
};
use types::{Health, Visual};

const INPUT: SystemId = SystemId(0);
const INPUT_HEALTH: SystemId = SystemId(4);
const SIMULATE: SystemId = SystemId(1);
const RESOLVE: SystemId = SystemId(2);
const PUBLISH: SystemId = SystemId(3);
const STRUCTURE: u64 = 7;
// V1 game policy: bound one axis of admitted player motion in metres/second.
const MAX_CONTROL_SPEED: f64 = 1_000.0;

fn component(key: ComponentKey, mode: AccessMode) -> Access {
    Access {
        key: AccessKey::Component(key),
        mode,
    }
}

fn setup(
    epoch: WorldEpoch,
    config: SimConfig,
) -> Result<(GameRuntime<Page, Rules>, Rules, Candidate), SimError> {
    use AccessMode::{Read, Write};
    let mut registry = ComponentRegistry::default();
    let ownership = registry
        .register::<Ownership>(ComponentSchemaId(1))
        .map_err(|_| SimError::InvalidConfig)?;
    let velocity = registry
        .register::<Velocity>(ComponentSchemaId(2))
        .map_err(|_| SimError::InvalidConfig)?;
    let health = registry
        .register::<Health>(ComponentSchemaId(3))
        .map_err(|_| SimError::InvalidConfig)?;
    let position = registry
        .register::<Position>(ComponentSchemaId(4))
        .map_err(|_| SimError::InvalidConfig)?;
    let visual = registry
        .register::<Visual>(ComponentSchemaId(5))
        .map_err(|_| SimError::InvalidConfig)?;
    let systems = vec![
        SystemSpec {
            id: INPUT,
            stage: Stage::Input,
            accesses: vec![
                component(velocity, Write),
                Access {
                    key: AccessKey::External(0),
                    mode: Read,
                },
            ],
            after: vec![],
        },
        SystemSpec {
            id: INPUT_HEALTH,
            stage: Stage::Input,
            accesses: vec![
                component(health, Write),
                Access {
                    key: AccessKey::External(0),
                    mode: Read,
                },
            ],
            after: vec![],
        },
        SystemSpec {
            id: SIMULATE,
            stage: Stage::Simulate,
            accesses: vec![component(position, Write), component(velocity, Read)],
            after: vec![],
        },
        SystemSpec {
            id: RESOLVE,
            stage: Stage::Resolve,
            accesses: vec![
                component(position, Write),
                component(velocity, Write),
                component(health, Write),
                component(ownership, Read),
                Access {
                    key: AccessKey::External(1),
                    mode: Read,
                },
                Access {
                    key: AccessKey::External(STRUCTURE),
                    mode: Write,
                },
            ],
            after: vec![],
        },
        SystemSpec {
            id: PUBLISH,
            stage: Stage::Publish,
            accesses: vec![
                component(position, Write),
                component(velocity, Write),
                component(health, Write),
                component(ownership, Read),
                component(visual, Read),
            ],
            after: vec![],
        },
    ];
    let [input, input_health, simulate, resolve, publish]: [SystemSpec; 5] =
        systems.try_into().map_err(|_| SimError::InvalidConfig)?;
    let systems = vec![
        RegisteredSystem::parallel(input, |game: &Rules, world, executor, tick, output| {
            game.input_velocity(world, executor, tick, output);
            Ok(())
        }),
        RegisteredSystem::parallel(input_health, Rules::input_health),
        RegisteredSystem::direct(simulate, Rules::simulate),
        RegisteredSystem::new(resolve, Rules::resolve),
        RegisteredSystem::new(publish, |_, _, _, _, _| Ok(())),
    ];
    let runtime = GameRuntime::new(epoch, config.entity_capacity, registry, systems, || {
        Page::new(epoch, config.entity_capacity)
    })
    .map_err(map_runtime)?;
    let candidate = Candidate::new(
        config.entity_capacity,
        runtime
            .write_read2::<Position, Velocity>(SIMULATE)
            .map_err(map_runtime)?,
        runtime
            .read2::<Position, Health>(PUBLISH)
            .map_err(map_runtime)?,
    );
    let rules = Rules {
        config,
        resource_revision: 0,
        peers: Vec::with_capacity(config.peer_capacity),
        commands: Vec::with_capacity(128),
        committed_inputs: Vec::with_capacity(config.peer_capacity),
        changed_positions: 0,
        position_write: runtime
            .write_permit::<Position>(RESOLVE)
            .map_err(map_runtime)?,
        resolve_position_read: runtime
            .read_permit::<Position>(RESOLVE)
            .map_err(map_runtime)?,
        velocity_write: runtime
            .write_permit::<Velocity>(RESOLVE)
            .map_err(map_runtime)?,
        health_write: runtime
            .write_permit::<Health>(RESOLVE)
            .map_err(map_runtime)?,
        structure: runtime
            .structural_permit(RESOLVE, STRUCTURE)
            .map_err(map_runtime)?,
        position_read: runtime
            .read_permit::<Position>(PUBLISH)
            .map_err(map_runtime)?,
        velocity_read: runtime
            .read_permit::<Velocity>(PUBLISH)
            .map_err(map_runtime)?,
        health_read: runtime
            .read_permit::<Health>(PUBLISH)
            .map_err(map_runtime)?,
        input_health_read: runtime
            .read_permit::<Health>(INPUT_HEALTH)
            .map_err(map_runtime)?,
        resolve_health_read: runtime
            .read_permit::<Health>(RESOLVE)
            .map_err(map_runtime)?,
        resolve_velocity_read: runtime
            .read_permit::<Velocity>(RESOLVE)
            .map_err(map_runtime)?,
        resolve_ownership_read: runtime
            .read_permit::<Ownership>(RESOLVE)
            .map_err(map_runtime)?,
        ownership_read: runtime
            .read_permit::<Ownership>(PUBLISH)
            .map_err(map_runtime)?,
        visual_read: runtime
            .read_permit::<Visual>(PUBLISH)
            .map_err(map_runtime)?,
    };
    Ok((runtime, rules, candidate))
}

fn map_runtime(error: RuntimeError) -> SimError {
    match error {
        RuntimeError::World(RegisteredWorldError::Capacity) => SimError::Capacity,
        RuntimeError::World(RegisteredWorldError::DuplicateId) => SimError::DuplicateId,
        RuntimeError::World(RegisteredWorldError::InvalidHandle) => SimError::InvalidHandle,
        RuntimeError::World(RegisteredWorldError::SnapshotMismatch)
        | RuntimeError::WrongCapture
        | RuntimeError::NoCommittedTick => SimError::InvalidSnapshot,
        RuntimeError::World(_)
        | RuntimeError::Schedule(_)
        | RuntimeError::UndeclaredAccess
        | RuntimeError::RuntimeExhausted => SimError::InvalidConfig,
        RuntimeError::AlreadyStarted => SimError::AlreadyStarted,
    }
}

/// One V1 game world over the engine's generic checked authority.
pub struct Simulation {
    runtime: GameRuntime<Page, Rules>,
    rules: Rules,
    candidate: Candidate,
}

/// Game-owned resource state paired with the engine's opaque registered capture.
pub struct SimulationCapture {
    runtime: RuntimeCapture,
    peers: Vec<PeerControl>,
    resource_revision: u64,
}

impl SimulationCapture {
    /// Hash engine-owned identity, free-list and change-tick metadata for a
    /// complete-state replay comparison alongside game field values.
    pub fn hash_authority_metadata(&self, hash: &mut impl std::hash::Hasher) {
        use std::hash::Hash;

        self.runtime.hash_authority_metadata(hash);
        self.resource_revision.hash(hash);
        self.peers.len().hash(hash);
        for peer in &self.peers {
            peer.binding.peer.hash(hash);
            peer.binding.connection_epoch.hash(hash);
            peer.binding.actor.hash(hash);
            peer.handle.hash(hash);
            peer.active.hash(hash);
            peer.last_sequence.hash(hash);
            peer.axes.map(f32::to_bits).hash(hash);
            peer.recompute_velocity.hash(hash);
            if let Some(frame) = &peer.pending {
                true.hash(hash);
                frame.origin.hash(hash);
                frame.actor.hash(hash);
                frame.tick.hash(hash);
                frame.axes.map(f32::to_bits).hash(hash);
                frame.actions.len().hash(hash);
                for action in &frame.actions {
                    match action {
                        SimAction::Impulse(values) => {
                            0_u8.hash(hash);
                            values.map(f64::to_bits).hash(hash);
                        }
                        SimAction::Damage(value) => {
                            1_u8.hash(hash);
                            value.hash(hash);
                        }
                    }
                }
            } else {
                false.hash(hash);
            }
        }
    }

    /// Retained baseline component and peer-resource storage.
    pub fn reserved_bytes(&self) -> usize {
        self.runtime.reserved_bytes() + peer_storage_bytes(&self.peers)
    }
}

fn peer_storage_bytes(peers: &Vec<PeerControl>) -> usize {
    peers.capacity() * size_of::<PeerControl>()
        + peers
            .iter()
            .filter_map(|peer| peer.pending.as_ref())
            .filter(|frame| frame.actions.spilled())
            .map(|frame| frame.actions.capacity() * size_of::<SimAction>())
            .sum::<usize>()
}

impl Simulation {
    /// Capture one committed endpoint and all game resources needed for replay.
    pub fn capture(&self) -> SimulationCapture {
        SimulationCapture {
            runtime: self.runtime.capture(),
            peers: self.rules.peers.clone(),
            resource_revision: self.rules.resource_revision,
        }
    }

    /// Restore a compatible endpoint; the next tick runs the same game rules.
    /// Existing immutable page leases remain valid as historical endpoints.
    ///
    /// # Errors
    ///
    /// The capture belongs to another world epoch, capacity or schema.
    pub fn restore(&mut self, capture: &SimulationCapture) -> Result<(), SimError> {
        let epoch = self.runtime.epoch();
        let capacity = self.rules.config.entity_capacity;
        self.runtime
            .restore(&capture.runtime, || Page::new(epoch, capacity))
            .map_err(map_runtime)?;
        self.rules.peers.clone_from(&capture.peers);
        self.rules.resource_revision = capture.resource_revision;
        self.rules.commands.clear();
        self.rules.committed_inputs.clear();
        self.rules.changed_positions = 0;
        self.runtime
            .publish_initial(&mut self.rules, &mut self.candidate);
        Ok(())
    }

    /// Build the V1 game rules and registered component schema.
    ///
    /// # Errors
    ///
    /// Invalid capacity, movement speed or schedule declaration.
    pub fn new(epoch: WorldEpoch, config: SimConfig) -> Result<Self, SimError> {
        if config.entity_capacity > u32::MAX as usize
            || !config.movement_speed.is_finite()
            || config.movement_speed < 0.0
            || config.movement_speed > MAX_CONTROL_SPEED
        {
            return Err(SimError::InvalidConfig);
        }
        let (runtime, rules, candidate) = setup(epoch, config)?;
        Ok(Self {
            runtime,
            rules,
            candidate,
        })
    }

    /// Fixed V1 game tick rate; the engine clock is independently configurable.
    pub fn tick_hz(&self) -> std::num::NonZeroU32 {
        self.rules.config.tick_hz
    }
    /// Last complete world tick.
    pub fn tick(&self) -> u64 {
        self.runtime.tick()
    }
    /// Presentation branch generation of the latest committed endpoint.
    pub fn timeline(&self) -> u64 {
        self.runtime.timeline()
    }
    /// Select the engine executor. Preferred Rayon stays serial for this game
    /// until a measured crossover; forced Rayon exercises the resident pool.
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        let selected = if mode == ExecutionMode::Rayon {
            ExecutionMode::Serial
        } else {
            mode
        };
        self.runtime.set_execution_mode(selected);
    }
    /// Checked world generation.
    pub fn epoch(&self) -> WorldEpoch {
        self.runtime.epoch()
    }
    /// Live entity count.
    pub fn len(&self) -> usize {
        self.runtime.len()
    }
    /// Whether this world is empty.
    pub fn is_empty(&self) -> bool {
        self.runtime.is_empty()
    }
    /// Resolve stable identity to a checked physical handle.
    pub fn handle(&self, id: StableId) -> Option<Handle> {
        self.runtime.handle(id)
    }
    /// Whether the next tick can acquire an unleased page.
    pub fn can_publish_next(&self) -> bool {
        self.runtime.can_publish_next()
    }
    /// Whether the latest immutable page covers the current committed tick.
    pub fn has_current_page(&self) -> bool {
        self.runtime.has_current_publication()
    }
    /// Reserved game page and engine lease storage, including nested columns.
    pub fn publication_reserved_bytes(&self) -> usize {
        self.runtime
            .publication_reserved_bytes(Page::reserved_bytes)
    }

    /// Seed one game bundle before initial publication or input registration.
    ///
    /// # Errors
    ///
    /// Setup has started, the ID repeats, capacity is full or values are nonfinite.
    pub fn seed(&mut self, entity: ComponentEntity) -> Result<Handle, SimError> {
        if !self.rules.peers.is_empty() {
            return Err(SimError::AlreadyStarted);
        }
        if !finite(entity.position.0) || !finite(entity.velocity.0) {
            return Err(SimError::NonFinite);
        }
        let handle = self
            .runtime
            .seed(entity.id, |writer| {
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
            .map_err(map_runtime)?;
        Ok(handle)
    }

    /// Read committed V1 fields through a checked handle.
    ///
    /// # Errors
    ///
    /// The handle is stale or from another world epoch.
    pub fn get(&self, handle: Handle) -> Result<ComponentView, SimError> {
        let world = self.runtime.read();
        let id = world
            .stable_id(handle)
            .map_err(|_| SimError::InvalidHandle)?;
        let position = world
            .with(&self.rules.position_read, handle, |value| *value)
            .map_err(|_| SimError::InvalidHandle)?;
        let health = world
            .with(&self.rules.health_read, handle, |value| value.0)
            .map_err(|_| SimError::InvalidHandle)?;
        Ok(ComponentView {
            handle,
            id,
            position,
            health,
            velocity: world
                .with(&self.rules.velocity_read, handle, |value| *value)
                .unwrap_or_default(),
            ownership: world
                .with(&self.rules.ownership_read, handle, |value| *value)
                .ok(),
            visual: world
                .with(&self.rules.visual_read, handle, |value| value.0)
                .ok(),
        })
    }

    /// Lease the latest complete V1 game page, sealing tick zero if needed.
    ///
    /// # Errors
    ///
    /// The engine page ring is unexpectedly unavailable.
    pub fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        self.runtime
            .publish_initial(&mut self.rules, &mut self.candidate);
        if !self
            .runtime
            .publish_current_if_available(&mut self.rules, &mut self.candidate)
        {
            return Err(SimError::PublicationBusy);
        }
        self.runtime
            .latest()
            .map(PublishedPage)
            .ok_or(SimError::PublicationBusy)
    }

    /// Register one controlled actor after validating its ownership component.
    ///
    /// # Errors
    ///
    /// Binding, epoch, actor, handle or peer capacity is invalid.
    pub fn register_peer(&mut self, binding: Ownership, handle: Handle) -> Result<(), SimError> {
        let next_revision = self
            .rules
            .resource_revision
            .checked_add(1)
            .ok_or(SimError::TickExhausted)?;
        let owned = self
            .runtime
            .read()
            .with(&self.rules.ownership_read, handle, |value| *value)
            .map_err(|_| SimError::InvalidHandle)?;
        if owned != binding
            || self
                .rules
                .peers
                .iter()
                .any(|peer| peer.active && peer.handle == handle)
        {
            return Err(SimError::InvalidOwnership);
        }
        if let Some(peer) = self
            .rules
            .peers
            .iter_mut()
            .find(|peer| peer.binding.peer == binding.peer)
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
                recompute_velocity: true,
            };
            self.rules.resource_revision = next_revision;
            return Ok(());
        }
        if self.rules.peers.len() == self.rules.config.peer_capacity {
            return Err(SimError::Capacity);
        }
        self.rules.peers.push(PeerControl {
            binding,
            handle,
            active: true,
            last_sequence: None,
            axes: [0.0; 3],
            pending: None,
            recompute_velocity: true,
        });
        self.rules
            .peers
            .sort_unstable_by_key(|peer| peer.binding.peer);
        self.rules.resource_revision = next_revision;
        Ok(())
    }

    /// Check V1 action count and numeric values before reserving input timing.
    ///
    /// # Errors
    ///
    /// The action count or payload fails V1 admission.
    pub fn validate_frame_actions(&self, frame: &InputFrame<SimAction>) -> Result<(), FrameError> {
        validate_frame_actions(&self.rules.config, frame)
    }

    /// Admit exactly one validated frame for a peer's next tick.
    ///
    /// # Errors
    ///
    /// Input identity, timing, sequence, axes, actions or control binding fails.
    pub fn admit_frame(&mut self, frame: InputFrame<SimAction>) -> Result<(), FrameError> {
        validate_frame(&self.runtime, &self.rules, &frame)?;
        let peer = self
            .rules
            .peers
            .iter_mut()
            .find(|peer| peer.binding.peer == frame.origin.peer)
            .ok_or(FrameError::UnknownPeer)?;
        peer.last_sequence = Some(frame.origin.sequence);
        peer.pending = Some(frame);
        Ok(())
    }

    /// Run and publish one complete fixed V1 game tick.
    ///
    /// # Errors
    ///
    /// Invalid commands, candidate numbers or tick overflow. Optional page
    /// pressure never rejects an authoritative tick.
    ///
    /// # Panics
    ///
    /// Panics only if a game callback violates its validated commit invariant.
    pub fn step(&mut self, commands: &[Command]) -> Result<TickOutcome<'_>, SimError> {
        validate_commands(&self.runtime, commands)?;
        self.runtime
            .publish_initial(&mut self.rules, &mut self.candidate);
        let simulated_entities = self.runtime.len();
        self.rules.commands.clear();
        self.rules.commands.extend_from_slice(commands);
        let outcome = self.runtime.step(&mut self.rules, &mut self.candidate);
        let tick = outcome.map_err(|error| match error {
            TickError::TickExhausted | TickError::RevisionExhausted => SimError::TickExhausted,
            TickError::BaselineRequired => SimError::PublicationBusy,
            TickError::Game(error) => error,
            TickError::Edit(_) => SimError::InvalidEdit,
        })?;
        Ok(TickOutcome {
            tick,
            simulated_entities,
            changed_positions: self.rules.changed_positions,
            committed_inputs: &self.rules.committed_inputs,
        })
    }
}

fn validate_frame_actions(
    config: &SimConfig,
    frame: &InputFrame<SimAction>,
) -> Result<(), FrameError> {
    if frame.actions.len() > config.actions_per_frame {
        return Err(FrameError::TooManyActions);
    }
    for (axis, &held) in frame.axes.iter().enumerate() {
        let mut speed = f64::from(held) * config.movement_speed;
        for action in &frame.actions {
            if let SimAction::Impulse(values) = action {
                let impulse = values[axis];
                if !impulse.is_finite() || impulse.abs() > MAX_CONTROL_SPEED {
                    return Err(FrameError::InvalidAction);
                }
                speed += impulse;
                if !speed.is_finite() || speed.abs() > MAX_CONTROL_SPEED {
                    return Err(FrameError::InvalidAction);
                }
            }
        }
    }
    Ok(())
}

fn validate_frame(
    runtime: &GameRuntime<Page, Rules>,
    rules: &Rules,
    frame: &InputFrame<SimAction>,
) -> Result<(), FrameError> {
    frame.validate_axes()?;
    validate_frame_actions(&rules.config, frame)?;
    let peer = rules
        .peers
        .iter()
        .find(|peer| peer.binding.peer == frame.origin.peer)
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
    if runtime.tick().checked_add(1) != Some(frame.tick) {
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
    if runtime
        .read()
        .with(&rules.ownership_read, peer.handle, |value| *value)
        .ok()
        != Some(peer.binding)
    {
        return Err(FrameError::InvalidControl);
    }
    Ok(())
}

fn validate_commands(
    runtime: &GameRuntime<Page, Rules>,
    commands: &[Command],
) -> Result<(), SimError> {
    let mut free = runtime.free_count();
    let mut spawns = 0;
    for (index, command) in commands.iter().enumerate() {
        match command {
            Command::Spawn(entity) => {
                if !finite(entity.position.0) || !finite(entity.velocity.0) {
                    return Err(SimError::NonFinite);
                }
                if runtime.handle(entity.id).is_some()
                    || commands[..index].iter().any(
                        |prior| matches!(prior, Command::Spawn(other) if other.id == entity.id),
                    )
                {
                    return Err(SimError::DuplicateId);
                }
                spawns += 1;
            }
            Command::Despawn(handle) => {
                runtime
                    .read()
                    .stable_id(*handle)
                    .map_err(|_| SimError::InvalidHandle)?;
                free += usize::from(
                    runtime
                        .reusable_after_despawn(*handle)
                        .map_err(|_| SimError::InvalidHandle)?,
                );
            }
            Command::SetPosition(handle, value) => {
                runtime
                    .read()
                    .stable_id(*handle)
                    .map_err(|_| SimError::InvalidHandle)?;
                if !finite(value.0) {
                    return Err(SimError::NonFinite);
                }
            }
            Command::SetVelocity(handle, value) => {
                runtime
                    .read()
                    .stable_id(*handle)
                    .map_err(|_| SimError::InvalidHandle)?;
                if !finite(value.0) {
                    return Err(SimError::NonFinite);
                }
            }
            Command::SetHealth(handle, _) => {
                runtime
                    .read()
                    .stable_id(*handle)
                    .map_err(|_| SimError::InvalidHandle)?;
            }
        }
        if commands[..index]
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
