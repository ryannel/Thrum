//! Visual delta admission, sequencing and retention checks.

use thrum_core::sim::Simulation;

use super::{ACTORS, PARTITIONS, Packet, VisualDeltaLoop, envelope, quantize};
use crate::scene;

fn advance(sim: &mut Simulation, tick: u64) {
    advance_axes(sim, tick, [1.0, 0.0, 0.0]);
}

fn advance_axes(sim: &mut Simulation, tick: u64, axes: [f32; 3]) {
    for peer in 1..=scene::PEER_COUNT {
        sim.admit_frame(scene::control_frame(
            thrum_core::PeerId(u64::try_from(peer).expect("four peers")),
            axes,
            tick,
            tick,
        ))
        .expect("controlled input");
    }
    sim.step(&[]).expect("world tick");
}

#[test]
fn quantization_refuses_nonfinite_and_out_of_range_values() {
    assert!(quantize(f64::NAN, 1_000.0, 21).is_err());
    assert!(quantize(1_049.0, 1_000.0, 21).is_err());
    assert_eq!(quantize(0.0, 1_000.0, 21), Ok(1 << 20));
}

#[test]
fn four_partitions_cover_128_interests_and_skip_unchanged_groups() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four interests per peer");
    for peer in &visual.peers {
        assert_eq!(peer.partitions.len(), PARTITIONS);
        assert_eq!(
            peer.partitions
                .iter()
                .map(|part| part.base.len())
                .sum::<usize>(),
            128
        );
        assert_eq!(peer.partitions[0].base[0].wire_id, 1);
        assert_eq!(peer.partitions[0].base[3].wire_id, 4);
        assert_eq!(peer.partitions[3].base[ACTORS - 1].wire_id, 128);
        assert_eq!(
            peer.partitions
                .iter()
                .map(|part| part.base_id)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }
    for tick in 1..=9 {
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("only active partition sends");
    }
    assert_eq!(visual.report.sent, 12);
    assert_eq!(visual.report.unchanged_partitions, 36);
}

#[test]
fn same_epoch_interest_rebaseline_retires_old_generation_packets() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    for tick in 1..=3 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("old generation sample");
    }
    let old_sequence = visual.peers[0].client_datagram_sequence;
    let old_image_id = visual.peers[0].next_image_id;
    let packet = visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0)
        .expect("old generation packet");
    packet.arrival = 5;
    visual
        .rebaseline(&sim, bindings[0])
        .expect("same-epoch membership baseline");
    assert_eq!(visual.peers[0].partitions[0].interest_generation, 2);
    assert!(visual.peers[0].partitions[0].base_id > old_image_id);
    assert_eq!(visual.peers[0].client_datagram_sequence, old_sequence);
    for tick in 4..=5 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("retire old generation");
    }
    assert_eq!(visual.report.stale, 1);
    assert_eq!(visual.peers[0].partitions[0].received_tick, 3);
}

#[test]
fn late_partition_projection_failure_leaves_all_visual_state_unchanged() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    for tick in 1..=3 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("retained candidate");
    }
    let before_ids: Vec<_> = visual.peers[0]
        .partitions
        .iter()
        .map(|part| part.base_id)
        .collect();
    let before_generations: Vec<_> = visual.peers[0]
        .partitions
        .iter()
        .map(|part| part.interest_generation)
        .collect();
    let before_base = visual.peers[0].partitions[0].base;
    let before_candidate = visual.images[0].candidates[0].expect("first candidate").id;
    let before_sequence = visual.peers[0].client_datagram_sequence;
    let missing = sim
        .handle(thrum_core::StableId(1623))
        .expect("last static actor");
    sim.step(&[thrum_core::sim::Command::Despawn(missing)])
        .expect("world structural tick");
    assert!(visual.rebaseline(&sim, bindings[0]).is_err());
    assert_eq!(
        visual.peers[0]
            .partitions
            .iter()
            .map(|part| part.base_id)
            .collect::<Vec<_>>(),
        before_ids
    );
    assert_eq!(
        visual.peers[0]
            .partitions
            .iter()
            .map(|part| part.interest_generation)
            .collect::<Vec<_>>(),
        before_generations
    );
    assert_eq!(visual.peers[0].partitions[0].base, before_base);
    assert_eq!(
        visual.images[0].candidates[0]
            .expect("retained candidate")
            .id,
        before_candidate
    );
    assert_eq!(visual.peers[0].client_datagram_sequence, before_sequence);
}

