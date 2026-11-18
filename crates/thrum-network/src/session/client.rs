use std::collections::VecDeque;

use super::{
    Disposition, EndReason, Input, MAX_COMMANDS, MAX_PENDING, PROTOCOL, Refusal, ResultEvent,
    SLOTS, SessionError, Settings, World,
    clock::{Clock, INITIAL_BUFFER, Lead, Playout, Skip, Stall},
    frame,
    wire::{self, Connection, INPUT_PREFIX, Read},
};
use crate::{
    envelope::{self, HEADER_BYTES, Kind, Message},
    input_batch::{self, ActionWire, EncodeRecord, MAX_ACTIONS, MAX_RECORDS},
};
use thrum_transport::{CloseReason, DATAGRAM_LIMIT, Event, Lane, Link};

/// State a game can present for its connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientState {
    /// Waiting for the link.
    Connecting,
    /// Hello sent.
    Negotiating,
    /// The host returned a reason before closing.
    Refused(Refusal),
    /// Restoring a baseline and catching up.
    Synchronizing,
    /// Playing with ordinary playout.
    Playing,
    /// Skipping a backlog above the behind threshold.
    Behind,
    /// The committed authority endpoint was reached.
    Ended(EndReason, u64),
    /// The link closed without a final authority endpoint.
    Closed(CloseReason),
    /// A local world step failed, or its digest disagreed.
    Diverged(u64),
}

/// Controls sampled only when the authoring clock chooses a new target tick.
pub struct Controls<A> {
    /// Normalized axes for this timed frame.
    pub axes: [f32; 3],
    /// Timed actions to include in this frame.
    pub actions: Vec<A>,
}

impl<A> Controls<A> {
    /// Axes with no timed actions.
    pub fn axes(axes: [f32; 3]) -> Self {
        Self {
            axes,
            actions: Vec::new(),
        }
    }
}

struct Baseline {
    tick: u64,
    size: usize,
    bytes: Vec<u8>,
}

enum Payload<A> {
    Encoded(Vec<u8>),
    Ready(Box<wire::Set<A>>),
}

struct Stored<A> {
    tick: u64,
    payload: Payload<A>,
}

/// One replica. Its world only changes by restoring a link baseline or
/// committing the next contiguous admitted set received from that link.
pub struct Client<W: World, Codec> {
    replica: Replica<W, Codec>,
    link: Link,
    state: ClientState,
    settings: Option<Settings>,
    seed: Option<u64>,
    slot: usize,
    protocol: u16,
    connection: Connection,
    baseline: Option<Baseline>,
    ending: Option<(EndReason, u64)>,
    clock: Option<Clock>,
    origin: u64,
    hello: u64,
    rtt: u64,
    lead: Option<Lead>,
    pinned: Option<u16>,
    held: u64,
    newest: u64,
    baseline_tick: u64,
    packet: Vec<u8>,
    scratch: Box<[u8]>,
    recent: VecDeque<thrum_core::sim::InputFrame<W::Action>>,
    repeat_limit: u8,
    commands: VecDeque<(u64, Vec<u8>)>,
    frame_sequence: u64,
    command_sequence: u64,
    last_target: u64,
    report_at: u64,
    obsolete: u64,
}

struct Replica<W: World, Codec> {
    world: W,
    codec: Codec,
    axes: [[f32; 3]; SLOTS],
    sets: Vec<Option<Stored<W::Action>>>,
    spare: Vec<Vec<u8>>,
    // Preserve the boxes at the high-water mark of the bounded receive buffer.
    ready_spare: Vec<Box<wire::Set<W::Action>>>,
    waiting_frames: VecDeque<(u64, u64)>,
    waiting_commands: VecDeque<u64>,
    results: VecDeque<ResultEvent>,
    fault: Option<ClientState>,
}

