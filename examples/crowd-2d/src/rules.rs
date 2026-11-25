use std::{any::Any, num::NonZeroUsize};
use thrum_core::{
    ActorId, Handle, Origin, PeerId, StableId, WorldEpoch,
    profile::{
        Digest, Number as N, StateDecode, StateDecodeError, StateEncode, StateReader, Vector,
        random,
    },
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, ComponentSchemaId, Edit, EditWriter,
        EntityWriter, Executor, GameRuntime, HistoryGame, InputFrame, ReadPermit, RegisteredSystem,
        RegisteredWorldError, RuntimeHistory, Stage, StructuralPermit, SystemId, SystemSpec,
        TickGame, WorldRead, WritePermit,
    },
};
use thrum_sim2d::{Circle, CircleIndex, Shape};

/// Complete body authority. Kinds are player=0, enemy=1 and projectile=2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Planar position.
    pub position: Vector,
    /// Per-tick displacement.
    pub velocity: Vector,
    /// Collision radius.
    pub radius: N,
    /// Role in the reference rules.
    pub kind: u32,
    /// Player index or projectile owner.
    pub player: u32,
    /// Remaining health for enemies, lifetime for projectiles.
    pub health: i32,
}
impl StateEncode for Body {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.position.encode(digest);
        self.velocity.encode(digest);
        self.radius.encode(digest);
        self.kind.encode(digest);
        self.player.encode(digest);
        self.health.encode(digest);
    }
    fn is_finite(&self) -> bool {
        self.position.is_finite() && self.velocity.is_finite() && self.radius.is_finite()
    }
}
impl StateDecode for Body {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        let body = Self {
            position: Vector::decode(reader)?,
            velocity: Vector::decode(reader)?,
            radius: N::decode(reader)?,
            kind: u32::decode(reader)?,
            player: u32::decode(reader)?,
            health: i32::decode(reader)?,
        };
        let arena = N::from_i32(120);
        let speed = N::from_i32(8);
        if body.kind > 2
            || body.player >= 4
            || body.radius <= N::ZERO
            || body.radius > N::from_i32(6)
            || body.position.x.abs() > arena
            || body.position.y.abs() > arena
            || body.velocity.x.abs() > speed
            || body.velocity.y.abs() > speed
            || body.health <= 0
        {
            return Err(StateDecodeError::Invalid);
        }
        Ok(body)
    }
}

/// The portion of the player that moves in response to timed controls.
#[derive(Clone, Copy, Debug)]
pub struct PlayerController {
    /// Planar position.
    pub position: Vector,
    /// Per-tick displacement.
    pub velocity: Vector,
}

/// Immutable constants of the reference crowd's player movement.
#[derive(Clone, Copy)]
pub struct PlayerMovement {
    divisor: N,
    low: N,
    high: N,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            divisor: N::from_i32(12),
            low: N::from_i32(-120),
            high: N::from_i32(120),
        }
    }
}

impl PlayerMovement {
    /// Advance the player from its prior state and this frame's canonical controls.
    pub fn step(self, state: PlayerController, controls: [N; 2]) -> PlayerController {
        let velocity = Vector {
            x: controls[0] / self.divisor,
            y: controls[1] / self.divisor,
        };
        PlayerController {
            position: Vector {
                x: (state.position.x + velocity.x).clamp(self.low, self.high),
                y: (state.position.y + velocity.y).clamp(self.low, self.high),
            },
            velocity,
        }
    }
}

impl Body {
    /// Extract the self-contained player movement state.
    pub fn controller(&self) -> PlayerController {
        PlayerController {
            position: self.position,
            velocity: self.velocity,
        }
    }
}

/// Authority resources restored with world history.
#[derive(Clone, Default)]
pub struct Resources {
    /// Held controls, already converted by the engine.
    pub controls: [[N; 2]; 4],
    /// Last accepted sequence per peer.
    pub sequences: [u64; 4],
    /// Next stable identity, never reused.
    pub next_id: u64,
    /// Successful projectile hits.
    pub hits: u64,
}
impl StateEncode for Resources {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.controls.encode(digest);
        self.sequences.encode(digest);
        self.next_id.encode(digest);
        self.hits.encode(digest);
    }
    fn is_finite(&self) -> bool {
        self.controls.is_finite()
            && self.sequences.is_finite()
            && self.next_id.is_finite()
            && self.hits.is_finite()
    }
}
impl StateDecode for Resources {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        Ok(Self {
            controls: <[[N; 2]; 4]>::decode(reader)?,
            sequences: <[u64; 4]>::decode(reader)?,
            next_id: u64::decode(reader)?,
            hits: u64::decode(reader)?,
        })
    }
}

