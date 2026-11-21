//! Session negotiation before input admission.
/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Game, Match, Rule, local, shaped};
use thrum_network::{
    envelope::{self, Header, Kind},
    session::{ClientState, Controls, PROTOCOL, Refusal, World},
};
use thrum_transport::{Lane, LinkError};

#[test]
fn a_queued_hello_cannot_renew_an_expired_negotiation() {
    for (hello_ms, welcomed) in [(3_999, true), (4_000, false), (4_001, false)] {
        let mut fixture = Match::new(local(), 1, None, false);
        fixture.memory.advance(0);
        fixture.host.advance(3_999_000, 0);
        let at = hello_ms * 1_000;
        fixture.clients[0]
            .advance(at, |_| Controls::axes([0.0; 3]))
            .unwrap();
        fixture.memory.advance(hello_ms);
        fixture.host.advance(at, 0);
        fixture.memory.advance(hello_ms);
        fixture.clients[0]
            .advance(at, |_| Controls::axes([0.0; 3]))
            .unwrap();
        assert_eq!(fixture.clients[0].slot(), welcomed.then_some(0));
        if !welcomed {
            assert!(matches!(fixture.clients[0].state(), ClientState::Closed(_)));
        }
    }
}

#[test]
fn refusals_cross_a0_before_link_close_and_bad_startups_expire() {
    let mut fixture = Match::new(shaped(), 0, None, false);
    let add = Game::new(20, Rule::Add, None, 0, true);
    let subtract = Game::new(21, Rule::Subtract, None, 0, true);
    assert_ne!(add.identity(), subtract.identity());

    fixture.add(Rule::Add, PROTOCOL + 1);
    fixture.add(Rule::Subtract, PROTOCOL);
    for _ in 0..4 {
        fixture.add(Rule::Add, PROTOCOL);
    }
    let [bad, bad_host] = fixture.memory.connect().unwrap();
    let [silent, silent_host] = fixture.memory.connect().unwrap();
    fixture.host.connect(bad_host, 0).unwrap();
    fixture.host.connect(silent_host, 0).unwrap();
    let mut sent_wrong_kind = false;
    let mut observed = [false; 3];
    for _ in 0..2_100 {
        if fixture.clients.len() == 6 && (0..4).all(|slot| fixture.host.ready(slot)) {
            fixture.add(Rule::Add, PROTOCOL);
        }
        if bad.is_open() && !sent_wrong_kind {
            let mut packet = [0; 64];
            let size = envelope::encode(
                Header {
                    kind: Kind::Command,
                    match_id: [0; 16],
                    connection_epoch: 0,
                    message_sequence: 0,
                    server_tick: 0,
                    acknowledged_sequence: 0,
                },
                &[],
                &mut packet,
            )
            .unwrap();
            bad.send(Lane::Reliable, &packet[..size], fixture.now / 1_000)
                .unwrap();
            sent_wrong_kind = true;
        }

        fixture.neutral(0);
        for (slot, (index, reason)) in [
            (0, Refusal::Protocol),
            (1, Refusal::RuleSet),
            (6, Refusal::Full),
        ]
        .into_iter()
        .enumerate()
        {
            if index >= fixture.clients.len() {
                continue;
            }
            if fixture.clients[index].state() == &ClientState::Refused(reason) && !observed[slot] {
                assert!(
                    fixture.links[index][1].is_open(),
                    "host waited for the refusal to arrive"
                );
                assert_eq!(fixture.clients[index].slot(), None);
                assert_eq!(fixture.clients[index].last_frame(), (0, 0));
                observed[slot] = true;
            }
        }
    }
    assert_eq!(observed, [true; 3]);
    assert!(sent_wrong_kind);
    assert_eq!(
        bad.send(Lane::Reliable, &[1], fixture.now / 1_000),
        Err(LinkError::NotOpen)
    );
    assert_eq!(
        silent.send(Lane::Reliable, &[1], fixture.now / 1_000),
        Err(LinkError::NotOpen)
    );
    let mut slots = fixture.clients[2..6]
        .iter()
        .map(|client| client.slot().unwrap())
        .collect::<Vec<_>>();
    slots.sort_unstable();
    assert_eq!(slots, [0, 1, 2, 3]);
    for client in &fixture.clients[2..6] {
        let slot = client.slot().unwrap();
        assert_eq!(client.settings(), Some(fixture.settings));
        assert_eq!(client.seed(), Some(341));
        assert!(fixture.host.ready(slot));
    }
    assert_eq!(fixture.host.world().tick(), 0);
}
