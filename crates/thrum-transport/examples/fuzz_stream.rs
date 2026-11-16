//! Stdin fuzz target for the reliable stream's decoder: the link hello, then length-prefixed messages. The
//! first input byte picks a chunk size, so a fuzzer also explores how the stream is cut. Compile once, then run
//! the binary under an external stdin fuzzer; it panics if the decoder breaks its contract.

use std::{hint::black_box, io::Read};

use thrum_transport::{
    RELIABLE_LIMIT,
    wire::{Decoder, Frame},
};

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(4 * (RELIABLE_LIMIT as u64 + 8))
        .read_to_end(&mut bytes)?;
    let Some((&chunk, stream)) = bytes.split_first() else {
        return Ok(());
    };
    let mut decoder = Decoder::new();
    let mut delivered = Vec::new();
    for mut piece in stream.chunks(usize::from(chunk) + 1) {
        while !piece.is_empty() {
            let Ok((used, frame)) = decoder.decode(piece) else {
                return Ok(());
            };
            piece = &piece[used..];
            if let Some(Frame::Message(message)) = frame {
                assert!((1..=RELIABLE_LIMIT).contains(&message.len()));
                delivered.push(black_box(message));
            }
            assert!(decoder.held() <= RELIABLE_LIMIT);
        }
    }
    let encoded = thrum_transport::wire::encode_stream(delivered.iter().map(Vec::as_slice));
    assert!(stream.starts_with(&encoded) || delivered.is_empty());
    Ok(())
}