impl<W: World, Codec: ActionWire<W::Action>> Client<W, Codec> {
    /// Construct a fresh replica. Protocol and fixed lead can be set for
    /// compatibility and input-lead controls in the headless proof.
    pub fn new(
        world: W,
        codec: Codec,
        link: Link,
        now: u64,
        protocol: u16,
        pinned: Option<u16>,
    ) -> Self {
        Self {
            replica: Replica {
                world,
                codec,
                axes: [[0.0; 3]; SLOTS],
                sets: Vec::new(),
                spare: Vec::new(),
                ready_spare: Vec::new(),
                waiting_frames: VecDeque::new(),
                waiting_commands: VecDeque::new(),
                results: VecDeque::new(),
                fault: None,
            },
            link,
            state: ClientState::Connecting,
            settings: None,
            seed: None,
            slot: 0,
            protocol,
            connection: Connection::default(),
            baseline: None,
            ending: None,
            clock: None,
            origin: now,
            hello: now,
            rtt: 0,
            lead: None,
            pinned,
            held: 0,
            newest: 0,
            baseline_tick: 0,
            packet: Vec::with_capacity(DATAGRAM_LIMIT),
            scratch: wire::scratch(),
            recent: VecDeque::with_capacity(MAX_RECORDS),
            repeat_limit: u8::MAX,
            commands: VecDeque::new(),
            frame_sequence: 0,
            command_sequence: 0,
            last_target: 0,
            report_at: now,
            obsolete: 0,
        }
    }

    /// A replica using this build's protocol and automatic lead.
    pub fn adaptive(world: W, codec: Codec, link: Link, now: u64) -> Self {
        Self::new(world, codec, link, now, PROTOCOL, None)
    }

    /// Limit this client's redundant frames within the host's advertised maximum.
    /// The newest frame is always retained even if the caller passes zero.
    #[must_use]
    pub fn with_repeat_limit(mut self, depth: u8) -> Self {
        self.repeat_limit = depth.max(1);
        let limit = self.settings.map_or(self.repeat_limit, |settings| {
            settings.repeats.min(self.repeat_limit)
        });
        while self.recent.len() > usize::from(limit) {
            self.recent.pop_front();
        }
        self
    }

    /// The committed replica world.
    pub fn world(&self) -> &W {
        &self.replica.world
    }

    /// Current connection state, including the host's terminal reason.
    pub fn state(&self) -> &ClientState {
        &self.state
    }

    /// Negotiated match settings.
    pub fn settings(&self) -> Option<Settings> {
        self.settings
    }

    /// Negotiated match seed.
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Assigned slot after welcome.
    pub fn slot(&self) -> Option<usize> {
        self.settings.map(|_| self.slot)
    }

    /// Bytes restored so far of the current baseline.
    pub fn baseline_progress(&self) -> Option<(usize, usize)> {
        self.baseline
            .as_ref()
            .map(|baseline| (baseline.bytes.len(), baseline.size))
    }

    /// Sample the playout position between advances.
    pub fn playout(&self, now: u64) -> Option<Playout> {
        Some(
            self.clock
                .as_ref()?
                .sample(now, self.world().tick(), self.settings?),
        )
    }

    /// Take the stall that just ended.
    pub fn take_stall(&mut self) -> Option<Stall> {
        self.clock.as_mut()?.completed_stall.take()
    }

    /// Take the range of ticks skipped by the last advance.
    pub fn take_skip(&mut self) -> Option<Skip> {
        self.clock.as_mut()?.skip.take()
    }

    /// Tick assigned to the last authored frame and its sequence.
    pub fn last_frame(&self) -> (u64, u64) {
        (self.last_target, self.frame_sequence)
    }

    /// Estimated lead in ticks from the observed set stream and handshake.
    pub fn reported_lead(&self) -> Option<i64> {
        let lead = self.lead.as_ref()?;
        let distance =
            i128::from(lead.target) - i128::from(self.newest) - i128::from(lead.half_rtt);
        i64::try_from(distance).ok()
    }

    /// Count changes made by the margin controller.
    pub fn lead_changes(&self) -> u64 {
        self.lead.as_ref().map_or(0, |lead| lead.changes)
    }

    /// Set records discarded because the tick was already held.
    pub fn obsolete_sets(&self) -> u64 {
        self.obsolete
    }

    /// Last observed authority tick in a set.
    pub fn newest(&self) -> u64 {
        self.newest
    }

    /// Highest tick for which a client holds all earlier sets.
    pub fn held(&self) -> u64 {
        self.held
    }

    /// Read the next resolved result. Poll this as part of consuming session events.
    pub fn result(&mut self) -> Option<ResultEvent> {
        self.replica.results.pop_front()
    }

    /// Queue a reliable ordered action, including while an image is in transit.
    ///
    /// # Errors
    /// The action is unencodable, the command/result capacity is full or the match ended.
    pub fn command(&mut self, value: &W::Action) -> Result<u64, SessionError> {
        if !self.active() || self.ending.is_some() {
            return Err(SessionError::Closed);
        }
        if self.replica.waiting_commands.len() == MAX_COMMANDS || self.pending() >= MAX_PENDING {
            return Err(SessionError::Full);
        }
        let mut bytes = Vec::new();
        wire::action(&mut bytes, value, &self.replica.codec).ok_or(SessionError::Input)?;
        self.command_sequence += 1;
        self.commands.push_back((self.command_sequence, bytes));
        self.replica
            .waiting_commands
            .push_back(self.command_sequence);
        Ok(self.command_sequence)
    }

