//! Game-owned visual-pose schema over the running four-peer conditioned world.
//! The first integrated lane keeps a fixed baseline. Selective ACK rebasing,
//! lifecycle and exact controller closure are the next A5 slices.

use std::time::Instant;

use thrum_core::{PeerId, StableId, sim::Simulation};
use thrum_network::{
    envelope::{self, Header, Kind},
    state_delta::{Actor, DeltaError, Image, Scope, decode_delta, encode_delta},
    state_schema,
};

use crate::scene::PEER_COUNT;

use super::{MATCH_ID, Ownership};

const ACTORS: usize = 32;
const PACKET_BYTES: usize = 512;
const PENDING: usize = 16;
const SAMPLE_PERIOD: u64 = 3;
const POSITION_SCALE: f64 = 1_000.0;
const VELOCITY_SCALE: f64 = 16_384.0;
const SEED: u64 = 0x5A17_2026_1104;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct VisualPose {
    px: u32,
    py: u32,
    pz: u32,
    vx: u32,
    vy: u32,
    vz: u32,
}

state_schema! {
    struct VisualPoseWire for VisualPose, schema = 1 {
        0 => px: 21,
        1 => py: 21,
        2 => pz: 21,
        3 => vx: 20,
        4 => vy: 20,
        5 => vz: 20,
    }
}

fn quantize(value: f64, scale: f64, bits: u32) -> Result<u32, DeltaError> {
    if !value.is_finite() {
        return Err(DeltaError::FieldRange);
    }
    let rounded = (value * scale).round();
    let bias = 1i64 << (bits - 1);
    if rounded < -(bias as f64) || rounded >= bias as f64 {
        return Err(DeltaError::FieldRange);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite integral value has a checked 20/21-bit range"
    )]
    let signed = rounded as i64;
    u32::try_from(signed + bias).map_err(|_| DeltaError::FieldRange)
}

fn pose(position: [f64; 3], velocity: [f64; 3]) -> Result<VisualPose, DeltaError> {
    Ok(VisualPose {
        px: quantize(position[0], POSITION_SCALE, 21)?,
        py: quantize(position[1], POSITION_SCALE, 21)?,
        pz: quantize(position[2], POSITION_SCALE, 21)?,
        vx: quantize(velocity[0], VELOCITY_SCALE, 20)?,
        vy: quantize(velocity[1], VELOCITY_SCALE, 20)?,
        vz: quantize(velocity[2], VELOCITY_SCALE, 20)?,
    })
}

fn project(sim: &Simulation, peer: PeerId) -> Result<[Actor<VisualPose>; ACTORS], String> {
    let mut actors = [Actor {
        wire_id: 0,
        value: VisualPose::default(),
    }; ACTORS];
    let peer_index = usize::try_from(peer.0 - 1).map_err(|_| "peer index overflow")?;
    for (index, actor) in actors.iter_mut().enumerate() {
        let entity_index = peer_index * ACTORS + index;
        let handle = sim
            .handle(StableId(
                u128::try_from(entity_index).map_err(|_| "entity ID overflow")?,
            ))
            .ok_or("visual interest actor missing")?;
        let view = sim
            .get(handle)
            .map_err(|error| format!("visual actor view: {error:?}"))?;
        actor.wire_id = u64::try_from(entity_index + 1).map_err(|_| "wire ID overflow")?;
        actor.value = pose(view.position.0, view.velocity.0)
            .map_err(|error| format!("visual pose quantization: {error:?}"))?;
    }
    Ok(actors)
}

#[derive(Clone, Copy)]
struct PeerVisual {
    base: [Actor<VisualPose>; ACTORS],
    received: [Actor<VisualPose>; ACTORS],
    scratch: [Actor<VisualPose>; ACTORS],
    base_id: u64,
    base_tick: u64,
    received_tick: u64,
    epoch: u64,
}

impl PeerVisual {
    fn image(&self, partition_id: u32) -> Image<'_, VisualPose> {
        Image {
            scope: Scope {
                partition_id,
                interest_generation: 1,
            },
            id: self.base_id,
            tick: self.base_tick,
            actors: &self.base,
        }
    }
}

struct Packet {
    peer_index: usize,
    arrival: u64,
    bytes: [u8; PACKET_BYTES],
    len: usize,
    expected: [Actor<VisualPose>; ACTORS],
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct VisualReport {
    pub(super) sent: u64,
    pub(super) dropped: u64,
    pub(super) delivered: u64,
    pub(super) stale: u64,
    pub(super) reordered: u64,
    pub(super) sent_bytes: u64,
    pub(super) encode_ns: u128,
    pub(super) decode_ns: u128,
    pub(super) max_age: u64,
}

pub(super) struct VisualDeltaLoop {
    peers: Vec<PeerVisual>,
    pending: Box<[Option<Packet>]>,
    random_state: u64,
    report: VisualReport,
}

impl VisualDeltaLoop {
    pub(super) fn new(
        sim: &Simulation,
        bindings: &[Ownership; PEER_COUNT],
    ) -> Result<Self, String> {
        let mut peers = Vec::with_capacity(PEER_COUNT);
        for binding in bindings {
            let base = project(sim, binding.peer)?;
            peers.push(PeerVisual {
                base,
                received: base,
                scratch: base,
                base_id: 1,
                base_tick: sim.tick(),
                received_tick: sim.tick(),
                epoch: binding.connection_epoch,
            });
        }
        Ok(Self {
            peers,
            pending: std::iter::repeat_with(|| None)
                .take(PENDING)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            random_state: SEED,
            report: VisualReport::default(),
        })
    }

