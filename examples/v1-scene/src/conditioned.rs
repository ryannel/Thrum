//! Four-peer 10k-world no-socket diagnostic with deterministic delayed input.
//! The same V1 simulation owns gameplay; N1 only stages future frames.

mod confirmed;
mod predicted;
mod visual_delta;

use std::{fmt, num::NonZeroUsize, time::Instant};

use thrum_core::{
    ActorId, Origin, PeerId, StableId,
    sim::{Command, ComponentEntity, InputFrame, Ownership, SimAction, Simulation, Velocity},
};
use thrum_network::{
    envelope::{self, Header, Kind},
    future_input::{FutureInputError, FutureInputWindow, StageResult},
    input_batch::{self, ActionView, ActionWire, EncodeRecord, InputCodecError},
};

use crate::scene::{self, PEER_COUNT};
use confirmed::ConfirmedLoop;
use visual_delta::{VisualDeltaLoop, VisualReport};

const TICKS: u64 = 600;
const LEAD_TICKS: u64 = 5;
const FUTURE_TICKS: usize = 8;
const HISTORY: usize = 4;
const PACKET_CAPACITY: usize = 32;
const PACKET_BYTES: usize = 256;
const SEED: u64 = 0xA5A5_2026_1104;
const PEERS: [PeerId; PEER_COUNT] = [PeerId(1), PeerId(2), PeerId(3), PeerId(4)];
const MATCH_ID: [u8; 16] = [0xA5; 16];
const RECONNECT_TICK: u64 = 300;
const QUIET_START: u64 = 290;
const REPLACEMENT_ID: StableId = StableId(10_000);
const REPLACEMENT_ACTOR: ActorId = ActorId(10_001);

/// Selects an ordinary conditioned run or a drained, single-peer reconnect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionedScenario {
    /// Four uninterrupted controllers.
    Standard,
    /// Peer one stops sending, drains committed results, then adopts a new actor.
    DrainedReconnect,
}

struct SceneActionWire;

impl ActionWire<SimAction> for SceneActionWire {
    fn encode(
        &self,
        action: &SimAction,
        destination: &mut [u8],
    ) -> Result<(u32, usize), InputCodecError> {
        match action {
            SimAction::Impulse(values) => {
                if values.iter().any(|value| !value.is_finite()) {
                    return Err(InputCodecError::InvalidAction);
                }
                let output = destination
                    .get_mut(..24)
                    .ok_or(InputCodecError::DestinationTooSmall)?;
                for (index, value) in values.iter().enumerate() {
                    output[index * 8..index * 8 + 8].copy_from_slice(&value.to_le_bytes());
                }
                Ok((1, 24))
            }
            SimAction::Damage(amount) => {
                destination
                    .get_mut(..2)
                    .ok_or(InputCodecError::DestinationTooSmall)?
                    .copy_from_slice(&amount.to_le_bytes());
                Ok((2, 2))
            }
        }
    }

    fn decode(&self, action: ActionView<'_>) -> Result<SimAction, InputCodecError> {
        match (action.schema_id, action.payload.len()) {
            (1, 24) => {
                let values = std::array::from_fn(|index| {
                    f64::from_le_bytes(
                        action.payload[index * 8..index * 8 + 8]
                            .try_into()
                            .expect("checked impulse length"),
                    )
                });
                if values.iter().any(|value| !value.is_finite()) {
                    return Err(InputCodecError::InvalidAction);
                }
                Ok(SimAction::Impulse(values))
            }
            (2, 2) => Ok(SimAction::Damage(u16::from_le_bytes(
                action.payload.try_into().expect("checked damage length"),
            ))),
            _ => Err(InputCodecError::InvalidAction),
        }
    }
}

#[derive(Clone, Copy)]
struct Sample {
    binding: Ownership,
    target: u64,
    origin: Origin,
    axes: [f32; 3],
}

struct Packet {
    peer: PeerId,
    arrival: u64,
    bytes: [u8; PACKET_BYTES],
    len: usize,
}

#[derive(Clone, Copy)]
struct BurstLoss {
    peer: PeerId,
    first_tick: u64,
    last_tick: u64,
}

struct ConditionedLink {
    random_state: u64,
    pending: Vec<Packet>,
    burst_loss: Option<BurstLoss>,
    sent: u64,
    sent_bytes: u64,
    dropped: u64,
    delivered: u64,
    delivered_bytes: u64,
    encode_ns: u128,
    min_delay: u64,
    max_delay: u64,
}

impl ConditionedLink {
    fn new() -> Self {
        Self {
            random_state: SEED,
            pending: Vec::with_capacity(PACKET_CAPACITY),
            burst_loss: None,
            sent: 0,
            sent_bytes: 0,
            dropped: 0,
            delivered: 0,
            delivered_bytes: 0,
            encode_ns: 0,
            min_delay: u64::MAX,
            max_delay: 0,
        }
    }

    fn random(&mut self) -> u64 {
        let mut x = self.random_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.random_state = x;
        x
    }

