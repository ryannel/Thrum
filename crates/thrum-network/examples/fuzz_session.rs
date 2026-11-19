//! Stdin entry point for session decoders through the in-memory link.
//!
//! The second byte selects the kind by index after its lane bit (zero is
//! reliable). Selector bit 2 accepts a complete envelope as the payload.
//! Selector bit 3 sends to the opposite role to exercise state validation.

use std::io::{self, Read};

use thrum_core::sim::{InputFrame, RuleSetIdentity};
use thrum_network::{
    envelope::{self, Header, Kind, MAX_MESSAGE_BYTES},
    input_batch::{ActionView, ActionWire, InputCodecError},
    session::{Client, Controls, Host, RosterEvents, SLOTS, Settings, World},
};
use thrum_transport::{
    DATAGRAM_LIMIT, Lane, Profile,
    memory::{Conditioner, Memory},
};

struct Endpoint(u64);

impl World for Endpoint {
    type Action = u8;
    type Error = ();

    fn tick(&self) -> u64 {
        self.0
    }
    fn identity(&self) -> RuleSetIdentity {
        RuleSetIdentity([0; 32])
    }
    fn step(&mut self, _: &[InputFrame<u8>; SLOTS], _: RosterEvents) -> Result<(), ()> {
        self.0 += 1;
        Ok(())
    }
    fn digest(&self) -> u64 {
        self.0
    }
    fn image(&self, output: &mut Vec<u8>) -> Result<(), ()> {
        output.clear();
        output.extend(self.0.to_le_bytes());
        Ok(())
    }
    fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), ()> {
        if image != tick.to_le_bytes() {
            return Err(());
        }
        self.0 = tick;
        Ok(())
    }
}

struct ByteWire;

impl ActionWire<u8> for ByteWire {
    fn encode(&self, action: &u8, output: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        output[0] = *action;
        Ok((1, 1))
    }
    fn decode(&self, action: ActionView<'_>) -> Result<u8, InputCodecError> {
        match (action.schema_id, action.payload) {
            (1, [value]) => Ok(*value),
            _ => Err(InputCodecError::InvalidAction),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take(MAX_MESSAGE_BYTES as u64 + 2)
        .read_to_end(&mut bytes)?;
    let [selector, mode, payload @ ..] = bytes.as_slice() else {
        return Ok(());
    };
    let mut carrier = Memory::new(
        Profile::default(),
        Conditioner {
            delay_ms: 0,
            jitter_ms: 0,
            loss_ppm: 0,
            seed: 29,
            duplicate: false,
        },
    )?;
    let links = carrier.connect()?;
    let mut authority = Host::new(
        Endpoint(0),
        ByteWire,
        Settings::default(),
        [4; 16],
        341,
        false,
        0,
    )?;
    authority.connect(links[1].clone(), 0)?;
    let mut client = Client::adaptive(Endpoint(0), ByteWire, links[0].clone(), 0);
    let lane = if mode & 1 == 0 {
        Lane::Reliable
    } else {
        Lane::Datagram
    };
    let kinds = [
        Kind::Hello,
        Kind::Welcome,
        Kind::Reject,
        Kind::BaselineChunk,
        Kind::BaselineEnd,
        Kind::AdmittedSets,
        Kind::InputBatch,
        Kind::Command,
        Kind::MatchEnded,
        Kind::Divergence,
        Kind::BaselineBegin,
        Kind::Close,
    ];
    let kind = kinds[usize::from(mode / 2) % kinds.len()];
    let host_kind = matches!(
        kind,
        Kind::Hello | Kind::InputBatch | Kind::Command | Kind::Divergence | Kind::Close
    );
    let to_authority = host_kind ^ (selector & 8 != 0);
    let raw = selector & 4 != 0;

    let mut now = 0;
    carrier.advance(0);
    client.advance(0, |_| Controls::axes([0.0; 3]))?;
    carrier.advance(0);
    if matches!(kind, Kind::Hello | Kind::Welcome | Kind::Reject) {
        drop(links[1].receive(Lane::Reliable));
    } else if matches!(kind, Kind::BaselineChunk | Kind::BaselineEnd) {
        authority.advance(0, 0);
        carrier.advance(0);
        let welcome = links[0].receive(Lane::Reliable).expect("fixture welcome");
        let chunk = links[0].receive(Lane::Reliable).expect("fixture chunk");
        drop(links[0].receive(Lane::Reliable));
        links[1].send(Lane::Reliable, &welcome, 0)?;
        if kind == Kind::BaselineEnd {
            links[1].send(Lane::Reliable, &chunk, 0)?;
        }
        carrier.advance(0);
        client.advance(0, |_| Controls::axes([0.0; 3]))?;
    } else {
        authority.start(0);
        while now <= 120_000 {
            carrier.advance(now / 1_000);
            authority.advance(now, 6);
            client.advance(now, |_| Controls::axes([0.0; 3]))?;
            now += 2_500;
        }
    }
    let identity = !matches!(kind, Kind::Hello | Kind::Reject);
    let generation = match kind {
        Kind::BaselineBegin => 2,
        Kind::Welcome
        | Kind::BaselineChunk
        | Kind::BaselineEnd
        | Kind::AdmittedSets
        | Kind::InputBatch
        | Kind::Divergence => 1,
        _ => 0,
    };
    let mut framed = vec![0; MAX_MESSAGE_BYTES];
    let bytes = if raw {
        payload
    } else {
        let header = Header {
            kind,
            match_id: if identity { [4; 16] } else { [0; 16] },
            connection_epoch: if identity { 2 } else { 0 },
            message_sequence: match kind {
                Kind::Command | Kind::AdmittedSets => 1,
                Kind::BaselineEnd | Kind::InputBatch => 8,
                _ => 0,
            },
            server_tick: if kind == Kind::AdmittedSets { 7 } else { 0 },
            acknowledged_sequence: generation,
        };
        let Ok(length) = envelope::encode(header, payload, &mut framed) else {
            return Ok(());
        };
        &framed[..length]
    };
    let limit = if lane == Lane::Datagram {
        DATAGRAM_LIMIT
    } else {
        MAX_MESSAGE_BYTES
    };
    if bytes.is_empty() || bytes.len() > limit {
        return Ok(());
    }
    let from = usize::from(!to_authority);
    let time = now / 1_000;
    if links[from].send(lane, bytes, time).is_ok() {
        carrier.advance(time);
        if to_authority {
            authority.advance(now, 6);
        } else {
            client.advance(now, |_| Controls::axes([0.0; 3]))?;
        }
    }
    Ok(())
}
