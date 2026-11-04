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

fn ready(sim: &mut Simulation, visual: &mut VisualDeltaLoop) {
    for tick in 1..=18 {
        advance_axes(sim, tick, [0.0; 3]);
        visual
            .step(sim, tick)
            .expect("reliable full-image admission");
    }
    assert!(visual.peers.iter().all(|peer| peer.full.acked));
    assert_eq!(visual.report.full_images, 16);
}

#[test]
fn lost_initial_full_partition_retries_without_partial_client_admission() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    advance(&mut sim, 1);
    visual.step(&sim, 1).expect("send initial full images");
    let lost = visual
        .pending_full
        .iter_mut()
        .find(|slot| {
            slot.as_ref()
                .is_some_and(|packet| packet.peer_index == 0 && packet.partition == 2)
        })
        .expect("third partition in flight");
    *lost = None;
    visual.report.full_dropped += 1;
    for tick in 2..=6 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("other images may stage");
    }
    assert!(!visual.peers[0].full.client_admitted);
    assert!(
        visual.peers[0]
            .partitions
            .iter()
            .all(|state| state.client_initial_id == 0 && state.received_tick == 0)
    );
    assert!(
        !visual
            .pending
            .iter()
            .flatten()
            .any(|packet| packet.peer_index == 0)
    );
    for tick in 7..=42 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("retry and later delta");
    }
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 16);
    assert!(visual.report.full_retries > 0);
    assert!(visual.peers[0].partitions[0].received_tick >= 33);
    assert_ne!(
        visual.peers[0].partitions[0].received,
        visual.peers[0].partitions[0].base
    );
}

#[test]
fn malformed_full_partition_refuses_before_four_partition_publish() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    advance(&mut sim, 1);
    visual.step(&sim, 1).expect("send full images");
    let packet = visual
        .pending_full
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0 && packet.partition == 0)
        .expect("first full partition");
    packet.bytes[envelope::HEADER_BYTES + 57] = 0;
    packet.arrival = 2;
    advance(&mut sim, 2);
    visual
        .step(&sim, 2)
        .expect("malformed visual packet is shed");
    assert!(!visual.peers[0].full.client_admitted);
    assert!(
        visual.peers[0]
            .partitions
            .iter()
            .all(|state| state.client_initial_id == 0 && state.received_tick == 0)
    );
    assert_eq!(visual.report.full_acks_sent, 0);
    assert_eq!(visual.report.full_invalid, 1);
    for tick in 3..=30 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("repaired full image");
    }
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 16);
}

#[test]
fn malformed_duplicate_cannot_poison_an_already_staged_full_partition() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    advance_axes(&mut sim, 1, [0.0; 3]);
    visual.step(&sim, 1).expect("send initial images");
    let first = visual
        .pending_full
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0 && packet.partition == 0)
        .expect("first partition packet");
    first.arrival = 2;
    advance_axes(&mut sim, 2, [0.0; 3]);
    visual.step(&sim, 2).expect("stage first partition");
    assert_eq!(visual.peers[0].full.staged_mask & 1, 1);
    let mut duplicate = visual.peers[0].full.packets[0];
    duplicate.bytes[envelope::HEADER_BYTES + 57] = 0;
    duplicate.arrival = 3;
    *visual
        .pending_full
        .iter_mut()
        .find(|slot| slot.is_none())
        .expect("free link slot") = Some(duplicate);
    visual.report.full_sent += 1;
    visual.report.full_sent_bytes += u64::try_from(duplicate.len).expect("bounded packet");
    advance_axes(&mut sim, 3, [0.0; 3]);
    visual.step(&sim, 3).expect("discard duplicate");
    assert_eq!(visual.report.full_invalid, 1);
    assert_eq!(visual.peers[0].full.staged_mask & 1, 1);
    assert_eq!(
        visual.peers[0].partitions[0].scratch,
        visual.peers[0].partitions[0].base
    );
}