    fn send(
        &mut self,
        now: u64,
        newest: Sample,
        datagram_sequence: u64,
        first_created: u64,
        force_delivery: bool,
    ) -> Result<(), String> {
        let started = Instant::now();
        let peer = newest.binding.peer;
        let available = now
            .checked_sub(first_created)
            .and_then(|span| span.checked_add(1))
            .ok_or("packet history start exceeds current tick")?;
        let count = HISTORY.min(usize::try_from(available).map_err(|_| "tick count overflow")?);
        let frames = std::array::from_fn::<_, HISTORY, _>(|age| {
            let created = now
                .saturating_sub(u64::try_from(age).expect("four history slots"))
                .max(first_created);
            let sample = if age == 0 {
                newest
            } else {
                control_sample(newest.binding, created)
            };
            InputFrame {
                origin: sample.origin,
                actor: sample.binding.actor,
                tick: sample.target,
                axes: sample.axes,
                actions: Vec::new(),
            }
        });
        let records = std::array::from_fn::<_, HISTORY, _>(|index| EncodeRecord {
            frame: &frames[index],
            actor_wire_id: frames[index].actor.0,
        });
        let mut payload = [0u8; PACKET_BYTES - envelope::HEADER_BYTES];
        let payload_len = input_batch::encode(&records[..count], &SceneActionWire, &mut payload)
            .map_err(|error| format!("input batch encode: {error:?}"))?;
        let mut bytes = [0u8; PACKET_BYTES];
        let len = envelope::encode(
            Header {
                kind: Kind::InputBatch,
                match_id: MATCH_ID,
                connection_epoch: newest.binding.connection_epoch,
                message_sequence: datagram_sequence,
                server_tick: now,
                acknowledged_sequence: 0,
            },
            &payload[..payload_len],
            &mut bytes,
        )
        .map_err(|error| format!("input envelope encode: {error:?}"))?;
        self.sent += 1;
        self.sent_bytes += u64::try_from(len).map_err(|_| "packet length overflow")?;
        self.encode_ns += started.elapsed().as_nanos();
        let random = self.random();
        if !force_delivery
            && (random % 100 < 2
                || self.burst_loss.is_some_and(|burst| {
                    burst.peer == peer && (burst.first_tick..=burst.last_tick).contains(&now)
                }))
        {
            self.dropped += 1;
            return Ok(());
        }
        if self.pending.len() == PACKET_CAPACITY {
            return Err("conditioned link packet capacity exhausted".to_owned());
        }
        let delay = 2 + (random >> 8) % 3;
        self.min_delay = self.min_delay.min(delay);
        self.max_delay = self.max_delay.max(delay);
        self.pending.push(Packet {
            peer,
            arrival: now + delay,
            bytes,
            len,
        });
        Ok(())
    }

