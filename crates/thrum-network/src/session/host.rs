use std::{collections::VecDeque, num::NonZeroUsize};

use thrum_transport::{CloseReason, DATAGRAM_LIMIT, Event, Lane, Link, LinkError, RELIABLE_LIMIT};

use super::{
    CHUNK, Close, DigestPart, Divergence, EndReason, MATCH_EPOCH, PROTOCOL, PlayerToken, Refusal,
    Roster, RosterEvents, SLOTS, SessionError, Settings, World, binding, frame,
    wire::{self, Connection, INPUT_PREFIX, Read, SET_PREFIX, SlotResult},
};
use crate::{
    envelope::{self, HEADER_BYTES, Kind, Message},
    future_input::{FutureInputError, FutureInputWindow, StageResult},
    input_batch::{self, ActionWire, EncodeRecord},
};

/// Clocked authority state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostState {
    /// Accepting connections before the first clocked tick.
    Lobby,
    /// Committing ticks from its caller's clock.
    Running,
    /// Draining committed sets and the final message.
    Ended(EndReason),
}

enum Phase {
    Hello,
    Baseline {
        bytes: Vec<u8>,
        offset: usize,
        began: bool,
        tick: u64,
        roster: Roster,
        axes: [[f32; 3]; SLOTS],
    },
    CatchingUp,
    Playing,
    FallenBehind,
    Terminal(Option<u8>),
}

struct Peer<A> {
    link: Link,
    local_close: Option<u32>,
    connection: Connection,
    slot: Option<usize>,
    phase: Phase,
    deadline: u64,
    commands: VecDeque<(u64, A, usize)>,
    command_sequence: u64,
    held: u64,
    offered: u64,
    stepped: u64,
    reliable: u64,
    packet: u64,
    last_datagram: Option<u64>,
    margin: i16,
    margin_sequence: u64,
    late: u64,
    reliable_sets: u64,
    datagram_sizes: [u64; 188],
    behind_since: Option<u64>,
    last_input: u64,
    input_sequence: u64,
    last_staged: u64,
    staged_sequence: u64,
    input_credit: u64,
    credit_tick: u64,
    progress_sequence: Option<u64>,
    last_command_committed: u64,
    reliable_blocked: Option<u64>,
    stale: u64,
    repairs: u8,
    generation: u64,
}

impl<A> Peer<A> {
    fn close(&mut self, code: u32) {
        if self.link.is_open() {
            self.local_close.get_or_insert(code);
        }
        self.link.close(code);
    }

    fn sent(
        &mut self,
        lane: Lane,
        result: Result<(), LinkError>,
        now: u64,
        blocked_limit: u64,
    ) -> bool {
        match result {
            Ok(()) => {
                if lane == Lane::Reliable {
                    self.reliable_blocked = None;
                }
                true
            }
            Err(LinkError::Full) => {
                let since = *self.reliable_blocked.get_or_insert(now);
                if now - since >= blocked_limit {
                    self.close(Close::Synchronization as u32);
                }
                false
            }
            Err(LinkError::NotOpen) => false,
            Err(_) => {
                self.close(wire::PROTOCOL_CLOSE);
                false
            }
        }
    }

    fn terminal(&mut self, value: Option<u8>, now: u64, settings: Settings) {
        self.phase = Phase::Terminal(value);
        self.deadline = now + u64::from(settings.response_ms) * 1_000;
    }
}

#[derive(Clone, Copy, Default)]
enum Slot {
    #[default]
    Free,
    Reserved(PlayerToken),
    Present(PlayerToken),
    Away {
        token: PlayerToken,
        until: u64,
        returning: bool,
    },
    Local,
}

struct DigestEndpoint {
    tick: u64,
    digest: u64,
    parts: Vec<DigestPart>,
}

struct StoredSet {
    tick: u64,
    bytes: Vec<u8>,
}

/// Per-peer observations made at the authority's link boundary.
#[derive(Clone, Debug)]
pub struct PeerStream {
    /// Assigned player slot, once hello succeeds.
    pub slot: Option<usize>,
    /// Session connection epoch, zero before welcome.
    pub epoch: u64,
    /// Timed records in old datagrams that were refused by tick.
    pub late_frames: u64,
    /// Sets offered on the reliable lane.
    pub reliable_sets: u64,
    /// Histogram indexed by sets per accepted datagram.
    pub datagram_sizes: [u64; 188],
    /// Latest contiguous held acknowledgement.
    pub held: u64,
    /// Latest stepped acknowledgement.
    pub stepped: u64,
    /// Rejected datagrams from an earlier connection epoch.
    pub stale_epoch: u64,
    /// Current repair count.
    pub repairs: u8,
    /// Bytes in the retained set ring for this peer and its current image and commands.
    pub retained_bytes: usize,
    /// Timed frames retained in this slot's staging window.
    pub staged_frames: usize,
    /// The peer is still transferring or catching up to a baseline.
    pub synchronizing: bool,
    /// Authority endpoint when a new authored frame last arrived.
    pub last_input: u64,
    /// Authority endpoint when a new timed frame last entered the staging window.
    pub last_staged: u64,
}

/// A transport close observed by the authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerDisconnect {
    /// Identifier of the closed link.
    pub link: u64,
    /// Session connection epoch, zero before welcome.
    pub epoch: u64,
    /// Most recently committed authority tick when the close was observed.
    pub tick: u64,
    /// Reason reported by the link.
    pub reason: CloseReason,
}

