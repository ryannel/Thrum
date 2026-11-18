use super::*;
use crate::input_batch::InputCodecError;

struct ByteWire;
impl ActionWire<u8> for ByteWire {
    fn encode(&self, action: &u8, output: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        output[0] = *action;
        Ok((19, 1))
    }

    fn decode(&self, action: ActionView<'_>) -> Result<u8, InputCodecError> {
        match (action.schema_id, action.payload) {
            (19, [value]) => Ok(*value),
            _ => Err(InputCodecError::InvalidAction),
        }
    }
}

#[test]
fn every_truncation_of_a_settings_or_set_payload_is_refused() {
    let mut settings = Vec::new();
    write_settings(&mut settings, Settings::default());
    for length in 0..settings.len() {
        assert!(read_settings(&mut Read::new(&settings[..length])).is_none());
    }
    assert_eq!(
        read_settings(&mut Read::new(&settings)),
        Some(Settings::default())
    );
    let prior = [[0.0; 3]; SLOTS];
    let mut frames = std::array::from_fn(|slot| frame(slot, MATCH_EPOCH, 31, 31, [1.0, -1.0, 0.0]));
    frames[0].actions.extend(0..8);
    let mut results = [SlotResult::default(); SLOTS];
    results[0] = SlotResult {
        frame: u64::MAX,
        last_command: u64::MAX,
        commands: 8,
    };
    let set = Set {
        frames,
        results,
        joined: 15,
        digest: Some(115),
    };
    let mut wire = Vec::new();
    write_set(&mut wire, &set, prior, &ByteWire).expect("valid fixture");
    for length in 0..wire.len() {
        assert!(
            read_set(&wire[..length], 31, prior, &ByteWire).is_none(),
            "cut at {length}"
        );
    }
    let restored = read_set(&wire, 31, prior, &ByteWire).expect("intact control");
    assert_eq!(restored.frames, set.frames);
    assert_eq!(restored.results[0].frame, u64::MAX);
    assert_eq!(restored.results[0].last_command, u64::MAX);
    wire.push(0);
    assert!(read_set(&wire, 31, prior, &ByteWire).is_none());
}

#[test]
fn random_set_payloads_and_overlong_integers_never_partially_decode() {
    let prior = [[0.0; 3]; SLOTS];
    let mut random = 0xc2b2_ae3d_27d4_eb4f_u64;
    let mut bytes = [0; 98];
    for length in (0..bytes.len()).cycle().take(4_000) {
        for byte in &mut bytes[..length] {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            *byte = random.to_le_bytes()[0];
        }
        if let Some(set) = read_set(&bytes[..length], 1, prior, &ByteWire) {
            assert!(
                set.frames
                    .iter()
                    .all(|frame| frame.actions.len() <= MAX_ACTIONS)
            );
        }
    }
    assert!(Read::new(&[255; 10]).uint().is_none());
    assert!(Read::new(&[128; 9]).uint().is_none());
}