#[test]
fn old_full_packets_cannot_stage_a_new_interest_generation() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    advance(&mut sim, 1);
    visual.step(&sim, 1).expect("old generation full transfer");
    visual
        .rebaseline(&sim, bindings[0])
        .expect("new same-epoch interest generation");
    for tick in 2..=30 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("ignore old full packets");
    }
    assert!(visual.report.stale >= 1);
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.peers[0].partitions[0].interest_generation, 2);
    assert_eq!(visual.peers[0].partitions[0].client_initial_tick, 1);
    assert_eq!(visual.report.full_images, 16);
}

#[test]
fn lost_baseline_ack_replays_exact_full_images_without_double_admission() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    for tick in 1..=5 {
        advance_axes(&mut sim, tick, [0.0; 3]);
        visual.step(&sim, tick).expect("stage initial baseline");
    }
    assert!(visual.peers[0].full.client_admitted);
    let ack = visual
        .pending_full_acks
        .iter_mut()
        .find(|slot| slot.as_ref().is_some_and(|ack| ack.peer_index == 0))
        .expect("first baseline ACK in flight");
    *ack = None;
    visual.report.full_acks_dropped += 1;
    for tick in 6..=30 {
        advance_axes(&mut sim, tick, [0.0; 3]);
        visual.step(&sim, tick).expect("full-image ACK retry");
    }
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 16);
    assert!(visual.report.full_retries > 0);
    assert!(visual.report.full_acks_sent > 4);
}

