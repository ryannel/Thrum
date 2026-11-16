//! A host and a native client on this machine's loopback, through the public interface.
//! Run with `cargo run -p thrum-transport --features native --example host_and_client`.

use std::time::{Duration, Instant};
use thrum_transport::{Lane, Profile, native};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = Profile::default();
    let host = native::Host::start("127.0.0.1:0".parse()?, profile.clone())?;
    let started = Instant::now();
    // The host makes its key and certificate on its own thread; its ticket appears when that is done.
    let ticket = loop {
        if let Some(ticket) = host.ticket() {
            break ticket;
        }
        assert!(!host.failed() && started.elapsed() < Duration::from_secs(5), "host failed to start");
        std::thread::sleep(Duration::from_millis(1));
    };
    let text = ticket.to_string();
    println!("join ticket: {text}");
    // A player pastes the ticket's text; parsing refuses anything but the exact form.
    let client = native::connect(&text.parse()?, profile)?;
    let mut accepted = None;
    let mut now = 0;
    while !(client.is_open() && accepted.as_ref().is_some_and(thrum_transport::Link::is_open)) {
        accepted = accepted.or_else(|| host.accept());
        assert!(started.elapsed() < Duration::from_secs(5), "link did not open: {:?}", client.event());
        std::thread::sleep(Duration::from_millis(1));
    }
    let link = accepted.expect("accepted");
    println!("client {:?}, host {:?}", client.event(), link.event());
    client.send(Lane::Reliable, b"hello host", now)?;
    link.send(Lane::Datagram, &[1; 189], now)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let (mut message, mut datagram) = (None, None);
    while Instant::now() < deadline && (message.is_none() || datagram.is_none()) {
        message = message.or_else(|| link.receive(Lane::Reliable));
        datagram = datagram.or_else(|| client.receive(Lane::Datagram));
        std::thread::sleep(Duration::from_millis(1));
        now += 1;
    }
    println!("host took {message:?}; client took a datagram: {}", datagram.is_some());
    println!("host link counters: {:?}", link.counters(now));
    println!("host counters: {:?}", host.counters());
    link.close(3);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut closed = None;
    while Instant::now() < deadline && closed.is_none() {
        closed = client.event();
        std::thread::sleep(Duration::from_millis(1));
    }
    println!("client after the host closed: {closed:?}, host link: {:?}", link.event());
    Ok(())
}
