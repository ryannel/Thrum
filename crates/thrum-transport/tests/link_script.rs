//! Portable link conformance and fault controls, with caller-supplied virtual time.
use thrum_transport::{
    CloseReason, DATAGRAM_LIMIT, Event, Lane, LinkError, Profile, RELIABLE_LIMIT,
    memory::{Conditioner, Memory},
    proof::Script,
};

fn exchange(duplicate: bool) -> Result<Vec<thrum_transport::proof::Report>, CloseReason> {
    let mut memory = Memory::new(
        Profile::default(),
        Conditioner {
            duplicate,
            ..Conditioner::default()
        },
    )
    .expect("profile");
    let ends = memory.connect().expect("connect");
    memory.advance(120);
    for end in &ends {
        assert_eq!(end.event(), Some(Event::Open));
    }
    let mut scripts = [Script::new(0), Script::new(1)];
    // Neither game sent during startup. The host speaks first.
    scripts[1]
        .send(&ends[1], Lane::Reliable, 1, 120)
        .expect("host first");
    memory.advance(121);
    memory.advance(240);
    scripts[0].drain(&ends[0])?;
    assert_eq!(scripts[0].report().reliable_received, 1);
    for end in &ends {
        for lane in [Lane::Reliable, Lane::Datagram] {
            assert_eq!(end.send(lane, &[], 240), Err(LinkError::Empty));
            let limit = if lane == Lane::Reliable {
                RELIABLE_LIMIT
            } else {
                DATAGRAM_LIMIT
            };
            assert_eq!(
                end.send(lane, &vec![0; limit + 1], 240),
                Err(LinkError::TooLarge)
            );
        }
    }
    for now in 241..30_241 {
        for direction in 0..2 {
            scripts[direction].tick(&ends[direction], now)?;
        }
        memory.advance(now);
        for direction in 0..2 {
            scripts[direction].drain(&ends[direction])?;
        }
    }
    memory.advance(31_000);
    for direction in 0..2 {
        scripts[direction].drain(&ends[direction])?;
    }
    for direction in 0..2 {
        let report = scripts[direction].report();
        let other = scripts[1 - direction].report();
        assert_eq!(report.reliable_received, other.reliable_sent);
        assert!(report.datagrams_received < other.datagrams_sent);
        assert!(report.datagrams_received * 10 >= other.datagrams_sent * 9);
        assert!(report.reordered > 0);
        let counts = ends[direction].counters(31_000);
        assert_eq!(counts.lanes[0].sent, report.reliable_sent);
        assert_eq!(counts.lanes[1].received, report.datagrams_received);
    }
    Ok(scripts.iter().map(|s| s.report().clone()).collect())
}
#[test]
fn deterministic_lanes_and_duplicate_control() {
    assert_eq!(
        exchange(false).expect("script"),
        exchange(false).expect("repeat")
    );
    assert_eq!(exchange(true), Err(CloseReason::Protocol));
}
#[test]
fn queue_limits_expire_without_more_sends_and_stack_arrives_late() {
    let profile = Profile::default();
    let mut memory = Memory::new(
        profile.clone(),
        Conditioner {
            loss_ppm: 0,
            ..Conditioner::default()
        },
    )
    .expect("profile");
    let ends = memory.connect().expect("connect");
    memory.advance(120);
    memory.stall(true);
    for end in &ends {
        for _ in 0..16 {
            end.send(Lane::Reliable, &vec![7; RELIABLE_LIMIT], 121)
                .expect("1 MiB");
        }
        assert_eq!(end.send(Lane::Reliable, &[7], 121), Err(LinkError::Full));
        for _ in 0..100 {
            end.send(Lane::Datagram, &vec![7; DATAGRAM_LIMIT], 121)
                .expect("burst");
        }
    }
    memory.advance(121);
    for now in 122..700 {
        for end in &ends {
            end.send(Lane::Datagram, &vec![7; DATAGRAM_LIMIT], now)
                .expect("saturate");
        }
        memory.advance(now);
    }
    // Stop sending, leave the stack stalled, and let the timer expire the final queue.
    memory.advance(1_000);
    for end in &ends {
        let counts = end.counters(1_000);
        assert_eq!(counts.lanes[1].queued_bytes, 0);
        assert!(counts.expired > 0);
        assert!(counts.dropped_cap > 0);
        assert_eq!(
            counts.lanes[1].sent,
            counts.handed_on + counts.expired + counts.dropped_cap
        );
        assert_eq!(counts.handed_on, 4);
    }
    memory.stall(false);
    memory.advance(1_001);
    memory.advance(1_200);
    for end in &ends {
        let mut datagrams = 0;
        while end.receive(Lane::Datagram).is_some() {
            datagrams += 1;
        }
        assert_eq!(datagrams, 4, "only the allowance arrives, late");
        let mut reliable = 0;
        while end.receive(Lane::Reliable).is_some() {
            reliable += 1;
        }
        assert_eq!(reliable, 16, "all accepted reliable messages drain");
    }
}
#[test]
fn lifecycle_idle_cut_startup_close_and_new_identity() {
    let mut memory = Memory::new(Profile::default(), Conditioner::default()).expect("profile");
    memory.cut(true);
    let startup = memory.connect().expect("startup");
    memory.advance(5_001);
    for end in &startup {
        assert_eq!(end.event(), Some(Event::Closed(CloseReason::TimedOut)));
    }
    memory.cut(false);
    let ends = memory.connect().expect("connect");
    memory.advance(5_121);
    for end in &ends {
        assert_eq!(end.event(), Some(Event::Open));
    }
    memory.cut(true);
    memory.advance(9_122);
    for end in &ends {
        assert_eq!(end.event(), Some(Event::Closed(CloseReason::TimedOut)));
    }
    memory.cut(false);
    let next = memory.connect().expect("reconnect");
    assert_ne!(ends[0].id(), next[0].id());
    memory.advance(9_242);
    next[0].close(42);
    memory.advance(9_243);
    assert_eq!(next[1].event(), Some(Event::Open));
    assert_eq!(next[1].event(), Some(Event::Closed(CloseReason::Peer(42))));
    assert_eq!(
        next[0].send(Lane::Reliable, &[1], 9_243),
        Err(LinkError::NotOpen)
    );
}