/// An authority that owns one world and commits through a single input barrier.
pub struct Host<W: World, Codec> {
    world: W,
    codec: Codec,
    settings: Settings,
    id: [u8; 16],
    seed: u64,
    local: bool,
    state: HostState,
    next: u64,
    epoch: u64,
    peers: Vec<Peer<W::Action>>,
    owners: [Slot; SLOTS],
    roster: Roster,
    events: RosterEvents,
    history: VecDeque<DigestEndpoint>,
    divergences: VecDeque<Divergence>,
    disconnects: VecDeque<PeerDisconnect>,
    staging: FutureInputWindow<W::Action>,
    axes: [[f32; 3]; SLOTS],
    neutral: u8,
    sets: VecDeque<StoredSet>,
    spare: Vec<Vec<u8>>,
    packet: Vec<u8>,
    scratch: Box<[u8]>,
    admitted: [bool; SLOTS],
    step_error: Option<W::Error>,
}

impl<W: World, Codec: ActionWire<W::Action>> Host<W, Codec> {
    /// Build an authority with optional local slot zero.
    ///
    /// # Errors
    /// Invalid settings or a zero match identity.
    ///
    /// # Panics
    /// The fixed player slot bindings are invalid.
    pub fn new(
        world: W,
        codec: Codec,
        settings: Settings,
        id: [u8; 16],
        seed: u64,
        local: bool,
        now: u64,
    ) -> Result<Self, SessionError> {
        if !settings.valid() || id == [0; 16] {
            return Err(SessionError::Settings);
        }
        let bindings: [_; SLOTS] = std::array::from_fn(|slot| binding(slot, MATCH_EPOCH));
        let window =
            NonZeroUsize::new(usize::from(settings.staging)).expect("validated positive staging");
        let staging = FutureInputWindow::new(&bindings, window, world.tick())
            .expect("distinct fixed slot bindings");
        let mut owners = [Slot::Free; SLOTS];
        if local {
            owners[0] = Slot::Local;
        }
        Ok(Self {
            world,
            codec,
            settings,
            id,
            seed,
            local,
            state: HostState::Lobby,
            next: now,
            epoch: MATCH_EPOCH,
            peers: Vec::with_capacity(usize::from(settings.link_cap)),
            owners,
            roster: Roster::default(),
            events: RosterEvents {
                joined: u8::from(local),
                ..RosterEvents::default()
            },
            history: VecDeque::with_capacity(usize::from(settings.digest_history)),
            divergences: VecDeque::with_capacity(usize::from(settings.digest_history)),
            disconnects: VecDeque::with_capacity(usize::from(settings.link_cap)),
            staging,
            axes: [[0.0; 3]; SLOTS],
            neutral: 0,
            sets: VecDeque::with_capacity(usize::from(settings.backlog)),
            spare: Vec::new(),
            packet: Vec::with_capacity(RELIABLE_LIMIT),
            scratch: wire::scratch(),
            admitted: [false; SLOTS],
            step_error: None,
        })
    }

    /// The committed authority world, also used by a listen host for presentation.
    pub fn world(&self) -> &W {
        &self.world
    }

    /// Authority pacing or terminal state.
    pub fn state(&self) -> HostState {
        self.state
    }

    /// Whether the specified slot has finished synchronization.
    pub fn ready(&self, slot: usize) -> bool {
        matches!(self.owners.get(slot), Some(Slot::Present(_) | Slot::Local))
    }

    /// Roster at the last committed authority endpoint.
    pub fn roster(&self) -> Roster {
        self.roster
    }

    /// Oldest retained diagnostic report.
    pub fn take_divergence(&mut self) -> Option<Divergence> {
        self.divergences.pop_front()
    }

    /// Take the oldest observed close. The most recent link-cap closes are retained.
    pub fn take_disconnect(&mut self) -> Option<PeerDisconnect> {
        self.disconnects.pop_front()
    }

    /// Own-frame admissions at the last committed tick.
    pub fn admissions(&self) -> [bool; SLOTS] {
        self.admitted
    }

    /// Take the rule's authority failure, if one occurred.
    pub fn take_step_error(&mut self) -> Option<W::Error> {
        self.step_error.take()
    }

    /// Count sets still retained for this connection.
    pub fn retained(&self, link_id: u64) -> usize {
        self.peers
            .iter()
            .find(|peer| peer.link.id() == link_id)
            .map_or(0, |peer| {
                if matches!(peer.phase, Phase::FallenBehind) || peer.slot.is_none() {
                    0
                } else {
                    pending(&self.sets, peer.held + 1).count()
                }
            })
    }

    /// Read per-link stream counters.
    pub fn stream(&self, link_id: u64) -> Option<PeerStream> {
        let peer = self.peers.iter().find(|peer| peer.link.id() == link_id)?;
        Some(PeerStream {
            slot: peer.slot,
            epoch: peer.connection.epoch,
            late_frames: peer.late,
            reliable_sets: peer.reliable_sets,
            datagram_sizes: peer.datagram_sizes,
            held: peer.held,
            stepped: peer.stepped,
            stale_epoch: peer.stale,
            repairs: peer.repairs,
            retained_bytes: self.retained_bytes(peer),
            synchronizing: matches!(peer.phase, Phase::Baseline { .. } | Phase::CatchingUp),
            last_input: peer.last_input,
            last_staged: peer.last_staged,
            staged_frames: peer.slot.map_or(0, |slot| {
                self.staging.retained(binding(slot, MATCH_EPOCH).peer)
            }),
        })
    }

