//! Retained full partition images across a lossy, delayed local link.
//! One peer's four images stage together before the client publishes them.

use thrum_network::{
    envelope::{self, Header, Kind},
    state_delta::{Actor, Image, Scope, decode_full, encode_full},
};

use super::{
    ACTORS, FULL_BYTES, ImageRecord, MATCH_ID, PARTITIONS, VisualDeltaLoop, VisualPose,
    VisualPoseWire,
};

const RETRY_TICKS: u64 = 9;
const ALL_STAGED: u8 = (1 << PARTITIONS) - 1;

#[derive(Clone, Copy)]
pub(super) struct FullPacket {
    pub(super) peer_index: usize,
    pub(super) partition: usize,
    pub(super) arrival: u64,
    pub(super) id: u64,
    pub(super) tick: u64,
    pub(super) sequence: u64,
    pub(super) bytes: [u8; FULL_BYTES],
    pub(super) len: usize,
}

impl FullPacket {
    const fn empty() -> Self {
        Self {
            peer_index: 0,
            partition: 0,
            arrival: 0,
            id: 0,
            tick: 0,
            sequence: 0,
            bytes: [0; FULL_BYTES],
            len: 0,
        }
    }
}

pub(super) struct FullTransfer {
    pub(super) packets: [FullPacket; PARTITIONS],
    pub(super) last_sequence: u64,
    pub(super) last_send_tick: u64,
    pub(super) last_ack_send_tick: u64,
    pub(super) attempts: u32,
    pub(super) staged_mask: u8,
    pub(super) client_admitted: bool,
    pub(super) acked: bool,
}

#[derive(Clone, Copy)]
pub(super) struct FullAck {
    pub(super) arrival: u64,
    pub(super) peer_index: usize,
    pub(super) bytes: [u8; envelope::HEADER_BYTES],
}

pub(super) fn prepare_transfer(
    peer_index: usize,
    epoch: u64,
    sequence_start: u64,
    bases: &[[Actor<VisualPose>; ACTORS]; PARTITIONS],
    generations: [u32; PARTITIONS],
    first_id: u64,
    tick: u64,
) -> Result<FullTransfer, String> {
    let mut packets = [FullPacket::empty(); PARTITIONS];
    for (partition, packet) in packets.iter_mut().enumerate() {
        let sequence = sequence_start
            .checked_add(u64::try_from(partition + 1).map_err(|_| "partition overflow")?)
            .ok_or("reliable visual sequence exhausted")?;
        let id = first_id
            .checked_add(u64::try_from(partition).map_err(|_| "partition overflow")?)
            .ok_or("full visual image ID exhausted")?;
        let scope = Scope {
            partition_id: u32::try_from(partition + 1).map_err(|_| "partition overflow")?,
            interest_generation: generations[partition],
        };
        let mut payload = [0u8; FULL_BYTES - envelope::HEADER_BYTES];
        let encoded = encode_full(
            &VisualPoseWire,
            Image {
                scope,
                id,
                tick,
                actors: &bases[partition],
            },
            &mut payload,
        )
        .map_err(|error| format!("full visual image encode: {error:?}"))?;
        let mut bytes = [0u8; FULL_BYTES];
        let len = envelope::encode(
            Header {
                kind: Kind::StateGroup,
                match_id: MATCH_ID,
                connection_epoch: epoch,
                message_sequence: sequence,
                server_tick: tick,
                acknowledged_sequence: 0,
            },
            &payload[..encoded.bytes],
            &mut bytes,
        )
        .map_err(|error| format!("full visual envelope: {error:?}"))?;
        *packet = FullPacket {
            peer_index,
            partition,
            arrival: 0,
            id,
            tick,
            sequence,
            bytes,
            len,
        };
    }
    Ok(FullTransfer {
        packets,
        last_sequence: sequence_start
            .checked_add(u64::try_from(PARTITIONS).map_err(|_| "partition overflow")?)
            .ok_or("reliable visual sequence exhausted")?,
        last_send_tick: 0,
        last_ack_send_tick: 0,
        attempts: 0,
        staged_mask: 0,
        client_admitted: false,
        acked: false,
    })
}

impl VisualDeltaLoop {
    pub(super) fn send_full(&mut self, tick: u64) -> Result<(), String> {
        for peer_index in 0..self.peers.len() {
            let transfer = &self.peers[peer_index].full;
            if transfer.acked
                || (transfer.attempts != 0
                    && tick.saturating_sub(transfer.last_send_tick) < RETRY_TICKS)
            {
                continue;
            }
            if transfer.attempts != 0 {
                self.report.full_retries += 1;
            }
            for partition in 0..PARTITIONS {
                let mut packet = self.peers[peer_index].full.packets[partition];
                self.report.full_sent += 1;
                self.report.full_sent_bytes +=
                    u64::try_from(packet.len).map_err(|_| "full byte count overflow")?;
                let random = self.random();
                if random % 100 < 2 {
                    self.report.full_dropped += 1;
                    continue;
                }
                let Some(slot) = self.pending_full.iter_mut().find(|slot| slot.is_none()) else {
                    self.report.full_dropped += 1;
                    continue;
                };
                packet.arrival = tick + 2 + (random >> 8) % 3;
                *slot = Some(packet);
            }
            let transfer = &mut self.peers[peer_index].full;
            transfer.last_send_tick = tick;
            transfer.attempts = transfer
                .attempts
                .checked_add(1)
                .ok_or("full visual retry count exhausted")?;
        }
        Ok(())
    }