    fn pending(&self) -> usize {
        self.replica.waiting_frames.len()
            + self.replica.waiting_commands.len()
            + self.replica.results.len()
    }

    fn active(&self) -> bool {
        matches!(
            self.state,
            ClientState::Connecting
                | ClientState::Negotiating
                | ClientState::Synchronizing
                | ClientState::Playing
                | ClientState::Behind
        )
    }

    /// Drain available messages, commit according to the caller's time and
    /// sample controls if a new target is due.
    ///
    /// # Errors
    /// Locally sampled controls are unencodable in a single timed frame.
    pub fn advance(
        &mut self,
        now: u64,
        mut sample: impl FnMut(u64) -> Controls<W::Action>,
    ) -> Result<(), SessionError> {
        if !self.active() {
            return Ok(());
        }
        let send_time = now.saturating_sub(self.origin) / 1_000;
        if self.state == ClientState::Connecting && self.link.is_open() {
            self.packet.clear();
            self.packet.extend(self.protocol.to_le_bytes());
            self.packet.extend(self.world().identity().0);
            if wire::send(
                &self.link,
                Lane::Reliable,
                Connection::default().header(Kind::Hello, 0, 0),
                &self.packet,
                send_time,
                &mut self.scratch,
            )
            .is_ok()
            {
                self.hello = now;
                self.state = ClientState::Negotiating;
            }
        }
        while let Some(event) = self.link.event() {
            if let Event::Closed(reason) = event {
                self.terminate(ClientState::Closed(reason), Disposition::Uncertain);
                return Ok(());
            }
        }
        let prior = self.held;
        self.receive(now);
        if !self.active() {
            return Ok(());
        }
        let Some(settings) = self.settings else {
            return Ok(());
        };
        self.send_commands(send_time);
        if self.baseline.is_some() {
            return Ok(());
        }
        self.step(now, prior, settings);
        if self.lead.is_none() && self.newest > self.baseline_tick {
            self.lead = Some(Lead::new(now, self.newest, self.rtt, settings, self.pinned));
        }
        if let Some(fault) = self.replica.fault.take() {
            self.terminate(fault, Disposition::Uncertain);
            self.link.close(wire::PROTOCOL_CLOSE);
            return Ok(());
        }
        if let Some((reason, tick)) = self.ending
            && self.world().tick() == tick
        {
            self.terminate(ClientState::Ended(reason, tick), Disposition::NotCommitted);
            self.link.close(wire::RESPONSE_CLOSE);
            return Ok(());
        }

        let pending = self.pending();
        if self.ending.is_none()
            && let Some(lead) = &mut self.lead
            && let Some(target) = lead.author(now, self.newest, settings)
            && target > self.last_target
            && pending < MAX_PENDING
        {
            self.author(target, sample(target), settings)?;
        }
        if now >= self.report_at {
            self.report_at += ((now - self.report_at) / u64::from(settings.tick_us) + 1)
                * u64::from(settings.tick_us);
            self.send_report(send_time);
        }
        Ok(())
    }

    fn terminate(&mut self, state: ClientState, disposition: Disposition) {
        self.state = state;
        self.replica
            .results
            .extend(
                self.replica
                    .waiting_frames
                    .drain(..)
                    .map(|(_, sequence)| ResultEvent {
                        input: Input::Frame(sequence),
                        disposition,
                    }),
            );
        self.replica
            .results
            .extend(
                self.replica
                    .waiting_commands
                    .drain(..)
                    .map(|sequence| ResultEvent {
                        input: Input::Command(sequence),
                        disposition,
                    }),
            );
        self.commands.clear();
        self.baseline = None;
        self.clock = None;
    }

    fn receive(&mut self, now: u64) {
        for lane in [Lane::Reliable, Lane::Datagram] {
            for _ in 0..128 {
                let Some(bytes) = self.link.receive(lane) else {
                    break;
                };
                if self.message(now, lane, &bytes).is_none() {
                    self.terminate(
                        ClientState::Closed(CloseReason::Protocol),
                        Disposition::Uncertain,
                    );
                    self.link.close(wire::PROTOCOL_CLOSE);
                }
                if !self.active() {
                    return;
                }
            }
        }
    }

