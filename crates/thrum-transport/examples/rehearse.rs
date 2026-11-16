//! Rehearse a link with no socket: the in-memory link under A0's conditioner, in virtual time.
//! Run with `cargo run -p thrum-transport --example rehearse`.

use std::io::Write;
use thrum_transport::{
    DATAGRAM_LIMIT, Event, Lane, LinkError, Profile, RELIABLE_LIMIT,
    memory::{Conditioner, Memory},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 50 ms ± 10 ms each way and 2% loss, with a fixed seed: the same calls give the same events.
    let mut memory = Memory::new(Profile::default(), Conditioner::default())?;
    let [client, host] = memory.connect()?;
    let mut now = 0;
    while !(client.is_open() && host.is_open()) {
        now += 1;
        memory.advance(now);
    }
    assert_eq!(client.event(), Some(Event::Open));
    assert_eq!(host.event(), Some(Event::Open));
    // Either end may speak first. Sizes outside each lane's limits are refused at the call.
    assert_eq!(
        host.send(Lane::Reliable, &vec![0; RELIABLE_LIMIT + 1], now),
        Err(LinkError::TooLarge)
    );
    assert_eq!(
        client.send(Lane::Datagram, &vec![0; DATAGRAM_LIMIT + 1], now),
        Err(LinkError::TooLarge)
    );
    host.send(Lane::Reliable, b"welcome", now)?;
    let mut datagrams = 0;
    for tick in 0..120 {
        now += 16;
        client.send(Lane::Datagram, &[tick; 189], now)?;
        memory.advance(now);
        while host.receive(Lane::Datagram).is_some() {
            datagrams += 1;
        }
    }
    now += 500;
    memory.advance(now);
    while host.receive(Lane::Datagram).is_some() {
        datagrams += 1;
    }
    assert_eq!(
        client.receive(Lane::Reliable).as_deref(),
        Some(b"welcome".as_slice())
    );
    let counters = client.counters(now);
    writeln!(
        std::io::stdout(),
        "datagrams sent 120, arrived {datagrams}; client counters: {counters:?}"
    )?;
    client.close(7);
    memory.advance(now + 200);
    writeln!(
        std::io::stdout(),
        "client: {:?}, host: {:?}",
        client.event(),
        host.event()
    )?;
    Ok(())
}
