use std::{collections::VecDeque, num::NonZeroUsize};

use thrum_transport::{DATAGRAM_LIMIT, Lane, Link, LinkError, RELIABLE_LIMIT};

use super::{
    CHUNK, EndReason, MATCH_EPOCH, MAX_COMMANDS, MAX_LINKS, PROTOCOL, Refusal, SLOTS, SessionError,
    Settings, World, binding, frame,
    wire::{self, Connection, INPUT_PREFIX, Read, SET_PREFIX, SlotResult},
};
use crate::{
    envelope::{self, HEADER_BYTES, Kind, Message},
    future_input::{FutureInputError, FutureInputWindow},
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
    },
    CatchingUp,
    Playing,
    Terminal(Option<u8>),
}

struct Peer<A> {
    link: Link,
    connection: Connection,
    slot: Option<usize>,
    phase: Phase,
    deadline: u64,
    commands: VecDeque<(u64, A)>,
    command_sequence: u64,
    held: u64,
    offered: u64,
    stepped: u64,
    reliable: u64,
    packet: u64,
    margin: i16,
    margin_sequence: u64,
    late: u64,
    reliable_sets: u64,
    datagram_sizes: [u64; 188],
    behind_since: Option<u64>,
}

impl<A> Peer<A> {
    fn terminal(&mut self, value: Option<u8>, now: u64, settings: Settings) {
        self.phase = Phase::Terminal(value);
        self.deadline = now + u64::from(settings.response_ms) * 1_000;
    }
}

struct StoredSet {
    tick: u64,
    bytes: Vec<u8>,
}