    fn retained_bytes(&self, peer: &Peer<W::Action>) -> usize {
        let sets = if matches!(peer.phase, Phase::FallenBehind) || peer.slot.is_none() {
            0
        } else {
            pending(&self.sets, peer.held + 1)
                .map(|set| set.bytes.len())
                .sum()
        };
        let image = match &peer.phase {
            Phase::Baseline { bytes, .. } => bytes.len(),
            _ => 0,
        };
        sets + image
            + peer
                .commands
                .iter()
                .map(|(_, _, bytes)| *bytes)
                .sum::<usize>()
    }

    /// Begin authoritative pacing. Negotiation can be polled in the lobby.
    pub fn start(&mut self, now: u64) {
        if self.state == HostState::Lobby {
            self.state = HostState::Running;
            self.next = now + u64::from(self.settings.tick_us);
        }
    }

    /// Offer a transport connection to the application handshake.
    ///
    /// # Errors
    /// The pending connection cap was reached or the host has ended.
    pub fn connect(&mut self, link: Link, now: u64) -> Result<(), SessionError> {
        if matches!(self.state, HostState::Ended(_)) {
            return Err(SessionError::Closed);
        }
        if self.peers.len() == usize::from(self.settings.link_cap)
            || self.peers.iter().any(|peer| peer.link.id() == link.id())
        {
            return Err(SessionError::Full);
        }
        self.peers.push(Peer {
            link,
            local_close: None,
            connection: Connection::default(),
            slot: None,
            phase: Phase::Hello,
            deadline: now + u64::from(self.settings.response_ms) * 1_000,
            commands: VecDeque::new(),
            command_sequence: 0,
            held: 0,
            offered: 0,
            stepped: 0,
            reliable: 0,
            packet: 0,
            last_datagram: None,
            margin: 0,
            margin_sequence: 0,
            late: 0,
            reliable_sets: 0,
            datagram_sizes: [0; 188],
            behind_since: None,
            last_input: self.world.tick(),
            input_sequence: 0,
            last_staged: self.world.tick(),
            staged_sequence: 0,
            input_credit: self.settings.input_burst(),
            credit_tick: now / u64::from(self.settings.tick_us),
            progress_sequence: None,
            last_command_committed: 0,
            reliable_blocked: None,
            stale: 0,
            repairs: 0,
            generation: 0,
        });
        Ok(())
    }

    /// Stage the listen host's frame before the same barrier used by remote players.
    ///
    /// # Errors
    /// The frame is invalid for the wire, slot or current staging window.
    pub fn stage_local(
        &mut self,
        tick: u64,
        axes: [f32; 3],
        actions: impl IntoIterator<Item = W::Action>,
    ) -> Result<(), SessionError> {
        if !self.local || matches!(self.state, HostState::Ended(_)) {
            return Err(SessionError::Closed);
        }
        let axes = input_batch::canonical_axes(axes).map_err(|_| SessionError::Input)?;
        let mut input = frame(0, MATCH_EPOCH, tick, tick, axes);
        for action in actions {
            if input.actions.len() == input_batch::MAX_ACTIONS {
                return Err(SessionError::Input);
            }
            input.actions.push(action);
        }
        let mut buffer = [0; DATAGRAM_LIMIT - HEADER_BYTES - INPUT_PREFIX];
        input_batch::encode(
            &[EncodeRecord {
                frame: &input,
                actor_wire_id: 1,
            }],
            &self.codec,
            &mut buffer,
        )
        .map_err(|_| SessionError::Input)?;
        self.staging
            .stage(input, |_| Ok(()))
            .map_err(|_| SessionError::Input)?;
        Ok(())
    }

    /// End the match at the latest committed endpoint.
    pub fn stop(&mut self) {
        if !matches!(self.state, HostState::Ended(_)) {
            self.state = HostState::Ended(EndReason::Stopped);
        }
    }

    /// Poll links and commit due authority ticks, at most the configured catch-up
    /// cap. A caller can cap the endpoint for a headless recording.
    pub fn advance(&mut self, now: u64, through: u64) {
        self.receive(now);
        self.prune();
        let send_time = now / 1_000;
        self.baselines(send_time);
        if self.state == HostState::Running {
            for _ in 0..self.settings.catch_up {
                if now < self.next || self.world.tick() >= through {
                    break;
                }
                if !self.commit() {
                    break;
                }
                self.next += u64::from(self.settings.tick_us);
                self.send_sets(now, send_time, true);
            }
        }
        self.send_sets(now, send_time, false);
    }

