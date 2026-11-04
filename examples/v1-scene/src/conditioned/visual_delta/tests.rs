//! Visual delta admission, sequencing and retention checks.

use thrum_core::sim::Simulation;

use super::{Packet, VisualDeltaLoop, envelope, quantize};
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
        sim.step(&[]).expect("world tick");
        visual.step(&sim, tick).expect("first ACK rebases");
    }
    assert!(visual.peers[0].base_id > 1);
    for tick in 25..=100 {
        sim.step(&[]).expect("world tick");
        visual
            .step(&sim, tick)
            .expect("current base remains decodable");
        for slot in &mut visual.pending_acks {
            if slot.as_ref().is_some_and(|ack| ack.peer_index == 0) {
                *slot = None;
            }
        }
    }
    assert!(visual.peers[0].received_tick >= 96);
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
        sim.step(&[]).expect("world tick");
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
        sim.step(&[]).expect("world tick");
        visual.step(&sim, tick).expect("newer then older ACK");
    }
    let newer_base = visual.peers[0].base_id;
    assert!(newer_base > old_image_id);
    sim.step(&[]).expect("world tick 20");
    visual.step(&sim, 20).expect("older ACK is stale");
    assert!(visual.peers[0].base_id >= newer_base);
    assert_eq!(visual.peers[0].base_tick + 1, visual.peers[0].base_id);
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
        sim.step(&[]).expect("world tick");
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
    assert_eq!(visual.peers[0].base_id, 1);
    assert!(visual.report.unknown_acks > 0);
    assert!(visual.peers[0].received_tick >= 96);
}
