//! Stdin fuzz target for the untrusted state-group envelope and mode-0 delta.

use std::{hint::black_box, io::Read};

use thrum_network::{
    envelope::{self, Kind},
    state_ack,
    state_delta::{Actor, Image, Scope, decode_delta},
    state_schema,
};

#[derive(Clone, Copy)]
struct Pose {
    x: u32,
    y: u32,
}

state_schema! {
    struct PoseWire for Pose, schema = 1 {
        0 => x: 21,
        1 => y: 21,
    }
}

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(u64::try_from(envelope::MAX_MESSAGE_BYTES + 1).expect("64 KiB bound"))
        .read_to_end(&mut bytes)?;
    let base = [
        Actor {
            wire_id: 1,
            value: Pose { x: 1, y: 2 },
        },
        Actor {
            wire_id: 3,
            value: Pose { x: 3, y: 4 },
        },
    ];
    let image = Image {
        scope: Scope {
            partition_id: 1,
            interest_generation: 1,
        },
        id: 1,
        tick: 0,
        actors: &base,
    };
    let mut scratch = base;
    let _ = black_box(decode_delta(&PoseWire, image, &bytes, &mut scratch));
    let _ = black_box(state_ack::decode(&bytes));
    if let Ok(message) = envelope::decode(&bytes)
        && message.header.kind == Kind::StateGroup
    {
        let _ = black_box(decode_delta(
            &PoseWire,
            image,
            message.payload,
            &mut scratch,
        ));
    }
    if let Ok(message) = envelope::decode(&bytes)
        && message.header.kind == Kind::StateAck
    {
        let _ = black_box(state_ack::decode(message.payload));
    }
    Ok(())
}