    fn message(&mut self, now: u64, lane: Lane, bytes: &[u8]) -> Option<()> {
        let message = envelope::decode(bytes).ok()?;
        if self.state == ClientState::Negotiating {
            if lane != Lane::Reliable {
                return Some(());
            }
            return match message.header.kind {
                Kind::Welcome => self.welcome(now, message),
                Kind::Reject if Connection::default().matches(message.header) => {
                    let refusal = match message.payload {
                        [1] => Refusal::Protocol,
                        [2] => Refusal::RuleSet,
                        [3] => Refusal::Full,
                        _ => return None,
                    };
                    self.terminate(ClientState::Refused(refusal), Disposition::Refused);
                    self.link.close(wire::RESPONSE_CLOSE);
                    Some(())
                }
                _ => None,
            };
        }
        if !self.connection.matches(message.header) {
            return (lane == Lane::Datagram).then_some(());
        }
        match (lane, message.header.kind) {
            (Lane::Reliable, Kind::BaselineChunk | Kind::BaselineEnd) => self.image(message),
            (_, Kind::AdmittedSets) => self.sets(lane, message),
            (Lane::Reliable, Kind::MatchEnded) => {
                let reason = match message.payload {
                    [1] => EndReason::AuthorityStep,
                    [2] => EndReason::Stopped,
                    _ => return None,
                };
                if self.baseline.is_some()
                    || self.ending.is_some()
                    || message.header.server_tick != self.held
                {
                    return None;
                }
                self.ending = Some((reason, message.header.server_tick));
                Some(())
            }
            _ => None,
        }
    }

    fn welcome(&mut self, now: u64, message: Message<'_>) -> Option<()> {
        let header = message.header;
        if header.match_id == [0; 16]
            || header.connection_epoch == 0
            || header.message_sequence != 0
            || header.acknowledged_sequence != 0
            || header.server_tick > u64::MAX - 100_000
        {
            return None;
        }
        let mut data = Read::new(message.payload);
        let slot = usize::from(data.byte()?);
        let seed = data.u64()?;
        let size = u32::from_le_bytes(data.fixed()?) as usize;
        let mut axes = [[0.0; 3]; SLOTS];
        for axis in &mut axes {
            *axis = wire::read_axes(&mut data)?;
        }
        let settings = wire::read_settings(&mut data)?;
        data.done()?;
        if slot >= SLOTS || size.checked_sub(SLOTS * 6)? > settings.image_limit as usize {
            return None;
        }
        let size = size - SLOTS * 6;
        self.replica.axes = axes;
        self.held = header.server_tick;
        self.newest = self.held;
        self.baseline_tick = self.held;
        self.slot = slot;
        self.seed = Some(seed);
        self.settings = Some(settings);
        self.connection = Connection {
            id: header.match_id,
            epoch: header.connection_epoch,
        };
        self.rtt = now - self.hello;
        self.replica
            .sets
            .resize_with(usize::from(settings.backlog), || None);
        self.baseline = Some(Baseline {
            tick: self.held,
            size,
            bytes: Vec::with_capacity(size),
        });
        self.state = ClientState::Synchronizing;
        Some(())
    }

    fn image(&mut self, message: Message<'_>) -> Option<()> {
        let baseline = self.baseline.as_mut()?;
        if message.header.server_tick != baseline.tick
            || message.header.message_sequence != baseline.bytes.len() as u64
        {
            return None;
        }
        if message.header.kind == Kind::BaselineChunk {
            if message.payload.is_empty()
                || message.payload.len() > super::CHUNK
                || message.payload.len() > baseline.size - baseline.bytes.len()
            {
                return None;
            }
            baseline.bytes.extend(message.payload);
        } else {
            if !message.payload.is_empty() || baseline.size != baseline.bytes.len() {
                return None;
            }
            self.replica
                .world
                .restore(baseline.tick, &baseline.bytes)
                .ok()?;
            self.baseline = None;
        }
        Some(())
    }

