//! Game-owned visual-pose schema over the running four-peer conditioned world.
//! The integrated lane now selectively ACKs admitted images and rebases one
//! visual partition. Full-image repair, lifecycle and exact controller
//! closure remain separate A5 slices.

use std::{mem::size_of, time::Instant};

use thrum_core::{PeerId, StableId, sim::Simulation};
use thrum_network::{
    envelope::{self, Header, Kind},
    state_ack::{self, StateAck},
    state_delta::{Actor, DeltaError, Image, Scope, decode_delta, encode_delta},
    state_schema,
};

use crate::scene::PEER_COUNT;

use super::{MATCH_ID, Ownership};

const ACTORS: usize = 32;
const PACKET_BYTES: usize = 512;
const PENDING: usize = 16;
const IMAGES: usize = 16;
const ACK_BYTES: usize = envelope::HEADER_BYTES + state_ack::BYTES;
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
struct ImageRecord {
    id: u64,
    tick: u64,
    actors: [Actor<VisualPose>; ACTORS],
}

struct PeerVisual {
    base: [Actor<VisualPose>; ACTORS],
    client_initial: [Actor<VisualPose>; ACTORS],
    client_initial_id: u64,
    client_initial_tick: u64,
    client_active: ImageRecord,
    received: [Actor<VisualPose>; ACTORS],
    scratch: [Actor<VisualPose>; ACTORS],
    candidates: Box<[Option<ImageRecord>]>,
    candidate_cursor: usize,
    client_images: Box<[Option<ImageRecord>]>,
    client_cursor: usize,
    base_id: u64,
    base_tick: u64,
    received_tick: u64,
    epoch: u64,
    client_datagram_sequence: u64,
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

struct AckPacket {
    peer_index: usize,
    arrival: u64,
    bytes: [u8; ACK_BYTES],
    len: usize,
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
    pub(super) ack_sent: u64,
    pub(super) ack_dropped: u64,
    pub(super) ack_delivered: u64,
    pub(super) rebases: u64,
    pub(super) unknown_acks: u64,
    /// Peer image and packet slot backing storage, excluding allocator metadata.
    pub(super) retained_bytes: usize,
}

pub(super) struct VisualDeltaLoop {
    peers: Vec<PeerVisual>,
    pending: Box<[Option<Packet>]>,
    pending_acks: Box<[Option<AckPacket>]>,
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
                client_initial: base,
                client_initial_id: 1,
                client_initial_tick: sim.tick(),
                client_active: ImageRecord {
                    id: 1,
                    tick: sim.tick(),
                    actors: base,
                },
                received: base,
                scratch: base,
                candidates: vec![None; IMAGES].into_boxed_slice(),
                candidate_cursor: 0,
                client_images: vec![None; IMAGES].into_boxed_slice(),
                client_cursor: 0,
                base_id: 1,
                base_tick: sim.tick(),
                received_tick: sim.tick(),
                epoch: binding.connection_epoch,
                client_datagram_sequence: 0,
            });
        }
        let retained_bytes = PEER_COUNT
            * (size_of::<PeerVisual>() + 2 * IMAGES * size_of::<Option<ImageRecord>>())
            + PENDING * (size_of::<Option<Packet>>() + size_of::<Option<AckPacket>>());
        Ok(Self {
            peers,
            pending: std::iter::repeat_with(|| None)
                .take(PENDING)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            pending_acks: std::iter::repeat_with(|| None)
                .take(PENDING)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            random_state: SEED,
            report: VisualReport {
                retained_bytes,
                ..VisualReport::default()
            },
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
        peer.client_initial = base;
        peer.client_initial_id = sim.tick();
        peer.client_initial_tick = sim.tick();
        peer.client_active = ImageRecord {
            id: sim.tick(),
            tick: sim.tick(),
            actors: base,
        };
        peer.received = base;
        peer.scratch = base;
        peer.candidates.fill(None);
        peer.candidate_cursor = 0;
        peer.client_images.fill(None);
        peer.client_cursor = 0;
        peer.base_id = sim.tick();
        peer.base_tick = sim.tick();
        peer.received_tick = sim.tick();
        peer.epoch = binding.connection_epoch;
        peer.client_datagram_sequence = 0;
        Ok(())
    }

    pub(super) fn next_client_sequence(&mut self, index: usize) -> Result<u64, String> {
        let peer = self
            .peers
            .get_mut(index)
            .ok_or("client datagram peer missing")?;
        let sequence = peer
            .client_datagram_sequence
            .checked_add(1)
            .ok_or("client datagram sequence exhausted")?;
        peer.client_datagram_sequence = sequence;
        Ok(sequence)
    }

    pub(super) fn step(&mut self, sim: &Simulation, tick: u64) -> Result<(), String> {
        self.deliver_acks(tick)?;
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
        let peer = self.peers.get_mut(index).ok_or("visual peer missing")?;
        peer.candidates[peer.candidate_cursor] = Some(ImageRecord {
            id: tick + 1,
            tick,
            actors: current,
        });
        peer.candidate_cursor = (peer.candidate_cursor + 1) % IMAGES;
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
        for slot in 0..self.pending.len() {
            if self.pending[slot]
                .as_ref()
                .is_some_and(|packet| packet.arrival <= tick)
            {
                let packet = self.pending[slot]
                    .take()
                    .ok_or("due visual packet missing")?;
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
                let base_id = u64::from_le_bytes(
                    message
                        .payload
                        .get(24..32)
                        .ok_or("short visual delta header")?
                        .try_into()
                        .expect("checked eight-byte image ID"),
                );
                let (base_tick, base_actors) = if base_id == peer.client_active.id {
                    (peer.client_active.tick, &peer.client_active.actors)
                } else if base_id == peer.client_initial_id {
                    (peer.client_initial_tick, &peer.client_initial)
                } else {
                    let prior = peer
                        .client_images
                        .iter()
                        .flatten()
                        .find(|image| image.id == base_id)
                        .ok_or("visual base image missing")?;
                    (prior.tick, &prior.actors)
                };
                let decoded = decode_delta(
                    &VisualPoseWire,
                    Image {
                        scope: Scope {
                            partition_id: u32::try_from(packet.peer_index + 1)
                                .map_err(|_| "partition overflow")?,
                            interest_generation: 1,
                        },
                        id: base_id,
                        tick: base_tick,
                        actors: base_actors,
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
                if base_id != peer.client_active.id {
                    peer.client_active = ImageRecord {
                        id: base_id,
                        tick: base_tick,
                        actors: *base_actors,
                    };
                }
                peer.received.copy_from_slice(&peer.scratch);
                peer.received_tick = decoded.tick;
                peer.client_images[peer.client_cursor] = Some(ImageRecord {
                    id: decoded.image_id,
                    tick: decoded.tick,
                    actors: peer.scratch,
                });
                peer.client_cursor = (peer.client_cursor + 1) % IMAGES;
                self.report.max_age = self.report.max_age.max(tick - decoded.tick);
                self.report.decode_ns += started.elapsed().as_nanos();
                self.send_ack(packet.peer_index, decoded.image_id, decoded.tick, tick)?;
            }
        }
        Ok(())
    }

    fn send_ack(
        &mut self,
        index: usize,
        image_id: u64,
        sample_tick: u64,
        now: u64,
    ) -> Result<(), String> {
        let sequence = self.next_client_sequence(index)?;
        let peer = self.peers.get(index).ok_or("visual ACK peer missing")?;
        let ack = StateAck {
            scope: Scope {
                partition_id: u32::try_from(index + 1).map_err(|_| "ACK partition overflow")?,
                interest_generation: 1,
            },
            image_id,
            tick: sample_tick,
        };
        let mut payload = [0u8; state_ack::BYTES];
        state_ack::encode(ack, &mut payload)
            .map_err(|error| format!("visual ACK encode: {error:?}"))?;
        let mut bytes = [0u8; ACK_BYTES];
        let len = envelope::encode(
            Header {
                kind: Kind::StateAck,
                match_id: MATCH_ID,
                connection_epoch: peer.epoch,
                message_sequence: sequence,
                server_tick: sample_tick,
                acknowledged_sequence: 0,
            },
            &payload,
            &mut bytes,
        )
        .map_err(|error| format!("visual ACK envelope: {error:?}"))?;
        self.report.ack_sent += 1;
        let random = self.random();
        if random % 100 < 2 {
            self.report.ack_dropped += 1;
            return Ok(());
        }
        let slot = self
            .pending_acks
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or("visual ACK link capacity exhausted")?;
        *slot = Some(AckPacket {
            peer_index: index,
            arrival: now + 2 + (random >> 8) % 3,
            bytes,
            len,
        });
        Ok(())
    }

    fn deliver_acks(&mut self, tick: u64) -> Result<(), String> {
        for slot in &mut self.pending_acks {
            if slot.as_ref().is_some_and(|packet| packet.arrival <= tick) {
                let packet = slot.take().ok_or("due ACK missing")?;
                self.report.ack_delivered += 1;
                let message = envelope::decode(&packet.bytes[..packet.len])
                    .map_err(|error| format!("ACK envelope decode: {error:?}"))?;
                let peer = self
                    .peers
                    .get_mut(packet.peer_index)
                    .ok_or("ACK peer missing")?;
                if message.header.kind != Kind::StateAck
                    || message.header.match_id != MATCH_ID
                    || message.header.acknowledged_sequence != 0
                {
                    return Err("ACK session identity mismatch".to_owned());
                }
                if message.header.connection_epoch != peer.epoch {
                    continue;
                }
                let ack = state_ack::decode(message.payload)
                    .map_err(|error| format!("ACK payload: {error:?}"))?;
                if ack.scope.partition_id
                    != u32::try_from(packet.peer_index + 1).map_err(|_| "partition overflow")?
                    || ack.scope.interest_generation != 1
                    || ack.tick != message.header.server_tick
                {
                    return Err("ACK identity mismatch".to_owned());
                }
                if ack.image_id <= peer.base_id || ack.tick <= peer.base_tick {
                    continue;
                }
                let Some(candidate) =
                    peer.candidates.iter().flatten().find(|candidate| {
                        candidate.id == ack.image_id && candidate.tick == ack.tick
                    })
                else {
                    self.report.unknown_acks += 1;
                    continue;
                };
                peer.base = candidate.actors;
                peer.base_id = candidate.id;
                peer.base_tick = candidate.tick;
                self.report.rebases += 1;
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
            || self.report.rebases == 0
            || self.peers.iter().any(|peer| peer.received_tick < 590)
        {
            return Err("conditioned visual delta coverage or packet accounting failed".to_owned());
        }
        Ok(self.report)
    }
}

#[cfg(test)]
mod tests;