/// Reference rule set and registered capabilities.
pub struct Crowd {
    mixed: bool,
    /// Diagnostic injection used solely by the platform-sine negative control.
    pub control: Option<fn(N) -> N>,
    read: ReadPermit<Body>,
    write: WritePermit<Body>,
    structure: StructuralPermit,
    /// Committed resources.
    pub resources: Resources,
    frames: [InputFrame<()>; 4],
}

/// Reusable derived state and candidate edits.
pub struct Candidate {
    steering: Vec<Vec<(u32, Body, u64)>>,
    /// Native forced-parallel steering chunks in the last step.
    pub parallel_chunks: u64,
    /// Oversize circles indexed in the last step.
    pub oversize: u64,
    /// Fast capsule hits whose endpoint alone misses the same target.
    pub swept_only_hits: u64,
    rows: Vec<(Handle, StableId, Body)>,
    circles: Vec<Circle>,
    index: CircleIndex,
    batch: Vec<(Handle, Body)>,
    births: Vec<Body>,
    dead: Vec<u32>,
    pub(super) damage: Vec<u32>,
    nearest: Vec<(N, u32)>,
    resources: Resources,
    /// Neighbours actually visited during this tick's separation queries.
    pub neighbours: u64,
}

fn vector(x: i32, y: i32) -> Vector {
    Vector {
        x: N::from_i32(x),
        y: N::from_i32(y),
    }
}
fn length(value: Vector) -> N {
    (value.x * value.x + value.y * value.y).sqrt()
}
fn direction(from: Vector, to: Vector) -> Vector {
    let delta = Vector {
        x: to.x - from.x,
        y: to.y - from.y,
    };
    let scale = length(delta).max(N::ONE / N::from_i32(1024));
    Vector {
        x: delta.x / scale,
        y: delta.y / scale,
    }
}
fn enemy(tick: u64, id: u64) -> Body {
    let x = random(42, tick, u128::from(id), 0) * N::from_i32(2) - N::ONE;
    let y = random(42, tick, u128::from(id), 1) * N::from_i32(2) - N::ONE;
    let normal = direction(vector(0, 0), Vector { x, y });
    Body {
        position: Vector {
            x: normal.x * N::from_i32(80),
            y: normal.y * N::from_i32(80),
        },
        velocity: vector(0, 0),
        radius: N::ONE / N::from_i32(3),
        kind: 1,
        player: 0,
        health: 1,
    }
}
fn batch(candidate: &Candidate) -> &dyn Any {
    &candidate.batch
}

fn steer(
    slot: u32,
    body: &mut Body,
    neighbours: &mut u64,
    players: [Vector; 4],
    index: &CircleIndex,
    control: Option<fn(N) -> N>,
) {
    let mut target = players[0];
    let dx = target.x - body.position.x;
    let dy = target.y - body.position.y;
    let mut distance = dx * dx + dy * dy;
    for player in players {
        let dx = player.x - body.position.x;
        let dy = player.y - body.position.y;
        let squared = dx * dx + dy * dy;
        if squared < distance {
            target = player;
            distance = squared;
        }
    }
    let mut chase = direction(body.position, target);
    if let Some(control) = control {
        chase.x = control(chase.x);
    }
    let mut push = vector(0, 0);
    index
        .overlap(body.position, N::from_i32(2), |circle| {
            if circle.slot == slot {
                return;
            }
            *neighbours += 1;
            let dx = body.position.x - circle.centre.x;
            let dy = body.position.y - circle.centre.y;
            let squared = dx * dx + dy * dy;
            if squared > N::ZERO && squared < N::ONE {
                let scale = (N::ONE - squared) / squared.max(N::ONE / N::from_i32(64));
                push.x = push.x + dx * scale;
                push.y = push.y + dy * scale;
            }
        })
        .expect("enemy in domain");
    body.velocity = Vector {
        x: (chase.x / N::from_i32(128) + push.x / N::from_i32(32)).clamp(-N::ONE, N::ONE),
        y: (chase.y / N::from_i32(128) + push.y / N::from_i32(32)).clamp(-N::ONE, N::ONE),
    };
}
fn birth(
    candidate: &Candidate,
    index: usize,
    writer: &mut EntityWriter<'_>,
) -> Result<(), RegisteredWorldError> {
    writer.add(candidate.births[index])?;
    Ok(())
}

