//! Stdin fuzz target for the link hello and the length prefix, each a four-byte frame checked whole. Compile
//! once, then run the binary under an external stdin fuzzer.

use std::{hint::black_box, io::Read};

use thrum_transport::{RELIABLE_LIMIT, wire};

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin().take(4_096).read_to_end(&mut bytes)?;
    for &frame in bytes.as_chunks::<4>().0 {
        black_box(wire::hello(frame).is_ok());
        if let Ok(length) = wire::prefix(frame) {
            assert!((1..=RELIABLE_LIMIT).contains(&length));
            assert_eq!(wire::encode_prefix(length), frame);
        }
    }
    Ok(())
}
