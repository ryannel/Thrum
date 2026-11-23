//! Shared integer-rule authority and real in-memory transport fixture.
#![expect(
    missing_docs,
    reason = "Only the session integration tests import this fixture"
)]

use thrum_core::{
    WorldEpoch,
    profile::{Digest, StateBytes, StateDecode, StateDecodeError, StateEncode, StateReader},
    sim::{
        Access, AccessKey, AccessMode, ComponentRegistry, EditWriter, Executor, GameRuntime,
        InputFrame, RegisteredSystem, RegisteredWorldError, RuleSetIdentity, Stage, SystemId,
        SystemSpec, TickError, TickGame, WorldRead,
    },
};
use thrum_network::{
    input_batch::{ActionView, ActionWire, InputCodecError},
    session::{
        Client, ClientState, Controls, Host, PlayerToken, RosterEvents, SLOTS, Settings, World,
    },
};
use thrum_transport::{
    Link, Profile,
    memory::{Conditioner, Memory},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Add,
    Subtract,
}

#[derive(Default)]
struct Resources {
    signature: u64,
    padding: u16,
    heroes: [u64; SLOTS],
    positions: [i64; SLOTS],
}

impl StateEncode for Resources {
    fn encode(&self, sink: &mut Digest<'_>) {
        self.signature.encode(sink);
        self.padding.encode(sink);
        self.heroes.encode(sink);
        self.positions.encode(sink);
        for _ in 0..self.padding {
            sink.bytes(&[0; 1_024]);
        }
    }
}

impl StateDecode for Resources {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        let signature = u64::decode(reader)?;
        let padding = u16::decode(reader)?;
        let heroes = <[u64; SLOTS]>::decode(reader)?;
        let positions = <[i64; SLOTS]>::decode(reader)?;
        if padding > 192 {
            return Err(StateDecodeError::Invalid);
        }
        for _ in 0..padding {
            if reader.read::<1_024>()? != [0; 1_024] {
                return Err(StateDecodeError::Invalid);
            }
        }
        Ok(Self {
            signature,
            padding,
            heroes,
            positions,
        })
    }
}

#[derive(Default)]
struct Candidate {
    resources: Resources,
    actions: Vec<(u64, usize, u16)>,
    events: RosterEvents,
}

struct Rules {
    resources: Resources,
    rule: Rule,
    fail_at: Option<u64>,
    pending: [Option<(usize, u16)>; SLOTS * 8],
    log: Vec<(u64, usize, u16)>,
    roster: RosterEvents,
    motion: [i64; SLOTS],
    roster_log: Vec<RosterRow>,
}

impl TickGame for Rules {
    type Error = RegisteredWorldError;
    type Candidate = Candidate;
    type ParallelOutput = ();

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.resources.signature = self.resources.signature;
        candidate.resources.padding = self.resources.padding;
        candidate.resources.heroes = self.resources.heroes;
        candidate.resources.positions = self.resources.positions;
        candidate.events = self.roster;
        candidate.actions.clear();
    }

    fn profile_resources<'a>(&self, candidate: &'a Candidate) -> Option<&'a dyn StateDecode> {
        Some(&candidate.resources)
    }

    fn reset_parallel_output(&self, (): &mut ()) {}

    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        _: &mut Candidate,
        _: u64,
        _: &mut EditWriter<'_, Candidate>,
    ) {
    }

    fn apply(&mut self, _: &WorldRead<'_>, tick: u64, candidate: &Candidate) {
        self.resources.signature = candidate.resources.signature;
        self.log.extend(&candidate.actions);
        self.resources.heroes = candidate.resources.heroes;
        self.resources.positions = candidate.resources.positions;
        self.roster_log.push(RosterRow {
            tick,
            events: candidate.events,
            heroes: self.resources.heroes,
            positions: self.resources.positions,
        });
    }
}

