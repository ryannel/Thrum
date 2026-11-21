//! Property tests for every parser of a peer's or a player's bytes: the link hello, the reliable stream's length
//! prefix and a ticket's text. They drive the decoders the adapters use (`wire::Decoder`, `wire::hello`,
//! `wire::prefix` and `Ticket::from_str`) over valid input cut at every byte, the edge lengths and random bytes.
//! A counting allocator checks that no input makes a decoder allocate more than the lane's cap.
use thrum_transport::{
    HELLO, RELIABLE_LIMIT, TICKET_LIMIT, Ticket,
    wire::{self, Decoder, Frame, PREFIX_BYTES, WireError},
};

#[global_allocator]
static ALLOCATOR: thrum_test_support::CountingAllocator = thrum_test_support::CountingAllocator;

/// Run `work` and return its result with the largest single allocation it made on this thread.
fn measured<T>(work: impl FnOnce() -> T) -> (T, usize) {
    let (result, counted) = thrum_test_support::allocations::count(work);
    (
        result,
        usize::try_from(counted.largest).expect("allocation size fits in usize"),
    )
}

/// Deterministic generator, so a failure names a case that reproduces.
struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).expect("small bound")
    }
    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next().to_le_bytes()[0]).collect()
    }
}

/// What a decoder made of a stream fed in the given chunk sizes.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    hello: bool,
    messages: Vec<Vec<u8>>,
    error: Option<WireError>,
    held: usize,
}
fn feed(stream: &[u8], cuts: impl IntoIterator<Item = usize>) -> Outcome {
    let mut decoder = Decoder::new();
    let mut outcome = Outcome {
        hello: false,
        messages: Vec::new(),
        error: None,
        held: 0,
    };
    let mut start = 0;
    let mut ends: Vec<usize> = cuts
        .into_iter()
        .filter(|&c| c > 0 && c < stream.len())
        .collect();
    ends.sort_unstable();
    ends.push(stream.len());
    for end in ends {
        let mut chunk = &stream[start..end];
        start = end;
        while !chunk.is_empty() {
            match decoder.decode(chunk) {
                Ok((used, frame)) => {
                    assert!(used <= chunk.len());
                    chunk = &chunk[used..];
                    match frame {
                        Some(Frame::Hello) => {
                            assert!(
                                !outcome.hello && outcome.messages.is_empty(),
                                "second hello"
                            );
                            outcome.hello = true;
                        }
                        Some(Frame::Message(bytes)) => {
                            assert!(outcome.hello, "message before the hello");
                            outcome.messages.push(bytes);
                        }
                        None => assert!(chunk.is_empty(), "decoder stopped short of a frame"),
                    }
                }
                Err(error) => {
                    outcome.error = Some(error);
                    assert_eq!(
                        decoder.decode(&[0]),
                        Err(error),
                        "a refused stream must stay refused"
                    );
                    outcome.held = decoder.held();
                    return outcome;
                }
            }
        }
    }
    outcome.held = decoder.held();
    outcome
}

fn sample_messages(random: &mut Random) -> Vec<Vec<u8>> {
    [
        1,
        2,
        3,
        4,
        5,
        255,
        1_000,
        16_384,
        RELIABLE_LIMIT - 1,
        RELIABLE_LIMIT,
    ]
    .into_iter()
    .map(|length| random.bytes(length))
    .collect()
}

#[test]
fn a_valid_stream_cut_at_every_byte_decodes_to_exactly_what_was_encoded() {
    let mut random = Random(1);
    let small: Vec<Vec<u8>> = (1..=12).map(|length| random.bytes(length)).collect();
    let stream = wire::encode_stream(small.iter().map(Vec::as_slice));
    // Every single cut, and every pair of cuts, of a stream of short messages.
    for first in 0..=stream.len() {
        assert_eq!(feed(&stream, [first]).messages, small, "cut at {first}");
        for second in first..=stream.len() {
            let outcome = feed(&stream, [first, second]);
            assert!(outcome.hello && outcome.error.is_none() && outcome.held == 0);
            assert_eq!(outcome.messages, small, "cuts at {first} and {second}");
        }
    }
    // Every cut of a stream with messages up to the limit, and one byte at a time.
    let large = sample_messages(&mut random);
    let stream = wire::encode_stream(large.iter().map(Vec::as_slice));
    for cut in 0..=stream.len() {
        let (outcome, largest) = measured(|| feed(&stream, [cut]));
        assert_eq!(outcome.messages, large, "cut at {cut}");
        assert!(largest <= RELIABLE_LIMIT, "allocated {largest} bytes");
    }
    assert_eq!(feed(&stream, 0..stream.len()).messages, large);
    // A stream that stops part-way delivers the whole messages before the cut and holds the rest, bounded.
    for cut in 0..stream.len() {
        let outcome = feed(&stream[..cut], []);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.messages, large[..outcome.messages.len()]);
        assert!(outcome.held <= RELIABLE_LIMIT);
        let delivered: usize = outcome
            .messages
            .iter()
            .map(|m| PREFIX_BYTES + m.len())
            .sum();
        // What it holds is the partial frame after the last whole one (a body's prefix is not kept).
        let partial = cut - delivered - if outcome.hello { HELLO.len() } else { 0 };
        assert!(
            outcome.held <= partial && outcome.held + PREFIX_BYTES >= partial,
            "cut {cut}"
        );
    }
}

