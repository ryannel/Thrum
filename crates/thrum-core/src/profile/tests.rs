use super::*;

#[test]
fn encoding_cannot_observe_nan_sign_or_payload() {
    let mut canonical = Digest::default();
    0x7fc0_0000_u32.encode(&mut canonical);
    for bits in [0x7fc0_0001, 0x7fa0_0002, 0xffc0_0042, 0xffa0_4321] {
        let value = Number(f32::from_bits(bits));
        let mut actual = Digest::default();
        value.encode(&mut actual);
        assert_eq!(actual.finish(), canonical.finish());
        assert!(!actual.1);
    }
}

#[test]
fn philox_matches_random123_published_known_answers() {
    // https://github.com/DEShawResearch/random123/blob/main/tests/kat_vectors
    for (counter, key, expected) in [
        (0, 0, 0xff1d_ae59_u32),
        (u64::MAX, 0xffff_ffff, 0x2c3f_628b),
        (0x85a3_08d3_243f_6a88, 0x1319_8a2e, 0xdd7c_e038),
    ] {
        let expected = (expected >> 8) as f32 / 16_777_216.0;
        assert_eq!(random(key, counter, 0, 0).0.to_bits(), expected.to_bits());
    }
}

#[test]
fn arithmetic_fixture_matches_retained_words() {
    let retained = include_str!("vectors.json");
    let expected: Vec<u32> = retained
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|word| word.trim().parse().expect("retained word"))
        .collect();
    assert_eq!(conformance_vectors(), expected);
}

#[test]
fn fixture_includes_every_wire_value_and_rounding_boundary() {
    let vectors = conformance_vectors();
    let arithmetic_words = 3790;
    let wire_values = 65535;
    let boundary_values = 65534 * 3;
    assert_eq!(
        vectors.len(),
        arithmetic_words + wire_values + boundary_values + 10
    );
    for (index, value) in (-32767_i32..=32767).enumerate() {
        let decoded = value as f32 / 32767.0;
        // Both encodings of zero canonicalize to positive zero on the wire.
        assert_eq!(vectors[arithmetic_words + index], decoded.to_bits());
    }
}

#[test]
fn random_has_no_call_order_or_thread_state() {
    let expected: Vec<_> = (0..256)
        .map(|tick| random(123, tick, u128::from(tick), 7).0.to_bits())
        .collect();
    let mut reverse: Vec<_> = (0..256)
        .rev()
        .map(|tick| random(123, tick, u128::from(tick), 7).0.to_bits())
        .collect();
    reverse.reverse();
    assert_eq!(reverse, expected);
}