    fn deliver(
        &mut self,
        now: u64,
        mut receive: impl FnMut(PeerId, &[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].arrival <= now {
                let packet = self.pending.swap_remove(index);
                self.delivered += 1;
                self.delivered_bytes +=
                    u64::try_from(packet.len).map_err(|_| "packet length overflow")?;
                receive(packet.peer, &packet.bytes[..packet.len])?;
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}

fn control_sample(binding: Ownership, created: u64) -> Sample {
    let peer = binding.peer;
    let direction = if (created / 45 + peer.0).is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let axes = if peer.0.is_multiple_of(2) {
        [0.0, direction, 0.0]
    } else {
        [direction, 0.0, 0.0]
    };
    Sample {
        binding,
        target: created + LEAD_TICKS,
        origin: Origin {
            peer,
            connection_epoch: binding.connection_epoch,
            sequence: created + LEAD_TICKS,
        },
        axes,
    }
}

fn controlled_positions(sim: &Simulation) -> Result<[[f64; 3]; PEER_COUNT], String> {
    let mut positions = [[0.0; 3]; PEER_COUNT];
    for (index, position) in positions.iter_mut().enumerate() {
        *position = sim
            .controller_state(PEERS[index])
            .map_err(|error| format!("controlled actor: {error:?}"))?
            .position
            .0;
    }
    Ok(positions)
}

/// A measured, local four-peer input run. This is not a network transport result.
#[derive(Debug)]
pub struct ConditionedReport {
    scenario: ConditionedScenario,
    committed_ticks: u64,
    sent_packets: u64,
    sent_bytes: u64,
    dropped_packets: u64,
    delivered_packets: u64,
    delivered_bytes: u64,
    in_flight_packets: usize,
    min_delay: u64,
    max_delay: u64,
    duplicate_frames: u64,
    late_frames: u64,
    due_misses: u64,
    accepted_frames: u64,
    stale_packets: u64,
    confirmed_groups: u64,
    confirmed_in_flight: usize,
    retired_return_groups: u64,
    rebaselines: u64,
    confirmed_max_age: u64,
    confirmed_ticks: [u64; PEER_COUNT],
    predicted_ticks: [u64; PEER_COUNT],
    prediction_matches: u64,
    prediction_corrections: u64,
    prediction_fallbacks: u64,
    prediction_resolved_origins: u64,
    prediction_missed_authored: u64,
    prediction_origin_mismatches: u64,
    replay_steps: u64,
    max_replay_steps: usize,
    max_correction_m: f64,
    prediction_retained_bytes: usize,
    visual: VisualReport,
    tick_p50_ns: u64,
    tick_p95_ns: u64,
    encode_ns: u128,
    decode_ns: u128,
    final_positions: [[f64; 3]; PEER_COUNT],
}

impl fmt::Display for ConditionedReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "scenario={:?} ticks={} peers={} packets sent/dropped/delivered/in_flight={}/{}/{}/{} bytes sent/delivered={}/{} delay_ticks={}-{} duplicate/late/stale={}/{}/{} accepted={} due_misses={} confirmed delivered/in_flight/retired/rebaselines/max_age={}/{}/{}/{}/{} client_ticks={:?} prediction ticks={:?} matches/corrections/fallbacks/replay/max_replay={}/{}/{}/{}/{} origin resolved/missed/mismatch={}/{}/{} max_correction_m={:.6} prediction_retained_bytes={} visual sent/dropped/delivered/stale/reordered={}/{}/{}/{}/{} visual ACK sent/dropped/delivered/rebased={}/{}/{}/{} visual bytes={} visual full images/payload bytes={}/{} visual retained_bytes={} visual unchanged_partitions={} visual max_age={} visual project/encode/decode={}/{}/{} us tick p50/p95={}/{} us encode/decode total={}/{} us final_positions={:?}",
            self.scenario,
            self.committed_ticks,
            PEER_COUNT,
            self.sent_packets,
            self.dropped_packets,
            self.delivered_packets,
            self.in_flight_packets,
            self.sent_bytes,
            self.delivered_bytes,
            self.min_delay,
            self.max_delay,
            self.duplicate_frames,
            self.late_frames,
            self.stale_packets,
            self.accepted_frames,
            self.due_misses,
            self.confirmed_groups,
            self.confirmed_in_flight,
            self.retired_return_groups,
            self.rebaselines,
            self.confirmed_max_age,
            self.confirmed_ticks,
            self.predicted_ticks,
            self.prediction_matches,
            self.prediction_corrections,
            self.prediction_fallbacks,
            self.replay_steps,
            self.max_replay_steps,
            self.prediction_resolved_origins,
            self.prediction_missed_authored,
            self.prediction_origin_mismatches,
            self.max_correction_m,
            self.prediction_retained_bytes,
            self.visual.sent,
            self.visual.dropped,
            self.visual.delivered,
            self.visual.stale,
            self.visual.reordered,
            self.visual.ack_sent,
            self.visual.ack_dropped,
            self.visual.ack_delivered,
            self.visual.rebases,
            self.visual.sent_bytes,
            self.visual.full_images,
            self.visual.full_payload_bytes,
            self.visual.retained_bytes,
            self.visual.unchanged_partitions,
            self.visual.max_age,
            self.visual.project_ns / 1_000,
            self.visual.encode_ns / 1_000,
            self.visual.decode_ns / 1_000,
            self.tick_p50_ns / 1_000,
            self.tick_p95_ns / 1_000,
            self.encode_ns / 1_000,
            self.decode_ns / 1_000,
            self.final_positions
        )
    }
}

struct PacketReceiver<'a> {
    sim: &'a Simulation,
    window: &'a mut FutureInputWindow<SimAction>,
    bindings: &'a [Ownership; PEER_COUNT],
    duplicate_frames: &'a mut u64,
    late_frames: &'a mut u64,
    stale_packets: &'a mut u64,
    decode_ns: &'a mut u128,
}

impl PacketReceiver<'_> {
    fn receive(&mut self, peer: PeerId, bytes: &[u8]) -> Result<(), String> {
        let decode_started = Instant::now();
        let message =
            envelope::decode(bytes).map_err(|error| format!("conditioned envelope: {error:?}"))?;
        let binding = *self
            .bindings
            .iter()
            .find(|binding| binding.peer == peer)
            .ok_or("conditioned packet peer has no current binding")?;
        if message.header.kind != Kind::InputBatch
            || message.header.match_id != MATCH_ID
            || message.header.acknowledged_sequence != 0
        {
            return Err("conditioned input session identity mismatch".to_owned());
        }
        if message.header.connection_epoch != binding.connection_epoch {
            *self.stale_packets += 1;
            return Ok(());
        }
        let batch = input_batch::decode(message.payload)
            .map_err(|error| format!("conditioned input bytes: {error:?}"))?;
        let mut frames = std::array::from_fn::<_, { input_batch::MAX_RECORDS }, _>(|_| None);
        for (slot, record) in frames.iter_mut().zip(batch.records()) {
            let frame = record
                .decode_frame(binding, binding.actor.0, &SceneActionWire)
                .map_err(|error| format!("conditioned game action: {error:?}"))?;
            self.sim
                .validate_frame_actions(&frame)
                .map_err(|error| format!("conditioned W1 action: {error:?}"))?;
            *slot = Some(frame);
        }
        *self.decode_ns += decode_started.elapsed().as_nanos();
        for frame in frames.into_iter().flatten() {
            match self
                .window
                .stage(frame, |frame| self.sim.validate_frame_actions(frame))
            {
                Ok(StageResult::Queued) => {}
                Ok(StageResult::Duplicate) => *self.duplicate_frames += 1,
                Err(FutureInputError::Late) => *self.late_frames += 1,
                Err(error) => return Err(format!("conditioned input staging: {error:?}")),
            }
        }
        Ok(())
    }
}

