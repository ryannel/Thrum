use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};

use serde_json::{Value, json};
use thrum_core::sim::InputFrame;
use thrum_network::{
    envelope::{self, Kind},
    session::{
        Client, ClientState, Controls, Host, Input, PlayerToken, ResultEvent, Settings, World,
    },
};
use thrum_transport::{
    Lane, Link, Profile,
    memory::{Conditioner, DatagramEvent, Memory},
};

use super::{
    super::world::{
        CrowdWorld, Time,
        fault_world::{CrowdController, FaultWire, PlayerRule, Shared, Witness},
    },
    prediction::Trace,
};

pub(super) type Replica = Client<Shared, FaultWire, CrowdController>;
pub(super) type Authority = Host<Shared, FaultWire>;

pub(super) struct Player {
    pub client: Replica,
    pub world: Rc<RefCell<Witness>>,
    pub carrier: Memory,
    pub links: [Link; 2],
    pub input: Vec<u8>,
    pub sets: Vec<u8>,
    pub budget: u8,
    pub poll: bool,
    pub send: bool,
    pub most_sets: usize,
    pub most_bytes: usize,
    pub max_staged: usize,
    pub max_link_bytes: usize,
    pub last_input: u64,
    pub last_staged: u64,
    pub backlog: Vec<(u64, u64, u64, usize)>,
    pub admissions: u64,
    pub commands: Vec<ResultEvent>,
    pub measure_catchup: bool,
    pub catchup_ns: Time,
    pub catchup_steps: u64,
    pub catchup_engine_ns: u64,
    pub restored_at: Vec<u64>,
    pub prediction: Option<Trace>,
}

impl Player {
    pub(super) fn host_link(&self) -> u64 {
        self.links[1].id()
    }

    pub(super) fn slot(&self) -> usize {
        self.client.slot().expect("negotiated player")
    }

    pub(super) fn paused(&self) -> bool {
        !self.poll || !self.send
    }
}

pub(super) struct Match {
    pub host: Authority,
    pub world: Rc<RefCell<Witness>>,
    pub players: Vec<Player>,
    pub now: u64,
    pub settings: Settings,
    pub count: u32,
    pub measured: bool,
    pub host_times: Time,
    pub images: Vec<Value>,
    pub diagnostics: Vec<thrum_network::session::Divergence>,
    pub finished: Option<u64>,
    pub admitted: Option<Vec<[bool; 4]>>,
}

impl Match {
    pub(super) fn new(count: u32, settings: Settings, measured: bool) -> Self {
        let shared = Shared::new(CrowdWorld::new(count, 40, true, measured));
        let world = Rc::clone(&shared.0);
        let mut host = Host::new(shared, FaultWire, settings, [3; 16], 29_721, false, 0)
            .expect("valid conditioned match fixture");
        host.start(0);
        Self {
            world,
            host,
            settings,
            count,
            measured,
            now: 0,
            players: Vec::new(),
            host_times: Time::default(),
            images: Vec::new(),
            diagnostics: Vec::new(),
            finished: None,
            admitted: None,
        }
    }

    pub(super) fn add(&mut self, profile: Profile, token: Option<PlayerToken>) -> usize {
        self.add_conditioned(
            profile,
            token,
            Conditioner {
                seed: 29_721 + self.players.len() as u64,
                // Preserve exact bytes for the stale-epoch carrier control.
                duplicate: true,
                ..Conditioner::default()
            },
            None,
            None,
        )
    }

