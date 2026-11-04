//! Four-peer 10k-world no-socket diagnostic with deterministic delayed input.
//! The same V1 simulation owns gameplay; N1 only stages future frames.

use std::{fmt, num::NonZeroUsize, time::Instant};

use thrum_core::{PeerId, StableId, sim::Simulation};
use thrum_network::future_input::{FutureInputError, FutureInputWindow, StageResult};

use crate::scene::{self, ENTITY_COUNT, PEER_COUNT};

const TICKS: u64 = 600;
const LEAD_TICKS: u64 = 5;
const FUTURE_TICKS: usize = 8;
const HISTORY: usize = 4;
const PACKET_CAPACITY: usize = 32;
const SEED: u64 = 0xA5A5_2026_1104;
const PEERS: [PeerId; PEER_COUNT] = [PeerId(1), PeerId(2), PeerId(3), PeerId(4)];

#[derive(Clone, Copy)]
struct Sample {
    peer: PeerId,
    target: u64,
    axes: [f32; 3],
}

struct Packet {
    arrival: u64,
    samples: [Option<Sample>; HISTORY],
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
    dropped: u64,
    delivered: u64,
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
            dropped: 0,
            delivered: 0,
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

    fn send(&mut self, peer: PeerId, now: u64) -> Result<(), String> {
        self.sent += 1;
        let random = self.random();
        if random % 100 < 2
            || self.burst_loss.is_some_and(|burst| {
                burst.peer == peer && (burst.first_tick..=burst.last_tick).contains(&now)
            })
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
        let mut samples = [None; HISTORY];
        for (age, slot) in samples.iter_mut().enumerate() {
            let Some(created) =
                now.checked_sub(u64::try_from(age).map_err(|_| "history age overflow")?)
            else {
                break;
            };
            if created == 0 {
                break;
            }
            *slot = Some(control_sample(peer, created));
        }
        self.pending.push(Packet {
            arrival: now + delay,
            samples,
        });
        Ok(())
    }

    fn deliver(
        &mut self,
        now: u64,
        mut receive: impl FnMut(Sample) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].arrival <= now {
                let packet = self.pending.swap_remove(index);
                self.delivered += 1;
                for sample in packet.samples.into_iter().flatten() {
                    receive(sample)?;
                }
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}

fn control_sample(peer: PeerId, created: u64) -> Sample {
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
        peer,
        target: created + LEAD_TICKS,
        axes,
    }
}

fn controlled_positions(sim: &Simulation) -> Result<[[f64; 3]; PEER_COUNT], String> {
    let mut positions = [[0.0; 3]; PEER_COUNT];
    for (index, position) in positions.iter_mut().enumerate() {
        let id = StableId(
            u128::try_from(ENTITY_COUNT - index - 1).map_err(|_| "fixture identity overflow")?,
        );
        let handle = sim.handle(id).ok_or("controlled actor missing")?;
        *position = sim
            .get(handle)
            .map_err(|error| format!("controlled actor: {error:?}"))?
            .position
            .0;
    }
    Ok(positions)
}

/// A measured, local four-peer input run. This is not a network transport result.
#[derive(Debug)]
pub struct ConditionedReport {
    committed_ticks: u64,
    sent_packets: u64,
    dropped_packets: u64,
    delivered_packets: u64,
    in_flight_packets: usize,
    min_delay: u64,
    max_delay: u64,
    duplicate_frames: u64,
    late_frames: u64,
    due_misses: u64,
    accepted_frames: u64,
    tick_p50_ns: u64,
    tick_p95_ns: u64,
    final_positions: [[f64; 3]; PEER_COUNT],
}

impl fmt::Display for ConditionedReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ticks={} peers={} packets sent/dropped/delivered/in_flight={}/{}/{}/{} delay_ticks={}-{} duplicate/late={}/{} accepted={} due_misses={} tick p50/p95={}/{} us final_positions={:?}",
            self.committed_ticks,
            PEER_COUNT,
            self.sent_packets,
            self.dropped_packets,
            self.delivered_packets,
            self.in_flight_packets,
            self.min_delay,
            self.max_delay,
            self.duplicate_frames,
            self.late_frames,
            self.accepted_frames,
            self.due_misses,
            self.tick_p50_ns / 1_000,
            self.tick_p95_ns / 1_000,
            self.final_positions
        )
    }
}

/// One live no-socket instance of the V1 four-peer workload. Setup reserves
/// every link, input-window and timing slot before the first fixed tick.
pub struct ConditionedGame {
    sim: Simulation,
    window: FutureInputWindow<thrum_core::sim::SimAction>,
    link: ConditionedLink,
    held: [[f32; 3]; PEER_COUNT],
    expected: [[f64; 3]; PEER_COUNT],
    times: Vec<u64>,
    duplicate_frames: u64,
    late_frames: u64,
    due_misses: u64,
    accepted_frames: u64,
}

