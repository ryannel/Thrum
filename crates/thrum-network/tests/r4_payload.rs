//! Fixed R4 payload size, structural refusal and typed-record round-trip tests.

use thrum_network::r4::{
    ACTOR_BYTES, ActionResult, Actor, Event, GroupError, GroupHeader, GroupInput, Lifecycle,
    MAX_MESSAGE_BYTES, MAX_PAYLOAD_BYTES, decode, encode,
};

fn actor(id: u64) -> Actor {
    Actor {
        wire_id: id,
        sector: [-7, 8, i32::MAX],
        offset: [i32::MIN, 5, -6],
        quaternion: [i16::MIN, -1, 1, i16::MAX],
        velocity: [-40, 0, 40],
        mode: 3,
        flags: 0x81,
        health: 42,
        inventory_revision: 65535,
        owner: 2,
        visibility_generation: 65534,
    }
}

fn result(sequence: u64) -> ActionResult {
    ActionResult {
        input_sequence: sequence,
        resolved_tick: 7,
        first_event_id: 99,
        action_type: 3,
        reason: 0,
        event_count: 1,
        disposition: 0,
    }
}

fn event(id: u64) -> Event {
    Event {
        event_id: id,
        commit_tick: 7,
        origin_epoch: 8,
        origin_input_sequence: 9,
        kind: 2,
        ordinal: 0,
        flags: 1,
    }
}

fn header() -> GroupHeader {
    GroupHeader {
        group_id: 4,
        interest_generation: 2,
        commit_tick: 7,
        resolved_through: 9,
    }
}

fn golden_bytes() -> Vec<u8> {
    // Independently laid out from NETWORK-SPEC's field order, including signed LE values.
    let hex = concat!(
        "04 03 02 01 08 07 06 05 18 17 16 15 14 13 12 11 28 27 26 25 24 23 22 21 01 00 01 00 01 00 01 00 ",
        "38 37 36 35 34 33 32 31 01 00 00 00 fe ff ff ff 78 56 34 12 fd ff ff ff 04 00 00 00 05 00 00 00 ff ff 02 00 fd ff 04 00 05 00 fa ff 07 00 08 09 0b 0a 0d 0c 0f 0e 11 10 ",
        "48 47 46 45 44 43 42 41 15 14 13 12 17 16 18 19 ",
        "58 57 56 55 54 53 52 51 08 07 06 05 04 03 02 01 68 67 66 65 64 63 62 61 74 73 72 71 00 00 01 00 ",
        "68 67 66 65 64 63 62 61 08 07 06 05 04 03 02 01 88 87 86 85 84 83 82 81 58 57 56 55 54 53 52 51 94 93 92 91 96 95 98 97",
    );
    hex.split_ascii_whitespace()
        .map(|pair| u8::from_str_radix(pair, 16).unwrap())
        .collect()
}

#[test]
fn independent_golden_bytes_cover_every_fixed_r4_field() {
    let golden = golden_bytes();
    assert_eq!(golden.len(), 32 + 56 + 16 + 32 + 40);
    let expected_header = GroupHeader {
        group_id: 0x01020304,
        interest_generation: 0x05060708,
        commit_tick: 0x1112131415161718,
        resolved_through: 0x2122232425262728,
    };
    let expected_actor = Actor {
        wire_id: 0x3132333435363738,
        sector: [1, -2, 0x12345678],
        offset: [-3, 4, 5],
        quaternion: [-1, 2, -3, 4],
        velocity: [5, -6, 7],
        mode: 8,
        flags: 9,
        health: 0x0a0b,
        inventory_revision: 0x0c0d,
        owner: 0x0e0f,
        visibility_generation: 0x1011,
    };
    let expected_lifecycle = Lifecycle {
        wire_id: 0x4142434445464748,
        visibility_generation: 0x12131415,
        schema_id: 0x1617,
        kind: 0x18,
        flags: 0x19,
    };
    let expected_result = ActionResult {
        input_sequence: 0x5152535455565758,
        resolved_tick: 0x0102030405060708,
        first_event_id: 0x6162636465666768,
        action_type: 0x71727374,
        reason: 0,
        event_count: 1,
        disposition: 0,
    };
    let expected_event = Event {
        event_id: 0x6162636465666768,
        commit_tick: 0x0102030405060708,
        origin_epoch: 0x8182838485868788,
        origin_input_sequence: 0x5152535455565758,
        kind: 0x91929394,
        ordinal: 0x9596,
        flags: 0x9798,
    };
    let view = decode(&golden).unwrap();
    assert_eq!(view.header, expected_header);
    assert_eq!(view.actors().next(), Some(expected_actor));
    assert_eq!(view.lifecycle().next(), Some(expected_lifecycle));
    assert_eq!(view.results().next(), Some(expected_result));
    assert_eq!(view.events().next(), Some(expected_event));
    let mut encoded = [0_u8; MAX_PAYLOAD_BYTES];
    let len = encode(
        GroupInput {
            header: expected_header,
            actors: &[expected_actor],
            lifecycle: &[expected_lifecycle],
            results: &[expected_result],
            events: &[expected_event],
        },
        &mut encoded,
    )
    .unwrap();
    assert_eq!(&encoded[..len], golden);
}