    pub(super) fn add_conditioned(
        &mut self,
        profile: Profile,
        token: Option<PlayerToken>,
        conditioner: Conditioner,
        lead: Option<u16>,
        prediction: Option<Trace>,
    ) -> usize {
        let index = self.players.len();
        let mut carrier =
            Memory::new(profile, conditioner).expect("valid conditioned match fixture");
        carrier.advance(self.now / 1_000);
        let links = carrier.connect().expect("valid conditioned match fixture");
        self.host
            .connect(links[1].clone(), self.now)
            .expect("valid conditioned match fixture");
        let shared = Shared::new(CrowdWorld::new(
            self.count,
            41 + index as u128,
            false,
            self.measured,
        ));
        let world = Rc::clone(&shared.0);
        let rules = prediction
            .as_ref()
            .filter(|trace| trace.enabled)
            .map(|trace| PlayerRule {
                movement: crowd_2d::PlayerMovement::default(),
                timing: trace.timing.as_ref().map(Rc::<Cell<Time>>::clone),
            });
        let mut client =
            Client::with_controller(shared, FaultWire, links[0].clone(), self.now, lead, rules);
        if let Some(token) = token {
            client = client.with_token(token);
        }
        self.players.push(Player {
            client,
            world,
            links,
            carrier,
            input: Vec::new(),
            sets: Vec::new(),
            budget: u8::MAX,
            poll: true,
            send: true,
            most_sets: 0,
            most_bytes: 0,
            max_staged: 0,
            max_link_bytes: 0,
            last_input: 0,
            last_staged: 0,
            backlog: Vec::new(),
            admissions: 0,
            commands: Vec::new(),
            measure_catchup: self.measured,
            catchup_ns: Time::default(),
            catchup_steps: 0,
            catchup_engine_ns: 0,
            restored_at: Vec::new(),
            prediction,
        });
        index
    }

    pub(super) fn tick(&self) -> u64 {
        self.host.world().tick()
    }

    pub(super) fn advance(&mut self) {
        let period = u64::from(self.settings.tick_us);
        self.now += period;
        for player in &mut self.players {
            player
                .carrier
                .advance_observed(self.now / 1_000, |direction, event, bytes| {
                    if direction == 0
                        && let Some(trace) = &mut player.prediction
                    {
                        trace.datagram(event, bytes);
                    }
                    if event != DatagramEvent::Delivered {
                        return;
                    }
                    let message = envelope::decode(bytes).expect("session packet");
                    match (direction, message.header.kind) {
                        (0, Kind::InputBatch) => player.input = bytes.to_vec(),
                        (1, Kind::AdmittedSets) => player.sets = bytes.to_vec(),
                        _ => {}
                    }
                });
        }

        let before = self.world.borrow().crowd.image_time.borrow().count;
        let start = self.measured.then(Instant::now);
        self.host
            .advance(self.now, self.finished.unwrap_or(u64::MAX));
        let after = self.tick();
        if let Some(admitted) = &mut self.admitted
            && after > 0
            && after > admitted.len() as u64
        {
            admitted.push(self.host.admissions());
        }
        assert_eq!(
            after,
            (self.now / period).min(self.finished.unwrap_or(u64::MAX)),
            "authority commits on every virtual deadline"
        );
        if let Some(start) = start {
            let total = nanos(start);
            let world = self.world.borrow();
            let images = world.crowd.image_time.borrow();
            if images.count != before {
                self.images.push(json!({
                    "baseline_tick": after - 1, "writing_tick": after,
                    "tick_ns": total, "image_bytes": world.crowd.image_bytes.get(),
                    "image_write_ns": images.last,
                }));
            } else if self.finished.is_none() {
                self.host_times.add(total);
            }
        }
        while let Some(report) = self.host.take_divergence() {
            self.diagnostics.push(report);
        }

        for player in &mut self.players {
            if player.client.slot().is_some()
                && self.host.admissions()[player.slot()]
                && player.links[1].is_open()
            {
                player.admissions += 1;
            }
            if let Some(stream) = self.host.stream(player.host_link()) {
                let sets = self.host.retained(player.host_link());
                let staged = stream.staged_frames;
                player.last_input = stream.last_input;
                player.last_staged = stream.last_staged;
                player.most_sets = player.most_sets.max(sets);
                player.max_staged = player.max_staged.max(staged);
                let queue = player.links[1].counters(self.now / 1_000);
                let link_bytes = queue
                    .lanes
                    .iter()
                    .map(|lane| lane.queued_bytes)
                    .sum::<usize>()
                    + queue.stack_bytes.unwrap_or(0);
                player.max_link_bytes = player.max_link_bytes.max(link_bytes);
                let retained =
                    stream.retained_bytes + staged * size_of::<InputFrame<u8>>() + link_bytes;
                player.most_bytes = player.most_bytes.max(retained);
                assert!(sets <= usize::from(self.settings.backlog));
                assert!(staged <= usize::from(self.settings.staging));
                assert!(
                    stream.retained_bytes
                        <= usize::from(self.settings.backlog) * 9_000
                            + self.settings.image_limit as usize
                            + usize::from(self.settings.command_cap) * 1_000
                );
                player
                    .backlog
                    .push((after, stream.held, stream.stepped, sets));
            }
            if !player.poll {
                continue;
            }
            if !player.send {
                // Keep taking messages from the carrier while originating none.
                for lane in [Lane::Reliable, Lane::Datagram] {
                    while player.links[0].receive(lane).is_some() {}
                }
                continue;
            }
            let before = player.client.world().tick();
            let engine_before = player.world.borrow().crowd.engine.total;
            let start = player.measure_catchup.then(Instant::now);
            let trace = &mut player.prediction;
            let prediction_run = trace.is_some();
            let slot = player.client.slot().unwrap_or(0);
            let timing = trace
                .as_ref()
                .and_then(|trace| trace.timing.as_ref())
                .map(|_| Instant::now());
            player
                .client
                .advance_with_prediction_events(
                    self.now,
                    player.budget,
                    |target| {
                        if prediction_run {
                            return Controls::axes(crate::seeded_axes(29_721, slot, target * 11));
                        }
                        // Keep motion on one axis during shorter recovery controls.
                        // Alternate before the hero would reach the arena wall.
                        let sign = if (target / 300).is_multiple_of(2) {
                            1.0
                        } else {
                            -1.0
                        };
                        Controls::axes([sign * 0.7, 0.1, 0.0])
                    },
                    |event| {
                        if let Some(trace) = trace {
                            trace.events.push(event);
                        }
                    },
                )
                .expect("valid conditioned match fixture");
            if let Some(trace) = trace {
                trace.sample(
                    &player.client,
                    &player.world.borrow(),
                    after,
                    self.now,
                    timing,
                );
            }
            {
                let world = player.world.borrow();
                if world.restores.len() > player.restored_at.len() {
                    player.restored_at.push(after);
                    player.catchup_ns = Time::default();
                    player.catchup_steps = 0;
                    player.catchup_engine_ns = 0;
                }
                if let Some(start) = start.filter(|_| !world.restores.is_empty()) {
                    let after = world.crowd.tick();
                    let start_tick = world
                        .restores
                        .last()
                        .map_or(before, |&(tick, _, _)| tick.max(before));
                    if after > start_tick {
                        player.catchup_steps += after - start_tick;
                        player.catchup_ns.add(nanos(start));
                        player.catchup_engine_ns += world.crowd.engine.total - engine_before;
                    }
                }
            }
            while let Some(result) = player.client.result() {
                if matches!(result.input, Input::Command(_)) {
                    player.commands.push(result);
                }
            }
        }
    }