/// One live no-socket instance of the V1 four-peer workload. Setup reserves
/// every link, input-window and timing slot before the first fixed tick.
pub struct ConditionedGame {
    sim: Simulation,
    scenario: ConditionedScenario,
    bindings: [Ownership; PEER_COUNT],
    window: FutureInputWindow<SimAction>,
    link: ConditionedLink,
    confirmed: ConfirmedLoop,
    visual: VisualDeltaLoop,
    held: [[f32; 3]; PEER_COUNT],
    expected: [[f64; 3]; PEER_COUNT],
    expected_history: Vec<[[f64; 3]; PEER_COUNT]>,
    times: Vec<u64>,
    duplicate_frames: u64,
    late_frames: u64,
    due_misses: u64,
    accepted_frames: u64,
    stale_packets: u64,
    decode_ns: u128,
}

impl ConditionedGame {
    /// Prepare the same 10k V1 world and four peer input owners.
    ///
    /// # Errors
    /// Returns a world construction or timing-window setup fault.
    pub fn new(scenario: ConditionedScenario) -> Result<Self, String> {
        let sim = scene::simulation()?;
        let bindings = PEERS.map(scene::binding);
        let window = FutureInputWindow::new(
            &bindings,
            NonZeroUsize::new(FUTURE_TICKS).ok_or("zero future input horizon")?,
            sim.tick(),
        )
        .map_err(|error| format!("future input setup: {error:?}"))?;
        let expected = controlled_positions(&sim)?;
        let mut states = [sim
            .controller_state(PEERS[0])
            .map_err(|error| format!("initial controller: {error:?}"))?;
            PEER_COUNT];
        for (index, state) in states.iter_mut().enumerate() {
            *state = sim
                .controller_state(PEERS[index])
                .map_err(|error| format!("initial controller: {error:?}"))?;
        }
        let rule = sim.controller_rule();
        let visual = VisualDeltaLoop::new(&sim, &bindings)?;
        Ok(Self {
            sim,
            scenario,
            bindings,
            window,
            link: ConditionedLink::new(),
            confirmed: ConfirmedLoop::new(&states, rule),
            visual,
            held: [[0.0; 3]; PEER_COUNT],
            expected,
            expected_history: Vec::with_capacity(
                usize::try_from(TICKS).map_err(|_| "tick count overflow")?,
            ),
            times: Vec::with_capacity(usize::try_from(TICKS).map_err(|_| "tick count overflow")?),
            duplicate_frames: 0,
            late_frames: 0,
            due_misses: 0,
            accepted_frames: 0,
            stale_packets: 0,
            decode_ns: 0,
        })
    }

    /// Advance one exact fixed tick. N1 stages input; W1 alone commits state.
    ///
    /// # Errors
    /// Returns an explicit link, staging, admission, tick or capacity fault.
    pub fn step(&mut self) -> Result<(), String> {
        if self.sim.tick() >= TICKS {
            return Err("conditioned run is complete".to_owned());
        }
        let tick = self.sim.tick() + 1;
        let started = Instant::now();
        let reconnect =
            self.scenario == ConditionedScenario::DrainedReconnect && tick == RECONNECT_TICK;
        if reconnect {
            self.confirmed.can_rebaseline(0)?;
        }
        for (index, binding) in self.bindings.into_iter().enumerate() {
            if self.scenario == ConditionedScenario::DrainedReconnect
                && index == 0
                && (QUIET_START..=RECONNECT_TICK).contains(&tick)
            {
                if tick < RECONNECT_TICK {
                    self.confirmed.hold_author(index, tick + LEAD_TICKS)?;
                }
                continue;
            }
            let sample = control_sample(binding, tick);
            self.confirmed
                .author(index, sample.target, sample.origin, sample.axes, &[])?;
            let first_created = if self.scenario == ConditionedScenario::DrainedReconnect
                && index == 0
                && binding.connection_epoch > 1
            {
                RECONNECT_TICK + 1
            } else {
                1
            };
            let datagram_sequence = self.visual.next_client_sequence(index)?;
            self.link
                .send(tick, sample, datagram_sequence, first_created, false)?;
        }
        let mut receiver = PacketReceiver {
            sim: &self.sim,
            window: &mut self.window,
            bindings: &self.bindings,
            duplicate_frames: &mut self.duplicate_frames,
            late_frames: &mut self.late_frames,
            stale_packets: &mut self.stale_packets,
            decode_ns: &mut self.decode_ns,
        };
        self.link
            .deliver(tick, |peer, bytes| receiver.receive(peer, bytes))?;
        let mut due_origins = [None; PEER_COUNT];
        for (index, held_axes) in self.held.iter_mut().enumerate() {
            let peer = PEERS[index];
            if let Some(frame) = self
                .window
                .take_due(peer)
                .map_err(|error| format!("due frame: {error:?}"))?
            {
                let origin = frame.origin;
                *held_axes = frame.axes;
                self.sim
                    .admit_frame(frame)
                    .map_err(|error| format!("W1 input admission: {error:?}"))?;
                due_origins[index] = Some(origin);
            } else if tick > LEAD_TICKS {
                self.due_misses += 1;
            }
            for (position, axis) in self.expected[index].iter_mut().zip(*held_axes) {
                *position += f64::from(axis) * 0.1;
            }
        }
        let outcome = if reconnect {
            let old = self
                .sim
                .handle(scene::CONTROLLED_ID)
                .ok_or("reconnect old actor missing")?;
            let mut position = self
                .sim
                .get(old)
                .map_err(|error| format!("old controller: {error:?}"))?
                .position;
            position.0[0] += 1.0;
            let replacement = ComponentEntity {
                id: REPLACEMENT_ID,
                position,
                velocity: Velocity([0.0; 3]),
                health: 100,
                ownership: Some(self.replacement_binding()),
                visual: Some(1),
            };
            self.sim
                .step(&[Command::Despawn(old), Command::Spawn(replacement)])
        } else {
            self.sim.step(&[])
        }
        .map_err(|error| format!("W1 tick: {error:?}"))?;
        if !outcome
            .committed_inputs
            .iter()
            .copied()
            .eq(due_origins.iter().flatten().copied())
        {
            return Err("W1 committed input origins diverged from N1 due handoff".to_owned());
        }
        self.accepted_frames += u64::try_from(outcome.committed_inputs.len())
            .map_err(|_| "committed input count overflow")?;
        self.window
            .commit_tick(outcome.tick)
            .map_err(|error| format!("N1 cursor: {error:?}"))?;
        if outcome.tick != tick {
            return Err("world tick diverged from conditioned run".to_owned());
        }
        if reconnect {
            self.commit_reconnect(tick)?;
            for binding in self.bindings {
                self.visual.rebaseline(&self.sim, binding)?;
            }
        }
        self.visual.step(&self.sim, tick)?;
        for (index, peer) in PEERS.into_iter().enumerate() {
            let state = self
                .sim
                .controller_state(peer)
                .map_err(|error| format!("committed controller: {error:?}"))?;
            if state
                .position
                .0
                .iter()
                .zip(self.expected[index])
                .any(|(actual, expected)| (actual - expected).abs() > 1e-8)
            {
                return Err("confirmed controller differs from independent trace".to_owned());
            }
            if !(reconnect && index == 0) {
                self.confirmed
                    .send(index, tick, state, due_origins[index])?;
            }
        }
        self.expected_history.push(self.expected);
        self.confirmed.deliver(tick)?;
        self.times
            .push(u64::try_from(started.elapsed().as_nanos()).map_err(|_| "tick timer overflow")?);
        Ok(())
    }