    fn random(&mut self) -> u64 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value;
        value
    }

    pub(super) fn rebaseline(
        &mut self,
        sim: &Simulation,
        binding: Ownership,
    ) -> Result<(), String> {
        let index = usize::try_from(binding.peer.0 - 1).map_err(|_| "peer index overflow")?;
        let base = project(sim, binding.peer)?;
        let peer = self.peers.get_mut(index).ok_or("visual peer missing")?;
        peer.base = base;
        peer.received = base;
        peer.scratch = base;
        peer.base_id = sim.tick();
        peer.base_tick = sim.tick();
        peer.received_tick = sim.tick();
        peer.epoch = binding.connection_epoch;
        Ok(())
    }

    pub(super) fn step(&mut self, sim: &Simulation, tick: u64) -> Result<(), String> {
        self.deliver(tick)?;
        if tick.is_multiple_of(SAMPLE_PERIOD) {
            for index in 0..self.peers.len() {
                if tick > self.peers[index].base_tick {
                    self.send(sim, tick, index)?;
                }
            }
        }
        Ok(())
    }

    fn send(&mut self, sim: &Simulation, tick: u64, index: usize) -> Result<(), String> {
        let started = Instant::now();
        let peer_number = u64::try_from(index + 1).map_err(|_| "peer ordinal overflow")?;
        let current = project(sim, PeerId(peer_number))?;
        let mut payload = [0u8; PACKET_BYTES - envelope::HEADER_BYTES];
        let peer = self.peers.get(index).ok_or("visual peer missing")?;
        let report = encode_delta(
            &VisualPoseWire,
            peer.image(u32::try_from(index + 1).map_err(|_| "partition overflow")?),
            &current,
            tick + 1,
            tick,
            &mut payload,
        )
        .map_err(|error| format!("visual delta encode: {error:?}"))?;
        let mut bytes = [0u8; PACKET_BYTES];
        let len = envelope::encode(
            Header {
                kind: Kind::StateGroup,
                match_id: MATCH_ID,
                connection_epoch: peer.epoch,
                message_sequence: tick / SAMPLE_PERIOD,
                server_tick: tick,
                acknowledged_sequence: 0,
            },
            &payload[..report.bytes],
            &mut bytes,
        )
        .map_err(|error| format!("visual envelope encode: {error:?}"))?;
        self.report.encode_ns += started.elapsed().as_nanos();
        self.report.sent += 1;
        self.report.sent_bytes += u64::try_from(len).map_err(|_| "visual byte count overflow")?;
        let random = self.random();
        if random % 100 < 2 {
            self.report.dropped += 1;
            return Ok(());
        }
        let slot = self
            .pending
            .iter_mut()
            .find(|packet| packet.is_none())
            .ok_or("visual link capacity exhausted")?;
        *slot = Some(Packet {
            peer_index: index,
            arrival: tick + 2 + (random >> 8) % 3,
            bytes,
            len,
            expected: current,
        });
        Ok(())
    }

