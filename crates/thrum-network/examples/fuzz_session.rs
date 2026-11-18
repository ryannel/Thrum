//! Stdin entry point for session payload refusal through the in-memory link.

/// Integer rule set and virtual carrier shared with the integration tests.
#[path = "../tests/support/session.rs"]
pub mod session_fixture;

use std::io::{self, Read};

use session_fixture::{Match, local};
use thrum_network::{
    envelope::{self, Header, Kind, MAX_MESSAGE_BYTES},
    session::{Controls, World},
};
use thrum_transport::{DATAGRAM_LIMIT, Lane};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .lock()
        .take(MAX_MESSAGE_BYTES as u64 + 2)
        .read_to_end(&mut bytes)?;
    let [selector, mode, payload @ ..] = bytes.as_slice() else {
        return Ok(());
    };
    let mut fixture = Match::new(local(), 1, None, false);
    let negotiated = selector & 1 != 0;
    let to_authority = selector & 2 != 0;
    let raw = selector & 4 != 0;
    if negotiated {
        fixture.ready();
        while fixture.host.world().tick() < 6 {
            fixture.neutral(6);
        }
    } else {
        // Open the link and put the client in negotiation. Captured hello is
        // removed so the test message is the next message across this lane.
        fixture.memory.advance(0);
        fixture.clients[0].advance(0, |_| Controls::axes([0.0; 3]))?;
        fixture.memory.advance(0);
        drop(fixture.links[0][1].receive(Lane::Reliable));
    }
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
    ];
    let kind = kinds[usize::from(mode / 2) % kinds.len()];
    let identity = negotiated && kind != Kind::Hello;
    let mut framed = vec![0; MAX_MESSAGE_BYTES];
    let bytes = if raw {
        payload
    } else {
        let header = Header {
            kind,
            match_id: if identity { [4; 16] } else { [0; 16] },
            connection_epoch: if identity { 2 } else { 0 },
            message_sequence: u64::from(kind == Kind::Command),
            server_tick: if kind == Kind::AdmittedSets { 7 } else { 0 },
            acknowledged_sequence: 0,
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
    let time = fixture.now / 1_000;
    if fixture.links[0][from].send(lane, bytes, time).is_ok() {
        fixture.memory.advance(time);
        if to_authority {
            fixture.host.advance(fixture.now, 6);
        } else {
            fixture.clients[0].advance(fixture.now, |_| Controls::axes([0.0; 3]))?;
        }
    }
    Ok(())
}