    pub(super) fn until(&mut self, limit: u64, mut condition: impl FnMut(&Self) -> bool) {
        for _ in 0..limit {
            self.advance();
            if condition(self) {
                return;
            }
        }
        panic!(
            "condition not reached at host tick {}: {:?}",
            self.tick(),
            self.players
                .iter()
                .map(|player| (
                    player.client.state(),
                    player.client.world().tick(),
                    player.client.held(),
                    player.client.baseline_progress(),
                    self.host.stream(player.host_link()).map(|stream| (
                        stream.held,
                        stream.stepped,
                        stream.repairs
                    ))
                ))
                .collect::<Vec<_>>()
        );
    }

    pub(super) fn advance_to(&mut self, tick: u64) {
        if self.tick() < tick {
            self.until(tick - self.tick(), |game| game.tick() == tick);
        }
    }

    pub(super) fn joined(&self, player: usize) -> bool {
        let player = &self.players[player];
        player.client.slot().is_some_and(|slot| {
            self.host.ready(slot)
                && self.host.roster().occupied & (1 << slot) != 0
                && player.client.roster().occupied & !player.client.roster().away & (1 << slot) != 0
                && player.links[1].is_open()
        })
    }

    pub(super) fn finish(&mut self) {
        let end = self.tick();
        self.finished = Some(end);
        self.host.stop();
        self.until(800, |game| {
            game.players.iter().all(|player| {
                matches!(player.client.state(), ClientState::Ended(_, last) if *last == end)
                    || !player.links[0].is_open()
                    || player.paused()
            })
        });
    }