    fn deliver(&mut self, tick: u64) -> Result<(), String> {
        for slot in &mut self.pending {
            if slot.as_ref().is_some_and(|packet| packet.arrival <= tick) {
                let packet = slot.take().ok_or("due visual packet missing")?;
                self.report.delivered += 1;
                let started = Instant::now();
                let message = envelope::decode(&packet.bytes[..packet.len])
                    .map_err(|error| format!("visual envelope decode: {error:?}"))?;
                let peer = self
                    .peers
                    .get_mut(packet.peer_index)
                    .ok_or("visual packet peer missing")?;
                if message.header.kind != Kind::StateGroup
                    || message.header.match_id != MATCH_ID
                    || message.header.acknowledged_sequence != 0
                    || message.header.server_tick == 0
                {
                    return Err("visual session identity mismatch".to_owned());
                }
                if message.header.connection_epoch != peer.epoch {
                    self.report.stale += 1;
                    continue;
                }
                if message.header.server_tick <= peer.received_tick {
                    self.report.reordered += 1;
                    continue;
                }
                let decoded = decode_delta(
                    &VisualPoseWire,
                    Image {
                        scope: Scope {
                            partition_id: u32::try_from(packet.peer_index + 1)
                                .map_err(|_| "partition overflow")?,
                            interest_generation: 1,
                        },
                        id: peer.base_id,
                        tick: peer.base_tick,
                        actors: &peer.base,
                    },
                    message.payload,
                    &mut peer.scratch,
                )
                .map_err(|error| format!("visual delta decode: {error:?}"))?;
                if decoded.tick != message.header.server_tick || peer.scratch != packet.expected {
                    return Err(
                        "visual delta diverged from independently projected W1 sample".to_owned(),
                    );
                }
                peer.received.copy_from_slice(&peer.scratch);
                peer.received_tick = decoded.tick;
                self.report.max_age = self.report.max_age.max(tick - decoded.tick);
                self.report.decode_ns += started.elapsed().as_nanos();
            }
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<VisualReport, String> {
        let in_flight = u64::try_from(
            self.pending
                .iter()
                .filter(|packet| packet.is_some())
                .count(),
        )
        .map_err(|_| "visual packet count overflow")?;
        if self.report.sent != self.report.dropped + self.report.delivered + in_flight
            || self.report.delivered == 0
            || self.peers.iter().any(|peer| peer.received_tick < 590)
        {
            return Err("conditioned visual delta coverage or packet accounting failed".to_owned());
        }
        Ok(self.report)
    }
}

#[cfg(test)]
mod tests {
    use thrum_core::sim::Simulation;

    use super::{Packet, VisualDeltaLoop, quantize};
    use crate::scene;

    #[test]
    fn quantization_refuses_nonfinite_and_out_of_range_values() {
        assert!(quantize(f64::NAN, 1_000.0, 21).is_err());
        assert!(quantize(1_049.0, 1_000.0, 21).is_err());
        assert_eq!(quantize(0.0, 1_000.0, 21), Ok(1 << 20));
    }

    #[test]
    fn reordered_visual_sample_cannot_regress_the_client_image() {
        let mut sim = scene::simulation().expect("V1 world");
        let bindings = std::array::from_fn(|index| {
            scene::binding(thrum_core::PeerId(
                u64::try_from(index + 1).expect("four peers"),
            ))
        });
        let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
        for tick in 1..=6 {
            sim.step(&[]).expect("world tick");
            visual.step(&sim, tick).expect("conditioned visual sample");
            if tick == 3 {
                let early = visual
                    .pending
                    .iter_mut()
                    .flatten()
                    .find(|packet| packet.peer_index == 0)
                    .expect("first peer sample");
                early.arrival = 9;
            }
            if tick == 6 {
                let later = visual
                    .pending
                    .iter_mut()
                    .flatten()
                    .filter(|packet| packet.peer_index == 0)
                    .nth(1)
                    .expect("second peer sample");
                later.arrival = 8;
            }
        }
        for tick in 7..=9 {
            sim.step(&[]).expect("world tick");
            visual
                .step(&sim, tick)
                .expect("conditioned visual delivery");
        }
        assert_eq!(visual.peers[0].received_tick, 6);
        assert!(visual.report.reordered > 0);
    }

    #[test]
    fn old_epoch_visual_packet_is_discarded_after_rebaseline() {
        let mut sim: Simulation = scene::simulation().expect("V1 world");
        let bindings = std::array::from_fn(|index| {
            scene::binding(thrum_core::PeerId(
                u64::try_from(index + 1).expect("four peers"),
            ))
        });
        let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
        for tick in 1..=3 {
            sim.step(&[]).expect("world tick");
            visual.step(&sim, tick).expect("conditioned visual sample");
        }
        let pending = visual
            .pending
            .iter_mut()
            .flatten()
            .find(|packet| packet.peer_index == 0)
            .expect("old epoch packet");
        pending.arrival = 5;
        let mut new_binding = bindings[0];
        new_binding.connection_epoch += 1;
        visual
            .rebaseline(&sim, new_binding)
            .expect("new connection visual base");
        for tick in 4..=5 {
            sim.step(&[]).expect("world tick");
            visual.step(&sim, tick).expect("deliver old packet");
        }
        assert_eq!(visual.report.stale, 1);
        assert_eq!(visual.peers[0].received_tick, 3);
    }

    #[test]
    fn duplicate_visual_packet_does_not_publish_twice() {
        let mut sim = scene::simulation().expect("V1 world");
        let bindings = std::array::from_fn(|index| {
            scene::binding(thrum_core::PeerId(
                u64::try_from(index + 1).expect("four peers"),
            ))
        });
        let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
        for tick in 1..=3 {
            sim.step(&[]).expect("world tick");
            visual.step(&sim, tick).expect("visual sample");
        }
        let original = visual
            .pending
            .iter_mut()
            .flatten()
            .find(|packet| packet.peer_index == 0)
            .expect("first visual packet");
        original.arrival = 4;
        let duplicate = Packet {
            peer_index: original.peer_index,
            arrival: 5,
            bytes: original.bytes,
            len: original.len,
            expected: original.expected,
        };
        *visual
            .pending
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("free link slot") = Some(duplicate);
        for tick in 4..=5 {
            sim.step(&[]).expect("world tick");
            visual.step(&sim, tick).expect("duplicate delivery");
        }
        assert_eq!(visual.peers[0].received_tick, 3);
        assert_eq!(visual.report.reordered, 1);
    }
}