    fn sets(&mut self, lane: Lane, message: Message<'_>) -> Option<()> {
        if self.baseline.is_some() && lane == Lane::Reliable {
            return None;
        }
        if self.ending.is_some() {
            return (lane == Lane::Datagram).then_some(());
        }
        let settings = self.settings?;
        let mut data = Read::new(message.payload);
        let margin = i16::from_le_bytes(data.fixed()?);
        let margin_sequence = data.u64()?;
        if !(-600..=600).contains(&margin) || margin_sequence > self.frame_sequence {
            return None;
        }
        let mut tick = message.header.server_tick;
        let bytes = data.rest();
        if tick == 0 || tick > u64::MAX - 100_000 {
            return None;
        }
        let stepped = self.replica.world.tick().max(self.baseline_tick);
        let cap = u64::from(settings.backlog);
        // Check message bounds before any set from it enters the queue.
        let mut encoded = Read::new(bytes);
        let mut remaining = bytes.len();
        while remaining != 0 {
            let size = usize::from(encoded.u16()?);
            if !(5..=wire::MAX_SET).contains(&size) {
                return None;
            }
            encoded.take(size)?;
            remaining -= 2 + size;
        }
        let mut encoded = Read::new(bytes);
        let mut remaining = bytes.len();
        while remaining != 0 {
            let size = usize::from(encoded.u16()?);
            let set = encoded.take(size)?;
            remaining -= 2 + size;
            if tick <= self.held {
                self.obsolete += 1;
            } else if tick <= stepped + cap {
                let index = usize::try_from(tick % cap).expect("bounded backlog");
                if self.replica.sets[index].is_none() {
                    let mut saved = self
                        .replica
                        .spare
                        .pop()
                        .unwrap_or_else(|| Vec::with_capacity(64));
                    saved.extend(set);
                    self.replica.sets[index] = Some(Stored {
                        tick,
                        payload: Payload::Encoded(saved),
                    });
                }
            }
            self.newest = self.newest.max(tick);
            tick += 1;
        }
        while self.held < stepped + cap {
            let next = self.held + 1;
            if !self.replica.hold(next, self.slot)? {
                break;
            }
            self.held = next;
        }
        if let Some(lead) = &mut self.lead {
            lead.observe(margin_sequence, margin);
        }
        Some(())
    }

    fn step(&mut self, now: u64, prior: u64, settings: Settings) {
        let stepped = self.world().tick();
        let replica = &mut self.replica;
        if let Some(clock) = &mut self.clock {
            if self.held > prior {
                clock.observe(self.held, now, settings);
            }
            clock.advance(now, stepped, self.held, settings, || replica.commit());
            self.state = if clock.skip.is_some() {
                ClientState::Behind
            } else {
                ClientState::Playing
            };
        } else if self.held - stepped > u64::from(settings.behind) {
            for _ in 0..settings.catch_up {
                if !replica.commit() {
                    break;
                }
            }
        } else if self.held - stepped >= INITIAL_BUFFER || self.ending.is_some() {
            self.clock = Some(Clock::new(stepped, now));
            self.state = ClientState::Playing;
        }
    }

    fn author(
        &mut self,
        target: u64,
        sample: Controls<W::Action>,
        settings: Settings,
    ) -> Result<(), SessionError> {
        let sequence = self.frame_sequence + 1;
        if sample.actions.len() > MAX_ACTIONS {
            return Err(SessionError::Input);
        }
        let mut authored = frame(
            self.slot,
            self.connection.epoch,
            sequence,
            target,
            sample.axes,
        );
        authored.actions.extend(sample.actions);
        // Prove at authoring time that the newest frame can enter a datagram on its own.
        let mut storage = [0; DATAGRAM_LIMIT - HEADER_BYTES - INPUT_PREFIX];
        input_batch::encode(
            &[EncodeRecord {
                frame: &authored,
                actor_wire_id: self.slot as u64 + 1,
            }],
            &self.replica.codec,
            &mut storage,
        )
        .map_err(|_| SessionError::Input)?;
        if self.recent.len() == usize::from(settings.repeats.min(self.repeat_limit)) {
            self.recent.pop_front();
        }
        self.recent.push_back(authored);
        self.replica.waiting_frames.push_back((target, sequence));
        self.frame_sequence = sequence;
        self.last_target = target;
        Ok(())
    }

    fn send_commands(&mut self, send_time: u64) {
        if self.ending.is_some() {
            return;
        }
        while let Some((sequence, bytes)) = self.commands.front() {
            let header = self.connection.header(Kind::Command, 0, *sequence);
            if wire::send(
                &self.link,
                Lane::Reliable,
                header,
                bytes,
                send_time,
                &mut self.scratch,
            )
            .is_err()
            {
                break;
            }
            self.commands.pop_front();
        }
    }