/// Per-peer observations made at the authority's link boundary.
#[derive(Clone, Debug)]
pub struct PeerStream {
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
    origin: u64,
    next: u64,
    epoch: u64,
    peers: Vec<Peer<W::Action>>,
    owners: [bool; SLOTS],
    ready: [bool; SLOTS],
    joined: u8,
    staging: FutureInputWindow<W::Action>,
    axes: [[f32; 3]; SLOTS],
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
        let mut owners = [false; SLOTS];
        owners[0] = local;
        Ok(Self {
            world,
            codec,
            settings,
            id,
            seed,
            local,
            state: HostState::Lobby,
            origin: now,
            next: now,
            epoch: MATCH_EPOCH,
            peers: Vec::with_capacity(MAX_LINKS),
            owners,
            ready: owners,
            joined: u8::from(local),
            staging,
            axes: [[0.0; 3]; SLOTS],
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
        self.ready.get(slot).copied().unwrap_or_default()
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
                self.sets
                    .iter()
                    .rev()
                    .take_while(|set| set.tick > peer.held)
                    .count()
            })
    }

    /// Read per-link stream counters.
    pub fn stream(&self, link_id: u64) -> Option<PeerStream> {
        let peer = self.peers.iter().find(|peer| peer.link.id() == link_id)?;
        Some(PeerStream {
            late_frames: peer.late,
            reliable_sets: peer.reliable_sets,
            datagram_sizes: peer.datagram_sizes,
            held: peer.held,
            stepped: peer.stepped,
        })
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
        if self.peers.len() == MAX_LINKS
            || self.peers.iter().any(|peer| peer.link.id() == link.id())
        {
            return Err(SessionError::Full);
        }
        self.peers.push(Peer {
            link,
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
            margin: 0,
            margin_sequence: 0,
            late: 0,
            reliable_sets: 0,
            datagram_sizes: [0; 188],
            behind_since: None,
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
        let send_time = now.saturating_sub(self.origin) / 1_000;
        self.baselines(now, send_time);
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
        let mut results = [SlotResult::default(); SLOTS];
        let frames = std::array::from_fn(|slot| {
            let input = self
                .staging
                .take_due(binding(slot, MATCH_EPOCH).peer)
                .expect("registered player slot");
            let mut admitted = frame(slot, MATCH_EPOCH, tick, tick, self.axes[slot]);
            if let Some(input) = input.filter(|_| self.ready[slot]) {
                results[slot].frame = input.origin.sequence;
                admitted.axes = input.axes;
                admitted.actions = input.actions;
            }
            if self.ready[slot]
                && let Some(peer) = self.peers.iter_mut().find(|peer| peer.slot == Some(slot))
            {
                while admitted.actions.len() < input_batch::MAX_ACTIONS {
                    let Some((sequence, action)) = peer.commands.pop_front() else {
                        break;
                    };
                    admitted.actions.push(action);
                    results[slot].last_command = sequence;
                    results[slot].commands += 1;
                }
            }
            admitted
        });
        self.world.barrier(tick);
        if let Err(error) = self.world.step(&frames, self.joined) {
            self.state = HostState::Ended(EndReason::AuthorityStep);
            self.step_error = Some(error);
            return false;
        }
        self.staging
            .commit_tick(tick)
            .expect("complete pre-tick barrier");
        let digest = tick
            .is_multiple_of(u64::from(self.settings.digest_interval))
            .then(|| self.world.digest());
        let set = wire::Set {
            frames,
            results,
            joined: self.joined,
            digest,
        };
        let mut bytes = self.spare.pop().unwrap_or_else(|| Vec::with_capacity(64));
        wire::write_set(&mut bytes, &set, self.axes, &self.codec).expect("validated set encoding");
        self.axes = set.frames.map(|input| input.axes);
        self.admitted = results.map(|result| result.frame != 0);
        self.joined = 0;
        if self.sets.len() == usize::from(self.settings.backlog) {
            let oldest = self.sets.front().expect("full ring").tick;
            for peer in &self.peers {
                if peer.slot.is_some() && peer.held < oldest {
                    peer.link.close(wire::TIMEOUT_CLOSE);
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
            .filter(|peer| peer.slot.is_some() && peer.link.is_open())
            .map(|peer| peer.held)
            .min()
            .unwrap_or_else(|| self.world.tick());
        while self.sets.front().is_some_and(|set| set.tick <= held) {
            self.spare
                .push(self.sets.pop_front().expect("checked front").bytes);
        }
    }

    fn receive(&mut self, now: u64) {
        // Move the reusable table out while processing so the authority can reserve slots.
        let mut peers = std::mem::take(&mut self.peers);
        for peer in &mut peers {
            while let Some(event) = peer.link.event() {
                if let thrum_transport::Event::Closed(_) = event {
                    if let Some(slot) = peer.slot.take() {
                        self.ready[slot] = false;
                        self.owners[slot] = false;
                    }
                    peer.deadline = now;
                }
            }
            for lane in [Lane::Reliable, Lane::Datagram] {
                for _ in 0..128 {
                    let Some(bytes) = peer.link.receive(lane) else {
                        break;
                    };
                    if self.message(peer, now, lane, &bytes).is_none() {
                        peer.link.close(wire::PROTOCOL_CLOSE);
                        break;
                    }
                }
            }
            if now >= peer.deadline {
                peer.link.close(wire::TIMEOUT_CLOSE);
            }
        }
        peers.retain(|peer| peer.deadline > now || peer.slot.is_some());
        self.peers = peers;
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
        if !peer.connection.matches(message.header) {
            return (lane == Lane::Datagram).then_some(());
        }
        match (lane, message.header.kind) {
            (Lane::Datagram, Kind::InputBatch) => self.input(peer, now, message.payload),
            (Lane::Reliable, Kind::Command) => self.command(peer, message),
            _ => None,
        }
    }

    fn hello(&mut self, peer: &mut Peer<W::Action>, now: u64, bytes: &[u8]) -> Option<()> {
        let mut data = Read::new(bytes);
        let protocol = data.u16()?;
        let identity = data.fixed::<32>()?;
        data.done()?;
        let refusal = if protocol != PROTOCOL {
            Some(Refusal::Protocol)
        } else if identity != self.world.identity().0 {
            Some(Refusal::RuleSet)
        } else if self.owners.iter().all(|taken| *taken)
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

        let slot = self.owners.iter().position(|taken| !taken)?;
        let mut bytes = Vec::new();
        if self.world.image(&mut bytes).is_err() || bytes.len() > self.settings.image_limit as usize
        {
            peer.link.close(wire::RESPONSE_CLOSE);
            return Some(());
        }
        self.epoch += 1;
        self.staging
            .reconnect(binding(slot, self.epoch))
            .expect("new slot epoch");
        self.owners[slot] = true;
        peer.slot = Some(slot);
        peer.connection = Connection {
            id: self.id,
            epoch: self.epoch,
        };
        peer.held = self.world.tick();
        peer.offered = peer.held;
        peer.reliable = peer.held;
        peer.stepped = peer.held;
        peer.deadline = now + u64::from(self.settings.join_ms) * 1_000;
        // Baseline axes are snapshotted at precisely the same barrier as the image.
        let mut axes = Vec::with_capacity(SLOTS * 6);
        for axes_value in self.axes {
            wire::write_axes(&mut axes, axes_value);
        }
        axes.append(&mut bytes);
        peer.phase = Phase::Baseline {
            bytes: axes,
            offset: 0,
            began: false,
        };
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
        if sequence != peer.command_sequence + 1 || peer.commands.len() >= MAX_COMMANDS {
            return None;
        }
        peer.commands.push_back((sequence, action));
        peer.command_sequence = sequence;
        Some(())
    }

    fn input(&mut self, peer: &mut Peer<W::Action>, now: u64, payload: &[u8]) -> Option<()> {
        let mut data = Read::new(payload);
        let held = data.u64()?;
        let stepped = data.u64()?;
        if stepped > held || held > peer.offered {
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
                if record.sequence <= previous.0 || record.target_tick <= previous.1 {
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
        peer.held = peer.held.max(held);
        peer.stepped = peer.stepped.max(stepped);
        let backlog = peer.held - peer.stepped;
        if matches!(peer.phase, Phase::CatchingUp)
            && self.world.tick() - peer.held <= u64::from(self.settings.behind / 2)
            && backlog <= u64::from(self.settings.behind)
        {
            self.ready[slot] = true;
            self.joined |= 1 << slot;
            peer.phase = Phase::Playing;
            peer.deadline = u64::MAX;
        }
        if matches!(peer.phase, Phase::Playing) && backlog > u64::from(self.settings.behind) {
            let since = peer.behind_since.get_or_insert(now);
            if now.saturating_sub(*since) >= u64::from(self.settings.join_ms) * 1_000 {
                return None;
            }
        } else {
            peer.behind_since = None;
        }
        if matches!(self.state, HostState::Ended(_)) {
            return Some(());
        }
        let committed = self.world.tick();
        for input in decoded.into_iter().flatten() {
            if input.origin.sequence > peer.margin_sequence {
                let offset = i128::from(input.tick) - i128::from(committed) - 1;
                peer.margin = i16::try_from(offset.clamp(-600, 600)).expect("clamped margin");
                peer.margin_sequence = input.origin.sequence;
            }
            match self.staging.stage(input, |_| Ok(())) {
                Ok(_) | Err(FutureInputError::TooEarly) => {}
                Err(FutureInputError::Late) => peer.late += 1,
                Err(_) => return None,
            }
        }
        Some(())
    }

    fn baselines(&mut self, now: u64, send_time: u64) {
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
                } => {
                    if !*began {
                        let Some(slot) = peer.slot else {
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
                        self.packet.extend(&bytes[..SLOTS * 6]);
                        wire::write_settings(&mut self.packet, self.settings);
                        if wire::send(
                            &peer.link,
                            Lane::Reliable,
                            peer.connection.header(Kind::Welcome, peer.held, 0),
                            &self.packet,
                            send_time,
                            &mut self.scratch,
                        )
                        .is_err()
                        {
                            continue;
                        }
                        *offset = SLOTS * 6;
                        *began = true;
                    }
                    for _ in 0..8 {
                        let end = (*offset + CHUNK).min(bytes.len());
                        let position = (*offset - SLOTS * 6) as u64;
                        let kind = if end == *offset {
                            Kind::BaselineEnd
                        } else {
                            Kind::BaselineChunk
                        };
                        if wire::send(
                            &peer.link,
                            Lane::Reliable,
                            peer.connection.header(kind, peer.held, position),
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
                            peer.deadline = now + u64::from(self.settings.join_ms) * 1_000;
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
            } else if !fresh_tick && (peer.held == tick || peer.offered == tick) {
                continue;
            }
            let count = pack(
                &mut self.packet,
                &self.sets,
                next,
                lane,
                peer.margin,
                peer.margin_sequence,
            );
            if count == 0 && !(fresh_tick && lane == Lane::Datagram) {
                if let Some(reason) = ending
                    && wire::send(
                        &peer.link,
                        Lane::Reliable,
                        peer.connection.header(Kind::MatchEnded, tick, 0),
                        &[reason as u8],
                        send_time,
                        &mut self.scratch,
                    )
                    .is_ok()
                {
                    peer.terminal(None, now, self.settings);
                }
                continue;
            }

            let header = peer
                .connection
                .header(Kind::AdmittedSets, next, peer.packet + 1);
            match wire::send(
                &peer.link,
                lane,
                header,
                &self.packet,
                send_time,
                &mut self.scratch,
            ) {
                Ok(()) => {
                    peer.packet += 1;
                    let last = next - 1 + count as u64;
                    peer.offered = peer.offered.max(last);
                    if lane == Lane::Reliable {
                        peer.reliable = last;
                        peer.reliable_sets += count as u64;
                    } else {
                        peer.datagram_sizes[count] += 1;
                    }
                    offered |= fresh_tick && last == tick;
                }
                Err(LinkError::Full | LinkError::NotOpen) => {}
                Err(_) => peer.link.close(wire::PROTOCOL_CLOSE),
            }
        }
        if offered {
            self.world.set_offered(tick);
        }
    }
}

fn pending(sets: &VecDeque<StoredSet>, next: u64) -> impl Iterator<Item = &StoredSet> {
    let index = sets
        .front()
        .map_or(0, |first| next.saturating_sub(first.tick)) as usize;
    sets.iter().skip(index)
}

fn pack(
    packet: &mut Vec<u8>,
    sets: &VecDeque<StoredSet>,
    next: u64,
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
        if packet.len() + set.bytes.len() + 2 + HEADER_BYTES > cap {
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