impl Crowd {
    fn simulate(
        &self,
        world: &WorldRead<'_>,
        executor: Executor,
        tick: u64,
        candidate: &mut Candidate,
        edits: &mut EditWriter<'_, Candidate>,
    ) -> Result<(), RegisteredWorldError> {
        candidate.rebuild(world, &self.read)?;
        let column = world.column(&self.read)?;
        let mut players = [vector(0, 0); 4];
        for &(_, _, body) in &candidate.rows {
            if body.kind == 0 {
                players[body.player as usize] = body.position;
            }
        }
        for frame in &self.frames {
            let player = usize::try_from(frame.actor.0).expect("four players");
            if frame.tick != tick || frame.origin.sequence <= self.resources.sequences[player] {
                return Err(RegisteredWorldError::InvalidHandle);
            }
            candidate.resources.controls[player] =
                [N::from_axis(frame.axes[0]), N::from_axis(frame.axes[1])];
            candidate.resources.sequences[player] = frame.origin.sequence;
        }
        for &(handle, _, body) in &candidate.rows {
            if body.kind != 2 {
                continue;
            }
            let endpoint = Vector {
                x: body.position.x + body.velocity.x,
                y: body.position.y + body.velocity.y,
            };
            let mut hit = None;
            let shape = if self.mixed {
                Shape::Capsule {
                    start: body.position,
                    end: endpoint,
                    radius: body.radius,
                }
            } else {
                Shape::Circle {
                    centre: endpoint,
                    radius: body.radius,
                }
            };
            candidate
                .index
                .query(shape, |circle| {
                    if hit.is_none() {
                        hit = Some(circle.slot);
                    }
                })
                .expect("projectile in domain");
            if let Some(slot) = hit {
                if self.mixed {
                    let target = column.at_slot(slot).expect("indexed enemy");
                    let endpoint_shape = Shape::Circle {
                        centre: endpoint,
                        radius: body.radius,
                    };
                    let circle = Circle {
                        slot,
                        centre: target.position,
                        radius: target.radius,
                    };
                    let diameter = target.radius + target.radius;
                    if !endpoint_shape.intersects(&circle)
                        && body.velocity.x * body.velocity.x + body.velocity.y * body.velocity.y
                            > diameter * diameter
                    {
                        candidate.swept_only_hits += 1;
                    }
                }
                candidate.damage[slot as usize] += 1;
                candidate.resources.hits += 1;
                candidate.dead.push(handle.slot);
            } else if body.health <= 1 {
                candidate.dead.push(handle.slot);
            }
        }
        for chunk in &mut candidate.steering {
            chunk.clear();
        }
        for (row, &(handle, _, body)) in candidate.rows.iter().enumerate() {
            candidate.steering[row / 256].push((handle.slot, body, 0));
        }
        let index = &candidate.index;
        let control = self.control;
        executor.for_each_mut(&mut candidate.steering, |chunk| {
            for (slot, body, neighbours) in chunk {
                if body.kind == 1 {
                    steer(*slot, body, neighbours, players, index, control);
                }
            }
        });
        if executor.mode() == thrum_core::sim::ExecutionMode::RayonForced
            && !cfg!(target_arch = "wasm32")
        {
            candidate.parallel_chunks = candidate
                .steering
                .iter()
                .filter(|chunk| !chunk.is_empty())
                .count() as u64;
        }
        for (row, &(handle, _, mut body)) in candidate.rows.iter().enumerate() {
            if body.kind == 1 && candidate.damage[handle.slot as usize] > 0 {
                body.health -= i32::try_from(candidate.damage[handle.slot as usize])
                    .expect("bounded projectiles");
                if body.health <= 0 {
                    candidate.dead.push(handle.slot);
                    candidate.births.push(enemy(
                        tick,
                        candidate.resources.next_id + candidate.births.len() as u64,
                    ));
                }
            }
            if candidate.dead.contains(&handle.slot) {
                edits.push(Edit::despawn(&self.structure, handle));
                continue;
            }
            match body.kind {
                0 => {
                    let controls = candidate.resources.controls[body.player as usize];
                    if tick % 24 == u64::from(body.player) * 6 {
                        candidate
                            .index
                            .nearest(body.position, N::from_i32(12), 1, &mut candidate.nearest)
                            .expect("player in domain");
                        if let Some(&(_, slot)) = candidate.nearest.first() {
                            let target = column.at_slot(slot).expect("indexed enemy");
                            let normal = direction(body.position, target.position);
                            candidate.births.push(Body {
                                position: body.position,
                                velocity: Vector {
                                    x: if self.mixed {
                                        normal.x * N::from_i32(4)
                                    } else {
                                        normal.x / N::from_i32(2)
                                    },
                                    y: if self.mixed {
                                        normal.y * N::from_i32(4)
                                    } else {
                                        normal.y / N::from_i32(2)
                                    },
                                },
                                radius: N::ONE / N::from_i32(6),
                                kind: 2,
                                player: body.player,
                                health: 32,
                            });
                        }
                    }
                    let controller = PlayerMovement::default().step(body.controller(), controls);
                    body.position = controller.position;
                    body.velocity = controller.velocity;
                }
                1 => {
                    let (_, steered, visited) = candidate.steering[row / 256][row % 256];
                    body.velocity = steered.velocity;
                    candidate.neighbours += visited;
                }
                _ => {
                    body.health -= 1;
                }
            }
            if body.kind != 0 {
                body.position.x =
                    (body.position.x + body.velocity.x).clamp(N::from_i32(-120), N::from_i32(120));
                body.position.y =
                    (body.position.y + body.velocity.y).clamp(N::from_i32(-120), N::from_i32(120));
            }
            candidate.batch.push((handle, body));
        }
        edits.push(Edit::replace_batch(&self.write, batch));
        for index in 0..candidate.births.len() {
            let id = candidate.resources.next_id;
            candidate.resources.next_id += 1;
            edits.push(Edit::spawn(
                &self.structure,
                StableId(u128::from(id)),
                index,
                birth,
            ));
        }
        Ok(())
    }
}

