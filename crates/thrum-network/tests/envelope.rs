//! Independent fixed-envelope bytes and bounded structural refusal tests.

use thrum_network::envelope::{self, EnvelopeError, Header, Kind};

fn header(kind: Kind) -> Header {
    Header {
        kind,
        match_id: *b"0123456789abcdef",
        connection_epoch: 0x0807_0605_0403_0201,
        message_sequence: 0x1817_1615_1413_1211,
        server_tick: 0x2827_2625_2423_2221,
        acknowledged_sequence: 0x3837_3635_3433_3231,
    }
}

#[test]
fn exact_golden_header_and_borrowed_payload() {
    let payload = [0x91, 0xa2, 0xb3];
    let mut encoded = [0u8; 67];
    let size = envelope::encode(header(Kind::StateGroup), &payload, &mut encoded)
        .expect("test operation should succeed");
    assert_eq!(size, 67);
    let expected = [
        b'E', b'N', b'G', b'1', 1, 0, 10, 0, 3, 0, 0, 0, 0, 0, 0, 0, b'0', b'1', b'2', b'3', b'4',
        b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f', 1, 2, 3, 4, 5, 6, 7, 8,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,
        0x28, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x91, 0xa2, 0xb3,
    ];
    assert_eq!(encoded, expected);
    let decoded = envelope::decode(&encoded).expect("test operation should succeed");
    assert_eq!(decoded.header, header(Kind::StateGroup));
    assert_eq!(decoded.payload, payload);
    assert_eq!(decoded.payload.as_ptr(), encoded[64..].as_ptr());
}

#[test]
fn malformed_header_and_length_refuse() {
    let mut bytes = [0u8; 64];
    envelope::encode(header(Kind::Clock), &[], &mut bytes).expect("test operation should succeed");
    assert!(matches!(
        envelope::decode(&bytes[..63]),
        Err(EnvelopeError::Length)
    ));
    for (offset, value, expected) in [
        (0, b'X', EnvelopeError::Magic),
        (4, 2, EnvelopeError::Version),
        (6, 0, EnvelopeError::ReservedKind),
        (6, 15, EnvelopeError::ReservedKind),
        (7, 1, EnvelopeError::ReservedField),
        (12, 1, EnvelopeError::ReservedField),
        (8, 1, EnvelopeError::Length),
    ] {
        let mut invalid = bytes;
        invalid[offset] = value;
        assert!(matches!(envelope::decode(&invalid), Err(error) if error == expected));
    }
    for ordinal in 2..=13 {
        let mut recognized = bytes;
        recognized[6] = ordinal;
        assert_eq!(
            envelope::decode(&recognized)
                .expect("test operation should succeed")
                .header
                .kind as u8,
            ordinal
        );
    }
    let mut long = [0u8; 65];
    long[..64].copy_from_slice(&bytes);
    assert!(matches!(
        envelope::decode(&long),
        Err(EnvelopeError::Length)
    ));
}

#[test]
fn hello_zero_identity_and_exact_size_caps() {
    let hello = Header {
        kind: Kind::Hello,
        match_id: [0; 16],
        connection_epoch: 0,
        message_sequence: 0,
        server_tick: 0,
        acknowledged_sequence: 0,
    };
    let mut bytes = [0u8; 64];
    envelope::encode(hello, &[], &mut bytes).expect("test operation should succeed");
    assert_eq!(
        envelope::decode(&bytes)
            .expect("test operation should succeed")
            .header,
        hello
    );
    for invalid in [
        Header {
            message_sequence: 1,
            ..hello
        },
        Header {
            match_id: [1; 16],
            ..hello
        },
        Header {
            acknowledged_sequence: 1,
            ..hello
        },
    ] {
        assert_eq!(
            envelope::encode(invalid, &[], &mut bytes),
            Err(EnvelopeError::HelloIdentity)
        );
    }
    let payload = vec![0x5a; envelope::MAX_MESSAGE_BYTES - envelope::HEADER_BYTES];
    let mut max = vec![0u8; envelope::MAX_MESSAGE_BYTES];
    assert_eq!(
        envelope::encode(header(Kind::BaselineChunk), &payload, &mut max)
            .expect("test operation should succeed"),
        envelope::MAX_MESSAGE_BYTES
    );
    assert_eq!(
        envelope::decode(&max)
            .expect("test operation should succeed")
            .payload
            .len(),
        payload.len()
    );
    assert_eq!(
        envelope::encode(header(Kind::Clock), &payload, &mut max[..63]),
        Err(EnvelopeError::DestinationTooSmall)
    );
    assert_eq!(
        envelope::encode(
            header(Kind::Clock),
            &vec![0u8; envelope::MAX_MESSAGE_BYTES],
            &mut max
        ),
        Err(EnvelopeError::MessageTooLarge)
    );
    assert!(matches!(
        envelope::decode(&vec![0u8; envelope::MAX_MESSAGE_BYTES + 1]),
        Err(EnvelopeError::MessageTooLarge)
    ));
}