    fn commit(&mut self) -> bool {
        let tick = self.world.tick() + 1;
        self.expire_slots(tick);
        self.world.barrier(tick);
        let mut results = [SlotResult::default(); SLOTS];
        let frames = std::array::from_fn(|slot| {
            let input = self
                .staging
                .take_due(binding(slot, MATCH_EPOCH).peer)
                .expect("registered player slot");
            let mut admitted = frame(slot, MATCH_EPOCH, tick, tick, self.axes[slot]);
            if let Some(input) = input.filter(|_| self.ready(slot)) {
                results[slot].frame = input.origin.sequence;
                admitted.axes = input.axes;
                admitted.actions = input.actions;
            }
            if self.neutral & (1 << slot) != 0 {
                admitted.axes = [0.0; 3];
            }
            if self.ready(slot)
                && let Some(peer) = self.peers.iter_mut().find(|peer| peer.slot == Some(slot))
            {
                while admitted.actions.len() < input_batch::MAX_ACTIONS {
                    let Some((sequence, action, _)) = peer.commands.pop_front() else {
                        break;
                    };
                    admitted.actions.push(action);
                    results[slot].last_command = sequence;
                    peer.last_command_committed = sequence;
                    results[slot].commands += 1;
                }
            }
            admitted
        });
        if let Err(error) = self.world.step(&frames, self.events) {
            self.state = HostState::Ended(EndReason::AuthorityStep);
            self.step_error = Some(error);
            return false;
        }
        self.staging
            .commit_tick(tick)
            .expect("complete pre-tick barrier");
        let digest = tick
            .is_multiple_of(u64::from(self.settings.digest_interval))
            .then(|| {
                let mut parts = if self.history.len() == usize::from(self.settings.digest_history) {
                    self.history.pop_front().expect("full history").parts
                } else {
                    Vec::new()
                };
                let digest = wire::digest_parts(&self.world, &mut parts);
                self.history.push_back(DigestEndpoint {
                    tick,
                    digest,
                    parts,
                });
                digest
            });
        let set = wire::Set {
            frames,
            results,
            events: self.events,
            changed_axes: 0,
            digest,
        };
        let mut bytes = self.spare.pop().unwrap_or_else(|| Vec::with_capacity(64));
        wire::write_set(&mut bytes, &set, self.axes, &self.codec).expect("validated set encoding");
        self.axes = set.frames.map(|input| input.axes);
        self.admitted = results.map(|result| result.frame != 0);
        self.roster.apply(self.events);
        self.events = RosterEvents::default();
        self.neutral = 0;
        if self.sets.len() == usize::from(self.settings.backlog) {
            let oldest = self.sets.front().expect("full ring").tick;
            for peer in &mut self.peers {
                if peer.slot.is_some() && peer.held < oldest {
                    match peer.phase {
                        Phase::Playing => {
                            peer.phase = Phase::FallenBehind;
                            peer.reliable_blocked = None;
                        }
                        Phase::Baseline { .. } | Phase::CatchingUp => {
                            peer.close(Close::Synchronization as u32);
                        }
                        _ => {}
                    }
                }
            }
            let expired = self.sets.pop_front().expect("full ring");
            self.spare.push(expired.bytes);
        }
        self.sets.push_back(StoredSet { tick, bytes });
        true
    }

    fn prune(&mut self) {
        let held = self
            .peers
            .iter()
            .filter(|peer| {
                peer.slot.is_some()
                    && peer.link.is_open()
                    && !matches!(peer.phase, Phase::FallenBehind)
            })
            .map(|peer| peer.held)
            .min()
            .unwrap_or_else(|| self.world.tick());
        while self.sets.front().is_some_and(|set| set.tick <= held) {
            self.spare
                .push(self.sets.pop_front().expect("checked front").bytes);
        }
    }

    fn expire_slots(&mut self, tick: u64) {
        for (slot, owner) in self.owners.iter_mut().enumerate() {
            if matches!(*owner, Slot::Away { until, returning: false, .. } if tick >= until) {
                *owner = Slot::Free;
                self.events.left |= 1 << slot;
            }
        }
        let mut peers = std::mem::take(&mut self.peers);
        for peer in &mut peers {
            let Some(slot) = peer.slot.filter(|slot| self.ready(*slot)) else {
                continue;
            };
            let idle = tick - peer.last_input;
            if idle >= u64::from(self.settings.absence) {
                peer.close(Close::Absent as u32);
                self.detach(peer, false);
            } else if tick - peer.last_staged >= u64::from(self.settings.held_expiry) {
                self.neutral |= 1 << slot;
            }
        }
        self.peers = peers;
    }

    fn detach(&mut self, peer: &mut Peer<W::Action>, goodbye: bool) {
        let Some(slot) = peer.slot.take() else { return };
        let bit = 1 << slot;
        self.neutral |= 1 << slot;
        if self.roster.occupied & bit == 0 {
            self.events.joined &= !bit;
            self.owners[slot] = Slot::Free;
        } else if goodbye {
            self.events.left |= bit;
            self.events.back &= !bit;
            // Keep the slot unavailable until the left event commits.
            self.owners[slot] = Slot::Away {
                token: PlayerToken([0; 16]),
                until: self.world.tick() + 1,
                returning: false,
            };
        } else {
            match self.owners[slot] {
                Slot::Present(token) => {
                    self.events.away |= bit;
                    self.events.back &= !bit;
                    self.owners[slot] = Slot::Away {
                        token,
                        until: self.world.tick() + 1 + u64::from(self.settings.grace),
                        returning: false,
                    };
                }
                Slot::Away { token, until, .. } => {
                    self.owners[slot] = Slot::Away {
                        token,
                        until,
                        returning: false,
                    };
                }
                _ => {}
            }
        }
    }

