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
        (0, 0, 0xff1d_ae59_6cd1_0df2),
        (u64::MAX, 0xffff_ffff, 0x2c3f_628b_ab4f_d7ad),
        (0x85a3_08d3_243f_6a88, 0x1319_8a2e, 0xdd7c_e038_f62a_4c12),
    ] {
        let draw = random_u64(key, counter, 0, 0);
        assert_eq!(draw, expected);
        let fraction = Number((expected >> 40) as f32 / 16_777_216.0);
        assert_eq!(random_fraction(draw), fraction);
        assert_eq!(random(key, counter, 0, 0), fraction);
    }
}

#[test]
fn random_fraction_uses_only_the_top_24_bits() {
    assert_eq!(random_fraction((1 << 40) - 1), Number::ZERO);
    assert_eq!(random_fraction(1 << 40), Number(1.0 / 16_777_216.0));
    assert_eq!(random_fraction(u64::MAX), Number(1.0_f32.next_down()));
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
    let draw = |tick| random_u64(123, tick, 42, 7);
    let expected: Vec<_> = (0..256).map(draw).collect();
    assert!(expected.windows(2).any(|pair| pair[0] != pair[1]));
    let mut reverse: Vec<_> = std::thread::spawn(move || (0..256).rev().map(draw).collect())
        .join()
        .expect("draws on another thread");
    reverse.reverse();
    assert_eq!(reverse, expected);
}