#[test]
fn fixed_records_round_trip_without_decoded_array_ownership() {
    let actors = [actor(11), actor(12)];
    let lifecycle = [Lifecycle {
        wire_id: 12,
        visibility_generation: 65534,
        schema_id: 6,
        kind: 1,
        flags: 2,
    }];
    let results = [result(9)];
    let events = [event(99)];
    let mut bytes = [0_u8; MAX_PAYLOAD_BYTES];
    let size = encode(
        GroupInput {
            header: header(),
            actors: &actors,
            lifecycle: &lifecycle,
            results: &results,
            events: &events,
        },
        &mut bytes,
    )
    .unwrap();
    assert_eq!(size, 32 + 2 * ACTOR_BYTES + 16 + 32 + 40);
    let view = decode(&bytes[..size]).unwrap();
    assert_eq!(view.header, header());
    assert_eq!(view.actors().collect::<Vec<_>>(), actors);
    assert_eq!(view.lifecycle().collect::<Vec<_>>(), lifecycle);
    assert_eq!(view.results().collect::<Vec<_>>(), results);
    assert_eq!(view.events().collect::<Vec<_>>(), events);
}

#[test]
fn maximum_declared_counts_fit_one_message_and_refuse_overflow() {
    let actors: [Actor; 32] = std::array::from_fn(|index| actor(index as u64 + 1));
    let lifecycle: [Lifecycle; 32] = std::array::from_fn(|index| Lifecycle {
        wire_id: index as u64 + 1,
        visibility_generation: 1,
        schema_id: 1,
        kind: 1,
        flags: 0,
    });
    let results: [ActionResult; 64] = std::array::from_fn(|index| ActionResult {
        first_event_id: index as u64 + 100,
        ..result(index as u64 + 1)
    });
    let events: [Event; 64] = std::array::from_fn(|index| Event {
        event_id: index as u64 + 100,
        origin_input_sequence: index as u64 + 1,
        ..event(0)
    });
    let mut bytes = [0_u8; MAX_PAYLOAD_BYTES];
    let input = GroupInput {
        header: GroupHeader {
            resolved_through: 64,
            ..header()
        },
        actors: &actors,
        lifecycle: &lifecycle,
        results: &results,
        events: &events,
    };
    let size = encode(input, &mut bytes).unwrap();
    assert_eq!(size, MAX_PAYLOAD_BYTES);
    assert_eq!(size + 64, 7008);
    assert!(size + 64 <= MAX_MESSAGE_BYTES);
    let view = decode(&bytes[..size]).unwrap();
    assert_eq!(view.actors().len(), 32);
    assert_eq!(view.results().len(), 64);
    assert_eq!(view.events().len(), 64);

    let too_many = [actor(2); 33];
    let before = bytes;
    assert_eq!(
        encode(
            GroupInput {
                header: header(),
                actors: &too_many,
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut bytes,
        ),
        Err(GroupError::CountExceeded)
    );
    assert_eq!(bytes, before);
    assert_eq!(
        encode(
            GroupInput {
                header: header(),
                actors: &actors,
                lifecycle: &lifecycle,
                results: &results,
                events: &events,
            },
            &mut bytes[..size - 1],
        ),
        Err(GroupError::DestinationTooSmall)
    );
    assert_eq!(bytes, before);
}

#[test]
fn malformed_counts_lengths_and_result_ranges_refuse_before_admission() {
    let mut bytes = [0_u8; MAX_PAYLOAD_BYTES];
    let size = encode(
        GroupInput {
            header: header(),
            actors: &[actor(1)],
            lifecycle: &[],
            results: &[result(9)],
            events: &[],
        },
        &mut bytes,
    )
    .unwrap();
    assert!(decode(&bytes[..size]).is_ok());
    assert!(matches!(
        decode(&bytes[..size - 1]),
        Err(GroupError::LengthMismatch)
    ));
    assert!(matches!(
        decode(&bytes[..size + 1]),
        Err(GroupError::LengthMismatch)
    ));
    assert!(matches!(
        decode(&bytes[..31]),
        Err(GroupError::LengthMismatch)
    ));

    let original = bytes;
    bytes[24..26].copy_from_slice(&33_u16.to_le_bytes());
    assert!(matches!(
        decode(&bytes[..size]),
        Err(GroupError::CountExceeded)
    ));
    bytes = original;
    // Result starts after the 32-byte header and one 56-byte actor.
    let disposition = 32 + ACTOR_BYTES + 31;
    bytes[disposition] = 2;
    assert!(matches!(
        decode(&bytes[..size]),
        Err(GroupError::InvalidResult)
    ));
    bytes = original;
    bytes[32 + ACTOR_BYTES + 16..32 + ACTOR_BYTES + 24].copy_from_slice(&u64::MAX.to_le_bytes());
    bytes[32 + ACTOR_BYTES + 30] = 2; // Event range wraps after MAX.
    assert!(matches!(
        decode(&bytes[..size]),
        Err(GroupError::InvalidResult)
    ));
}

#[test]
#[ignore = "local isolated release-build codec cost probe"]
fn measure_r4_payload_cost() {
    use std::{hint::black_box, time::Instant};

    let actors: [Actor; 32] = std::array::from_fn(|index| actor(index as u64 + 1));
    let lifecycle: [Lifecycle; 32] = std::array::from_fn(|index| Lifecycle {
        wire_id: index as u64 + 1,
        visibility_generation: 1,
        schema_id: 1,
        kind: 1,
        flags: 0,
    });
    let results: [ActionResult; 64] = std::array::from_fn(|index| ActionResult {
        first_event_id: index as u64 + 100,
        ..result(index as u64 + 1)
    });
    let events: [Event; 64] = std::array::from_fn(|index| Event {
        event_id: index as u64 + 100,
        origin_input_sequence: index as u64 + 1,
        ..event(0)
    });
    let cases = [
        (
            "ordinary",
            &actors[..],
            &lifecycle[..0],
            &results[..0],
            &events[..0],
            100_000,
        ),
        (
            "max",
            &actors[..],
            &lifecycle[..],
            &results[..],
            &events[..],
            30_000,
        ),
    ];
    for (name, actors, lifecycle, results, events, rounds) in cases {
        let mut bytes = [0_u8; MAX_PAYLOAD_BYTES];
        let input = || GroupInput {
            header: header(),
            actors,
            lifecycle,
            results,
            events,
        };
        let len = encode(input(), &mut bytes).unwrap();
        let start = Instant::now();
        for _ in 0..rounds {
            let len = encode(black_box(input()), black_box(&mut bytes)).unwrap();
            black_box(len);
        }
        let encode_ns = start.elapsed().as_nanos() as f64 / rounds as f64;
        let start = Instant::now();
        let mut checksum = 0_u64;
        for _ in 0..rounds {
            let view = decode(black_box(&bytes[..len])).unwrap();
            for actor in view.actors() {
                checksum = checksum.wrapping_add(actor.wire_id);
                for value in actor.sector.into_iter().chain(actor.offset) {
                    checksum = checksum.wrapping_add(value as u64);
                }
                for value in actor.quaternion.into_iter().chain(actor.velocity) {
                    checksum = checksum.wrapping_add(value as u64);
                }
                for value in [
                    actor.mode as u64,
                    actor.flags as u64,
                    actor.health as u64,
                    actor.inventory_revision as u64,
                    actor.owner as u64,
                    actor.visibility_generation as u64,
                ] {
                    checksum = checksum.wrapping_add(value);
                }
            }
            for record in view.lifecycle() {
                checksum = checksum.wrapping_add(record.wire_id);
                checksum = checksum.wrapping_add(record.visibility_generation as u64);
                checksum = checksum.wrapping_add(record.schema_id as u64);
                checksum = checksum.wrapping_add(record.kind as u64);
                checksum = checksum.wrapping_add(record.flags as u64);
            }
            for result in view.results() {
                checksum = checksum.wrapping_add(result.input_sequence);
                checksum = checksum.wrapping_add(result.resolved_tick);
                checksum = checksum.wrapping_add(result.first_event_id);
                checksum = checksum.wrapping_add(result.action_type as u64);
                checksum = checksum.wrapping_add(result.reason as u64);
                checksum = checksum.wrapping_add(result.event_count as u64);
                checksum = checksum.wrapping_add(result.disposition as u64);
            }
            for event in view.events() {
                checksum = checksum.wrapping_add(event.event_id);
                checksum = checksum.wrapping_add(event.commit_tick);
                checksum = checksum.wrapping_add(event.origin_epoch);
                checksum = checksum.wrapping_add(event.origin_input_sequence);
                checksum = checksum.wrapping_add(event.kind as u64);
                checksum = checksum.wrapping_add(event.ordinal as u64);
                checksum = checksum.wrapping_add(event.flags as u64);
            }
        }
        let decode_ns = start.elapsed().as_nanos() as f64 / rounds as f64;
        println!(
            "{name},bytes={len},encode_ns={encode_ns:.1},decode_scan_ns={decode_ns:.1},checksum={checksum}"
        );
    }
}