    fn receive(&mut self, now: u64) {
        let mut peers = std::mem::take(&mut self.peers);
        for peer in &mut peers {
            if self.observe_close(peer) {
                self.detach(peer, false);
                peer.deadline = now;
                continue;
            }
            if now >= peer.deadline {
                let reason = if matches!(peer.phase, Phase::Baseline { .. } | Phase::CatchingUp) {
                    Close::Synchronization as u32
                } else {
                    wire::TIMEOUT_CLOSE
                };
                peer.close(reason);
                self.detach(peer, false);
                self.observe_removal(peer);
                continue;
            }
            'lanes: for lane in [Lane::Reliable, Lane::Datagram] {
                for _ in 0..128 {
                    let Some(bytes) = peer.link.receive(lane) else {
                        break;
                    };
                    if self.message(peer, now, lane, &bytes).is_none() {
                        peer.close(wire::PROTOCOL_CLOSE);
                    }
                    if !peer.link.is_open() {
                        self.detach(peer, false);
                        peer.deadline = now;
                        self.observe_removal(peer);
                        break 'lanes;
                    }
                }
            }
        }
        peers.retain(|peer| peer.deadline > now);
        self.peers = peers;
    }

    fn observe_close(&mut self, peer: &Peer<W::Action>) -> bool {
        while let Some(event) = peer.link.event() {
            if let Event::Closed(reason) = event {
                self.record_disconnect(peer, reason);
                return true;
            }
        }
        false
    }

    fn observe_removal(&mut self, peer: &Peer<W::Action>) {
        if let Some(code) = peer.local_close {
            // A Link clone can still collect the terminal event when the
            // session removes its own handle in this same receive pass.
            self.record_disconnect(peer, CloseReason::Local(code));
        } else {
            self.observe_close(peer);
        }
    }

    fn record_disconnect(&mut self, peer: &Peer<W::Action>, reason: CloseReason) {
        if self.disconnects.len() == usize::from(self.settings.link_cap) {
            self.disconnects.pop_front();
        }
        self.disconnects.push_back(PeerDisconnect {
            link: peer.link.id(),
            epoch: peer.connection.epoch,
            tick: self.world.tick(),
            reason,
        });
    }

    fn message(
        &mut self,
        peer: &mut Peer<W::Action>,
        now: u64,
        lane: Lane,
        bytes: &[u8],
    ) -> Option<()> {
        let message = envelope::decode(bytes).ok()?;
        if matches!(peer.phase, Phase::Terminal(_)) {
            return Some(());
        }
        if matches!(peer.phase, Phase::Hello) {
            if lane != Lane::Reliable || message.header.kind != Kind::Hello {
                return None;
            }
            return self.hello(peer, now, message.payload);
        }
        let header = message.header;
        if !peer.connection.matches(header) {
            if lane == Lane::Datagram && header.match_id == self.id {
                peer.stale += 1;
            }
            return (lane == Lane::Datagram).then_some(());
        }
        match (lane, header.kind) {
            (Lane::Datagram, Kind::InputBatch)
                if header.acknowledged_sequence <= peer.generation =>
            {
                self.input(peer, now, message)
            }
            (Lane::Reliable, Kind::Command)
                if header.acknowledged_sequence == 0
                    && header.server_tick == 0
                    && peer.slot.is_some_and(|slot| self.ready(slot)) =>
            {
                self.command(peer, message)
            }
            (Lane::Reliable, Kind::Divergence)
                if header.acknowledged_sequence <= peer.generation =>
            {
                self.divergence(peer, now, message)
            }
            (Lane::Reliable, Kind::Close)
                if header.acknowledged_sequence == 0
                    && header.server_tick == 0
                    && header.message_sequence == 0
                    && message.payload.is_empty() =>
            {
                self.detach(peer, true);
                peer.close(wire::RESPONSE_CLOSE);
                Some(())
            }
            _ => None,
        }
    }

    fn hello(&mut self, peer: &mut Peer<W::Action>, now: u64, bytes: &[u8]) -> Option<()> {
        let (protocol, identity, token) = wire::read_hello(bytes)?;
        let returning = self.owners.iter().enumerate().position(|(slot, owner)| {
            matches!(*owner, Slot::Away { token: credential, returning: false, until }
                if Some(credential) == token && self.world.tick() < until && (self.roster.away | self.events.away) & (1 << slot) != 0)
        });
        let free = self
            .owners
            .iter()
            .position(|owner| matches!(owner, Slot::Free));
        let refusal = if protocol != PROTOCOL {
            Some(Refusal::Protocol)
        } else if identity != self.world.identity().0 {
            Some(Refusal::RuleSet)
        } else if (returning.is_none() && free.is_none())
            || matches!(self.state, HostState::Ended(_))
        {
            Some(Refusal::Full)
        } else {
            None
        };
        if let Some(reason) = refusal {
            peer.terminal(Some(reason as u8), now, self.settings);
            return Some(());
        }
        let slot = returning.or(free)?;
        if let Slot::Away { token, until, .. } = self.owners[slot] {
            self.owners[slot] = Slot::Away {
                token,
                until,
                returning: true,
            };
        } else {
            let mut entropy = [0; 16];
            if getrandom::getrandom(&mut entropy).is_err() || entropy == [0; 16] {
                peer.close(wire::RESPONSE_CLOSE);
                return Some(());
            }
            self.owners[slot] = Slot::Reserved(PlayerToken(entropy));
        }
        self.epoch += 1;
        self.staging
            .reconnect(binding(slot, self.epoch))
            .expect("new slot epoch");
        peer.slot = Some(slot);
        peer.connection = Connection {
            id: self.id,
            epoch: self.epoch,
        };
        self.baseline(peer, now);
        Some(())
    }

    fn baseline(&self, peer: &mut Peer<W::Action>, now: u64) {
        let limit = self.settings.image_limit as usize;
        let mut output = super::StateBytes::new(limit);
        let bytes = self
            .world
            .image(&mut output)
            .ok()
            .and_then(|()| output.into_bytes());
        let Some(bytes) = bytes.filter(|bytes| bytes.len() <= limit) else {
            peer.close(Close::Synchronization as u32);
            return;
        };
        let tick = self.world.tick();
        peer.generation += 1;
        peer.held = tick;
        peer.offered = tick;
        peer.reliable = tick;
        peer.stepped = tick;
        peer.behind_since = None;
        peer.progress_sequence = None;
        peer.reliable_blocked = None;
        peer.deadline = now + u64::from(self.settings.join_ms) * 1_000;
        peer.phase = Phase::Baseline {
            bytes,
            offset: 0,
            began: false,
            tick,
            roster: self.roster,
            axes: self.axes,
        };
    }

    fn repair(&self, peer: &mut Peer<W::Action>, now: u64) {
        if peer.repairs == self.settings.repairs {
            peer.close(Close::RepairLimit as u32);
        } else {
            peer.repairs += 1;
            self.baseline(peer, now);
        }
    }

    fn divergence(
        &mut self,
        peer: &mut Peer<W::Action>,
        now: u64,
        message: Message<'_>,
    ) -> Option<()> {
        let header = message.header;
        if header.acknowledged_sequence < peer.generation {
            return Some(());
        }
        let (actual, parts) = wire::read_divergence(message.payload)?;
        let slot = peer.slot?;
        let tick = header.server_tick;
        if header.message_sequence != 0
            || tick > peer.offered
            || (actual.is_some() && !tick.is_multiple_of(u64::from(self.settings.digest_interval)))
            || matches!(peer.phase, Phase::Baseline { .. })
        {
            return None;
        }
        let endpoint = self.history.iter().find(|endpoint| endpoint.tick == tick);
        let mut differed = Vec::new();
        if let Some(endpoint) = endpoint {
            if actual == Some(endpoint.digest) {
                return None;
            }
            for id in endpoint.parts.iter().chain(&parts).map(|(id, _)| *id) {
                let find = |list: &[DigestPart]| {
                    list.binary_search_by_key(&id, |(key, _)| *key)
                        .ok()
                        .map(|index| list[index].1)
                };
                if find(&endpoint.parts) != find(&parts) {
                    differed.push(id);
                }
            }
            differed.sort_unstable();
            differed.dedup();
        }
        if self.divergences.len() == usize::from(self.settings.digest_history) {
            self.divergences.pop_front();
        }
        self.divergences.push_back(Divergence {
            link: peer.link.id(),
            slot,
            tick,
            actual,
            expected: endpoint.map(|value| value.digest),
            parts: differed,
        });
        self.repair(peer, now);
        Some(())
    }

    fn command(&self, peer: &mut Peer<W::Action>, message: Message<'_>) -> Option<()> {
        if matches!(self.state, HostState::Ended(_)) {
            return Some(());
        }
        let sequence = message.header.message_sequence;
        let mut data = Read::new(message.payload);
        let action = wire::read_action(&mut data, &self.codec)?;
        data.done()?;
        if sequence != peer.command_sequence + 1
            || peer.commands.len() >= usize::from(self.settings.command_cap)
        {
            return None;
        }
        peer.commands
            .push_back((sequence, action, message.payload.len()));
        peer.command_sequence = sequence;
        Some(())
    }

    fn input(&mut self, peer: &mut Peer<W::Action>, now: u64, message: Message<'_>) -> Option<()> {
        let header = message.header;
        if header.server_tick != 0 {
            return None;
        }
        let matching = header.acknowledged_sequence == peer.generation;
        if header.acknowledged_sequence == 0
            || (matching && matches!(peer.phase, Phase::Baseline { .. }))
        {
            return None;
        }
        let mut data = Read::new(message.payload);
        let held = data.u64()?;
        let stepped = data.u64()?;
        if stepped > held || (matching && held > peer.offered) {
            return None;
        }
        let slot = peer.slot?;
        let bytes = data.rest();
        let mut decoded = [const { None }; input_batch::MAX_RECORDS];
        if !bytes.is_empty() {
            let batch = input_batch::decode(bytes).ok()?;
            if batch.len() > usize::from(self.settings.repeats) {
                return None;
            }
            let mut previous = (0, 0);
            for (record, destination) in batch.records().zip(&mut decoded) {
                if record.sequence <= previous.0
                    || record.target_tick <= previous.1
                    || record.sequence > header.message_sequence
                {
                    return None;
                }
                previous = (record.sequence, record.target_tick);
                *destination = Some(
                    record
                        .decode_frame(
                            binding(slot, peer.connection.epoch),
                            slot as u64 + 1,
                            &self.codec,
                        )
                        .ok()?,
                );
            }
        }
        let committed = self.world.tick();
        if self.ready(slot) && !matches!(self.state, HostState::Ended(_)) {
            let mut fresh = 0;
            for input in decoded.iter().flatten() {
                match self.staging.validate_stage(input) {
                    Ok(StageResult::Queued) => fresh += 1,
                    Ok(StageResult::Duplicate)
                    | Err(FutureInputError::Late | FutureInputError::TooEarly) => {}
                    Err(_) => return None,
                }
            }
            // Independent client and socket clocks continue during a long
            // authority step. Bound incoming work by elapsed tick periods.
            let credit_tick = now / u64::from(self.settings.tick_us);
            let credit = (peer.input_credit + credit_tick.saturating_sub(peer.credit_tick))
                .min(self.settings.input_burst());
            peer.input_credit = credit.checked_sub(fresh)?;
            peer.credit_tick = credit_tick;
            for input in decoded.into_iter().flatten() {
                let sequence = input.origin.sequence;
                let tick = input.tick;
                let outcome = self.staging.stage(input, |_| Ok(()));
                if matches!(outcome, Ok(_) | Err(FutureInputError::Late))
                    && sequence > peer.input_sequence
                {
                    peer.last_input = committed;
                    peer.input_sequence = sequence;
                }
                if outcome == Ok(StageResult::Queued) && sequence > peer.staged_sequence {
                    peer.last_staged = committed;
                    peer.staged_sequence = sequence;
                }
                if outcome == Err(FutureInputError::Late) {
                    peer.late += 1;
                }
                if sequence > peer.margin_sequence {
                    let offset = i128::from(tick) - i128::from(committed) - 1;
                    peer.margin = i16::try_from(offset.clamp(-600, 600)).expect("clamped margin");
                    peer.margin_sequence = sequence;
                }
            }
        }
        if !matching {
            return Some(());
        }
        if matches!(peer.phase, Phase::FallenBehind) {
            self.repair(peer, now);
            return Some(());
        }
        let fresh = held >= peer.held
            && stepped >= peer.stepped
            && peer
                .progress_sequence
                .is_none_or(|sequence| header.message_sequence >= sequence);
        if !fresh {
            return Some(());
        }
        peer.progress_sequence = Some(header.message_sequence);
        peer.held = held;
        peer.stepped = stepped;
        let backlog = held - stepped;
        if matches!(peer.phase, Phase::CatchingUp)
            && backlog <= u64::from(self.settings.behind)
            && self.datagram_fits(held)
        {
            if !self.ready(slot) {
                let bit = 1 << slot;
                self.owners[slot] = match self.owners[slot] {
                    Slot::Reserved(token) => {
                        self.events.joined |= bit;
                        Slot::Present(token)
                    }
                    Slot::Away { token, .. } => {
                        self.events.back |= bit;
                        Slot::Present(token)
                    }
                    _ => return None,
                };
                peer.last_input = committed;
                peer.last_staged = committed;
            }
            peer.phase = Phase::Playing;
            peer.deadline = u64::MAX;
        }
        if matches!(peer.phase, Phase::Playing) && backlog > u64::from(self.settings.behind) {
            let since = peer.behind_since.get_or_insert(committed);
            if committed - *since >= u64::from(self.settings.catch_up_deadline) {
                peer.close(Close::UnableToKeepUp as u32);
            }
        } else {
            peer.behind_since = None;
        }
        Some(())
    }

    fn datagram_fits(&self, held: u64) -> bool {
        self.world.tick() - held <= 188
            && pending(&self.sets, held + 1)
                .map(|set| set.bytes.len() + 2)
                .sum::<usize>()
                + SET_PREFIX
                + HEADER_BYTES
                <= DATAGRAM_LIMIT
    }

    fn baselines(&mut self, send_time: u64) {
        for peer in &mut self.peers {
            match &mut peer.phase {
                Phase::Terminal(value) if peer.slot.is_none() => {
                    if let Some(reason) = *value
                        && wire::send(
                            &peer.link,
                            Lane::Reliable,
                            Connection::default().header(Kind::Reject, 0, 0),
                            &[reason],
                            send_time,
                            &mut self.scratch,
                        )
                        .is_ok()
                    {
                        *value = None;
                    }
                }
                Phase::Baseline {
                    bytes,
                    offset,
                    began,
                    tick,
                    roster,
                    axes,
                } => {
                    if !*began {
                        let Some(slot) = peer.slot else { continue };
                        let (Slot::Reserved(token)
                        | Slot::Present(token)
                        | Slot::Away { token, .. }) = self.owners[slot]
                        else {
                            continue;
                        };
                        self.packet.clear();
                        self.packet
                            .push(u8::try_from(slot).expect("fixed slot count"));
                        self.packet.extend(self.seed.to_le_bytes());
                        self.packet.extend(
                            u32::try_from(bytes.len())
                                .expect("bounded image")
                                .to_le_bytes(),
                        );
                        self.packet.extend(token.0);
                        self.packet.extend([roster.occupied, roster.away]);
                        self.packet
                            .extend(peer.last_command_committed.to_le_bytes());
                        for axis in *axes {
                            wire::write_axes(&mut self.packet, axis);
                        }
                        wire::write_settings(&mut self.packet, self.settings);
                        let kind = if peer.generation == 1 {
                            Kind::Welcome
                        } else {
                            Kind::BaselineBegin
                        };
                        let mut header = peer.connection.header(kind, *tick, 0);
                        header.acknowledged_sequence = peer.generation;
                        if wire::send(
                            &peer.link,
                            Lane::Reliable,
                            header,
                            &self.packet,
                            send_time,
                            &mut self.scratch,
                        )
                        .is_err()
                        {
                            continue;
                        }
                        *began = true;
                    }
                    for _ in 0..8 {
                        let end = (*offset + CHUNK).min(bytes.len());
                        let kind = if end == *offset {
                            Kind::BaselineEnd
                        } else {
                            Kind::BaselineChunk
                        };
                        let mut header = peer.connection.header(kind, *tick, *offset as u64);
                        header.acknowledged_sequence = peer.generation;
                        if wire::send(
                            &peer.link,
                            Lane::Reliable,
                            header,
                            &bytes[*offset..end],
                            send_time,
                            &mut self.scratch,
                        )
                        .is_err()
                        {
                            break;
                        }
                        *offset = end;
                        if kind == Kind::BaselineEnd {
                            peer.phase = Phase::CatchingUp;
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn send_sets(&mut self, now: u64, send_time: u64, fresh_tick: bool) {
        let ending = match self.state {
            HostState::Ended(reason) => Some(reason),
            _ => None,
        };
        let tick = self.world.tick();
        let blocked_limit = u64::from(self.settings.join_ms) * 1_000;
        let mut offered = false;
        for peer in &mut self.peers {
            if !matches!(peer.phase, Phase::CatchingUp | Phase::Playing) {
                continue;
            }
            let lag = tick - peer.held;
            let mut next = peer.held + 1;
            let mut lane = Lane::Datagram;
            let bytes: usize = pending(&self.sets, next)
                .map(|set| set.bytes.len() + 2)
                .take_while(|_| lag <= 188)
                .sum();
            if ending.is_some() || lag > 188 || bytes + SET_PREFIX + HEADER_BYTES > DATAGRAM_LIMIT {
                lane = Lane::Reliable;
                next = next.max(peer.reliable + 1);
            } else {
                // Acknowledgements can retire the reliable send without any
                // successful retry. This also applies when a datagram is not due.
                peer.reliable_blocked = None;
                if !fresh_tick
                    && (peer.held == tick
                        || peer.last_datagram.is_some_and(|last| {
                            now.saturating_sub(last) < u64::from(self.settings.tick_us)
                        }))
                {
                    continue;
                }
            }
            let count = pack(
                &mut self.packet,
                &self.sets,
                next,
                peer.stepped
                    .saturating_add(u64::from(self.settings.backlog)),
                lane,
                peer.margin,
                peer.margin_sequence,
            );
            if count == 0 && !(fresh_tick && lane == Lane::Datagram) {
                if let Some(reason) = ending
                    && peer.held.max(peer.reliable) >= tick
                {
                    let result = wire::send(
                        &peer.link,
                        Lane::Reliable,
                        peer.connection.header(Kind::MatchEnded, tick, 0),
                        &[reason as u8],
                        send_time,
                        &mut self.scratch,
                    );
                    if peer.sent(Lane::Reliable, result, now, blocked_limit) {
                        peer.terminal(None, now, self.settings);
                    }
                } else {
                    peer.reliable_blocked = None;
                }
                continue;
            }

            let mut header = peer
                .connection
                .header(Kind::AdmittedSets, next, peer.packet + 1);
            header.acknowledged_sequence = peer.generation;
            let result = wire::send(
                &peer.link,
                lane,
                header,
                &self.packet,
                send_time,
                &mut self.scratch,
            );
            if peer.sent(lane, result, now, blocked_limit) {
                peer.packet += 1;
                let last = next - 1 + count as u64;
                peer.offered = peer.offered.max(last);
                if lane == Lane::Reliable {
                    peer.reliable = last;
                    peer.reliable_sets += count as u64;
                } else {
                    peer.last_datagram = Some(now);
                    peer.datagram_sizes[count] += 1;
                }
                offered |= fresh_tick && last == tick;
            }
        }
        if offered {
            self.world.set_offered(tick);
        }
    }
}

fn pending(sets: &VecDeque<StoredSet>, next: u64) -> impl Iterator<Item = &StoredSet> {
    let index = usize::try_from(
        sets.front()
            .map_or(0, |first| next.saturating_sub(first.tick)),
    )
    .unwrap_or(usize::MAX);
    sets.iter().skip(index)
}

fn pack(
    packet: &mut Vec<u8>,
    sets: &VecDeque<StoredSet>,
    next: u64,
    through: u64,
    lane: Lane,
    margin: i16,
    sequence: u64,
) -> usize {
    packet.clear();
    packet.extend(margin.to_le_bytes());
    packet.extend(sequence.to_le_bytes());
    let cap = if lane == Lane::Reliable {
        RELIABLE_LIMIT
    } else {
        DATAGRAM_LIMIT
    };
    let mut count = 0;
    for set in pending(sets, next) {
        if set.tick > through || packet.len() + set.bytes.len() + 2 + HEADER_BYTES > cap {
            break;
        }
        packet.extend(
            u16::try_from(set.bytes.len())
                .expect("bounded set")
                .to_le_bytes(),
        );
        packet.extend(&set.bytes);
        count += 1;
    }
    count
}
