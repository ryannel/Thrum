//! Game-neutral input bytes, framing failures and independent field oracle.

use thrum_core::{
    ActorId, Origin, PeerId,
    sim::{ControlBinding, InputFrame},
};
use thrum_network::input_batch::{
    ActionView, ActionWire, EncodeRecord, InputCodecError, canonical_axes, decode, encode,
};

struct TinyAction;

impl ActionWire<u16> for TinyAction {
    fn encode(
        &self,
        action: &u16,
        destination: &mut [u8],
    ) -> Result<(u32, usize), InputCodecError> {
        let bytes = destination
            .get_mut(..2)
            .ok_or(InputCodecError::DestinationTooSmall)?;
        bytes.copy_from_slice(&action.to_le_bytes());
        Ok((7, 2))
    }

    fn decode(&self, action: ActionView<'_>) -> Result<u16, InputCodecError> {
        if action.schema_id != 7 || action.payload.len() != 2 {
            return Err(InputCodecError::InvalidAction);
        }
        Ok(u16::from_le_bytes(
            action.payload.try_into().expect("checked two-byte action"),
        ))
    }
}

fn frame(sequence: u64, tick: u64, axes: [f32; 3], actions: Vec<u16>) -> InputFrame<u16> {
    let mut frame = InputFrame::new(
        Origin {
            peer: PeerId(3),
            connection_epoch: 9,
            sequence,
        },
        ActorId(5),
        tick,
        axes,
    );
    frame.actions.extend(actions);
    frame
}

fn owner() -> ControlBinding {
    ControlBinding {
        peer: PeerId(3),
        connection_epoch: 9,
        actor: ActorId(5),
    }
}

#[test]
fn exact_little_endian_oracle_and_game_declared_action_round_trip() {
    let input = frame(0x0102_0304_0506_0708, 11, [1.0, -1.0, 0.0], vec![0x1234]);
    let mut bytes = [0u8; 128];
    let len = encode(
        &[EncodeRecord {
            frame: &input,
            actor_wire_id: 0x1112_1314_1516_1718,
        }],
        &TinyAction,
        &mut bytes,
    )
    .expect("one record");
    let expected = [
        1, // record count
        8, 7, 6, 5, 4, 3, 2, 1, // input sequence
        24, 23, 22, 21, 20, 19, 18, 17, // wire actor
        11, 0, 0, 0, 0, 0, 0, 0, // target tick
        255, 127, 1, 128, 0, 0, // signed normalized axes
        1, // action count
        7, 0, 0, 0, 2, 0, 52, 18, // schema, length and value
    ];
    assert_eq!(&bytes[..len], expected);
    let batch = decode(&bytes[..len]).expect("checked bytes");
    assert_eq!(batch.len(), 1);
    let record = batch.records().next().expect("one record");
    let decoded = record
        .decode_frame(owner(), 0x1112_1314_1516_1718, &TinyAction)
        .expect("game schema");
    assert_eq!(decoded, input);
    assert_eq!(
        record.decode_frame(owner(), 99, &TinyAction),
        Err(InputCodecError::WrongActor)
    );
}

#[test]
fn every_truncation_refuses_and_unknown_action_requires_game_validation() {
    let first = frame(1, 5, [0.25, 0.0, 0.0], vec![10]);
    let second = frame(2, 6, [0.0, 0.25, 0.0], vec![20]);
    let mut bytes = [0u8; 128];
    let len = encode(
        &[
            EncodeRecord {
                frame: &first,
                actor_wire_id: 5,
            },
            EncodeRecord {
                frame: &second,
                actor_wire_id: 5,
            },
        ],
        &TinyAction,
        &mut bytes,
    )
    .expect("two records");
    for end in 0..len {
        assert!(decode(&bytes[..end]).is_err(), "truncation at {end}");
    }
    assert_eq!(
        decode(&bytes[..=len]).err(),
        Some(InputCodecError::TrailingBytes)
    );
    bytes[32..36].copy_from_slice(&8u32.to_le_bytes());
    let checked = decode(&bytes[..len]).expect("structural schema ID");
    assert_eq!(
        checked
            .records()
            .next()
            .expect("first record")
            .decode_frame(owner(), 5, &TinyAction),
        Err(InputCodecError::InvalidAction)
    );
}

#[test]
fn quantizer_boundaries_and_invalid_axes_are_explicit() {
    let threshold = 0.5_f32 / 32767.0;
    let above = f32::from_bits(threshold.to_bits() + 1);
    let records = [
        frame(1, 5, [threshold, -threshold, 0.0], Vec::new()),
        frame(2, 6, [above, -above, 1.0], Vec::new()),
    ];
    let mut bytes = [0u8; 128];
    let encoded = [
        EncodeRecord {
            frame: &records[0],
            actor_wire_id: 5,
        },
        EncodeRecord {
            frame: &records[1],
            actor_wire_id: 5,
        },
    ];
    let len = encode(&encoded, &TinyAction, &mut bytes).expect("boundary axes");
    let decoded = decode(&bytes[..len]).expect("boundary packet");
    let axes: Vec<_> = decoded.records().map(|record| record.axes).collect();
    assert_eq!(axes[0], [0.0, 0.0, 0.0]);
    assert_eq!(axes[1], [1.0 / 32767.0, -1.0 / 32767.0, 1.0]);
    assert_eq!(canonical_axes(records[0].axes), Ok(axes[0]));
    assert_eq!(canonical_axes(records[1].axes), Ok(axes[1]));
    assert_eq!(
        canonical_axes([0.5, -0.5, 0.0]),
        Ok([16384.0 / 32767.0, -16384.0 / 32767.0, 0.0])
    );
    let mut reserved = bytes;
    reserved[25..27].copy_from_slice(&i16::MIN.to_le_bytes());
    assert_eq!(
        decode(&reserved[..len]).err(),
        Some(InputCodecError::InvalidAxis)
    );
    let invalid = frame(1, 5, [f32::NAN, 0.0, 0.0], Vec::new());
    assert_eq!(
        encode(
            &[EncodeRecord {
                frame: &invalid,
                actor_wire_id: 5,
            }],
            &TinyAction,
            &mut bytes,
        ),
        Err(InputCodecError::InvalidAxis)
    );
}
