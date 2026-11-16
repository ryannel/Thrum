//! A host with healthy links beside it, all driven from this one thread as a session's tick would drive them.
//!
//! The host and three healthy native clients run in this process over loopback sockets: each has its own I/O
//! thread and UDP socket, and every send and drain, host and client, is made from the calling thread. Healthy
//! links carry 400-byte reliable messages at 10 Hz each way, numbered and checked for order and content, and
//! 189-byte datagrams at 60 Hz from the client that the host echoes, which gives their round trip. Any link
//! the host opens after the healthy ones is an offender's: the game drains it like any other and its events,
//! counts and queues are recorded.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use thrum_transport::{HostCounters, Lane, Link, LinkError, Profile, Ticket, native};

const RELIABLE_BYTES: usize = 400;
const DATAGRAM_BYTES: usize = 189;

/// One direction of a healthy link's reliable lane.
#[derive(Default)]
struct Ordered {
    sent: u64,
    received: u64,
}
fn reliable_payload(seq: u64, direction: u8) -> Vec<u8> {
    let mut bytes = vec![seq.to_le_bytes()[0] ^ direction; RELIABLE_BYTES];
    bytes[0] = b'R';
    bytes[1] = direction;
    bytes[2..10].copy_from_slice(&seq.to_le_bytes());
    bytes
}

struct Healthy {
    client: Link,
    host: Link,
    up: Ordered,
    down: Ordered,
    next_reliable: u64,
    next_datagram: u64,
    datagram_seq: u64,
    echoes: u64,
}

/// A link the host opened for an offender.
pub(crate) struct Offender {
    pub case: String,
    pub link: Link,
    pub opened_ms: u64,
    pub events: Vec<(u64, String)>,
    pub reliable: u64,
    pub reliable_bytes: u64,
    pub datagrams: u64,
}

/// The largest queues seen on host links since the last reset, in bytes.
#[derive(Default, Clone, Copy)]
pub(crate) struct Peaks {
    pub reliable_out: usize,
    pub datagram_out: usize,
    pub reliable_in: usize,
    pub datagram_in: usize,
    pub host_total: usize,
    pub longest_tick_us: u64,
    pub longest_send_us: u64,
}