#[test]
fn shared_limits_script_and_ticket_identity() {
    use thrum_transport::Ticket;
    let mut memory = Memory::new(
        Profile::default(),
        Conditioner {
            loss_ppm: 0,
            ..Conditioner::default()
        },
    )
    .expect("profile");
    let ticket = memory.ticket();
    let text = ticket.to_string();
    assert_eq!(text.parse::<Ticket>(), Ok(ticket.clone()));
    let mut wrong = ticket.clone();
    wrong.hash[0] ^= 1;
    assert_eq!(
        memory.connect_ticket(&wrong).err(),
        Some(LinkError::InvalidTicket)
    );
    assert_eq!(
        text.replacen("thrum1@", "thrum2@", 1).parse::<Ticket>(),
        Err(LinkError::InvalidTicket)
    );
    let ends = memory.connect_ticket(&ticket).expect("correct identity");
    memory.advance(120);
    let mut scripts = [Script::new(0), Script::new(1)];
    scripts[0].identify(10);
    scripts[1].identify(20);
    for direction in 0..2 {
        scripts[direction]
            .limits(&ends[direction], 120)
            .expect("shared limits");
    }
    memory.advance(121);
    memory.advance(250);
    for direction in 0..2 {
        scripts[direction]
            .drain(&ends[direction])
            .expect("limits drain");
        assert_eq!(
            scripts[direction].report().reliable_received,
            scripts[1 - direction].report().reliable_sent
        );
    }
    assert_eq!(scripts[0].peer_identity(), Some(20));
    assert_eq!(scripts[1].peer_identity(), Some(10));
    scripts[0].identify(11);
    scripts[0]
        .send(&ends[0], Lane::Reliable, 32, 250)
        .expect("cross-link fault");
    memory.advance(251);
    memory.advance(400);
    assert_eq!(scripts[1].drain(&ends[1]), Err(CloseReason::Protocol));
    for closer in 0..2 {
        let mut carrier = Memory::new(Profile::default(), Conditioner::default()).expect("profile");
        let pair = carrier.connect().expect("pair");
        carrier.advance(120);
        pair[closer]
            .send(Lane::Reliable, &vec![1; RELIABLE_LIMIT], 120)
            .expect("pending send");
        pair[closer].close(9);
        carrier.advance(121);
        carrier.advance(500);
        assert_eq!(pair[1 - closer].receive(Lane::Reliable), None);
        assert_eq!(pair[1 - closer].event(), Some(Event::Open));
        assert_eq!(
            pair[1 - closer].event(),
            Some(Event::Closed(CloseReason::Peer(9)))
        );
    }
}