    fn replacement_binding(&self) -> Ownership {
        Ownership {
            peer: PEERS[0],
            connection_epoch: self.bindings[0].connection_epoch + 1,
            actor: REPLACEMENT_ACTOR,
        }
    }

    fn commit_reconnect(&mut self, tick: u64) -> Result<(), String> {
        let old_binding = self.bindings[0];
        let new_binding = self.replacement_binding();
        let handle = self
            .sim
            .handle(REPLACEMENT_ID)
            .ok_or("replacement actor missing after W1 commit")?;
        self.sim
            .register_peer(new_binding, handle)
            .map_err(|error| format!("register replacement control: {error:?}"))?;
        self.window
            .reconnect(new_binding)
            .map_err(|error| format!("N1 reconnect: {error:?}"))?;
        let state = self
            .sim
            .controller_state(PEERS[0])
            .map_err(|error| format!("replacement baseline: {error:?}"))?;
        self.confirmed.rebaseline(0, tick, state)?;
        self.bindings[0] = new_binding;
        self.expected[0] = state.position.0;
        self.held[0] = [0.0; 3];
        // A delayed packet from the old epoch must be counted and discarded.
        let old_datagram_sequence = self.visual.next_client_sequence(0)?;
        self.link.send(
            tick,
            control_sample(old_binding, tick),
            old_datagram_sequence,
            1,
            true,
        )?;
        Ok(())
    }