impl TickGame for Crowd {
    type Error = RegisteredWorldError;
    type Candidate = Candidate;
    type ParallelOutput = ();
    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.rows.clear();
        candidate.circles.clear();
        candidate.batch.clear();
        candidate.births.clear();
        candidate.dead.clear();
        candidate.damage.fill(0);
        candidate.resources.clone_from(&self.resources);
        candidate.neighbours = 0;
        candidate.parallel_chunks = 0;
        candidate.swept_only_hits = 0;
    }
    fn reset_parallel_output(&self, _output: &mut ()) {}
    fn profile_resources<'a>(&self, candidate: &'a Candidate) -> Option<&'a dyn StateDecode> {
        Some(&candidate.resources)
    }
    fn merge_parallel(
        &self,
        _system: SystemId,
        _output: &mut (),
        _candidate: &mut Candidate,
        _tick: u64,
        _edits: &mut EditWriter<'_, Candidate>,
    ) {
    }
    fn apply(&mut self, _world: &WorldRead<'_>, _tick: u64, candidate: &Candidate) {
        self.resources.clone_from(&candidate.resources);
    }
}

impl HistoryGame for Crowd {
    type HistoryState = Resources;
    type HistoryRecord = [InputFrame<()>; 4];
    type TickRequest = [InputFrame<()>; 4];
    fn history_revision(&self) -> u64 {
        0
    }
    fn history_state(&self) -> &Resources {
        &self.resources
    }
    fn restore_history_state(&mut self, state: &Resources) {
        self.resources.clone_from(state);
    }
    fn new_history_record(&self) -> Self::HistoryRecord {
        self.frames.clone()
    }
    fn record_history_tick(&self, request: &Self::TickRequest, record: &mut Self::HistoryRecord) {
        record.clone_from(request);
    }
    fn prepare_history_tick(
        &mut self,
        _runtime: &GameRuntime<Self>,
        tick: u64,
        record: &Self::HistoryRecord,
    ) -> Result<(), Self::Error> {
        for (index, frame) in record.iter().enumerate() {
            if frame.tick != tick
                || frame.actor.0 != index as u64
                || frame.origin.peer.0 != index as u64
                || frame.origin.connection_epoch != 1
                || frame.origin.sequence <= self.resources.sequences[index]
                || frame.validate_axes().is_err()
            {
                return Err(RegisteredWorldError::InvalidHandle);
            }
        }
        self.frames.clone_from(record);
        Ok(())
    }
}

