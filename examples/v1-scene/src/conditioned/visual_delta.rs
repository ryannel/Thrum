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

use crate::scene::{self, PEER_COUNT};

use super::{MATCH_ID, Ownership};

const ACTORS: usize = 32;
const PARTITIONS: usize = 4;
const PACKET_BYTES: usize = 512;
const PENDING: usize = 32;
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

fn project(sim: &Simulation, partition: usize) -> Result<[Actor<VisualPose>; ACTORS], String> {
    let mut actors = [Actor {
        wire_id: 0,
        value: VisualPose::default(),
    }; ACTORS];
    for (index, actor) in actors.iter_mut().enumerate() {
        let visual_index = partition * ACTORS + index;
        if visual_index < PEER_COUNT {
            let owner = sim
                .controller_state(PeerId(
                    u64::try_from(visual_index + 1).map_err(|_| "peer ordinal overflow")?,
                ))
                .map_err(|error| format!("visual controller: {error:?}"))?;
            actor.wire_id = owner.binding.actor.0;
            actor.value = pose(owner.position.0, owner.velocity.0)
                .map_err(|error| format!("visual controller pose: {error:?}"))?;
            continue;
        }
        let entity_index = scene::MOVING_COUNT + visual_index - PEER_COUNT;
        let handle = sim
            .handle(StableId(
                u128::try_from(entity_index).map_err(|_| "entity ID overflow")?,
            ))
            .ok_or("visual interest actor missing")?;
        let view = sim
            .get(handle)
            .map_err(|error| format!("visual actor view: {error:?}"))?;
        actor.wire_id = u64::try_from(visual_index + 1).map_err(|_| "wire ID overflow")?;
        actor.value = pose(view.position.0, view.velocity.0)
            .map_err(|error| format!("visual pose quantization: {error:?}"))?;
    }
    if partition == 0 {
        actors.sort_unstable_by_key(|actor| actor.wire_id);
    }
    Ok(actors)
}

#[derive(Clone, Copy)]
struct ImageRecord {
    partition: usize,
    id: u64,
    tick: u64,
    actors: [Actor<VisualPose>; ACTORS],
}

struct ImageHistory {
    candidates: Box<[Option<ImageRecord>]>,
    candidate_cursor: usize,
    client_images: Box<[Option<ImageRecord>]>,
    client_cursor: usize,
}

impl ImageHistory {
    fn new() -> Self {
        Self {
            candidates: vec![None; IMAGES].into_boxed_slice(),
            candidate_cursor: 0,
            client_images: vec![None; IMAGES].into_boxed_slice(),
            client_cursor: 0,
        }
    }

    fn reset(&mut self) {
        self.candidates.fill(None);
        self.client_images.fill(None);
        self.candidate_cursor = 0;
        self.client_cursor = 0;
    }
}

struct PartitionVisual {
    interest_generation: u32,
    base: [Actor<VisualPose>; ACTORS],
    client_initial: [Actor<VisualPose>; ACTORS],
    client_initial_id: u64,
    client_initial_tick: u64,
    client_active: ImageRecord,
    received: [Actor<VisualPose>; ACTORS],
    scratch: [Actor<VisualPose>; ACTORS],
    base_id: u64,
    base_tick: u64,
    received_tick: u64,
    uncertain_remote: bool,
}

impl PartitionVisual {
    fn image(&self, partition_id: u32) -> Image<'_, VisualPose> {
        Image {
            scope: Scope {
                partition_id,
                interest_generation: self.interest_generation,
            },
            id: self.base_id,
            tick: self.base_tick,
            actors: &self.base,
        }
    }
}

struct PeerVisual {
    partitions: Vec<PartitionVisual>,
    epoch: u64,
    client_datagram_sequence: u64,
    server_datagram_sequence: u64,
    next_image_id: u64,
}

struct Packet {
    peer_index: usize,
    partition: usize,
    arrival: u64,
    bytes: [u8; PACKET_BYTES],
    len: usize,
    expected: [Actor<VisualPose>; ACTORS],
}