#[test]
fn four_active_partitions_share_unique_images_under_reordered_delivery() {
    use thrum_core::sim::Command;

    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let handles = [1500u128, 1528, 1560, 1592].map(|id| {
        sim.handle(thrum_core::StableId(id))
            .expect("static actor in each partition")
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("shared image rings");
    for tick in 1..=120 {
        let edits = handles.map(|handle| {
            let mut position = sim.get(handle).expect("static actor view").position;
            position.0[0] += 0.001;
            Command::SetPosition(handle, position)
        });
        sim.step(&edits).expect("four independent partition edits");
        visual
            .step(&sim, tick)
            .expect("conditioned four-partition delivery");
        if tick == 3 {
            visual
                .pending
                .iter_mut()
                .flatten()
                .find(|packet| packet.peer_index == 0 && packet.partition == 0)
                .expect("first partition packet")
                .arrival = 7;
            visual
                .pending
                .iter_mut()
                .flatten()
                .find(|packet| packet.peer_index == 0 && packet.partition == 1)
                .expect("second partition packet")
                .arrival = 5;
        }
    }
    assert_eq!(visual.report.sent, 640);
    assert!(visual.report.rebases > 500);
    assert_eq!(visual.report.unknown_acks, 0);
    for peer in &visual.peers {
        assert!(peer.partitions.iter().all(|part| part.received_tick >= 114));
        let mut ids: Vec<_> = peer.partitions.iter().map(|part| part.base_id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PARTITIONS);
    }
}

#[test]
fn lost_ack_then_return_to_base_sends_zero_record_correction() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    for tick in 1..=3 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("first changed sample");
    }
    visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0)
        .expect("first changed packet")
        .arrival = 4;
    advance_axes(&mut sim, 4, [-1.0, 0.0, 0.0]);
    visual.step(&sim, 4).expect("first sample admitted");
    for slot in &mut visual.pending_acks {
        if slot.as_ref().is_some_and(|ack| ack.peer_index == 0) {
            *slot = None;
        }
    }
    for tick in 5..=9 {
        let axes = if tick <= 6 {
            [-1.0, 0.0, 0.0]
        } else {
            [0.0; 3]
        };
        advance_axes(&mut sim, tick, axes);
        visual.step(&sim, tick).expect("return toward the base");
        for slot in &mut visual.pending_acks {
            if slot.as_ref().is_some_and(|ack| ack.peer_index == 0) {
                *slot = None;
            }
        }
    }
    let correction = visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| {
            packet.peer_index == 0 && packet.expected == visual.peers[0].partitions[0].base
        })
        .expect("zero-record correction packet");
    assert_eq!(
        &correction.bytes[envelope::HEADER_BYTES + 2..envelope::HEADER_BYTES + 4],
        &[0, 0]
    );
    correction.arrival = 10;
    advance_axes(&mut sim, 10, [0.0; 3]);
    visual
        .step(&sim, 10)
        .expect("corrected base sample admitted");
    assert_eq!(visual.peers[0].partitions[0].received_tick, 9);
    assert_eq!(
        visual.peers[0].partitions[0].received,
        visual.peers[0].partitions[0].base
    );
    let sequence = visual.peers[0].server_datagram_sequence;
    visual
        .pending_acks
        .iter_mut()
        .flatten()
        .find(|ack| ack.peer_index == 0)
        .expect("correction ACK")
        .arrival = 11;
    for tick in 11..=18 {
        advance_axes(&mut sim, tick, [0.0; 3]);
        visual
            .step(&sim, tick)
            .expect("quiescent corrected interest");
    }
    assert_eq!(visual.peers[0].server_datagram_sequence, sequence);
    assert!(!visual.peers[0].partitions[0].uncertain_remote);
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
        advance(&mut sim, tick);
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
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("conditioned visual delivery");
    }
    assert_eq!(visual.peers[0].partitions[0].received_tick, 6);
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
        advance(&mut sim, tick);
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
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("deliver old packet");
    }
    assert_eq!(visual.report.stale, 1);
    assert_eq!(visual.peers[0].partitions[0].received_tick, 3);
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
        advance(&mut sim, tick);
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
        partition: original.partition,
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
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("duplicate delivery");
    }
    assert_eq!(visual.peers[0].partitions[0].received_tick, 3);
    assert_eq!(visual.report.reordered, 1);
}

#[test]
fn active_base_survives_more_than_one_image_ring_of_lost_acks() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    for tick in 1..=24 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("first ACK rebases");
    }
    assert!(visual.peers[0].partitions[0].base_id > 1);
    for tick in 25..=100 {
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("current base remains decodable");
        for slot in &mut visual.pending_acks {
            if slot.as_ref().is_some_and(|ack| ack.peer_index == 0) {
                *slot = None;
            }
        }
    }
    assert!(visual.peers[0].partitions[0].received_tick >= 96);
}

#[test]
fn delayed_older_ack_cannot_regress_the_sender_base() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    let mut held = false;
    let mut old_image_id = 0;
    for tick in 1..=12 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("first state and ACK");
        if !held
            && let Some(old) = visual
                .pending_acks
                .iter_mut()
                .flatten()
                .find(|ack| ack.peer_index == 0)
        {
            old_image_id = u64::from_le_bytes(
                old.bytes[envelope::HEADER_BYTES + 8..envelope::HEADER_BYTES + 16]
                    .try_into()
                    .expect("fixed ACK image ID"),
            );
            old.arrival = 20;
            held = true;
        }
    }
    assert!(held);
    for tick in 13..=19 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("newer then older ACK");
    }
    let newer_base = visual.peers[0].partitions[0].base_id;
    assert!(newer_base > old_image_id);
    advance(&mut sim, 20);
    visual.step(&sim, 20).expect("older ACK is stale");
    assert!(visual.peers[0].partitions[0].base_id >= newer_base);
    assert!(visual.peers[0].partitions[0].base_tick > 0);
}

#[test]
fn ack_after_candidate_eviction_is_ignored_without_changing_base() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("visual interests");
    let mut held_sequence = None;
    for tick in 1..=100 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("bounded ACK history");
        if held_sequence.is_none()
            && let Some(ack) = visual
                .pending_acks
                .iter_mut()
                .flatten()
                .find(|ack| ack.peer_index == 0)
        {
            let sequence = u64::from_le_bytes(
                ack.bytes[40..48]
                    .try_into()
                    .expect("fixed envelope sequence"),
            );
            ack.arrival = 100;
            held_sequence = Some(sequence);
        }
        for slot in &mut visual.pending_acks {
            if let Some(ack) = slot
                && ack.peer_index == 0
                && Some(u64::from_le_bytes(
                    ack.bytes[40..48]
                        .try_into()
                        .expect("fixed envelope sequence"),
                )) != held_sequence
            {
                *slot = None;
            }
        }
    }
    assert!(held_sequence.is_some());
    assert_eq!(visual.peers[0].partitions[0].base_id, 1);
    assert!(visual.report.unknown_acks > 0);
    assert!(visual.peers[0].partitions[0].received_tick >= 96);
}
