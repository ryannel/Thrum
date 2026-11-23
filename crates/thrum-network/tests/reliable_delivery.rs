//! Reliable set delivery uses its deadline only while the send remains blocked.

/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Match, local};
use thrum_network::session::{ClientState, Close, Controls, EndReason, Settings, World};
use thrum_transport::{CloseReason, DATAGRAM_LIMIT, Profile, RELIABLE_LIMIT};

#[test]
fn a_peer_that_drains_the_reliable_lane_returns_to_datagrams_and_stays_in_play() {
    let (mut game, full_at) = saturated();
    let link = &game.links[0][1];
    let before = link.counters(game.now / 1_000);
    // Let the client drain and acknowledge the outstanding sets before the
    // next authority poll. The carrier keeps the same link and epoch.
    for _ in 0..16 {
        game.now += 1_000;
        game.memory.advance(game.now / 1_000);
        poll_replica(&mut game);
    }
    let deadline = full_at + u64::from(game.settings.join_ms) * 1_000;
    while game.now <= deadline + 200_000 {
        game.neutral(u64::MAX);
        while game.clients[0].result().is_some() {}
        assert!(game.links[0][1].is_open());
    }

    let tick = game.host.world().tick();
    let link = &game.links[0][1];
    let counters = link.counters(game.now / 1_000);
    assert_eq!(counters.lanes[0].sent, before.lanes[0].sent);
    assert_eq!(counters.lanes[0].queued_bytes, 0);
    assert!(counters.lanes[1].sent > before.lanes[1].sent);
    assert!(game.clients[0].held() >= tick - 8);
    assert_eq!(game.clients[0].state(), &ClientState::Playing);
    assert_eq!(game.host.stream(link.id()).expect("live peer").repairs, 0);
    game.host.stop();
    game.finish(tick);
    assert_eq!(
        game.clients[0].state(),
        &ClientState::Ended(EndReason::Stopped, tick)
    );
    assert_eq!(game.clients[0].world().digest(), game.host.world().digest());
    println!(
        "reliable lane full at {full_at}us, recovered on datagrams past {deadline}us, matched host at tick {tick}"
    );
}

#[test]
fn a_peer_whose_reliable_send_stays_full_closes_at_the_deadline() {
    let (mut game, full_at) = saturated();
    let deadline = full_at + u64::from(game.settings.join_ms) * 1_000;
    let link = &game.links[0][1];
    while game.now < deadline {
        game.now += 1_000;
        game.host.advance(game.now, u64::MAX);
        assert_eq!(link.is_open(), game.now < deadline);
    }
    let tick = game.host.world().tick();
    game.memory.advance(game.now / 1_000);
    poll_replica(&mut game);
    assert_eq!(
        game.clients[0].state(),
        &ClientState::Closed(CloseReason::Peer(Close::Synchronization as u32))
    );
    assert_eq!(game.host.world().tick(), tick);
    println!(
        "reliable lane full at {full_at}us, still full and closed for synchronization at {deadline}us, tick {tick}"
    );
}

fn saturated() -> (Match, u64) {
    let settings = Settings {
        tick_us: 1_000,
        digest_interval: 1,
        absence: 65_000,
        response_ms: 250,
        join_ms: 2_000,
        catch_up_deadline: 4_000,
        ..Settings::default()
    };
    // The stall fills the set queue on a single link without a padded world image.
    let profile = Profile {
        links: 1,
        reliable_bytes: RELIABLE_LIMIT,
        datagram_bytes: DATAGRAM_LIMIT,
        stack_bytes: DATAGRAM_LIMIT,
        ..Profile::default()
    };
    let mut game = Match::with_profile(local(), 1, None, false, settings, profile);
    game.ready();
    while game.host.world().tick() < 100 {
        game.neutral(u64::MAX);
        while game.clients[0].result().is_some() {}
    }
    let link = &game.links[0][1];
    let refused = link.counters(game.now / 1_000).refused[2];
    // Pausing the carrier leaves accepted reliable messages in its bounded
    // outgoing queue. The authority keeps taking due ticks.
    let through = game.host.world().tick() + u64::from(settings.backlog);
    while game.host.world().tick() < through {
        game.host.advance(game.now, u64::MAX);
        assert!(link.is_open());
        if link.counters(game.now / 1_000).refused[2] > refused {
            let full_at = game.now;
            return (game, full_at);
        }
        game.now += 1_000;
    }
    panic!("expected reliable backpressure within the set ring");
}

fn poll_replica(game: &mut Match) {
    let client = &mut game.clients[0];
    client
        .advance(game.now, |_| Controls::axes([0.0; 3]))
        .expect("valid fixture controls");
    while client.result().is_some() {}
}