struct AckPacket {
    peer_index: usize,
    partition: usize,
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
    pub(super) project_ns: u128,
    pub(super) decode_ns: u128,
    pub(super) max_age: u64,
    pub(super) ack_sent: u64,
    pub(super) ack_dropped: u64,
    pub(super) ack_delivered: u64,
    pub(super) rebases: u64,
    pub(super) unknown_acks: u64,
    pub(super) unchanged_partitions: u64,
    /// Peer image and packet slot backing storage, excluding allocator metadata.
    pub(super) retained_bytes: usize,
}

pub(super) struct VisualDeltaLoop {
    peers: Vec<PeerVisual>,
    images: Vec<ImageHistory>,
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
        let mut images = Vec::with_capacity(PEER_COUNT);
        for binding in bindings {
            let mut partitions = Vec::with_capacity(PARTITIONS);
            for partition in 0..PARTITIONS {
                let base = project(sim, partition)?;
                let base_id = u64::try_from(partition + 1).map_err(|_| "baseline ID overflow")?;
                partitions.push(PartitionVisual {
                    interest_generation: 1,
                    base,
                    client_initial: base,
                    client_initial_id: base_id,
                    client_initial_tick: sim.tick(),
                    client_active: ImageRecord {
                        partition,
                        id: base_id,
                        tick: sim.tick(),
                        actors: base,
                    },
                    received: base,
                    scratch: base,
                    base_id,
                    base_tick: sim.tick(),
                    received_tick: sim.tick(),
                    uncertain_remote: false,
                });
            }
            peers.push(PeerVisual {
                partitions,
                epoch: binding.connection_epoch,
                client_datagram_sequence: 0,
                server_datagram_sequence: 0,
                next_image_id: u64::try_from(PARTITIONS).map_err(|_| "baseline count overflow")?,
            });
            images.push(ImageHistory::new());
        }
        let retained_bytes = PEER_COUNT
            * (size_of::<PeerVisual>()
                + PARTITIONS * size_of::<PartitionVisual>()
                + size_of::<ImageHistory>()
                + 2 * IMAGES * size_of::<Option<ImageRecord>>())
            + PENDING * (size_of::<Option<Packet>>() + size_of::<Option<AckPacket>>());
        Ok(Self {
            peers,
            images,
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
        let peer = self.peers.get(index).ok_or("visual peer missing")?;
        if peer.partitions.len() != PARTITIONS || self.images.get(index).is_none() {
            return Err("visual partition or image history missing".to_owned());
        }
        let same_epoch = peer.epoch == binding.connection_epoch;
        let first_id = if same_epoch {
            peer.next_image_id
                .checked_add(1)
                .ok_or("baseline image ID exhausted")?
        } else {
            1
        };
        let last_id = first_id
            .checked_add(u64::try_from(PARTITIONS - 1).map_err(|_| "partition count overflow")?)
            .ok_or("baseline image IDs exhausted")?;
        let empty = Actor {
            wire_id: 0,
            value: VisualPose::default(),
        };
        let mut bases = [[empty; ACTORS]; PARTITIONS];
        let mut generations = [0; PARTITIONS];
        for partition in 0..PARTITIONS {
            bases[partition] = project(sim, partition)?;
            generations[partition] = if same_epoch {
                peer.partitions[partition]
                    .interest_generation
                    .checked_add(1)
                    .ok_or("interest generation exhausted")?
            } else {
                1
            };
        }
        let peer = &mut self.peers[index];
        for (partition, state) in peer.partitions.iter_mut().enumerate() {
            let base = bases[partition];
            let base_id = first_id + u64::try_from(partition).expect("four staged partitions");
            state.interest_generation = generations[partition];
            state.base = base;
            state.client_initial = base;
            state.client_initial_id = base_id;
            state.client_initial_tick = sim.tick();
            state.client_active = ImageRecord {
                partition,
                id: base_id,
                tick: sim.tick(),
                actors: base,
            };
            state.received = base;
            state.scratch = base;
            state.base_id = base_id;
            state.base_tick = sim.tick();
            state.received_tick = sim.tick();
            state.uncertain_remote = false;
        }
        self.images[index].reset();
        peer.epoch = binding.connection_epoch;
        if !same_epoch {
            peer.client_datagram_sequence = 0;
            peer.server_datagram_sequence = 0;
        }
        peer.next_image_id = last_id;
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
            for partition in 0..PARTITIONS {
                let projection_started = Instant::now();
                let current = project(sim, partition)?;
                self.report.project_ns += projection_started.elapsed().as_nanos();
                for peer_index in 0..self.peers.len() {
                    if tick > self.peers[peer_index].partitions[partition].base_tick {
                        self.send(&current, tick, peer_index, partition)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn send(
        &mut self,
        current: &[Actor<VisualPose>; ACTORS],
        tick: u64,
        peer_index: usize,
        partition: usize,
    ) -> Result<(), String> {
        let started = Instant::now();
        let mut payload = [0u8; PACKET_BYTES - envelope::HEADER_BYTES];
        let peer = self.peers.get(peer_index).ok_or("visual peer missing")?;
        let state = peer.partitions.get(partition).ok_or("partition missing")?;
        if state.base == *current && !state.uncertain_remote {
            self.report.unchanged_partitions += 1;
            return Ok(());
        }
        let image_id = peer
            .next_image_id
            .checked_add(1)
            .ok_or("image ID exhausted")?;
        let report = encode_delta(
            &VisualPoseWire,
            state.image(u32::try_from(partition + 1).map_err(|_| "partition overflow")?),
            current,
            image_id,
            tick,
            &mut payload,
        )
        .map_err(|error| format!("visual delta encode: {error:?}"))?;
        let server_sequence = peer
            .server_datagram_sequence
            .checked_add(1)
            .ok_or("server datagram sequence exhausted")?;
        let mut bytes = [0u8; PACKET_BYTES];
        let len = envelope::encode(
            Header {
                kind: Kind::StateGroup,
                match_id: MATCH_ID,
                connection_epoch: peer.epoch,
                message_sequence: server_sequence,
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
        let peer = self
            .peers
            .get_mut(peer_index)
            .ok_or("visual peer missing")?;
        peer.next_image_id = image_id;
        peer.server_datagram_sequence = server_sequence;
        peer.partitions[partition].uncertain_remote = true;
        let images = self
            .images
            .get_mut(peer_index)
            .ok_or("image history missing")?;
        images.candidates[images.candidate_cursor] = Some(ImageRecord {
            partition,
            id: image_id,
            tick,
            actors: *current,
        });
        images.candidate_cursor = (images.candidate_cursor + 1) % IMAGES;
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
            peer_index,
            partition,
            arrival: tick + 2 + (random >> 8) % 3,
            bytes,
            len,
            expected: *current,
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
                let state = peer
                    .partitions
                    .get_mut(packet.partition)
                    .ok_or("visual packet partition missing")?;
                let images = self
                    .images
                    .get_mut(packet.peer_index)
                    .ok_or("visual image history missing")?;
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
                let generation = u32::from_le_bytes(
                    message
                        .payload
                        .get(16..20)
                        .ok_or("short visual generation header")?
                        .try_into()
                        .expect("checked generation width"),
                );
                if generation != state.interest_generation {
                    self.report.stale += 1;
                    continue;
                }
                if message.header.server_tick <= state.received_tick {
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
                let (base_tick, base_actors) = if base_id == state.client_active.id {
                    (state.client_active.tick, &state.client_active.actors)
                } else if base_id == state.client_initial_id {
                    (state.client_initial_tick, &state.client_initial)
                } else {
                    let prior = images
                        .client_images
                        .iter()
                        .flatten()
                        .find(|image| image.partition == packet.partition && image.id == base_id)
                        .ok_or("visual base image missing")?;
                    (prior.tick, &prior.actors)
                };
                let decoded = decode_delta(
                    &VisualPoseWire,
                    Image {
                        scope: Scope {
                            partition_id: u32::try_from(packet.partition + 1)
                                .map_err(|_| "partition overflow")?,
                            interest_generation: state.interest_generation,
                        },
                        id: base_id,
                        tick: base_tick,
                        actors: base_actors,
                    },
                    message.payload,
                    &mut state.scratch,
                )
                .map_err(|error| format!("visual delta decode: {error:?}"))?;
                if decoded.tick != message.header.server_tick || state.scratch != packet.expected {
                    return Err(
                        "visual delta diverged from independently projected W1 sample".to_owned(),
                    );
                }
                if base_id != state.client_active.id {
                    state.client_active = ImageRecord {
                        partition: packet.partition,
                        id: base_id,
                        tick: base_tick,
                        actors: *base_actors,
                    };
                }
                state.received.copy_from_slice(&state.scratch);
                state.received_tick = decoded.tick;
                images.client_images[images.client_cursor] = Some(ImageRecord {
                    partition: packet.partition,
                    id: decoded.image_id,
                    tick: decoded.tick,
                    actors: state.scratch,
                });
                images.client_cursor = (images.client_cursor + 1) % IMAGES;
                self.report.max_age = self.report.max_age.max(tick - decoded.tick);
                self.report.decode_ns += started.elapsed().as_nanos();
                self.send_ack(
                    packet.peer_index,
                    packet.partition,
                    decoded.image_id,
                    decoded.tick,
                    tick,
                )?;
            }
        }
        Ok(())
    }

    fn send_ack(
        &mut self,
        index: usize,
        partition: usize,
        image_id: u64,
        sample_tick: u64,
        now: u64,
    ) -> Result<(), String> {
        let sequence = self.next_client_sequence(index)?;
        let peer = self.peers.get(index).ok_or("visual ACK peer missing")?;
        let state = peer
            .partitions
            .get(partition)
            .ok_or("ACK partition missing")?;
        let ack = StateAck {
            scope: Scope {
                partition_id: u32::try_from(partition + 1).map_err(|_| "ACK partition overflow")?,
                interest_generation: state.interest_generation,
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
            partition,
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
                let state = peer
                    .partitions
                    .get_mut(packet.partition)
                    .ok_or("ACK partition missing")?;
                let images = self
                    .images
                    .get(packet.peer_index)
                    .ok_or("ACK image history missing")?;
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
                    != u32::try_from(packet.partition + 1).map_err(|_| "partition overflow")?
                    || ack.tick != message.header.server_tick
                {
                    return Err("ACK identity mismatch".to_owned());
                }
                if ack.scope.interest_generation != state.interest_generation {
                    continue;
                }
                if ack.image_id <= state.base_id || ack.tick <= state.base_tick {
                    continue;
                }
                let Some(candidate) = images.candidates.iter().flatten().find(|candidate| {
                    candidate.partition == packet.partition
                        && candidate.id == ack.image_id
                        && candidate.tick == ack.tick
                }) else {
                    self.report.unknown_acks += 1;
                    continue;
                };
                let uncertain_remote = images.candidates.iter().flatten().any(|newer| {
                    newer.partition == packet.partition
                        && newer.id > candidate.id
                        && newer.actors != candidate.actors
                });
                state.base = candidate.actors;
                state.base_id = candidate.id;
                state.base_tick = candidate.tick;
                state.uncertain_remote = uncertain_remote;
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
            || self
                .peers
                .iter()
                .any(|peer| peer.partitions[0].received_tick < 590)
        {
            return Err("conditioned visual delta coverage or packet accounting failed".to_owned());
        }
        Ok(self.report)
    }
}

#[cfg(test)]
mod tests;