    /// Finish after exactly 600 ticks and compare the committed controller
    /// positions with a separate held-axis integration trace.
    ///
    /// # Errors
    /// Returns an incomplete-run or divergent controlled-position fault.
    pub fn finish(mut self) -> Result<ConditionedReport, String> {
        if self.sim.tick() != TICKS {
            return Err("conditioned run ended before 600 ticks".to_owned());
        }
        let in_flight_packets = self.link.pending.len();
        if self.link.sent
            != self.link.dropped
                + self.link.delivered
                + u64::try_from(in_flight_packets).map_err(|_| "packet count overflow")?
        {
            return Err("conditioned link packet accounting diverged".to_owned());
        }
        let final_positions = controlled_positions(&self.sim)?;
        for (actual, predicted) in final_positions.iter().zip(self.expected) {
            if actual
                .iter()
                .zip(predicted)
                .any(|(actual, expected)| (actual - expected).abs() > 1e-8)
            {
                return Err(
                    "committed controlled pose differs from independent held-axis trace".to_owned(),
                );
            }
        }
        let (
            confirmed_sent,
            confirmed_groups,
            confirmed_in_flight,
            retired_return_groups,
            rebaselines,
            confirmed_max_age,
        ) = self.confirmed.counts();
        let expected_groups = TICKS
            * u64::try_from(PEER_COUNT).map_err(|_| "peer count overflow")?
            - u64::from(self.scenario == ConditionedScenario::DrainedReconnect);
        if confirmed_sent != expected_groups
            || confirmed_sent
                != confirmed_groups
                    + u64::try_from(confirmed_in_flight)
                        .map_err(|_| "return group count overflow")?
                    + retired_return_groups
        {
            return Err("confirmed return lane group accounting diverged".to_owned());
        }
        if self.scenario == ConditionedScenario::DrainedReconnect {
            if rebaselines != 1
                || retired_return_groups == 0
                || self.stale_packets == 0
                || self.bindings[0].connection_epoch != 2
                || self.bindings[0].actor != REPLACEMENT_ACTOR
            {
                return Err(
                    "drained reconnect did not exercise baseline and stale epoch".to_owned(),
                );
            }
        } else if rebaselines != 0 || retired_return_groups != 0 || self.stale_packets != 0 {
            return Err("ordinary run unexpectedly rebaselined a controller".to_owned());
        }
        let mut confirmed_ticks = [0; PEER_COUNT];
        let mut delivered_origins = 0;
        for (index, client) in self.confirmed.clients().iter().enumerate() {
            let history_index = usize::try_from(
                client
                    .tick()
                    .checked_sub(1)
                    .ok_or("client received no confirmed group")?,
            )
            .map_err(|_| "client tick overflow")?;
            let expected = self
                .expected_history
                .get(history_index)
                .ok_or("client confirmed beyond the authority")?[index];
            if client
                .position()
                .iter()
                .zip(expected)
                .any(|(actual, expected)| (actual - expected).abs() > 1e-8)
            {
                return Err("confirmed client diverged from authority trace".to_owned());
            }
            confirmed_ticks[index] = client.tick();
            delivered_origins += client.committed_count();
        }
        if self.accepted_frames != delivered_origins + self.confirmed.in_flight_committed() {
            return Err("confirmed result origin accounting diverged".to_owned());
        }
        let predictions = self.confirmed.prediction_reports();
        let mut predicted_ticks = [0; PEER_COUNT];
        let mut prediction_matches = 0;
        let mut prediction_corrections = 0;
        let mut prediction_fallbacks = 0;
        let mut prediction_resolved_origins = 0;
        let mut prediction_missed_authored = 0;
        let mut prediction_origin_mismatches = 0;
        let mut replay_steps = 0;
        let mut max_replay_steps = 0;
        let mut max_correction_m: f64 = 0.0;
        let mut prediction_retained_bytes = 0;
        for (index, prediction) in predictions.into_iter().enumerate() {
            if prediction.active {
                if prediction.tick != TICKS + LEAD_TICKS {
                    return Err(
                        "prediction branch did not reach the local input horizon".to_owned()
                    );
                }
                let mut expected_future = self
                    .expected_history
                    .get(usize::try_from(confirmed_ticks[index] - 1).map_err(|_| "tick index")?)
                    .ok_or("confirmed trace tick missing")?[index];
                for target in confirmed_ticks[index] + 1..=prediction.tick {
                    let sample = control_sample(self.bindings[index], target - LEAD_TICKS);
                    for (position, axis) in expected_future.iter_mut().zip(sample.axes) {
                        *position += f64::from(axis) * 0.1;
                    }
                }
                if prediction
                    .state
                    .position
                    .0
                    .iter()
                    .zip(expected_future)
                    .any(|(actual, expected)| (actual - expected).abs() > 1e-8)
                {
                    return Err(
                        "predicted controller differs from authored future trace".to_owned()
                    );
                }
            } else if prediction.tick != confirmed_ticks[index]
                || prediction.state != self.confirmed.clients()[index].state()
            {
                return Err("fallback did not present the latest confirmed controller".to_owned());
            }
            predicted_ticks[index] = prediction.tick;
            prediction_matches += prediction.matches;
            prediction_corrections += prediction.corrections;
            prediction_fallbacks += prediction.fallbacks;
            prediction_resolved_origins += prediction.resolved_origins;
            prediction_missed_authored += prediction.missed_authored;
            prediction_origin_mismatches += prediction.origin_mismatches;
            replay_steps += prediction.replay_steps;
            max_replay_steps = max_replay_steps.max(prediction.max_replay_steps);
            max_correction_m = max_correction_m.max(prediction.max_correction_m);
            prediction_retained_bytes += prediction.retained_bytes;
        }
        if prediction_fallbacks == 0 && prediction_resolved_origins != delivered_origins {
            return Err("predicted input origins diverged from confirmed results".to_owned());
        }
        self.times.sort_unstable();
        let visual = self.visual.finish()?;
        Ok(ConditionedReport {
            scenario: self.scenario,
            committed_ticks: self.sim.tick(),
            sent_packets: self.link.sent,
            sent_bytes: self.link.sent_bytes,
            dropped_packets: self.link.dropped,
            delivered_packets: self.link.delivered,
            delivered_bytes: self.link.delivered_bytes,
            in_flight_packets,
            min_delay: self.link.min_delay,
            max_delay: self.link.max_delay,
            duplicate_frames: self.duplicate_frames,
            late_frames: self.late_frames,
            due_misses: self.due_misses,
            accepted_frames: self.accepted_frames,
            stale_packets: self.stale_packets,
            confirmed_groups,
            confirmed_in_flight,
            retired_return_groups,
            rebaselines,
            confirmed_max_age,
            confirmed_ticks,
            predicted_ticks,
            prediction_matches,
            prediction_corrections,
            prediction_fallbacks,
            prediction_resolved_origins,
            prediction_missed_authored,
            prediction_origin_mismatches,
            replay_steps,
            max_replay_steps,
            max_correction_m,
            prediction_retained_bytes,
            visual,
            tick_p50_ns: *self
                .times
                .get(self.times.len() / 2)
                .ok_or("empty tick sample")?,
            tick_p95_ns: *self
                .times
                .get(self.times.len() * 95 / 100)
                .ok_or("empty tick sample")?,
            encode_ns: self.link.encode_ns,
            decode_ns: self.decode_ns,
            final_positions,
        })
    }
}