fn play(
    rules: &Rules,
    _: &WorldRead<'_>,
    _: Executor,
    tick: u64,
    candidate: &mut Candidate,
    _: &mut EditWriter<'_, Candidate>,
) -> Result<(), RegisteredWorldError> {
    if rules.fail_at == Some(tick) {
        return Err(RegisteredWorldError::InvalidHandle);
    }
    for slot in 0..SLOTS {
        let bit = 1 << slot;
        if candidate.events.joined & bit != 0 {
            candidate.resources.heroes[slot] = tick;
            candidate.resources.positions[slot] = 0;
        }
        if candidate.events.left & bit != 0 {
            candidate.resources.heroes[slot] = 0;
            candidate.resources.positions[slot] = 0;
        }
        if candidate.resources.heroes[slot] != 0 {
            candidate.resources.positions[slot] += rules.motion[slot];
        }
    }
    for &(slot, action) in rules.pending.iter().flatten() {
        let value = tick * 61_007 + (slot as u64) * 65_536 + u64::from(action);
        let signature = candidate
            .resources
            .signature
            .wrapping_mul(1_099_511_628_211);
        candidate.resources.signature = match rules.rule {
            Rule::Add => signature.wrapping_add(value),
            Rule::Subtract => signature.wrapping_sub(value),
        };
        candidate.actions.push((tick, slot, action));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RosterRow {
    pub tick: u64,
    pub events: RosterEvents,
    pub heroes: [u64; SLOTS],
    pub positions: [i64; SLOTS],
}

pub struct Game {
    rules: Rules,
    runtime: GameRuntime<Rules>,
    candidate: Candidate,
    epoch: u128,
}

#[derive(Debug)]
pub enum Failure {
    Tick(TickError<RegisteredWorldError>),
    Image,
}

impl Game {
    /// Construct the integer-state world with the requested rule.
    ///
    /// # Panics
    /// The fixed test schedule is invalid.
    pub fn new(
        epoch: u128,
        rule: Rule,
        fail_at: Option<u64>,
        padding: u16,
        published: bool,
    ) -> Self {
        let mut game = Self {
            rules: Rules {
                resources: Resources {
                    signature: 17,
                    padding,
                    ..Resources::default()
                },
                rule,
                fail_at,
                pending: [None; SLOTS * 8],
                log: Vec::new(),
                roster: RosterEvents::default(),
                motion: [0; SLOTS],
                roster_log: Vec::new(),
            },
            runtime: GameRuntime::new_published(
                WorldEpoch(epoch),
                1,
                ComponentRegistry::default(),
                vec![RegisteredSystem::new(
                    SystemSpec {
                        id: SystemId(1),
                        stage: Stage::Simulate,
                        accesses: vec![Access {
                            key: AccessKey::External(1),
                            mode: AccessMode::Write,
                        }],
                        after: Vec::new(),
                    },
                    play,
                )],
            )
            .expect("registered game"),
            candidate: Candidate::default(),
            epoch,
        };
        if published {
            game.runtime
                .publish_initial(&mut game.rules, &mut game.candidate);
        }
        game
    }

    pub fn roster_log(&self) -> &[RosterRow] {
        &self.rules.roster_log
    }

    pub fn log(&self) -> &[(u64, usize, u16)] {
        &self.rules.log
    }
}

impl World for Game {
    type Action = u16;
    type Error = Failure;

    fn identity(&self) -> RuleSetIdentity {
        let mut bytes = [0; 32];
        for (lane, output) in bytes.as_chunks_mut::<8>().0.iter_mut().enumerate() {
            let mut digest = Digest::default();
            digest.bytes(include_bytes!("support.rs"));
            digest.bytes(&u64::try_from(lane).expect("four lanes").to_le_bytes());
            digest.bytes(match self.rules.rule {
                Rule::Add => b"add",
                Rule::Subtract => b"subtract",
            });
            output.copy_from_slice(&digest.finish().to_le_bytes());
        }
        RuleSetIdentity(bytes)
    }

    fn tick(&self) -> u64 {
        self.runtime.tick()
    }

    fn step(
        &mut self,
        frames: &[InputFrame<u16>; SLOTS],
        roster: RosterEvents,
    ) -> Result<(), Failure> {
        self.rules.pending.fill(None);
        self.rules.roster = roster;
        self.rules.motion = frames.each_ref().map(|frame| {
            let axis = frame.axes[0];
            i64::from(axis > 0.0) - i64::from(axis < 0.0)
        });
        let actions = frames
            .iter()
            .enumerate()
            .flat_map(|(slot, frame)| frame.actions.iter().map(move |value| (slot, *value)));
        for (destination, action) in self.rules.pending.iter_mut().zip(actions) {
            *destination = Some(action);
        }
        self.runtime
            .step(&mut self.rules, &mut self.candidate)
            .map(|_| ())
            .map_err(Failure::Tick)
    }

    fn digest(&self) -> u64 {
        self.runtime
            .world_digest(&self.rules.resources, |_, _| {})
            .expect("test schema")
    }

    fn image(&self, destination: &mut StateBytes) -> Result<(), Failure> {
        self.runtime
            .write_bounded_world_image(self.identity(), &self.rules.resources, destination)
            .map_err(|_| Failure::Image)
    }

    fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), Failure> {
        let mut fresh = Self::new(self.epoch, self.rules.rule, None, 0, false);
        fresh
            .runtime
            .restore_world_image(self.identity(), &mut fresh.rules.resources, image, 1 << 20)
            .map_err(|_| Failure::Image)?;
        if fresh.tick() != tick {
            return Err(Failure::Image);
        }
        fresh
            .runtime
            .publish_initial(&mut fresh.rules, &mut fresh.candidate);
        *self = fresh;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Codec;

impl ActionWire<u16> for Codec {
    fn encode(&self, action: &u16, output: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        if *action == 0 {
            return Err(InputCodecError::InvalidAction);
        }
        output[..2].copy_from_slice(&action.to_le_bytes());
        Ok((7, 2))
    }

    fn decode(&self, action: ActionView<'_>) -> Result<u16, InputCodecError> {
        if action.schema_id != 7 {
            return Err(InputCodecError::InvalidAction);
        }
        let value = u16::from_le_bytes(
            action
                .payload
                .try_into()
                .map_err(|_| InputCodecError::InvalidAction)?,
        );
        if value == 0 {
            return Err(InputCodecError::InvalidAction);
        }
        Ok(value)
    }
}

pub struct Match {
    pub memory: Memory,
    pub host: Host<Game, Codec>,
    pub clients: Vec<Client<Game, Codec>>,
    pub links: Vec<[Link; 2]>,
    pub settings: Settings,
    pub now: u64,
}

impl Match {
    /// Form a lobby on the deterministic carrier.
    ///
    /// # Panics
    /// The fixture limits or carrier configuration are invalid.
    pub fn new(condition: Conditioner, clients: usize, fail_at: Option<u64>, padded: bool) -> Self {
        Self::with_settings(
            condition,
            clients,
            fail_at,
            padded,
            Settings {
                digest_interval: 1,
                ..Settings::default()
            },
        )
    }

    /// Form a lobby with explicit match settings.
    ///
    /// # Panics
    /// The fixture limits, settings or carrier configuration are invalid.
    pub fn with_settings(
        condition: Conditioner,
        clients: usize,
        fail_at: Option<u64>,
        padded: bool,
        settings: Settings,
    ) -> Self {
        let profile = Profile {
            links: 12,
            reliable_bytes: if padded { 65_536 } else { 1_048_576 },
            stack_bytes: 16_384,
            ..Profile::default()
        };
        Self::with_profile(condition, clients, fail_at, padded, settings, profile)
    }

    /// Form a lobby with explicit match and carrier settings.
    ///
    /// # Panics
    /// The fixture limits, settings or carrier configuration are invalid.
    pub fn with_profile(
        condition: Conditioner,
        clients: usize,
        fail_at: Option<u64>,
        padded: bool,
        settings: Settings,
        profile: Profile,
    ) -> Self {
        let mut fixture = Self {
            memory: Memory::new(profile, condition).expect("valid carrier"),
            host: Host::new(
                Game::new(10, Rule::Add, fail_at, if padded { 192 } else { 0 }, true),
                Codec,
                settings,
                [4; 16],
                341,
                false,
                0,
            )
            .expect("valid host"),
            clients: Vec::new(),
            links: Vec::new(),
            now: 0,
            settings,
        };
        for _ in 0..clients {
            fixture.add(Rule::Add, thrum_network::session::PROTOCOL);
        }
        fixture
    }

    /// Offer a new client with its own world.
    ///
    /// # Panics
    /// The fixture exceeds its link cap.
    pub fn add(&mut self, rules: Rule, protocol: u16) {
        self.add_token(rules, protocol, None);
    }

    /// Attach a fresh client that can present a previous slot token.
    ///
    /// # Panics
    /// The fixture exceeds its link cap.
    pub fn add_token(&mut self, rules: Rule, protocol: u16, token: Option<PlayerToken>) {
        let index = self.clients.len();
        let [link, authority] = self.memory.connect().expect("fixture link");
        self.host
            .connect(authority.clone(), self.now)
            .expect("host takes the fixture link");
        let mut client = Client::new(
            Game::new(11 + index as u128, rules, None, 0, false),
            Codec,
            link.clone(),
            self.now,
            protocol,
            None,
        );
        if let Some(token) = token {
            client = client.with_token(token);
        }
        self.clients.push(client);
        self.links.push([link, authority]);
    }

    /// Advance the carrier and every endpoint once.
    ///
    /// # Panics
    /// The fixture supplies invalid controls.
    pub fn poll(&mut self, through: u64, mut controls: impl FnMut(usize, u64) -> Controls<u16>) {
        self.memory.advance(self.now / 1_000);
        self.host.advance(self.now, through);
        for (index, client) in self.clients.iter_mut().enumerate() {
            client
                .advance(self.now, |tick| controls(index, tick))
                .expect("valid fixture controls");
        }
        self.now += 2_500;
    }

    pub fn neutral(&mut self, through: u64) {
        self.poll(through, |_, _| Controls::axes([0.0; 3]));
    }

    /// Wait in virtual time for each reserved slot to finish joining.
    ///
    /// # Panics
    /// At least one client failed to join.
    pub fn ready(&mut self) {
        for _ in 0..4_000 {
            if (0..self.clients.len()).all(|slot| self.host.ready(slot)) {
                self.host.start(self.now);
                return;
            }
            self.neutral(0);
        }
        panic!("lobby did not start");
    }

    /// Drain the match-ended message and remaining world steps on every link.
    ///
    /// # Panics
    /// At least one client failed to reach its terminal endpoint.
    pub fn finish(&mut self, tick: u64) {
        for _ in 0..4_000 {
            self.neutral(tick);
            if self
                .clients
                .iter()
                .all(|client| matches!(client.state(), ClientState::Ended(_, _)))
            {
                return;
            }
        }
        panic!("clients did not finish at {tick}");
    }
}

pub fn local() -> Conditioner {
    Conditioner {
        delay_ms: 0,
        jitter_ms: 0,
        loss_ppm: 0,
        seed: 29,
        duplicate: false,
    }
}

pub fn shaped() -> Conditioner {
    Conditioner {
        duplicate: true,
        ..Conditioner::default()
    }
}
