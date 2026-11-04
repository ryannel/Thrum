//! Independent small-schema bytes and malformed delta admission checks.

use thrum_network::{
    state_delta::{
        Actor, DeltaError, Image, Scope, decode_delta, decode_full, encode_delta, encode_full,
    },
    state_schema,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Tiny {
    x: u8,
    y: u8,
}

#[test]
fn full_image_rebuilds_membership_from_checked_bytes() {
    let replacement = [
        CURRENT[0],
        Actor {
            wire_id: 4,
            value: Tiny { x: 7, y: 31 },
        },
    ];
    let image = Image {
        scope: Scope {
            partition_id: 7,
            interest_generation: 2,
        },
        id: 11,
        tick: 6,
        actors: &replacement,
    };
    let mut bytes = [0xff; 128];
    let report = encode_full(&TinyWire, image, &mut bytes).expect("full partition image");
    assert_eq!(report.bytes, 62);
    assert_eq!(&bytes[56..62], &[1, 3, 21, 3, 3, 255]);
    let mut scratch = [Actor {
        wire_id: 0,
        value: Tiny::default(),
    }; 2];
    assert_eq!(
        decode_full(&TinyWire, image.scope, &bytes[..report.bytes], &mut scratch),
        Ok(report)
    );
    assert_eq!(scratch, replacement);

    let confirmed = BASE;
    for end in 0..report.bytes {
        assert!(decode_full(&TinyWire, image.scope, &bytes[..end], &mut scratch).is_err());
        assert_eq!(confirmed, BASE);
    }
    let mut corrupt = bytes;
    corrupt[59] = 0;
    assert_eq!(
        decode_full(
            &TinyWire,
            image.scope,
            &corrupt[..report.bytes],
            &mut scratch
        ),
        Err(DeltaError::ActorOrder)
    );
    corrupt = bytes;
    corrupt[57] = 1;
    assert_eq!(
        decode_full(
            &TinyWire,
            image.scope,
            &corrupt[..report.bytes],
            &mut scratch
        ),
        Err(DeltaError::Padding)
    );
    assert_eq!(confirmed, BASE);
}

state_schema! {
    struct TinyWire for Tiny, schema = 3 {
        0 => x: 3,
        1 => y: 5,
    }
}

const BASE: [Actor<Tiny>; 2] = [
    Actor {
        wire_id: 1,
        value: Tiny { x: 1, y: 2 },
    },
    Actor {
        wire_id: 3,
        value: Tiny { x: 2, y: 4 },
    },
];
const CURRENT: [Actor<Tiny>; 2] = [
    Actor {
        wire_id: 1,
        value: Tiny { x: 5, y: 2 },
    },
    Actor {
        wire_id: 3,
        value: Tiny { x: 2, y: 7 },
    },
];

fn base(actors: &[Actor<Tiny>]) -> Image<'_, Tiny> {
    Image {
        scope: Scope {
            partition_id: 7,
            interest_generation: 1,
        },
        id: 9,
        tick: 4,
        actors,
    }
}

#[test]
fn golden_mask_bits_and_canonical_actor_differences() {
    let mut bytes = [0xff; 128];
    let report = encode_delta(&TinyWire, base(&BASE), &CURRENT, 10, 5, &mut bytes)
        .expect("two changed actors");
    assert_eq!(report.changed_actors, 2);
    assert_eq!(report.bytes, 62);
    let expected: [u8; 62] = [
        0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, // mode, count and reserved
        7, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, // partition, generation, schema
        9, 0, 0, 0, 0, 0, 0, 0, // base image
        10, 0, 0, 0, 0, 0, 0, 0, // sample image
        5, 0, 0, 0, 0, 0, 0, 0, // sample tick
        0, 0, 0, 0, 0, 0, 0, 0, // no result watermark
        1, 1, 5, // actor 1, field 0, 3-bit value 5
        2, 2, 7, // actor 3 relative to actor 1, field 1, 5-bit value 7
    ];
    assert_eq!(&bytes[..report.bytes], &expected);
    let mut scratch = BASE;
    let received = decode_delta(&TinyWire, base(&BASE), &bytes[..report.bytes], &mut scratch)
        .expect("checked delta");
    assert_eq!(received, report);
    assert_eq!(scratch, CURRENT);
}

#[test]
fn malformed_or_stale_bytes_never_change_the_base_image() {
    let mut bytes = [0u8; 128];
    let len = encode_delta(&TinyWire, base(&BASE), &CURRENT, 10, 5, &mut bytes)
        .expect("valid bytes")
        .bytes;
    for end in 0..len {
        let mut scratch = BASE;
        assert!(
            decode_delta(&TinyWire, base(&BASE), &bytes[..end], &mut scratch).is_err(),
            "truncation at {end}"
        );
        assert_eq!(BASE[0].value, Tiny { x: 1, y: 2 });
    }
    let mut corrupt = bytes;
    corrupt[58] |= 0x80;
    let mut scratch = BASE;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &corrupt[..len], &mut scratch).err(),
        Some(DeltaError::Padding)
    );
    let mut corrupt = bytes;
    corrupt[56] = 0;
    let mut scratch = BASE;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &corrupt[..len], &mut scratch),
        Err(DeltaError::ActorOrder)
    );
    let mut corrupt = bytes;
    corrupt[24] = 8;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &corrupt[..len], &mut scratch),
        Err(DeltaError::Base)
    );
    assert_eq!(BASE[0].value, Tiny { x: 1, y: 2 });
    assert_eq!(BASE[1].value, Tiny { x: 2, y: 4 });
}

#[test]
fn noncanonical_records_and_out_of_range_fields_refuse() {
    let mut bytes = [0u8; 128];
    let len = encode_delta(&TinyWire, base(&BASE), &CURRENT, 10, 5, &mut bytes)
        .expect("valid packet")
        .bytes;
    let mut scratch = BASE;
    let mut overlong = bytes;
    overlong[56] = 0x81;
    overlong.copy_within(57..len, 58);
    overlong[57] = 0;
    overlong[2] = 2;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &overlong[..=len], &mut scratch),
        Err(DeltaError::Varint)
    );
    let mut absent = bytes;
    absent[59] = 9;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &absent[..len], &mut scratch),
        Err(DeltaError::ActorOrder)
    );
    let mut extra = bytes;
    extra[len] = 0;
    assert_eq!(
        decode_delta(&TinyWire, base(&BASE), &extra[..=len], &mut scratch),
        Err(DeltaError::TrailingBytes)
    );
    let mut bad_value = CURRENT;
    bad_value[0].value.x = 8;
    assert_eq!(
        encode_delta(&TinyWire, base(&BASE), &bad_value, 10, 5, &mut bytes),
        Err(DeltaError::FieldRange)
    );
}