/// Run the same 10k V1 simulation with four peer input streams through a
/// fixed-seed 2–4-tick, 2%-loss in-memory link. Encoded input packets pass
/// through the conditioner; the confirmed and speculative clients are local
/// diagnostic consumers. Visual full images seed and rebaseline via checked
/// payload bytes; deltas use selective ACK rebasing through conditioned
/// no-socket packets. Reliable repair and OS transport remain open.
///
/// # Errors
/// Returns a staging, capacity, authority or independent-position-check fault.
pub fn run(scenario: ConditionedScenario) -> Result<ConditionedReport, String> {
    let mut game = ConditionedGame::new(scenario)?;
    for _ in 0..TICKS {
        game.step()?;
    }
    game.finish()
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use thrum_core::{
        PeerId, StableId,
        sim::{Command, ComponentEntity, FrameError, SimAction, Velocity},
    };
    use thrum_network::{
        envelope::{self, Header, Kind},
        future_input::{FutureInputError, FutureInputWindow, StageResult},
        input_batch::{self, EncodeRecord},
    };

    use super::{
        BurstLoss, ConditionedGame, ConditionedScenario, MATCH_ID, PacketReceiver, SceneActionWire,
        run, scene,
    };

    #[test]
    fn four_conditioned_peers_advance_the_shared_10k_world() {
        let result = run(ConditionedScenario::Standard).expect("conditioned fixture");
        assert_eq!(result.committed_ticks, 600);
        assert!(result.dropped_packets > 0);
        assert!(result.delivered_packets > 0);
        assert!(result.accepted_frames > 0);
        assert!(result.duplicate_frames > 0);
        assert!(result.visual.sent > 0);
        assert!(result.visual.dropped > 0);
        assert!(result.visual.delivered > 0);
        assert_eq!(result.visual.full_images, 16);
        assert!(result.visual.full_payload_bytes > 0);
        assert!((2..=4).contains(&result.visual.max_age));
        assert!(result.confirmed_groups > 0);
        assert!(result.confirmed_in_flight > 0);
        assert!((2..=4).contains(&result.confirmed_max_age));
        assert!(
            result
                .confirmed_ticks
                .iter()
                .all(|tick| (596..=600).contains(tick))
        );
        assert_eq!(result.prediction_origin_mismatches, 0);
        assert_eq!(result.prediction_missed_authored, 0);
    }

    #[test]
    fn drained_reconnect_replaces_one_actor_without_losing_a_committed_origin() {
        let result = run(ConditionedScenario::DrainedReconnect).expect("drained reconnect fixture");
        assert_eq!(result.committed_ticks, 600);
        assert_eq!(result.rebaselines, 1);
        assert_eq!(result.stale_packets, 1);
        assert!(result.retired_return_groups > 0);
        assert_eq!(result.prediction_origin_mismatches, 0);
        assert_eq!(result.prediction_missed_authored, 0);
        assert_eq!(result.prediction_fallbacks, 0);
        assert!(result.accepted_frames > 2_300);
    }

    #[test]
    fn invalid_game_action_cannot_reserve_a_future_peer_tick() {
        let sim = scene::simulation().expect("V1 world");
        let peer = PeerId(2);
        let mut window = FutureInputWindow::new(
            &[scene::binding(peer)],
            NonZeroUsize::new(8).expect("nonzero"),
            sim.tick(),
        )
        .expect("future input window");
        let mut invalid = scene::control_frame(peer, [1.0, 0.0, 0.0], 1, 1);
        invalid
            .actions
            .push(SimAction::Impulse([f64::NAN, 0.0, 0.0]));
        assert_eq!(
            window.stage(invalid, |frame| sim.validate_frame_actions(frame)),
            Err(FutureInputError::InvalidPayload(FrameError::InvalidAction))
        );
        assert_eq!(
            window.stage(scene::control_frame(peer, [1.0, 0.0, 0.0], 1, 1), |frame| {
                sim.validate_frame_actions(frame)
            }),
            Ok(StageResult::Queued)
        );
    }

    #[test]
    fn burst_loss_holds_one_actor_then_recovers_without_skipping_world_ticks() {
        let mut game =
            ConditionedGame::new(ConditionedScenario::Standard).expect("10k conditioned game");
        game.link.burst_loss = Some(BurstLoss {
            peer: PeerId(1),
            first_tick: 89,
            last_tick: 95,
        });
        for _ in 0..600 {
            game.step().expect("authoritative tick");
        }
        let result = game.finish().expect("held-axis trace after burst");
        assert_eq!(result.committed_ticks, 600);
        assert!(result.due_misses > 0, "burst must open a due-input gap");
        assert!(
            result.accepted_frames > 2_300,
            "input must resume after burst"
        );
        assert!(
            result.prediction_corrections > 0,
            "burst must correct a speculative branch"
        );
        assert!(result.prediction_missed_authored > 0);
        assert!(result.max_replay_steps <= 9);
        assert!(result.max_correction_m > 0.0);
    }

    #[test]
    fn reconnect_flushes_old_input_before_new_actor_accepts_control() {
        let mut sim = scene::simulation().expect("10k world");
        let peer = PeerId(1);
        let id = StableId(u128::try_from(scene::ENTITY_COUNT - 1).expect("fixture ID"));
        let old_handle = sim.handle(id).expect("old actor");
        let old_position = sim.get(old_handle).expect("old position").position;
        let mut window = FutureInputWindow::new(
            &[scene::binding(peer)],
            NonZeroUsize::new(8).expect("nonzero"),
            sim.tick(),
        )
        .expect("future window");
        assert_eq!(
            window.stage(scene::control_frame(peer, [1.0, 0.0, 0.0], 2, 2), |frame| {
                sim.validate_frame_actions(frame)
            }),
            Ok(StageResult::Queued)
        );
        sim.step(&[Command::Despawn(old_handle)])
            .expect("retire actor at tick 1");
        window.commit_tick(1).expect("empty tick 1 handoff");

        let mut new_binding = scene::binding(peer);
        new_binding.connection_epoch = 2;
        sim.step(&[Command::Spawn(ComponentEntity {
            id,
            position: old_position,
            velocity: Velocity([0.0; 3]),
            health: 100,
            ownership: Some(new_binding),
            visual: Some(1),
        })])
        .expect("spawn new actor at tick 2");
        let new_handle = sim.handle(id).expect("replacement actor");
        sim.register_peer(new_binding, new_handle)
            .expect("new connection epoch");
        window.reconnect(new_binding).expect("flush old input");
        window.commit_tick(2).expect("flushed old due frame");
        assert_eq!(
            window.stage(scene::control_frame(peer, [1.0, 0.0, 0.0], 3, 3), |frame| {
                sim.validate_frame_actions(frame)
            }),
            Err(FutureInputError::WrongConnection)
        );
        let mut new_frame = scene::control_frame(peer, [1.0, 0.0, 0.0], 3, 1);
        new_frame.origin.connection_epoch = 2;
        assert_eq!(
            window.stage(new_frame, |frame| sim.validate_frame_actions(frame)),
            Ok(StageResult::Queued)
        );
        let due = window
            .take_due(peer)
            .expect("known peer")
            .expect("new due frame");
        sim.admit_frame(due).expect("new connection controls actor");
        sim.step(&[]).expect("new epoch tick 3");
        window.commit_tick(3).expect("matching N1 cursor");
        assert!(sim.get(old_handle).is_err(), "retired handle stays stale");
        let moved = sim.get(new_handle).expect("new actor position").position.0;
        assert!((moved[0] - old_position.0[0] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn malformed_second_record_and_wrong_epoch_leave_first_future_slot_empty() {
        let mut game = ConditionedGame::new(ConditionedScenario::Standard).expect("10k world");
        let peer = PeerId(1);
        let first = scene::control_frame(peer, [1.0, 0.0, 0.0], 5, 5);
        let mut second = scene::control_frame(peer, [0.0, 1.0, 0.0], 6, 6);
        second.actions.push(SimAction::Damage(1));
        let records = [
            EncodeRecord {
                frame: &first,
                actor_wire_id: 1,
            },
            EncodeRecord {
                frame: &second,
                actor_wire_id: 1,
            },
        ];
        let mut payload = [0u8; 192];
        let payload_len = input_batch::encode(&records, &SceneActionWire, &mut payload)
            .expect("two checked game records");
        let mut bytes = [0u8; 256];
        let header = Header {
            kind: Kind::InputBatch,
            match_id: MATCH_ID,
            connection_epoch: 0,
            message_sequence: 1,
            server_tick: 1,
            acknowledged_sequence: 0,
        };
        let len = envelope::encode(header, &payload[..payload_len], &mut bytes)
            .expect("wrong-epoch packet framing");
        let mut receiver = PacketReceiver {
            sim: &game.sim,
            window: &mut game.window,
            bindings: &game.bindings,
            duplicate_frames: &mut game.duplicate_frames,
            late_frames: &mut game.late_frames,
            stale_packets: &mut game.stale_packets,
            decode_ns: &mut game.decode_ns,
        };
        receiver
            .receive(peer, &bytes[..len])
            .expect("stale epoch ignored");
        assert_eq!(*receiver.stale_packets, 1);

        payload[63..67].copy_from_slice(&99u32.to_le_bytes());
        let len = envelope::encode(
            Header {
                connection_epoch: 1,
                ..header
            },
            &payload[..payload_len],
            &mut bytes,
        )
        .expect("unknown-schema packet framing");
        assert!(receiver.receive(peer, &bytes[..len]).is_err());
        assert_eq!(
            game.window
                .stage(first, |frame| game.sim.validate_frame_actions(frame)),
            Ok(StageResult::Queued),
            "neither malformed packet may reserve the earlier valid slot"
        );
    }
}