impl ConditionedGame {
    /// Prepare the same 10k V1 world and four peer input owners.
    ///
    /// # Errors
    /// Returns a world construction or timing-window setup fault.
    pub fn new() -> Result<Self, String> {
        let sim = scene::simulation()?;
        let bindings = PEERS.map(scene::binding);
        let window = FutureInputWindow::new(
            &bindings,
            NonZeroUsize::new(FUTURE_TICKS).ok_or("zero future input horizon")?,
            sim.tick(),
        )
        .map_err(|error| format!("future input setup: {error:?}"))?;
        let expected = controlled_positions(&sim)?;
        Ok(Self {
            sim,
            window,
            link: ConditionedLink::new(),
            held: [[0.0; 3]; PEER_COUNT],
            expected,
            times: Vec::with_capacity(usize::try_from(TICKS).map_err(|_| "tick count overflow")?),
            duplicate_frames: 0,
            late_frames: 0,
            due_misses: 0,
            accepted_frames: 0,
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
        for peer in PEERS {
            self.link.send(peer, tick)?;
        }
        self.link.deliver(tick, |sample| {
            let frame =
                scene::control_frame(sample.peer, sample.axes, sample.target, sample.target);
            match self
                .window
                .stage(frame, |frame| self.sim.validate_frame_actions(frame))
            {
                Ok(StageResult::Queued) => Ok(()),
                Ok(StageResult::Duplicate) => {
                    self.duplicate_frames += 1;
                    Ok(())
                }
                Err(FutureInputError::Late) => {
                    self.late_frames += 1;
                    Ok(())
                }
                Err(error) => Err(format!("conditioned input staging: {error:?}")),
            }
        })?;
        for (index, held_axes) in self.held.iter_mut().enumerate() {
            let peer = PEERS[index];
            if let Some(frame) = self
                .window
                .take_due(peer)
                .map_err(|error| format!("due frame: {error:?}"))?
            {
                *held_axes = frame.axes;
                self.sim
                    .admit_frame(frame)
                    .map_err(|error| format!("W1 input admission: {error:?}"))?;
                self.accepted_frames += 1;
            } else if tick > LEAD_TICKS {
                self.due_misses += 1;
            }
            for (position, axis) in self.expected[index].iter_mut().zip(*held_axes) {
                *position += f64::from(axis) * 0.1;
            }
        }
        let outcome = self
            .sim
            .step(&[])
            .map_err(|error| format!("W1 tick: {error:?}"))?;
        self.window
            .commit_tick(outcome.tick)
            .map_err(|error| format!("N1 cursor: {error:?}"))?;
        if outcome.tick != tick {
            return Err("world tick diverged from conditioned run".to_owned());
        }
        self.times
            .push(u64::try_from(started.elapsed().as_nanos()).map_err(|_| "tick timer overflow")?);
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
        self.times.sort_unstable();
        Ok(ConditionedReport {
            committed_ticks: self.sim.tick(),
            sent_packets: self.link.sent,
            dropped_packets: self.link.dropped,
            delivered_packets: self.link.delivered,
            in_flight_packets,
            min_delay: self.link.min_delay,
            max_delay: self.link.max_delay,
            duplicate_frames: self.duplicate_frames,
            late_frames: self.late_frames,
            due_misses: self.due_misses,
            accepted_frames: self.accepted_frames,
            tick_p50_ns: *self
                .times
                .get(self.times.len() / 2)
                .ok_or("empty tick sample")?,
            tick_p95_ns: *self
                .times
                .get(self.times.len() * 95 / 100)
                .ok_or("empty tick sample")?,
            final_positions,
        })
    }
}

/// Run the same 10k V1 simulation with four peer input streams through a
/// fixed-seed 2–4-tick, 2%-loss in-memory link. No packets or client replica
/// are encoded yet.
///
/// # Errors
/// Returns a staging, capacity, authority or independent-position-check fault.
pub fn run() -> Result<ConditionedReport, String> {
    let mut game = ConditionedGame::new()?;
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
    use thrum_network::future_input::{FutureInputError, FutureInputWindow, StageResult};

    use super::{BurstLoss, ConditionedGame, run, scene};

    #[test]
    fn four_conditioned_peers_advance_the_shared_10k_world() {
        let result = run().expect("conditioned fixture");
        assert_eq!(result.committed_ticks, 600);
        assert!(result.dropped_packets > 0);
        assert!(result.delivered_packets > 0);
        assert!(result.accepted_frames > 0);
        assert!(result.duplicate_frames > 0);
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
        let mut game = ConditionedGame::new().expect("10k conditioned game");
        game.link.burst_loss = Some(BurstLoss {
            peer: PeerId(1),
            first_tick: 100,
            last_tick: 105,
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
}
