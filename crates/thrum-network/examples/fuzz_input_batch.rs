//! Stdin fuzz target for the untrusted envelope and input-batch structural
//! parsers. Compile once, then run the binary under an external stdin fuzzer.

use std::{hint::black_box, io::Read};

use thrum_network::{envelope, input_batch};

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(u64::try_from(envelope::MAX_MESSAGE_BYTES + 1).expect("64 KiB bound"))
        .read_to_end(&mut bytes)?;
    if let Ok(message) = envelope::decode(&bytes)
        && message.header.kind == envelope::Kind::InputBatch
        && let Ok(batch) = input_batch::decode(message.payload)
    {
        for record in batch.records() {
            black_box((record.sequence, record.actor_wire_id, record.axes));
            for action in record.actions() {
                black_box((action.schema_id, action.payload));
            }
        }
    }
    if let Ok(batch) = input_batch::decode(&bytes) {
        for record in batch.records() {
            black_box((record.sequence, record.actor_wire_id, record.axes));
            for action in record.actions() {
                black_box((action.schema_id, action.payload));
            }
        }
    }
    Ok(())
}