#[test]
fn prefix_lengths_at_and_beyond_the_limit_are_decoded_or_refused_before_any_body() {
    let edge = |length: u32| {
        let mut stream = HELLO.to_vec();
        stream.extend_from_slice(&length.to_le_bytes());
        stream.extend(std::iter::repeat_n(
            7,
            usize::try_from(length).unwrap_or(0).min(RELIABLE_LIMIT + 1),
        ));
        stream
    };
    assert_eq!(wire::prefix(0_u32.to_le_bytes()), Err(WireError::Empty));
    assert_eq!(wire::prefix(1_u32.to_le_bytes()), Ok(1));
    assert_eq!(wire::prefix(65_536_u32.to_le_bytes()), Ok(RELIABLE_LIMIT));
    assert_eq!(
        wire::prefix(65_537_u32.to_le_bytes()),
        Err(WireError::TooLarge)
    );
    assert_eq!(
        wire::prefix(u32::MAX.to_le_bytes()),
        Err(WireError::TooLarge)
    );
    for (length, expected) in [
        (0, Some(WireError::Empty)),
        (1, None),
        (65_535, None),
        (65_536, None),
        (65_537, Some(WireError::TooLarge)),
        (1 << 24, Some(WireError::TooLarge)),
        (u32::MAX - 1, Some(WireError::TooLarge)),
        (u32::MAX, Some(WireError::TooLarge)),
    ] {
        let stream = edge(length);
        for cuts in [vec![], (0..12).collect(), vec![HELLO.len() + 2]] {
            let (outcome, largest) = measured(|| feed(&stream, cuts));
            assert_eq!(outcome.error, expected, "length {length}");
            assert!(
                largest <= RELIABLE_LIMIT,
                "length {length} allocated {largest} bytes"
            );
            if expected.is_some() {
                assert!(
                    outcome.messages.is_empty(),
                    "a refused prefix delivered bytes"
                );
            } else {
                assert_eq!(outcome.messages.len(), 1);
                assert_eq!(
                    outcome.messages[0].len(),
                    usize::try_from(length).expect("small")
                );
            }
        }
    }
    // A refused prefix after valid messages delivers those and none of the offending bytes.
    let mut stream = wire::encode_stream([[1_u8].as_slice(), [2, 2].as_slice()]);
    stream.extend_from_slice(&65_537_u32.to_le_bytes());
    stream.extend(std::iter::repeat_n(9, 70_000));
    let outcome = feed(&stream, []);
    assert_eq!(outcome.messages, [vec![1], vec![2, 2]]);
    assert_eq!(outcome.error, Some(WireError::TooLarge));
}

#[test]
fn the_hello_is_refused_at_its_first_wrong_byte() {
    assert_eq!(wire::hello(*HELLO), Ok(()));
    for index in 0..HELLO.len() {
        for value in 0..=u8::MAX {
            let mut frame = *HELLO;
            frame[index] = value;
            let expected = (value == HELLO[index])
                .then_some(())
                .ok_or(WireError::Hello);
            assert_eq!(wire::hello(frame), expected);
            let mut stream = frame.to_vec();
            stream.extend_from_slice(&wire::encode_prefix(3));
            stream.extend_from_slice(&[1, 2, 3]);
            let outcome = feed(&stream, [index + 1]);
            assert_eq!(outcome.error, expected.err());
            assert_eq!(outcome.hello, expected.is_ok());
            assert_eq!(outcome.messages.len(), usize::from(expected.is_ok()));
        }
    }
    // A message where the hello belongs is refused, never delivered.
    let outcome = feed(
        &wire::encode_stream([[9_u8; 4].as_slice()])[HELLO.len()..],
        [],
    );
    assert_eq!(
        (outcome.error, outcome.messages.len()),
        (Some(WireError::Hello), 0)
    );
}