    fn send_report(&mut self, send_time: u64) {
        self.packet.clear();
        self.packet.extend(self.held.to_le_bytes());
        self.packet.extend(self.world().tick().to_le_bytes());
        let actor_wire_id = self.slot as u64 + 1;
        let mut encoded = [0; DATAGRAM_LIMIT - HEADER_BYTES - INPUT_PREFIX];
        if let Some(newest) = self.recent.back() {
            let first = EncodeRecord {
                frame: newest,
                actor_wire_id,
            };
            let mut records =
                std::array::from_fn::<_, MAX_RECORDS, _>(|_| EncodeRecord { ..first });
            for (slot, input) in records.iter_mut().zip(&self.recent) {
                *slot = EncodeRecord {
                    frame: input,
                    actor_wire_id,
                };
            }
            for start in 0..self.recent.len() {
                if let Ok(size) = input_batch::encode(
                    &records[start..self.recent.len()],
                    &self.replica.codec,
                    &mut encoded,
                ) {
                    self.packet.extend(&encoded[..size]);
                    break;
                }
            }
        }
        let header = self
            .connection
            .header(Kind::InputBatch, 0, self.frame_sequence);
        match wire::send(
            &self.link,
            Lane::Datagram,
            header,
            &self.packet,
            send_time,
            &mut self.scratch,
        ) {
            Ok(()) | Err(thrum_transport::LinkError::NotOpen) => {}
            Err(_) => self.link.close(wire::PROTOCOL_CLOSE),
        }
    }
}

impl<W: World, Codec: ActionWire<W::Action>> Replica<W, Codec> {
    fn hold(&mut self, tick: u64, slot: usize) -> Option<bool> {
        let index = usize::try_from(tick % self.sets.len() as u64).expect("bounded backlog");
        if self.sets[index].as_ref().is_none_or(|set| set.tick != tick) {
            return Some(false);
        }
        let Some(Stored {
            payload: Payload::Encoded(mut bytes),
            ..
        }) = self.sets[index].take()
        else {
            return None;
        };
        let set = wire::read_set(&bytes, tick, self.axes, &self.codec);
        bytes.clear();
        self.spare.push(bytes);
        let set = set?;
        let result = set.results[slot];
        self.resolve(tick, result)?;
        self.axes = std::array::from_fn(|slot| set.frames[slot].axes);
        let ready = if let Some(mut ready) = self.ready_spare.pop() {
            *ready = set;
            ready
        } else {
            Box::new(set)
        };
        self.sets[index] = Some(Stored {
            tick,
            payload: Payload::Ready(ready),
        });
        Some(true)
    }

    fn resolve(&mut self, tick: u64, result: wire::SlotResult) -> Option<()> {
        let first = result
            .last_command
            .saturating_sub(u64::from(result.commands).saturating_sub(1));
        let commands = usize::from(result.commands);
        if commands > self.waiting_commands.len()
            || self
                .waiting_commands
                .iter()
                .take(commands)
                .copied()
                .ne((first..=result.last_command).take(commands))
        {
            return None;
        }
        if result.frame != 0
            && self
                .waiting_frames
                .binary_search(&(tick, result.frame))
                .is_err()
        {
            return None;
        }
        while self
            .waiting_frames
            .front()
            .is_some_and(|(target, _)| *target <= tick)
        {
            let (target, sequence) = self.waiting_frames.pop_front().expect("checked frame");
            let disposition = if target == tick && sequence == result.frame {
                Disposition::Committed(tick)
            } else {
                Disposition::Refused
            };
            self.results.push_back(ResultEvent {
                input: Input::Frame(sequence),
                disposition,
            });
        }
        for sequence in self.waiting_commands.drain(..commands) {
            self.results.push_back(ResultEvent {
                input: Input::Command(sequence),
                disposition: Disposition::Committed(tick),
            });
        }
        Some(())
    }

    fn commit(&mut self) -> bool {
        let tick = self.world.tick() + 1;
        let index = usize::try_from(tick % self.sets.len() as u64).expect("bounded backlog");
        if self.sets[index]
            .as_ref()
            .is_none_or(|set| set.tick != tick || !matches!(set.payload, Payload::Ready(_)))
        {
            return false;
        }
        let Some(Stored {
            payload: Payload::Ready(set),
            ..
        }) = self.sets[index].take()
        else {
            unreachable!("checked next ready set")
        };
        let stepped = self.world.step(&set.frames, set.joined).is_ok();
        let agrees = stepped
            && set
                .digest
                .is_none_or(|expected| self.world.digest() == expected);
        self.ready_spare.push(set);
        if !agrees {
            self.fault = Some(ClientState::Diverged(tick));
        }
        agrees
    }
}
