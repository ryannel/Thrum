//! Exact selective image ACK wire bytes and refusal checks.

use thrum_network::{
    state_ack::{self, AckError, StateAck},
    state_delta::Scope,
};

const ACK: StateAck = StateAck {
    scope: Scope {
        partition_id: 7,
        interest_generation: 3,
    },
    image_id: 0x0807_0605_0403_0201,
    tick: 0x1817_1615_1413_1211,
};

#[test]
fn independent_little_endian_ack_bytes() {
    let mut bytes = [0xff; 32];
    assert_eq!(state_ack::encode(ACK, &mut bytes), Ok(24));
    assert_eq!(
        &bytes[..24],
        &[
            7, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ]
    );
    assert_eq!(state_ack::decode(&bytes[..24]), Ok(ACK));
}

#[test]
fn truncated_extended_and_zero_identity_refuse() {
    let mut bytes = [0u8; 25];
    state_ack::encode(ACK, &mut bytes).expect("valid ACK");
    for end in 0..24 {
        assert_eq!(state_ack::decode(&bytes[..end]), Err(AckError::Length));
    }
    assert_eq!(state_ack::decode(&bytes), Err(AckError::Length));
    bytes[4..8].fill(0);
    assert_eq!(state_ack::decode(&bytes[..24]), Err(AckError::Identity));
    assert_eq!(
        state_ack::encode(ACK, &mut [0u8; 23]),
        Err(AckError::Length)
    );
}