#[test]
fn random_streams_either_decode_to_what_was_encoded_or_are_refused_within_the_cap() {
    let mut random = Random(2);
    for case in 0..20_000 {
        let length = random.below(if case % 10 == 0 { 140_000 } else { 64 });
        let mut stream = random.bytes(length);
        // Half the cases start with a valid hello, so the prefix decoder sees random lengths too.
        if case % 2 == 0 && stream.len() >= HELLO.len() {
            stream[..HELLO.len()].copy_from_slice(HELLO);
        }
        let cuts: Vec<usize> = (0..random.below(4))
            .map(|_| random.below(length + 1))
            .collect();
        let (outcome, largest) = measured(|| feed(&stream, cuts));
        assert!(
            largest <= RELIABLE_LIMIT,
            "case {case} allocated {largest} bytes"
        );
        assert!(outcome.held <= RELIABLE_LIMIT);
        // Whatever was delivered is exactly a valid prefix of the input, framed as it says.
        let expected = wire::encode_stream(outcome.messages.iter().map(Vec::as_slice));
        if outcome.hello {
            assert_eq!(stream[..expected.len()], expected[..], "case {case}");
        } else {
            assert_eq!(outcome.messages, Vec::<Vec<u8>>::new());
        }
    }
    // Valid streams with one random byte changed: still a prefix of what was sent, or refused.
    let messages = sample_messages(&mut random);
    let stream = wire::encode_stream(messages.iter().map(Vec::as_slice));
    for _ in 0..2_000 {
        let mut damaged = stream.clone();
        let index = random.below(damaged.len());
        damaged[index] ^= u8::try_from(random.below(255) + 1).expect("byte");
        let (outcome, largest) = measured(|| feed(&damaged, [random.below(damaged.len())]));
        assert!(largest <= RELIABLE_LIMIT);
        let encoded = wire::encode_stream(outcome.messages.iter().map(Vec::as_slice));
        assert_eq!(damaged[..encoded.len()], encoded[..]);
    }
}

fn ticket_text(random: &mut Random) -> String {
    let hash: [u8; 32] = std::array::from_fn(|_| random.next().to_le_bytes()[0]);
    let address = match random.below(3) {
        0 => format!("127.0.0.1:{}", random.below(65_536)),
        1 => format!("[::1]:{}", random.below(65_536)),
        _ => format!(
            "[2001:db8::{:x}:{:x}]:{}",
            random.below(65_536),
            random.below(65_536),
            random.below(65_536)
        ),
    };
    Ticket { address, hash }.to_string()
}

#[test]
fn ticket_text_round_trips_or_is_refused() {
    let mut random = Random(3);
    for _ in 0..200 {
        let text = ticket_text(&mut random);
        let ticket: Ticket = text.parse().expect("a ticket the host wrote parses");
        assert_eq!(ticket.to_string(), text);
        // Cut at every byte: only the whole text parses.
        for cut in 0..text.len() {
            assert!(text[..cut].parse::<Ticket>().is_err(), "{:?}", &text[..cut]);
        }
        // Every one-byte change either is refused or prints back as the changed text.
        for index in 0..text.len() {
            for value in [
                b'@', b'0', b'f', b'F', b'+', b'-', b' ', b':', b'[', b'g', 0, 0x7f,
            ] {
                let mut bytes = text.clone().into_bytes();
                bytes[index] = value;
                let Ok(changed) = String::from_utf8(bytes) else {
                    continue;
                };
                if let Ok(parsed) = changed.parse::<Ticket>() {
                    assert_eq!(parsed.to_string(), changed);
                }
            }
        }
    }
    // Forms that are the same value written differently are refused, so one ticket has one text.
    let ticket: Ticket = ticket_text(&mut random).parse().expect("ticket");
    let text = ticket.to_string();
    let digits = text.rsplit('@').next().expect("hash").to_owned();
    for variant in [
        text.replace(&digits, &digits.to_uppercase()),
        text.replacen(&digits[..2], &format!("+{}", &digits[1..2]), 1),
        text.replacen("thrum1@", "thrum2@", 1),
        text.replacen("thrum1@", "THRUM1@", 1),
        format!("{text}@"),
        format!(" {text}"),
        "thrum1@127.000.0.1:9@".to_owned() + &digits,
        "thrum1@localhost:9@".to_owned() + &digits,
        "thrum1@127.0.0.1@".to_owned() + &digits,
        "thrum1@127.0.0.1:99999@".to_owned() + &digits,
        "thrum1@[0:0::1]:9@".to_owned() + &digits,
    ] {
        if variant != text {
            assert!(variant.parse::<Ticket>().is_err(), "{variant}");
        }
    }
}

#[test]
fn ticket_lengths_and_random_text_are_refused_within_the_limit() {
    let mut random = Random(4);
    for length in [0, 1, TICKET_LIMIT, TICKET_LIMIT + 1, 1 << 16, 1 << 20] {
        let text = "a".repeat(length);
        let (result, largest) = measured(|| text.parse::<Ticket>());
        assert!(result.is_err());
        assert!(
            largest <= TICKET_LIMIT,
            "allocated {largest} bytes for {length}"
        );
    }
    for case in 0..50_000 {
        let length = random.below(if case % 100 == 0 { 4_096 } else { 140 });
        let bytes = if case % 2 == 0 {
            random.bytes(length)
        } else {
            // Random text built from the characters a ticket uses, so some inputs get deep into the parser.
            let alphabet = b"thrum1@0123456789abcdef.:[]";
            (0..length)
                .map(|_| alphabet[random.below(alphabet.len())])
                .collect()
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let (result, largest) = measured(|| text.parse::<Ticket>());
        assert!(
            largest <= TICKET_LIMIT,
            "case {case} allocated {largest} bytes"
        );
        if let Ok(ticket) = result {
            assert_eq!(ticket.to_string(), text);
        }
    }
}