    pub(super) fn deliver_full(&mut self, tick: u64) -> Result<(), String> {
        for slot_index in 0..self.pending_full.len() {
            if self.pending_full[slot_index]
                .as_ref()
                .is_none_or(|packet| packet.arrival > tick)
            {
                continue;
            }
            let packet = self.pending_full[slot_index]
                .take()
                .ok_or("due full image missing")?;
            self.report.full_delivered += 1;
            let Ok(message) = envelope::decode(&packet.bytes[..packet.len]) else {
                self.report.full_invalid += 1;
                continue;
            };
            let peer = self
                .peers
                .get_mut(packet.peer_index)
                .ok_or("full peer missing")?;
            if message.header.kind != Kind::StateGroup
                || message.header.match_id != MATCH_ID
                || message.header.acknowledged_sequence != 0
            {
                self.report.full_invalid += 1;
                continue;
            }
            if message.header.connection_epoch != peer.epoch
                || message.header.message_sequence != peer.full.packets[packet.partition].sequence
            {
                self.report.stale += 1;
                continue;
            }
            let state = peer
                .partitions
                .get_mut(packet.partition)
                .ok_or("full partition missing")?;
            let mut decoded_actors = [Actor {
                wire_id: 0,
                value: VisualPose::default(),
            }; ACTORS];
            let Ok(decoded) = decode_full(
                &VisualPoseWire,
                Scope {
                    partition_id: u32::try_from(packet.partition + 1)
                        .map_err(|_| "full partition overflow")?,
                    interest_generation: state.interest_generation,
                },
                message.payload,
                &mut decoded_actors,
            ) else {
                self.report.full_invalid += 1;
                continue;
            };
            if decoded.image_id != packet.id
                || decoded.tick != packet.tick
                || message.header.server_tick != packet.tick
                || decoded_actors != state.base
            {
                self.report.full_invalid += 1;
                continue;
            }
            state.scratch = decoded_actors;
            peer.full.staged_mask |= 1 << packet.partition;
            if peer.full.staged_mask == ALL_STAGED && !peer.full.client_admitted {
                for (partition, state) in peer.partitions.iter_mut().enumerate() {
                    let image = peer.full.packets[partition];
                    state.client_initial = state.scratch;
                    state.client_initial_id = image.id;
                    state.client_initial_tick = image.tick;
                    state.client_active = ImageRecord {
                        partition,
                        id: image.id,
                        tick: image.tick,
                        actors: state.scratch,
                    };
                    state.received = state.scratch;
                    state.received_tick = image.tick;
                }
                peer.full.client_admitted = true;
                self.report.full_images += u64::try_from(PARTITIONS).expect("four partitions");
                self.report.full_payload_bytes += peer
                    .full
                    .packets
                    .iter()
                    .map(|packet| {
                        u64::try_from(packet.len - envelope::HEADER_BYTES).expect("bounded image")
                    })
                    .sum::<u64>();
            }
            if peer.full.client_admitted && peer.full.last_ack_send_tick != tick {
                self.send_full_ack(packet.peer_index, tick)?;
            }
        }
        Ok(())
    }

    fn send_full_ack(&mut self, peer_index: usize, tick: u64) -> Result<(), String> {
        let peer = self
            .peers
            .get_mut(peer_index)
            .ok_or("full ACK peer missing")?;
        let sequence = peer
            .client_reliable_sequence
            .checked_add(1)
            .ok_or("client reliable sequence exhausted")?;
        let mut bytes = [0u8; envelope::HEADER_BYTES];
        envelope::encode(
            Header {
                kind: Kind::BaselineAck,
                match_id: MATCH_ID,
                connection_epoch: peer.epoch,
                message_sequence: sequence,
                server_tick: peer.full.packets[0].tick,
                acknowledged_sequence: peer.full.last_sequence,
            },
            &[],
            &mut bytes,
        )
        .map_err(|error| format!("full ACK envelope: {error:?}"))?;
        peer.client_reliable_sequence = sequence;
        peer.full.last_ack_send_tick = tick;
        self.report.full_acks_sent += 1;
        let random = self.random();
        if random % 100 < 2 {
            self.report.full_acks_dropped += 1;
            return Ok(());
        }
        let Some(slot) = self
            .pending_full_acks
            .iter_mut()
            .find(|slot| slot.is_none())
        else {
            self.report.full_acks_dropped += 1;
            return Ok(());
        };
        *slot = Some(FullAck {
            arrival: tick + 2 + (random >> 8) % 3,
            peer_index,
            bytes,
        });
        Ok(())
    }

    pub(super) fn deliver_full_acks(&mut self, tick: u64) -> Result<(), String> {
        for slot in &mut self.pending_full_acks {
            if slot.as_ref().is_none_or(|ack| ack.arrival > tick) {
                continue;
            }
            let ack = slot.take().ok_or("due full ACK missing")?;
            self.report.full_acks_delivered += 1;
            let Ok(message) = envelope::decode(&ack.bytes) else {
                self.report.full_acks_invalid += 1;
                continue;
            };
            let peer = self
                .peers
                .get_mut(ack.peer_index)
                .ok_or("full ACK peer missing")?;
            if message.header.kind != Kind::BaselineAck
                || message.header.match_id != MATCH_ID
                || !message.payload.is_empty()
                || message.header.message_sequence == 0
            {
                self.report.full_acks_invalid += 1;
                continue;
            }
            if message.header.connection_epoch != peer.epoch
                || message.header.acknowledged_sequence != peer.full.last_sequence
                || message.header.server_tick != peer.full.packets[0].tick
            {
                self.report.stale += 1;
                continue;
            }
            if !peer.full.client_admitted {
                self.report.full_acks_invalid += 1;
                continue;
            }
            peer.full.acked = true;
        }
        Ok(())
    }
}