    pub(super) fn compare(
        &self,
        player: usize,
        after: u64,
        through: u64,
        excluded: Option<u64>,
    ) -> usize {
        let authority = self.world.borrow();
        let expected = authority.crowd.digests.borrow();
        let replica = self.players[player].world.borrow();
        let actual = replica.crowd.digests.borrow();
        let span = |tick| tick > after && tick <= through && Some(tick) != excluded;
        let expected = expected.iter().filter(|(tick, _)| span(*tick));
        let actual = actual.iter().filter(|(tick, _)| span(*tick));
        let mut count = 0;
        for (authority, replica) in expected.zip(actual) {
            assert_eq!(authority, replica, "player {player}: match checkpoint");
            count += 1;
        }
        let required = (after + 1..=through)
            .filter(|tick| {
                span(*tick) && tick.is_multiple_of(u64::from(self.settings.digest_interval))
            })
            .count();
        assert_eq!(
            count, required,
            "player {player}: recorded every expected digest"
        );
        assert_eq!(
            count,
            self.players[player]
                .world
                .borrow()
                .crowd
                .digests
                .borrow()
                .iter()
                .filter(|(tick, _)| span(*tick))
                .count(),
            "no repeated digest ticks"
        );
        count
    }

    pub(super) fn compare_played(&self, player: usize, bad_tick: Option<u64>) -> usize {
        let world = self.players[player].world.borrow();
        let mut checked = 0;
        for (index, &(start, _, _)) in world.restores.iter().enumerate() {
            let next = world.restores.get(index + 1).map_or(u64::MAX, |row| row.0);
            let end = world
                .steps
                .iter()
                .rev()
                .find(|row| row.tick < next)
                .map_or(start, |row| row.tick.max(start));
            checked += self.compare(player, start, end, bad_tick);
        }
        checked
    }

    pub(super) fn event(
        &self,
        player: usize,
        after: u64,
        event: impl Fn(thrum_network::session::RosterEvents) -> u8,
    ) -> u64 {
        let mask = 1 << self.players[player].slot();
        let tick = self
            .world
            .borrow()
            .steps
            .iter()
            .find(|row| row.tick > after && event(row.roster) & mask != 0)
            .expect("host committed roster event")
            .tick;
        for peer in &self.players {
            let peer = peer.world.borrow();
            // A rejoiner starts at its own baseline and cannot replay transitions before it.
            if peer
                .restores
                .first()
                .is_some_and(|&(baseline, _, _)| baseline < tick)
                && peer.crowd.tick() >= tick
            {
                let row = peer
                    .steps
                    .iter()
                    .find(|row| row.tick == tick)
                    .expect("peer committed event tick");
                assert_ne!(event(row.roster) & mask, 0);
            }
        }
        tick
    }

    pub(super) fn neutral_at(&self, slot: usize, tick: u64) {
        let authority = self.world.borrow();
        let expected = authority
            .steps
            .iter()
            .find(|row| row.tick == tick)
            .expect("authority committed the neutral frame")
            .axes[slot]
            .map(f32::to_bits);
        assert_eq!(expected, [0; 3]);
        for player in &self.players {
            let world = player.world.borrow();
            if world
                .restores
                .first()
                .is_some_and(|&(baseline, _, _)| baseline < tick)
                && world.crowd.tick() >= tick
            {
                let frame = world
                    .steps
                    .iter()
                    .find(|row| row.tick == tick)
                    .expect("playing replica committed the neutral frame");
                assert_eq!(frame.axes[slot].map(f32::to_bits), expected);
            }
        }
    }

    pub(super) fn scheduled(&self) -> Value {
        let ticks = self.tick();
        let period = u64::from(self.settings.tick_us);
        json!({"host_ticks": ticks, "run_us": ticks * period, "tick_us": period,
            "verified_virtual_deadlines": ticks, "drain_us": self.now - ticks * period})
    }
}

pub(super) fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("valid conditioned match fixture")
}