#[test]
fn malformed_baseline_ack_is_counted_and_retried() {
    let mut sim = scene::simulation().expect("V1 world");
    let bindings = std::array::from_fn(|index| {
        scene::binding(thrum_core::PeerId(
            u64::try_from(index + 1).expect("four peers"),
        ))
    });
    let mut visual = VisualDeltaLoop::new(&sim, &bindings).expect("four full transfers");
    for tick in 1..=5 {
        advance_axes(&mut sim, tick, [0.0; 3]);
        visual.step(&sim, tick).expect("stage initial baseline");
    }
    let ack = visual
        .pending_full_acks
        .iter_mut()
        .flatten()
        .find(|ack| ack.peer_index == 0)
        .expect("first baseline ACK in flight");
    ack.bytes[0] = 0;
    ack.arrival = 6;
    for tick in 6..=30 {
        advance_axes(&mut sim, tick, [0.0; 3]);
        visual.step(&sim, tick).expect("malformed ACK is shed");
    }
    assert_eq!(visual.report.full_acks_invalid, 1);
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 16);
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
    ready(&mut sim, &mut visual);
    let unchanged_before = visual.report.unchanged_partitions;
    for tick in 19..=27 {
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("only active partition sends");
    }
    assert_eq!(visual.report.sent, 12);
    assert_eq!(visual.report.unchanged_partitions - unchanged_before, 36);
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
    ready(&mut sim, &mut visual);
    for tick in 19..=21 {
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
    packet.arrival = 23;
    visual
        .rebaseline(&sim, bindings[0])
        .expect("same-epoch membership baseline");
    assert_eq!(visual.peers[0].partitions[0].interest_generation, 2);
    assert!(visual.peers[0].partitions[0].base_id > old_image_id);
    assert_eq!(visual.peers[0].client_datagram_sequence, old_sequence);
    for tick in 22..=23 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("retire old generation");
    }
    assert_eq!(visual.report.stale, 1);
    assert!(!visual.peers[0].full.client_admitted);
    for tick in 24..=42 {
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("new generation full admission");
    }
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 20);
    assert!(
        visual.peers[0]
            .partitions
            .iter()
            .all(|part| part.interest_generation == 2 && part.client_initial_tick == 21)
    );
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
    ready(&mut sim, &mut visual);
    for tick in 19..=21 {
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
    ready(&mut sim, &mut visual);
    for tick in 19..=138 {
        let edits = handles.map(|handle| {
            let mut position = sim.get(handle).expect("static actor view").position;
            position.0[0] += 0.001;
            Command::SetPosition(handle, position)
        });
        sim.step(&edits).expect("four independent partition edits");
        visual
            .step(&sim, tick)
            .expect("conditioned four-partition delivery");
        if tick == 21 {
            visual
                .pending
                .iter_mut()
                .flatten()
                .find(|packet| packet.peer_index == 0 && packet.partition == 0)
                .expect("first partition packet")
                .arrival = 25;
            visual
                .pending
                .iter_mut()
                .flatten()
                .find(|packet| packet.peer_index == 0 && packet.partition == 1)
                .expect("second partition packet")
                .arrival = 23;
        }
    }
    assert_eq!(visual.report.sent, 640);
    assert!(visual.report.rebases > 500);
    assert_eq!(visual.report.unknown_acks, 0);
    for peer in &visual.peers {
        assert!(peer.partitions.iter().all(|part| part.received_tick >= 132));
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
    ready(&mut sim, &mut visual);
    for tick in 19..=21 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("first changed sample");
    }
    visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0)
        .expect("first changed packet")
        .arrival = 22;
    advance_axes(&mut sim, 22, [-1.0, 0.0, 0.0]);
    visual.step(&sim, 22).expect("first sample admitted");
    for slot in &mut visual.pending_acks {
        if slot.as_ref().is_some_and(|ack| ack.peer_index == 0) {
            *slot = None;
        }
    }
    for tick in 23..=27 {
        let axes = if tick <= 24 {
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
    correction.arrival = 28;
    advance_axes(&mut sim, 28, [0.0; 3]);
    visual
        .step(&sim, 28)
        .expect("corrected base sample admitted");
    assert_eq!(visual.peers[0].partitions[0].received_tick, 27);
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
        .arrival = 29;
    for tick in 29..=36 {
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
    ready(&mut sim, &mut visual);
    for tick in 19..=24 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("conditioned visual sample");
        if tick == 21 {
            let early = visual
                .pending
                .iter_mut()
                .flatten()
                .find(|packet| packet.peer_index == 0)
                .expect("first peer sample");
            early.arrival = 27;
        }
        if tick == 24 {
            let later = visual
                .pending
                .iter_mut()
                .flatten()
                .filter(|packet| packet.peer_index == 0)
                .nth(1)
                .expect("second peer sample");
            later.arrival = 26;
        }
    }
    for tick in 25..=27 {
        advance(&mut sim, tick);
        visual
            .step(&sim, tick)
            .expect("conditioned visual delivery");
    }
    assert_eq!(visual.peers[0].partitions[0].received_tick, 24);
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
    ready(&mut sim, &mut visual);
    for tick in 19..=21 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("conditioned visual sample");
    }
    let pending = visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0)
        .expect("old epoch packet");
    pending.arrival = 23;
    let mut new_binding = bindings[0];
    new_binding.connection_epoch += 1;
    visual
        .rebaseline(&sim, new_binding)
        .expect("new connection visual base");
    for tick in 22..=23 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("deliver old packet");
    }
    assert_eq!(visual.report.stale, 1);
    assert!(!visual.peers[0].full.client_admitted);
    for tick in 24..=42 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("new epoch full admission");
    }
    assert!(visual.peers[0].full.acked);
    assert_eq!(visual.report.full_images, 20);
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
    ready(&mut sim, &mut visual);
    for tick in 19..=21 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("visual sample");
    }
    let original = visual
        .pending
        .iter_mut()
        .flatten()
        .find(|packet| packet.peer_index == 0)
        .expect("first visual packet");
    original.arrival = 22;
    let duplicate = Packet {
        peer_index: original.peer_index,
        partition: original.partition,
        arrival: 23,
        bytes: original.bytes,
        len: original.len,
        expected: original.expected,
    };
    *visual
        .pending
        .iter_mut()
        .find(|slot| slot.is_none())
        .expect("free link slot") = Some(duplicate);
    for tick in 22..=23 {
        advance(&mut sim, tick);
        visual.step(&sim, tick).expect("duplicate delivery");
    }
    assert_eq!(visual.peers[0].partitions[0].received_tick, 21);
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