/// Registered runtime, rule resources and rollback retention for one crowd.
pub struct Simulation {
    /// Engine authority.
    pub runtime: GameRuntime<Crowd>,
    /// Game rules and resources.
    pub game: Crowd,
    /// Reusable candidate work.
    pub candidate: Candidate,
    /// Eight-tick sparse history.
    pub history: RuntimeHistory<Resources, [InputFrame<()>; 4]>,
}
impl Simulation {
    /// Rebuild X[n]'s derived index through the same path the registered rule uses.
    ///
    /// # Panics
    /// If the reference runtime's registered read capability is invalid.
    pub fn rebuild_index(&mut self) {
        self.candidate
            .rebuild(&self.runtime.read(), &self.game.read)
            .expect("registered index read");
    }
    /// Construct and publish a deterministic crowd.
    ///
    /// # Panics
    /// If count is outside the reference workload's supported 4..=10,000 bodies.
    pub fn new(count: u32, epoch: u128) -> Self {
        Self::with_trace(count, epoch, false, false)
    }

    /// Construct the Mixed trace: oversize enemies and swept fast projectiles.
    ///
    /// # Panics
    /// If the reference workload is outside 4..=10,000 bodies.
    pub fn mixed(count: u32, epoch: u128) -> Self {
        Self::with_trace(count, epoch, true, false)
    }

    /// Construct the Crowd trace with current and previous body components in each tick page.
    ///
    /// # Panics
    /// If count is outside the reference workload's supported 4..=10,000 bodies.
    pub fn for_presentation(count: u32, epoch: u128) -> Self {
        Self::with_trace(count, epoch, false, true)
    }

    fn with_trace(count: u32, epoch: u128, mixed: bool, publish_bodies: bool) -> Self {
        let (mut runtime, mut game, mut candidate) =
            Self::runtime_parts(count, epoch, mixed, publish_bodies);
        Self::seed(&mut runtime, count, mixed);
        runtime.publish_initial(&mut game, &mut candidate);
        let history = RuntimeHistory::new(&runtime, &game, NonZeroUsize::new(8).expect("window"));
        Self {
            runtime,
            game,
            candidate,
            history,
        }
    }

    /// Prepare the runtime, rule state and scratch for an incoming image.
    ///
    /// # Panics
    /// If the body count is outside the reference workload's supported range.
    pub fn image_parts(
        count: u32,
        epoch: u128,
        mixed: bool,
    ) -> (GameRuntime<Crowd>, Crowd, Candidate) {
        Self::runtime_parts(count, epoch, mixed, false)
    }