pub(crate) struct Arena {
    pub host: native::Host,
    pub ticket: Ticket,
    pub profile: Profile,
    epoch: Instant,
    healthy: Vec<Healthy>,
    pub offenders: Vec<Offender>,
    pub case: String,
    paused: bool,
    pub phase: String,
    rtts: BTreeMap<String, Vec<u64>>,
    pub peaks: Peaks,
    pub errors: Vec<String>,
    /// Extra host links the caller drives itself, such as a stalled peer's.
    pub claimed: Vec<Link>,
    claim_next: usize,
}
impl Arena {
    pub(crate) fn start(healthy: usize) -> Result<Self, String> {
        let profile = Profile::default();
        let host = native::Host::start("127.0.0.1:0".parse().map_err(|_| "address")?, profile.clone())
            .map_err(|e| e.to_string())?;
        let started = Instant::now();
        let ticket = loop {
            if let Some(ticket) = host.ticket() {
                break ticket;
            }
            if started.elapsed() > Duration::from_secs(10) {
                return Err("host startup".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        let mut arena = Self {
            host,
            ticket,
            profile,
            epoch: Instant::now(),
            healthy: Vec::new(),
            offenders: Vec::new(),
            case: "setup".into(),
            paused: false,
            phase: "setup".into(),
            rtts: BTreeMap::new(),
            peaks: Peaks::default(),
            errors: Vec::new(),
            claimed: Vec::new(),
            claim_next: 0,
        };
        for _ in 0..healthy {
            let client = native::connect(&arena.ticket, arena.profile.clone()).map_err(|e| e.to_string())?;
            let host = arena.wait_for_link()?;
            arena.run_until(Duration::from_secs(10), |_| client.is_open())?;
            for end in [&client, &host] {
                let opened: Vec<_> = std::iter::from_fn(|| end.event()).collect();
                if opened != [thrum_transport::Event::Open] {
                    return Err(format!("healthy link opened with {opened:?}"));
                }
            }
            let now = arena.now_ms();
            arena.healthy.push(Healthy {
                client,
                host,
                up: Ordered::default(),
                down: Ordered::default(),
                next_reliable: now,
                next_datagram: now,
                datagram_seq: 0,
                echoes: 0,
            });
        }
        Ok(arena)
    }
    pub(crate) fn now_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
    fn now_us(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
    /// Wait for the host's next link and keep it out of the offender list.
    pub(crate) fn wait_for_link(&mut self) -> Result<Link, String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        self.claim_next += 1;
        loop {
            if let Some(link) = self.host.accept() {
                self.claim_next -= 1;
                return Ok(link);
            }
            if Instant::now() > deadline {
                self.claim_next -= 1;
                return Err("host opened no link".into());
            }
            self.tick();
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    pub(crate) fn run_for(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.tick();
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    pub(crate) fn run_until(
        &mut self,
        timeout: Duration,
        mut done: impl FnMut(&mut Self) -> bool,
    ) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            self.tick();
            if done(self) {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!("{}: condition not met in {timeout:?}", self.case));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    /// One tick: take new links, drain and send on every link, and sample the host's queues.
    pub(crate) fn tick(&mut self) {
        let began = Instant::now();
        if self.claim_next == 0 {
            while let Some(link) = self.host.accept() {
                let opened_ms = self.now_ms();
                self.offenders.push(Offender {
                    case: self.case.clone(),
                    link,
                    opened_ms,
                    events: Vec::new(),
                    reliable: 0,
                    reliable_bytes: 0,
                    datagrams: 0,
                });
            }
        }
        let now = self.now_ms();
        let now_us = self.now_us();
        let mut errors = Vec::new();
        let mut rtts = Vec::new();
        let mut longest_send = 0;
        for (index, link) in self.healthy.iter_mut().enumerate() {
            for (end, other) in [(&link.client, 0_u8), (&link.host, 1)] {
                while let Some(event) = end.event() {
                    errors.push(format!("healthy {index} end {other} saw {event:?}"));
                }
            }
            // Host side: check the client's reliable messages and echo its datagrams.
            while let Some(bytes) = link.host.receive(Lane::Reliable) {
                if bytes != reliable_payload(link.up.received, 0) {
                    errors.push(format!("healthy {index}: client message {} out of order or corrupt", link.up.received));
                }
                link.up.received += 1;
            }
            while let Some(bytes) = link.host.receive(Lane::Datagram) {
                let sent = Instant::now();
                match link.host.send(Lane::Datagram, &bytes, now) {
                    Ok(()) => link.echoes += 1,
                    Err(error) => errors.push(format!("healthy {index}: echo refused {error:?}")),
                }
                longest_send = longest_send.max(sent.elapsed().as_micros());
            }
            // Client side: check the host's reliable messages and time the echoes.
            while let Some(bytes) = link.client.receive(Lane::Reliable) {
                if bytes != reliable_payload(link.down.received, 1) {
                    errors.push(format!("healthy {index}: host message {} out of order or corrupt", link.down.received));
                }
                link.down.received += 1;
            }
            while let Some(bytes) = link.client.receive(Lane::Datagram) {
                if bytes.len() == DATAGRAM_BYTES && bytes[0] == b'D' {
                    let sent = u64::from_le_bytes(bytes[9..17].try_into().expect("eight bytes"));
                    rtts.push(now_us.saturating_sub(sent));
                } else {
                    errors.push(format!("healthy {index}: corrupt echo"));
                }
            }
            if self.paused {
                continue;
            }
            if now >= link.next_reliable {
                for (end, ordered, direction) in [(&link.client, &mut link.up, 0), (&link.host, &mut link.down, 1)] {
                    let sent = Instant::now();
                    match end.send(Lane::Reliable, &reliable_payload(ordered.sent, direction), now) {
                        Ok(()) => ordered.sent += 1,
                        Err(LinkError::Full) => (),
                        Err(error) => errors.push(format!("healthy {index}: reliable send {error:?}")),
                    }
                    longest_send = longest_send.max(sent.elapsed().as_micros());
                }
                link.next_reliable += 100;
            }
            if now >= link.next_datagram {
                let mut bytes = vec![0x5a; DATAGRAM_BYTES];
                bytes[0] = b'D';
                bytes[1..9].copy_from_slice(&link.datagram_seq.to_le_bytes());
                bytes[9..17].copy_from_slice(&now_us.to_le_bytes());
                if let Err(error) = link.client.send(Lane::Datagram, &bytes, now) {
                    errors.push(format!("healthy {index}: datagram send {error:?}"));
                }
                link.datagram_seq += 1;
                link.next_datagram = now + 1_000 / 60;
            }
        }
        self.errors.extend(errors);
        self.rtts.entry(self.phase.clone()).or_default().extend(rtts);
        for offender in &mut self.offenders {
            while let Some(event) = offender.link.event() {
                offender.events.push((now, format!("{event:?}")));
            }
            while let Some(bytes) = offender.link.receive(Lane::Reliable) {
                offender.reliable += 1;
                offender.reliable_bytes += bytes.len() as u64;
            }
            while offender.link.receive(Lane::Datagram).is_some() {
                offender.datagrams += 1;
            }
        }
        let mut total = 0;
        let profile = self.profile.clone();
        let links: Vec<Link> = self
            .healthy
            .iter()
            .map(|h| h.host.clone())
            .chain(self.offenders.iter().map(|o| o.link.clone()))
            .chain(self.claimed.iter().cloned())
            .collect();
        for link in &links {
            let counts = link.counters(now);
            let [reliable, datagram] = counts.lanes;
            let peaks = &mut self.peaks;
            peaks.reliable_out = peaks.reliable_out.max(reliable.queued_bytes);
            peaks.datagram_out = peaks.datagram_out.max(datagram.queued_bytes);
            peaks.reliable_in = peaks.reliable_in.max(reliable.inbound_bytes);
            peaks.datagram_in = peaks.datagram_in.max(datagram.inbound_bytes);
            total += reliable.queued_bytes + datagram.queued_bytes + reliable.inbound_bytes + datagram.inbound_bytes;
            if reliable.queued_bytes > profile.reliable_bytes
                || datagram.queued_bytes > profile.datagram_bytes
                || reliable.inbound_bytes > profile.reliable_bytes
                || datagram.inbound_bytes > profile.datagram_bytes
            {
                self.errors.push(format!("{}: host link {} over its caps: {counts:?}", self.case, link.id()));
            }
        }
        self.peaks.host_total = self.peaks.host_total.max(total);
        self.peaks.longest_send_us = self
            .peaks
            .longest_send_us
            .max(u64::try_from(longest_send).unwrap_or(u64::MAX));
        self.peaks.longest_tick_us = self
            .peaks
            .longest_tick_us
            .max(u64::try_from(began.elapsed().as_micros()).unwrap_or(u64::MAX));
    }
    /// Stop healthy sends, wait until every reliable message sent has arrived both ways, record the counts and
    /// carry on. A lost, duplicated or reordered message fails here or in the tick's order check.
    pub(crate) fn quiesce(&mut self) -> Value {
        self.paused = true;
        let settled = self.run_until(Duration::from_secs(15), |arena| {
            arena
                .healthy
                .iter()
                .all(|h| h.up.received == h.up.sent && h.down.received == h.down.sent)
        });
        if let Err(error) = settled {
            self.errors.push(format!("{}: healthy links did not settle: {error}", self.case));
        }
        let links: Vec<Value> = self
            .healthy
            .iter()
            .map(|h| {
                json!({
                    "client_id": h.client.id(), "host_id": h.host.id(),
                    "open": h.client.is_open() && h.host.is_open(),
                    "reliable_client_to_host": {"sent": h.up.sent, "received": h.up.received},
                    "reliable_host_to_client": {"sent": h.down.sent, "received": h.down.received},
                    "datagrams_sent": h.datagram_seq, "echoes": h.echoes,
                })
            })
            .collect();
        for (index, h) in self.healthy.iter().enumerate() {
            if !(h.client.is_open() && h.host.is_open()) {
                self.errors.push(format!("{}: healthy link {index} closed", self.case));
            }
        }
        self.paused = false;
        json!(links)
    }
    pub(crate) fn host_counters(&self) -> HostCounters {
        self.host.counters()
    }
    pub(crate) fn rtt_summary(&self, phase: &str) -> Value {
        let mut samples = self.rtts.get(phase).cloned().unwrap_or_default();
        samples.sort_unstable();
        let at = |q: f64| -> Value {
            if samples.is_empty() {
                return Value::Null;
            }
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss, reason = "quantile index")]
            let index = ((samples.len() - 1) as f64 * q).round() as usize;
            json!(samples[index] as f64 / 1_000.0)
        };
        json!({"count": samples.len(), "median_ms": at(0.5), "p95_ms": at(0.95), "p99_ms": at(0.99), "maximum_ms": at(1.0)})
    }
    pub(crate) fn take_peaks(&mut self) -> Value {
        let peaks = std::mem::take(&mut self.peaks);
        json!({
            "reliable_out_bytes": peaks.reliable_out, "datagram_out_bytes": peaks.datagram_out,
            "reliable_in_bytes": peaks.reliable_in, "datagram_in_bytes": peaks.datagram_in,
            "host_total_bytes": peaks.host_total,
            "longest_tick_ms": peaks.longest_tick_us as f64 / 1_000.0,
            "longest_send_ms": peaks.longest_send_us as f64 / 1_000.0,
        })
    }
    pub(crate) fn caps(&self) -> Value {
        json!({
            "reliable_bytes_each_way": self.profile.reliable_bytes,
            "datagram_bytes_each_way": self.profile.datagram_bytes,
            "links": self.profile.links,
            "per_link_queue_bytes": 2 * (self.profile.reliable_bytes + self.profile.datagram_bytes),
        })
    }
}
