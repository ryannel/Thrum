use std::collections::VecDeque;

use super::{
    ControllerView, DigestPart, Disposition, EndReason, Input, MAX_PENDING, NoPrediction, PROTOCOL,
    PlayerToken, PredictedController, PredictionEvent, Refusal, ResultEvent, Roster, SLOTS,
    SessionError, Settings, World,
    clock::{Clock, INITIAL_BUFFER, Lead, Playout, Skip, Stall},
    frame,
    prediction::Prediction,
    wire::{self, Connection, INPUT_PREFIX, Read},
};
use crate::{
    envelope::{self, HEADER_BYTES, Kind, Message},
    input_batch::{self, ActionWire, EncodeRecord, MAX_ACTIONS, MAX_RECORDS},
};
use thrum_core::sim::InputFrame;
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
    /// Waiting for or installing a repair baseline on the same connection.
    Repairing,
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
    roster: Roster,
    bytes: Vec<u8>,
}

struct Fault {
    tick: u64,
    actual: Option<u64>,
    parts: Vec<DigestPart>,
}

struct Stored<A> {
    tick: u64,
    set: Box<wire::Set<A>>,
    ready: bool,
}

/// One replica. Its world only changes by restoring a link baseline or
/// committing the next contiguous admitted set received from that link.
pub struct Client<W: World, Codec, P: PredictedController<W> = NoPrediction> {
    replica: Replica<W, Codec>,
    prediction: Option<Box<Prediction<W, P>>>,
    link: Link,
    state: ClientState,
    settings: Option<Settings>,
    seed: Option<u64>,
    slot: usize,
    protocol: u16,
    connection: Connection,
    token: Option<PlayerToken>,
    generation: u64,
    installed_generation: u64,
    stale: u64,
    pending_fault: Option<Fault>,
    awaiting_repair: bool,
    leaving: bool,
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
    recent: VecDeque<InputFrame<W::Action>>,
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
    // Retain at most the set buffer's high-water mark of boxes across packets.
    spare: Vec<Box<wire::Set<W::Action>>>,
    packet_sets: Vec<Stored<W::Action>>,
    waiting_frames: VecDeque<(u64, u64)>,
    waiting_commands: VecDeque<u64>,
    results: VecDeque<ResultEvent>,
    fault: Option<Fault>,
    roster: Roster,
    own_present: bool,
    parts: Vec<DigestPart>,
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
        Self::connect(world, codec, link, now, protocol, pinned, None)
    }

    /// A replica using this build's protocol and automatic lead.
    pub fn adaptive(world: W, codec: Codec, link: Link, now: u64) -> Self {
        Self::new(world, codec, link, now, PROTOCOL, None)
    }
}

impl<W: World, Codec: ActionWire<W::Action>, P: PredictedController<W>> Client<W, Codec, P> {
    /// Create a replica with an optional game-declared controller. Passing no
    /// rule constants leaves prediction disabled for this connection.
    pub fn with_controller(
        world: W,
        codec: Codec,
        link: Link,
        now: u64,
        pinned: Option<u16>,
        rules: Option<P::Rules>,
    ) -> Self {
        Self::connect(world, codec, link, now, PROTOCOL, pinned, rules)
    }