    fn runtime_parts(
        count: u32,
        epoch: u128,
        mixed: bool,
        publish_bodies: bool,
    ) -> (GameRuntime<Crowd>, Crowd, Candidate) {
        assert!((4..=10_000).contains(&count), "reference crowd count");
        let capacity = count as usize + 128;
        let mut registry = ComponentRegistry::default();
        let body = registry
            .register_profile::<Body>(ComponentSchemaId(1))
            .expect("body schema");
        if publish_bodies {
            registry
                .select_publication::<Body>(true)
                .expect("registered body");
        }
        let system = SystemId(1);
        let runtime = GameRuntime::new_published(
            WorldEpoch(epoch),
            capacity,
            registry,
            vec![RegisteredSystem::new(
                SystemSpec {
                    id: system,
                    stage: Stage::Simulate,
                    accesses: vec![
                        Access {
                            key: AccessKey::Component(body),
                            mode: AccessMode::Write,
                        },
                        Access {
                            key: AccessKey::External(1),
                            mode: AccessMode::Write,
                        },
                    ],
                    after: vec![],
                },
                Crowd::simulate,
            )],
        )
        .expect("crowd runtime");
        let read = runtime.read_permit(system).expect("body read");
        let write = runtime.write_permit(system).expect("body write");
        let structure = runtime
            .structural_permit(system, 1)
            .expect("structural permit");
        let game = Crowd {
            mixed,
            control: None,
            read,
            write,
            structure,
            resources: Resources {
                next_id: u64::from(count) + 1,
                ..Resources::default()
            },
            frames: std::array::from_fn(|index| {
                InputFrame::new(
                    Origin {
                        peer: PeerId(index as u64),
                        connection_epoch: 1,
                        sequence: 0,
                    },
                    ActorId(index as u64),
                    0,
                    [0.0; 3],
                )
            }),
        };
        let candidate = Candidate {
            steering: (0..capacity.div_ceil(256))
                .map(|_| Vec::with_capacity(256))
                .collect(),
            parallel_chunks: 0,
            oversize: 0,
            swept_only_hits: 0,
            rows: Vec::with_capacity(capacity),
            circles: Vec::with_capacity(capacity),
            index: CircleIndex::new(N::from_i32(4)).expect("cell size"),
            batch: Vec::with_capacity(capacity),
            births: Vec::with_capacity(128),
            dead: Vec::with_capacity(128),
            damage: vec![0; capacity],
            nearest: Vec::with_capacity(1),
            resources: Resources::default(),
            neighbours: 0,
        };
        (runtime, game, candidate)
    }

    fn seed(runtime: &mut GameRuntime<Crowd>, count: u32, mixed: bool) {
        let width = N::from_i32(i32::try_from(count).expect("body count fits"))
            .sqrt()
            .ceil()
            .to_i32();
        for index in 0..count {
            let index_i32 = i32::try_from(index).expect("count fits");
            let mut body = enemy(0, u64::from(index));
            body.position = Vector {
                x: N::from_i32(index_i32 % width - width / 2) * N::from_i32(3) / N::from_i32(2),
                y: N::from_i32(index_i32 / width - width / 2) * N::from_i32(3) / N::from_i32(2),
            };
            if index < 4 {
                body.kind = 0;
                body.player = index;
                body.position = vector(index_i32 * 3 - 4, 0);
            } else if mixed && index % 997 == 0 {
                body.radius = N::from_i32(6);
            }
            runtime
                .seed(StableId(u128::from(index) + 1), |writer| {
                    writer.add(body)?;
                    Ok(())
                })
                .expect("seed body");
        }
    }
    /// Digest the current endpoint.
    ///
    /// # Panics
    /// If the reference body's portable schema has been replaced.
    pub fn digest(&self) -> u64 {
        self.runtime
            .world_digest(&self.game.resources, |_, _| {})
            .expect("portable schema")
    }
}

impl Candidate {
    fn rebuild(
        &mut self,
        world: &WorldRead<'_>,
        permit: &ReadPermit<Body>,
    ) -> Result<(), RegisteredWorldError> {
        self.rows.clear();
        self.circles.clear();
        let column = world.column(permit)?;
        for slot in 0..self.damage.len() {
            let slot = u32::try_from(slot).expect("slot fits");
            if let Some(live) = world.live_at_slot(slot)
                && let Some(body) = column.at_slot(slot)
            {
                self.rows.push((live.handle(), live.stable_id(), *body));
                if body.kind == 1 {
                    self.circles.push(Circle {
                        slot,
                        centre: body.position,
                        radius: body.radius,
                    });
                }
            }
        }
        self.index
            .build(&self.circles)
            .expect("validated reference circles");
        self.oversize = self
            .circles
            .iter()
            .filter(|circle| circle.radius > N::from_i32(4))
            .count() as u64;
        Ok(())
    }
}
