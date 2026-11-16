//! Host teardown with retained links, over real loopback sockets and the public API only.
//!
//! Two clients open; the game accepts both host links and keeps a clone of the first. Each side queues
//! reliable messages the other never reads, then the game drops the host. Every retained host-side handle
//! must become terminal with exactly one `Closed` event, refuse further sends and hand back nothing queued.
//! A third client is left connecting when the host drops, so its host link is never accepted, and a second
//! host is dropped before its startup completes.
use std::{
    thread,
    time::{Duration, Instant},
};
use thrum_transport::{CloseReason, Event, Lane, Link, Profile, native};

fn wait<T>(deadline: Instant, mut poll: impl FnMut() -> Option<T>) -> Result<T, String> {
    loop {
        if let Some(value) = poll() {
            return Ok(value);
        }
        if Instant::now() > deadline {
            return Err("deadline passed".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn events(link: &Link) -> Vec<Event> {
    std::iter::from_fn(|| link.event()).collect()
}

/// Run the probe and return its record; `pass` is false when any expectation fails.
pub(crate) fn run() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let profile = Profile::default();
    // Teardown during startup: the host is dropped before it can have published a ticket.
    drop(native::Host::start(
        "127.0.0.1:0".parse()?,
        profile.clone(),
    )?);

    let host = native::Host::start("127.0.0.1:0".parse()?, profile.clone())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let ticket = wait(deadline, || host.ticket())?;
    let clients = [
        native::connect(&ticket, profile.clone())?,
        native::connect(&ticket, profile.clone())?,
    ];
    let mut accepted = Vec::new();
    while accepted.len() < clients.len() {
        accepted.push(wait(deadline, || host.accept())?);
    }
    for link in clients.iter().chain(&accepted) {
        wait(deadline, || link.is_open().then_some(()))?;
        let opened = events(link);
        if opened != [Event::Open] {
            return Err(format!("expected one Open event, saw {opened:?}").into());
        }
    }
    let retained_clone = accepted[0].clone();
    let message = [7_u8; 1_000];
    for client in &clients {
        for _ in 0..16 {
            client.send(Lane::Reliable, &message, 0)?;
        }
    }
    // Let client messages reach the host's inbound queues, which the game never reads before the drop.
    thread::sleep(Duration::from_millis(200));
    let received_before: Vec<_> = accepted
        .iter()
        .map(|l| l.counters(0).lanes[0].received)
        .collect();
    let late = native::connect(&ticket, profile)?;
    // Fill the host's reliable queues last, so sends are still pending when the host drops.
    let mut host_sends = 0_u64;
    for link in &accepted {
        let mut sent = 0;
        while sent < 10_000 && link.send(Lane::Reliable, &message, 0).is_ok() {
            sent += 1;
        }
        host_sends += sent;
        link.send(Lane::Datagram, &message[..100], 0)?;
    }
    let queued_before: Vec<_> = accepted
        .iter()
        .map(|l| l.counters(0).lanes[0].queued_bytes)
        .collect();
    let dropped_at = Instant::now();
    drop(host);
    let closed_ms: Vec<_> = accepted
        .iter()
        .map(|link| {
            wait(Instant::now() + Duration::from_secs(2), || {
                (!link.is_open()).then(|| dropped_at.elapsed().as_millis())
            })
            .ok()
        })
        .collect();
    // Six seconds outlast the four-second idle timeout, so nothing late can reopen a link or deliver to it.
    thread::sleep(Duration::from_secs(6));

    let mut handles = Vec::new();
    let mut pass = true;
    for (index, link) in accepted.iter().enumerate() {
        let seen = events(link);
        let clone_seen = if index == 0 {
            events(&retained_clone)
        } else {
            Vec::new()
        };
        let refused = format!("{:?}", link.send(Lane::Reliable, b"after drop", 0));
        let clone_refused = format!(
            "{:?}",
            retained_clone.send(Lane::Datagram, b"after drop", 0)
        );
        let delivered = [Lane::Reliable, Lane::Datagram].map(|lane| link.receive(lane).is_some());
        let counters = link.counters(0);
        let ok = closed_ms[index].is_some()
            && queued_before[index] > 0
            && !link.is_open()
            && seen == [Event::Closed(CloseReason::Local(0))]
            && clone_seen.is_empty()
            && refused.starts_with("Err")
            && clone_refused.starts_with("Err")
            && delivered == [false, false]
            && counters.lanes[0].received == received_before[index]
            && counters.lanes.iter().all(|l| l.queued_bytes == 0);
        pass &= ok;
        handles.push(serde_json::json!({
            "link": index,
            "closed_after_drop_ms": closed_ms[index],
            "reliable_received_before_drop": received_before[index],
            "reliable_received_after_close": counters.lanes[0].received,
            "reliable_queued_bytes_before_drop": queued_before[index],
            "events": format!("{seen:?}"),
            "retained_clone_events": format!("{clone_seen:?}"),
            "send_after_close": refused,
            "clone_send_after_close": clone_refused,
            "received_after_close": delivered,
            "queued_bytes_after_close": counters.lanes.iter().map(|l| l.queued_bytes).collect::<Vec<_>>(),
            "pass": ok,
        }));
    }
    let mut remote = Vec::new();
    for client in clients.iter().chain(std::iter::once(&late)) {
        let closed = wait(Instant::now() + Duration::from_secs(8), || {
            events(client)
                .into_iter()
                .find(|e| matches!(e, Event::Closed(_)))
        });
        pass &= closed.is_ok() && !client.is_open();
        remote.push(format!("{closed:?}"));
    }
    Ok(serde_json::json!({
        "case": "host-drop",
        "host_reliable_sends_accepted": host_sends,
        "host_handles": handles,
        "client_closes": remote,
        "unaccepted_link": "the third client was connecting when the host dropped; its host link was never accepted",
        "startup_drop": "a second host was dropped before startup completed",
        "pass": pass,
    }))
}