    fn connect(
        world: W,
        codec: Codec,
        link: Link,
        now: u64,
        protocol: u16,
        pinned: Option<u16>,
        rules: Option<P::Rules>,
    ) -> Self {
        Self {
            prediction: rules.map(|rules| Box::new(Prediction::new(rules))),
            replica: Replica {
                world,
                codec,
                axes: [[0.0; 3]; SLOTS],
                sets: Vec::new(),
                spare: Vec::new(),
                packet_sets: Vec::new(),
                waiting_frames: VecDeque::new(),
                waiting_commands: VecDeque::new(),
                results: VecDeque::new(),
                fault: None,
                roster: Roster::default(),
                own_present: false,
                parts: Vec::new(),
            },
            link,
            state: ClientState::Connecting,
            settings: None,
            seed: None,
            slot: 0,
            protocol,
            connection: Connection::default(),
            token: None,
            generation: 0,
            installed_generation: 0,
            stale: 0,
            pending_fault: None,
            awaiting_repair: false,
            leaving: false,
            baseline: None,
            ending: None,
            clock: None,
            origin: 0,
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

    /// Map a virtual carrier's time zero to this client's local clock.
    /// Socket links measure queue age on their own clock.
    #[must_use]
    pub fn with_link_origin(mut self, origin_us: u64) -> Self {
        self.origin = origin_us;
        self
    }

    /// Present a previous connection's token in this client's first hello.
    #[must_use]
    pub fn with_token(mut self, token: PlayerToken) -> Self {
        if self.state == ClientState::Connecting {
            self.token = Some(token);
        }
        self
    }

    /// Credential for returning to this slot on a fresh client.
    pub fn token(&self) -> Option<PlayerToken> {
        self.settings.and(self.token)
    }

    /// Negotiated connection epoch.
    pub fn epoch(&self) -> Option<u64> {
        self.settings.map(|_| self.connection.epoch)
    }

    /// Roster at the latest committed replica endpoint.
    pub fn roster(&self) -> Roster {
        self.replica.roster
    }

    /// Stale datagrams from earlier connection epochs.
    pub fn stale_epoch(&self) -> u64 {
        self.stale
    }

    /// Ask the host to release the slot rather than retaining it for rejoin.
    ///
    /// # Errors
    /// The client has no slot, or the link could not accept goodbye yet.
    pub fn goodbye(&mut self, now: u64) -> Result<(), SessionError> {
        if !self.active() || self.settings.is_none() || self.leaving {
            return Err(SessionError::Closed);
        }
        wire::send(
            &self.link,
            Lane::Reliable,
            self.connection.header(Kind::Close, 0, 0),
            &[],
            now.saturating_sub(self.origin) / 1_000,
            &mut self.scratch,
        )
        .map_err(|error| {
            if error == thrum_transport::LinkError::Full {
                SessionError::Full
            } else {
                SessionError::Closed
            }
        })?;
        self.leaving = true;
        if let Some(prediction) = &mut self.prediction {
            prediction.suspend(true);
        }
        Ok(())
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

    /// Sample the playout position between advances, including the final
    /// interval after a clean end reaches its committed endpoint.
    pub fn playout(&self, now: u64) -> Option<Playout> {
        Some(
            self.clock
                .as_ref()?
                .sample(now, self.world().tick(), self.settings?),
        )
    }

    /// Sample the local character's adjacent endpoints with the confirmed
    /// playout fraction. After a drop these are the confirmed endpoints.
    pub fn controller(&self, now: u64) -> Option<ControllerView<P::State>> {
        let prediction = self.prediction.as_ref()?;
        let interpolation = self
            .clock
            .as_ref()
            .zip(self.settings)
            .map_or(1.0, |(clock, settings)| {
                clock.interpolation(now, self.world().tick(), settings)
            });
        prediction.view(interpolation)
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

    /// Read the next resolved timed-frame or command result.
    /// Drain after each advance, even if the game does not use dispositions.
    /// Unread results count towards the input capacity.
    pub fn result(&mut self) -> Option<ResultEvent> {
        self.replica.results.pop_front()
    }

    /// Queue a reliable ordered action, including while an image is in transit.
    ///
    /// # Errors
    /// The action is unencodable, the command/result capacity is full or the match ended.
    pub fn command(&mut self, value: &W::Action) -> Result<u64, SessionError> {
        if !self.active() || self.ending.is_some() || self.leaving {
            return Err(SessionError::Closed);
        }
        if self.replica.waiting_commands.len()
            >= usize::from(self.settings.unwrap_or_default().command_cap)
            || self.pending() >= MAX_PENDING
        {
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
                | ClientState::Repairing
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
        sample: impl FnMut(u64) -> Controls<W::Action>,
    ) -> Result<(), SessionError> {
        self.advance_with_step_budget(now, u8::MAX, sample)
    }

    /// Advance link and input clocks with a caller-imposed limit on replica steps.
    ///
    /// # Errors
    /// Locally sampled controls are unencodable in a single timed frame.
    pub fn advance_with_step_budget(
        &mut self,
        now: u64,
        budget: u8,
        sample: impl FnMut(u64) -> Controls<W::Action>,
    ) -> Result<(), SessionError> {
        self.advance_with_prediction_events(now, budget, sample, |_| {})
    }

    /// Advance as usual and report each bit comparison and correction
    /// synchronously. A game can use corrected endpoints to smooth presentation.
    ///
    /// # Errors
    /// Locally sampled controls are unencodable in a single timed frame.
    pub fn advance_with_prediction_events(
        &mut self,
        now: u64,
        budget: u8,
        mut sample: impl FnMut(u64) -> Controls<W::Action>,
        mut observe: impl FnMut(PredictionEvent<P::State>),
    ) -> Result<(), SessionError> {
        if !self.active() {
            return Ok(());
        }
        let send_time = now.saturating_sub(self.origin) / 1_000;
        if self.state == ClientState::Connecting && self.link.is_open() {
            self.packet.clear();
            self.packet.extend(self.protocol.to_le_bytes());
            self.packet.extend(self.world().identity().0);
            self.packet
                .extend(self.token.map_or([0; 16], |token| token.0));
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
        if self.leaving {
            return Ok(());
        }
        if self.replica.own_present {
            self.send_commands(send_time);
        }
        if self.lead.is_none() && self.newest > self.baseline_tick {
            self.lead = Some(Lead::new(now, self.newest, self.rtt, settings, self.pinned));
        }
        // Sample and offer input before stepping the replica. Catch-up can
        // consume several ticks of work in this poll, even while the input
        // deadline and the independent socket clock keep moving.
        self.author_and_report(now, send_time, settings, &mut sample)?;
        if self.baseline.is_none() && !self.awaiting_repair {
            self.step(now, prior, settings, budget, &mut observe);
        }
        if let Some(fault) = self.replica.fault.take() {
            self.request_repair(fault);
        }
        if let Some(fault) = &self.pending_fault {
            wire::write_divergence(&mut self.packet, fault.actual, &fault.parts);
            let mut header = self.connection.header(Kind::Divergence, fault.tick, 0);
            header.acknowledged_sequence = self.generation;
            if wire::send(
                &self.link,
                Lane::Reliable,
                header,
                &self.packet,
                send_time,
                &mut self.scratch,
            )
            .is_ok()
            {
                self.pending_fault = None;
            }
        }
        if let Some((reason, tick)) = self.ending
            && self.world().tick() == tick
        {
            self.terminate(ClientState::Ended(reason, tick), Disposition::NotCommitted);
            self.link.close(wire::RESPONSE_CLOSE);
            return Ok(());
        }

        Ok(())
    }

    fn author_and_report(
        &mut self,
        now: u64,
        send_time: u64,
        settings: Settings,
        sample: &mut impl FnMut(u64) -> Controls<W::Action>,
    ) -> Result<(), SessionError> {
        let mut authored = false;
        if self.ending.is_none() {
            for _ in 0..settings.catch_up {
                if self.pending() >= MAX_PENDING {
                    break;
                }
                let Some(target) = self
                    .lead
                    .as_mut()
                    .and_then(|lead| lead.author(now, self.newest, settings))
                    .filter(|target| *target > self.last_target)
                else {
                    break;
                };
                if authored {
                    self.send_report(send_time);
                }
                self.author(target, sample(target), settings)?;
                authored = true;
            }
        }
        let report_due = now >= self.report_at;
        if report_due {
            self.report_at += ((now - self.report_at) / u64::from(settings.tick_us) + 1)
                * u64::from(settings.tick_us);
        }
        // During an image transfer, an existing player continues to send
        // controls. Its old progress is ignored by the new stream generation.
        if (authored || report_due) && self.installed_generation > 0 {
            self.send_report(send_time);
        }
        Ok(())
    }

    fn request_repair(&mut self, fault: Fault) {
        self.state = ClientState::Repairing;
        self.awaiting_repair = true;
        self.pending_fault = Some(fault);
        self.clock = None;
        if let Some(prediction) = &mut self.prediction {
            prediction.suspend(false);
        }
    }

    fn terminate(&mut self, state: ClientState, disposition: Disposition) {
        if !matches!(state, ClientState::Ended(..)) {
            self.clock = None;
        }
        self.state = state;
        self.replica.packet_sets.clear();
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
        self.pending_fault = None;
        if let Some(prediction) = &mut self.prediction {
            prediction.suspend(true);
        }
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
                Kind::Reject
                    if Connection::default().matches(message.header)
                        && message.header.acknowledged_sequence == 0 =>
                {
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
        let header = message.header;
        if !self.connection.matches(header) {
            if lane == Lane::Datagram && header.match_id == self.connection.id {
                self.stale += 1;
            }
            return (lane == Lane::Datagram).then_some(());
        }
        if lane == Lane::Datagram && header.acknowledged_sequence != self.generation {
            return Some(());
        }
        match (lane, message.header.kind) {
            (Lane::Reliable, Kind::BaselineBegin) => self.welcome(now, message),
            (Lane::Reliable, Kind::BaselineChunk | Kind::BaselineEnd)
                if header.acknowledged_sequence == self.generation =>
            {
                self.image(message)
            }
            (_, Kind::AdmittedSets) if header.acknowledged_sequence == self.generation => {
                if self.baseline.is_some() {
                    (lane == Lane::Datagram).then_some(())
                } else if self.awaiting_repair {
                    Some(())
                } else {
                    self.sets(lane, message)
                }
            }
            (Lane::Reliable, Kind::MatchEnded) if header.acknowledged_sequence == 0 => {
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
            || header.acknowledged_sequence != self.generation + 1
            || header.server_tick > u64::MAX - 100_000
        {
            return None;
        }
        let mut data = Read::new(message.payload);
        let slot = usize::from(data.byte()?);
        let seed = data.u64()?;
        let size = u32::from_le_bytes(data.fixed()?) as usize;
        let token = PlayerToken(data.fixed()?);
        let roster = Roster {
            occupied: data.byte()?,
            away: data.byte()?,
        };
        let commands = data.u64()?;
        let mut axes = [[0.0; 3]; SLOTS];
        for axis in &mut axes {
            *axis = wire::read_axes(&mut data)?;
        }
        let settings = wire::read_settings(&mut data)?;
        data.done()?;
        let first = self.generation == 0;
        if slot >= SLOTS
            || size > settings.image_limit as usize
            || token.0 == [0; 16]
            || roster.occupied > 15
            || roster.away & !roster.occupied != 0
            || commands > self.command_sequence
            || (!first
                && (self.settings != Some(settings)
                    || self.slot != slot
                    || self.token != Some(token)
                    || self.seed != Some(seed)
                    || header.server_tick < self.world().tick()
                    || self.ending.is_some()))
        {
            return None;
        }
        self.token = Some(token);
        self.generation = header.acknowledged_sequence;
        self.clock = None;
        self.awaiting_repair = false;
        self.pending_fault = None;
        for entry in &mut self.replica.sets {
            if let Some(stored) = entry.take() {
                self.replica.spare.push(stored.set);
            }
        }
        while self
            .replica
            .waiting_frames
            .front()
            .is_some_and(|(tick, _)| *tick <= header.server_tick)
        {
            let (_, sequence) = self.replica.waiting_frames.pop_front()?;
            self.replica.results.push_back(ResultEvent {
                input: Input::Frame(sequence),
                disposition: Disposition::Uncertain,
            });
        }
        while self
            .replica
            .waiting_commands
            .front()
            .is_some_and(|sequence| *sequence <= commands)
        {
            let sequence = self.replica.waiting_commands.pop_front()?;
            self.replica.results.push_back(ResultEvent {
                input: Input::Command(sequence),
                disposition: Disposition::Uncertain,
            });
        }
        self.replica.axes = axes;
        self.held = header.server_tick;
        self.newest = self.held;
        self.baseline_tick = self.held;
        self.slot = slot;
        self.seed = Some(seed);
        self.settings = Some(settings);
        if let Some(prediction) = &mut self.prediction {
            prediction.suspend(self.connection.epoch != header.connection_epoch);
        }
        self.connection = Connection {
            id: header.match_id,
            epoch: header.connection_epoch,
        };
        if first {
            self.rtt = now - self.hello;
        }
        self.replica
            .sets
            .resize_with(usize::from(settings.backlog), || None);
        self.baseline = Some(Baseline {
            tick: self.held,
            size,
            roster,
            bytes: Vec::with_capacity(size),
        });
        self.state = if first {
            ClientState::Synchronizing
        } else {
            ClientState::Repairing
        };
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
            let baseline = self.baseline.take()?;
            if self
                .replica
                .world
                .restore(baseline.tick, &baseline.bytes)
                .is_err()
            {
                self.request_repair(Fault {
                    tick: baseline.tick,
                    actual: None,
                    parts: Vec::new(),
                });
            } else {
                self.installed_generation = self.generation;
                self.replica.roster = baseline.roster;
                if let Some(prediction) = &mut self.prediction {
                    prediction.install(
                        baseline.tick,
                        P::read(&self.replica.world, self.slot),
                        self.replica.axes[self.slot],
                    );
                }
                if self.generation > 1 {
                    let roster = baseline.roster;
                    self.replica.own_present =
                        (roster.occupied & !roster.away) & (1 << self.slot) != 0;
                }
            }
        }
        Some(())
    }

    fn sets(&mut self, lane: Lane, message: Message<'_>) -> Option<()> {
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
        // Decode a packet before publishing any of its sets. Axes that hold
        // their previous value are filled only when the set becomes contiguous.
        let mut encoded = Read::new(bytes);
        let mut remaining = bytes.len();
        let mut obsolete = 0;
        while remaining != 0 {
            let size = usize::from(encoded.u16()?);
            if !(5..=wire::MAX_SET).contains(&size) {
                return None;
            }
            let bytes = encoded.take(size)?;
            remaining -= size + 2;
            if tick <= self.held {
                obsolete += 1;
            } else if tick <= stepped + cap {
                let index = usize::try_from(tick % cap).expect("bounded backlog");
                if self.replica.sets[index].is_none() {
                    let set = wire::read_set(bytes, tick, [[0.0; 3]; SLOTS], &self.replica.codec)?;
                    if set.digest.is_some()
                        != tick.is_multiple_of(u64::from(settings.digest_interval))
                    {
                        return None;
                    }
                    let set = if let Some(mut saved) = self.replica.spare.pop() {
                        *saved = set;
                        saved
                    } else {
                        Box::new(set)
                    };
                    self.replica.packet_sets.push(Stored {
                        tick,
                        set,
                        ready: false,
                    });
                }
            }
            tick += 1;
        }
        let held = self
            .replica
            .checked_held(self.held, stepped + cap, self.slot)?;
        if tick > message.header.server_tick {
            self.newest = self.newest.max(tick - 1);
        }
        self.obsolete += obsolete;
        for stored in self.replica.packet_sets.drain(..) {
            let index = usize::try_from(stored.tick % cap).expect("bounded backlog");
            self.replica.sets[index] = Some(stored);
        }
        while self.held < held {
            let next = self.held + 1;
            self.replica.hold(next, self.slot);
            self.held = next;
        }
        if let Some(lead) = &mut self.lead {
            lead.observe(margin_sequence, margin);
        }
        Some(())
    }

    fn step(
        &mut self,
        now: u64,
        prior: u64,
        settings: Settings,
        mut budget: u8,
        observe: &mut impl FnMut(PredictionEvent<P::State>),
    ) {
        let stepped = self.world().tick();
        let replica = &mut self.replica;
        let slot = self.slot;
        let mut committed = |world: &W, frame: &InputFrame<W::Action>, verified| {
            if let Some(prediction) = &mut self.prediction {
                prediction.confirm(
                    frame.tick,
                    P::read(world, slot),
                    frame.axes,
                    verified,
                    observe,
                );
            }
        };
        if let Some(clock) = &mut self.clock {
            if self.held > prior {
                clock.observe(self.held, now, settings);
            }
            clock.advance(now, stepped, self.held, settings, || {
                if budget == 0 {
                    return false;
                }
                budget -= 1;
                replica.commit(slot, &mut committed)
            });
            self.state = if clock.behind() {
                ClientState::Behind
            } else {
                ClientState::Playing
            };
        } else if self.held - stepped > u64::from(settings.behind) {
            for _ in 0..settings.catch_up.min(budget) {
                if !replica.commit(slot, &mut committed) {
                    break;
                }
            }
        } else if self.held - stepped >= INITIAL_BUFFER || self.ending.is_some() {
            self.clock = Some(Clock::new(stepped, now));
            self.state = ClientState::Playing;
        }
        if self.state == ClientState::Playing
            && replica.fault.is_none()
            && let Some(prediction) = &mut self.prediction
        {
            prediction.resume();
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
        let bytes = input_batch::encode(
            &[EncodeRecord {
                frame: &authored,
                actor_wire_id: self.slot as u64 + 1,
            }],
            &self.replica.codec,
            &mut storage,
        )
        .map_err(|_| SessionError::Input)?;
        if let Some(prediction) = &mut self.prediction
            && let Some(predicted) =
                prediction_frame(&authored, &self.replica.codec, &storage[..bytes])
        {
            prediction.author(predicted, self.clock.as_ref().is_some_and(Clock::stalled));
        }
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
        let (held, stepped) = if self.installed_generation == self.generation {
            (self.held, self.world().tick())
        } else {
            // The authority ignores old progress but can still admit timed input.
            (0, 0)
        };
        self.packet.extend(held.to_le_bytes());
        self.packet.extend(stepped.to_le_bytes());
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
        let mut header = self
            .connection
            .header(Kind::InputBatch, 0, self.frame_sequence);
        header.acknowledged_sequence = self.installed_generation;
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

fn prediction_frame<A>(
    authored: &InputFrame<A>,
    codec: &impl ActionWire<A>,
    bytes: &[u8],
) -> Option<InputFrame<A>> {
    let axes = input_batch::canonical_axes(authored.axes).ok()?;
    let mut frame = InputFrame::new(authored.origin, authored.actor, authored.tick, axes);
    if !authored.actions.is_empty() {
        let decoded = input_batch::decode(bytes).ok()?;
        for action in decoded.records().next()?.actions() {
            frame.actions.push(codec.decode(action).ok()?);
        }
    }
    Some(frame)
}

impl<W: World, Codec: ActionWire<W::Action>> Replica<W, Codec> {
    // Validate the whole newly contiguous run, including sets already buffered
    // past a gap, before publishing progress or consuming any input receipts.
    fn checked_held(&self, mut held: u64, end: u64, slot: usize) -> Option<u64> {
        let mut incoming = self.packet_sets.iter().peekable();
        let mut commands_used = 0;
        while held < end {
            let tick = held + 1;
            let index = usize::try_from(tick % self.sets.len() as u64).expect("bounded backlog");
            let stored = if incoming.peek().is_some_and(|stored| stored.tick == tick) {
                incoming.next()
            } else {
                self.sets[index].as_ref()
            };
            let Some(stored) = stored.filter(|stored| stored.tick == tick) else {
                break;
            };
            let result = stored.set.results[slot];
            if stored.ready
                || (result.frame != 0
                    && self
                        .waiting_frames
                        .binary_search(&(tick, result.frame))
                        .is_err())
            {
                return None;
            }
            let commands = usize::from(result.commands);
            let first = result
                .last_command
                .saturating_sub(u64::from(result.commands).saturating_sub(1));
            for (offset, sequence) in (first..=result.last_command).take(commands).enumerate() {
                if self.waiting_commands.get(commands_used + offset) != Some(&sequence) {
                    return None;
                }
            }
            commands_used += commands;
            held = tick;
        }
        Some(held)
    }

    fn hold(&mut self, tick: u64, slot: usize) {
        let index = usize::try_from(tick % self.sets.len() as u64).expect("bounded backlog");
        let stored = self.sets[index].as_mut().expect("validated contiguous set");
        for (slot, frame) in stored.set.frames.iter_mut().enumerate() {
            if stored.set.changed_axes & (1 << slot) == 0 {
                frame.axes = self.axes[slot];
            }
        }
        let result = stored.set.results[slot];
        self.axes = std::array::from_fn(|slot| stored.set.frames[slot].axes);
        stored.ready = true;
        self.resolve(tick, result);
    }

    fn resolve(&mut self, tick: u64, result: wire::SlotResult) {
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
        for sequence in self.waiting_commands.drain(..usize::from(result.commands)) {
            self.results.push_back(ResultEvent {
                input: Input::Command(sequence),
                disposition: Disposition::Committed(tick),
            });
        }
    }

    fn commit(
        &mut self,
        slot: usize,
        committed: &mut impl FnMut(&W, &InputFrame<W::Action>, Option<wire::SlotResult>),
    ) -> bool {
        let tick = self.world.tick() + 1;
        let index = usize::try_from(tick % self.sets.len() as u64).expect("bounded backlog");
        if self.sets[index]
            .as_ref()
            .is_none_or(|stored| stored.tick != tick || !stored.ready)
        {
            return false;
        }
        let set = self.sets[index].take().expect("checked ready set").set;
        let stepped = self.world.step(&set.frames, set.events).is_ok();
        if stepped {
            self.roster.apply(set.events);
            let bit = 1 << slot;
            if (set.events.joined | set.events.back) & bit != 0 {
                self.own_present = true;
            } else if (set.events.away | set.events.left) & bit != 0 {
                self.own_present = false;
            }
        }
        let actual = if stepped && set.digest.is_some() {
            Some(wire::digest_parts(&self.world, &mut self.parts))
        } else {
            None
        };
        let agrees = stepped && actual == set.digest;
        if stepped {
            // A digest disagreement still leaves a new committed endpoint.
            let verified = agrees.then_some(set.results[slot]);
            committed(&self.world, &set.frames[slot], verified);
        }
        self.spare.push(set);
        if !agrees {
            self.fault = Some(Fault {
                tick,
                actual,
                parts: if actual.is_some() {
                    self.parts.clone()
                } else {
                    Vec::new()
                },
            });
        }
        agrees
    }
}
