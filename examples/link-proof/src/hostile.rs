//! M2's hostile and stalled peers, each beside three healthy links on real loopback sockets.
//!
//! Every offender is written directly against Quinn, below the engine's adapter, so it can send what the adapter
//! never would. It pins the host's certificate hash like any client: nothing here relaxes a certificate check.
//! The cases run one at a time. After each, the healthy links stop sending until every reliable message sent has
//! arrived both ways, and the record keeps what the host reported for the offender, its counters and the
//! largest queues it held.
use crate::arena::Arena;
use serde_json::{Value, json};
use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use thrum_transport::{HELLO, HostCounters, Lane, LinkError, RELIABLE_LIMIT, wire};
use web_transport_quinn::quinn::{self, rustls};

type Result<T> = std::result::Result<T, String>;
fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[derive(Debug)]
struct Pinned([u8; 32], Arc<rustls::crypto::CryptoProvider>);
impl rustls::client::danger::ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if web_transport_quinn::crypto::sha256(&self.1, end).as_ref() == self.0 {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ))
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            signature,
            &self.1.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            signature,
            &self.1.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.1.signature_verification_algorithms.supported_schemes()
    }
}

/// A Quinn client config that pins the host's hash and asks for the given ALPN.
fn config(hash: [u8; 32], alpn: &[u8]) -> Result<quinn::ClientConfig> {
    let provider = web_transport_quinn::crypto::default_provider();
    let mut tls = rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(text)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned(hash, provider)))
        .with_no_client_auth();
    tls.alpn_protocols = vec![alpn.to_vec()];
    let mut config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls).map_err(text)?,
    ));
    let mut transport = quinn::TransportConfig::default();
    // The offender keeps its own connection alive well past the host's idle timeout.
    transport.max_idle_timeout(Some(Duration::from_secs(20).try_into().map_err(text)?));
    transport.keep_alive_interval(Some(Duration::from_millis(500)));
    config.transport_config(Arc::new(transport));
    Ok(config)
}

struct Peer {
    host: SocketAddr,
    hash: [u8; 32],
}
impl Peer {
    fn endpoint() -> Result<quinn::Endpoint> {
        quinn::Endpoint::client("127.0.0.1:0".parse().map_err(text)?).map_err(text)
    }
    async fn connect(&self, alpn: &[u8]) -> Result<(quinn::Endpoint, quinn::Connection)> {
        let endpoint = Self::endpoint()?;
        let connection = endpoint
            .connect_with(config(self.hash, alpn)?, self.host, "localhost")
            .map_err(text)?
            .await
            .map_err(text)?;
        Ok((endpoint, connection))
    }
    /// A raw link up to the point the adapter would report it open: the stream, both hellos.
    async fn open(
        &self,
    ) -> Result<(
        quinn::Endpoint,
        quinn::Connection,
        quinn::SendStream,
        quinn::RecvStream,
    )> {
        let (endpoint, connection) = self.connect(b"thrum-1").await?;
        let (mut send, mut recv) = connection.open_bi().await.map_err(text)?;
        send.write_all(HELLO).await.map_err(text)?;
        let mut hello = [0; 4];
        recv.read_exact(&mut hello).await.map_err(text)?;
        if &hello != HELLO {
            return Err("host's hello".into());
        }
        Ok((endpoint, connection, send, recv))
    }
}
/// Close with a code and give the close frame time to leave before the endpoint is dropped.
async fn close(connection: &quinn::Connection, code: u32) {
    connection.close(code.into(), b"offender done");
    tokio::time::sleep(Duration::from_millis(300)).await;
}
async fn message(send: &mut quinn::SendStream, bytes: &[u8]) -> Result<()> {
    send.write_all(&wire::encode_prefix(bytes.len()))
        .await
        .map_err(text)?;
    send.write_all(bytes).await.map_err(text)
}
/// How the host ended the offender's connection, as the offender saw it, within a deadline.
async fn closed(connection: &quinn::Connection, within: Duration) -> Value {
    let began = Instant::now();
    match tokio::time::timeout(within, connection.closed()).await {
        Ok(error) => {
            json!({"closed_by_host": format!("{error}"), "after_ms": began.elapsed().as_millis()})
        }
        Err(_) => json!({"closed_by_host": null, "still_open_after_ms": within.as_millis()}),
    }
}

async fn offend(case: &str, peer: Peer) -> Result<Value> {
    let rule_break = |connection: quinn::Connection| async move {
        closed(&connection, Duration::from_secs(5)).await
    };
    match case {
        "random-udp" => random_udp(&peer).await,
        "wrong-alpn" => {
            let result = peer.connect(b"thrum-2").await;
            Ok(json!({"connected": result.is_ok(), "error": result.err()}))
        }
        "wrong-path" => {
            let endpoint = Peer::endpoint()?;
            let client = web_transport_quinn::Client::new(endpoint, config(peer.hash, b"h3")?);
            let url = url::Url::parse(&format!("https://{}/thrum/2", peer.host)).map_err(text)?;
            let result = client.connect(url).await;
            Ok(json!({"session": result.is_ok(), "error": result.err().map(|e| e.to_string())}))
        }
        "first-frame-not-hello" => {
            let (_endpoint, connection) = peer.connect(b"thrum-1").await?;
            let (mut send, _recv) = connection.open_bi().await.map_err(text)?;
            let mut bytes = b"THR2".to_vec();
            bytes.extend_from_slice(&wire::encode_prefix(5));
            bytes.extend_from_slice(b"valid");
            send.write_all(&bytes).await.map_err(text)?;
            Ok(rule_break(connection).await)
        }
        "zero-prefix" | "over-limit-prefix" => {
            let (_endpoint, connection, mut send, _recv) = peer.open().await?;
            message(&mut send, b"valid").await?;
            let length: u32 = if case == "zero-prefix" { 0 } else { 65_537 };
            send.write_all(&length.to_le_bytes()).await.map_err(text)?;
            drop(
                send.write_all(&vec![7; usize::try_from(length).map_err(text)?])
                    .await,
            );
            Ok(rule_break(connection).await)
        }
        "second-stream" | "uni-stream" => {
            let (_endpoint, connection, mut send, _recv) = peer.open().await?;
            message(&mut send, b"valid").await?;
            if case == "second-stream" {
                let (mut second, _) = connection.open_bi().await.map_err(text)?;
                drop(
                    second
                        .write_all(&wire::encode_stream([b"second".as_slice()]))
                        .await,
                );
            } else {
                let mut uni = connection.open_uni().await.map_err(text)?;
                drop(
                    uni.write_all(&wire::encode_stream([b"uni".as_slice()]))
                        .await,
                );
            }
            Ok(rule_break(connection).await)
        }
        "oversize-datagram" => {
            let (_endpoint, connection, mut send, _recv) = peer.open().await?;
            message(&mut send, b"valid").await?;
            let ceiling = connection.max_datagram_size();
            connection
                .send_datagram(vec![9; 1_001].into())
                .map_err(text)?;
            let mut result = rule_break(connection).await;
            result["datagram_ceiling"] = json!(ceiling);
            Ok(result)
        }
        "stalled-mid-message" => {
            let (_endpoint, connection, mut send, _recv) = peer.open().await?;
            message(&mut send, b"valid").await?;
            send.write_all(&wire::encode_prefix(1_000))
                .await
                .map_err(text)?;
            send.write_all(&[3; 10]).await.map_err(text)?;
            // Past the host's idle timeout: Quinn's keep-alives keep the path live, the message never finishes.
            let mut result = closed(&connection, Duration::from_secs(7)).await;
            close(&connection, 9).await;
            result["stalled_ms"] = json!(7_000);
            Ok(result)
        }
        "flood-reliable" => {
            let (_endpoint, connection, mut send, _recv) = peer.open().await?;
            let began = Instant::now();
            let mut sent = 0_u64;
            let bytes = vec![5; RELIABLE_LIMIT];
            while began.elapsed() < Duration::from_secs(4) {
                if message(&mut send, &bytes).await.is_err() {
                    break;
                }
                sent += 1;
            }
            let open = connection.close_reason().is_none();
            close(&connection, 9).await;
            Ok(
                json!({"messages_sent": sent, "bytes_sent": sent * RELIABLE_LIMIT as u64, "open_at_end": open}),
            )
        }
        "flood-datagram" => {
            let (_endpoint, connection, _send, _recv) = peer.open().await?;
            let began = Instant::now();
            let mut sent = 0_u64;
            while began.elapsed() < Duration::from_secs(4) {
                for _ in 0..64 {
                    if connection.send_datagram(vec![6; 1_000].into()).is_ok() {
                        sent += 1;
                    }
                }
                tokio::task::yield_now().await;
            }
            let open = connection.close_reason().is_none();
            let stats = connection.stats();
            close(&connection, 9).await;
            Ok(
                json!({"datagrams_queued": sent, "udp_datagrams_sent": stats.udp_tx.datagrams, "open_at_end": open}),
            )
        }
        "over-cap" => over_cap(&peer).await,
        "unfinished-handshakes" => unfinished_handshakes(&peer).await,
        "no-hello" => no_hello(&peer).await,
        _ => Err(format!("unknown case {case}")),
    }
}

async fn random_udp(peer: &Peer) -> Result<Value> {
    let socket = UdpSocket::bind("127.0.0.1:0").map_err(text)?;
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut sent = 0;
    for index in 0..20_000_u64 {
        let length = usize::try_from(next() % 1_500).map_err(text)? + 1;
        let mut bytes: Vec<u8> = (0..length).map(|_| next().to_le_bytes()[0]).collect();
        // A third look like QUIC version-1 long headers, so the host parses them further.
        if index % 3 == 0 && length > 6 {
            bytes[0] = 0xc0 | (bytes[0] & 0x0f);
            bytes[1..5].copy_from_slice(&1_u32.to_be_bytes());
        }
        if socket.send_to(&bytes, peer.host).is_ok() {
            sent += 1;
        }
        if index % 500 == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    Ok(json!({"udp_datagrams_sent": sent}))
}

async fn over_cap(peer: &Peer) -> Result<Value> {
    let attempts = (0..6).map(|_| async {
        match tokio::time::timeout(Duration::from_secs(6), peer.open()).await {
            Ok(Ok((endpoint, connection, send, recv))) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                close(&connection, 9).await;
                drop((endpoint, send, recv));
                "opened".to_owned()
            }
            Ok(Err(error)) => error,
            Err(_) => "no answer in 6 s".to_owned(),
        }
    });
    Ok(json!({"clients": futures_join(attempts).await}))
}

async fn unfinished_handshakes(peer: &Peer) -> Result<Value> {
    // A forwarder passes the clients' packets to the host and drops every answer, so no handshake can end.
    let forwarder = UdpSocket::bind("127.0.0.1:0").map_err(text)?;
    forwarder
        .set_read_timeout(Some(Duration::from_millis(20)))
        .map_err(text)?;
    let through = forwarder.local_addr().map_err(text)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let host = peer.host;
    let relay = std::thread::spawn(move || {
        let (mut forwarded, mut dropped) = (0_u64, 0_u64);
        let mut buffer = [0; 2_048];
        while !stopped.load(Ordering::Relaxed) {
            if let Ok((length, from)) = forwarder.recv_from(&mut buffer) {
                if from == host {
                    dropped += 1;
                } else if forwarder.send_to(&buffer[..length], host).is_ok() {
                    forwarded += 1;
                }
            }
        }
        (forwarded, dropped)
    });
    let attempts = (0..3).map(|_| async {
        let peer = Peer {
            host: through,
            hash: peer.hash,
        };
        match tokio::time::timeout(Duration::from_secs(7), peer.connect(b"thrum-1")).await {
            Ok(Ok(_)) => "handshake finished".to_owned(),
            Ok(Err(error)) => error,
            Err(_) => "unfinished after 7 s".to_owned(),
        }
    });
    let clients = futures_join(attempts).await;
    stop.store(true, Ordering::Relaxed);
    let (forwarded, dropped) = relay.join().map_err(|_| "forwarder panicked")?;
    Ok(json!({"clients": clients, "packets_to_host": forwarded, "host_answers_dropped": dropped}))
}

async fn no_hello(peer: &Peer) -> Result<Value> {
    // One never opens the stream; the other opens it and never writes, which the host can't tell apart.
    // One after the other, since beside three healthy links the host has room for one more connection.
    let mut clients = Vec::new();
    for open_stream in [false, true] {
        let (_endpoint, connection) = match peer.connect(b"thrum-1").await {
            Ok(pair) => pair,
            Err(error) => {
                clients.push(json!({"connected": false, "error": error}));
                continue;
            }
        };
        let _stream = if open_stream {
            connection.open_bi().await.ok()
        } else {
            None
        };
        let mut result = closed(&connection, Duration::from_secs(8)).await;
        result["opened_stream"] = json!(open_stream);
        clients.push(result);
    }
    Ok(json!({"clients": clients}))
}

/// Run futures together on this thread's runtime and keep their results in order.
async fn futures_join<F: Future>(futures: impl IntoIterator<Item = F>) -> Vec<F::Output> {
    let mut pending: Vec<Option<std::pin::Pin<Box<F>>>> =
        futures.into_iter().map(|f| Some(Box::pin(f))).collect();
    let mut outputs: Vec<Option<F::Output>> = pending.iter().map(|_| None).collect();
    std::future::poll_fn(|context| {
        for (slot, output) in pending.iter_mut().zip(&mut outputs) {
            if let Some(future) = slot
                && let std::task::Poll::Ready(value) = future.as_mut().poll(context)
            {
                *output = Some(value);
                *slot = None;
            }
        }
        if pending.iter().all(Option::is_none) {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    outputs
        .into_iter()
        .map(|value| value.expect("every future finished"))
        .collect()
}

fn delta(after: HostCounters, before: HostCounters) -> Value {
    json!({
        "opened": after.opened - before.opened,
        "refused_full": after.refused_full - before.refused_full,
        "refused_version": after.refused_version - before.refused_version,
        "handshake_failed": after.handshake_failed - before.handshake_failed,
        "broke_rules": after.broke_rules - before.broke_rules,
        "startup_timed_out": after.startup_timed_out - before.startup_timed_out,
        "startup_failed": after.startup_failed - before.startup_failed,
        "active_after": after.active,
    })
}

const CASES: [&str; 15] = [
    "random-udp",
    "wrong-alpn",
    "wrong-path",
    "zero-prefix",
    "over-limit-prefix",
    "first-frame-not-hello",
    "second-stream",
    "uni-stream",
    "oversize-datagram",
    "stalled-mid-message",
    "flood-reliable",
    "flood-datagram",
    "over-cap",
    "unfinished-handshakes",
    "no-hello",
];

/// What each case must show, beyond the healthy links and caps every case checks.
fn judge(case: &str, counters: &Value, offenders: &[Value], peer: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    let count = |name: &str| counters[name].as_u64().unwrap_or(0);
    let mut expect = |ok: bool, what: &str| {
        if !ok {
            failures.push(format!("{case}: {what}"));
        }
    };
    let protocol_close = |o: &Value| {
        o["events"].as_array().is_some_and(|events| {
            events
                .iter()
                .map(|e| e[1].as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                == ["Open", "Closed(Protocol)"]
        })
    };
    match case {
        "random-udp" => expect(
            offenders.is_empty() && count("opened") == 0,
            "random bytes opened a link",
        ),
        "wrong-alpn" | "wrong-path" => {
            expect(
                offenders.is_empty() && count("opened") == 0,
                "a wrong version opened a link",
            );
            expect(
                count("refused_version") >= 1,
                "the host did not report a version refusal",
            );
            expect(peer["error"].is_string(), "the offender was not refused");
        }
        "first-frame-not-hello" => {
            expect(
                offenders.is_empty() && count("opened") == 0,
                "a link opened without the hello",
            );
            expect(
                count("broke_rules") == 1,
                "the host did not report the broken rule",
            );
            expect(
                peer["closed_by_host"].is_string(),
                "the host left the connection open",
            );
        }
        "zero-prefix" | "over-limit-prefix" | "second-stream" | "uni-stream"
        | "oversize-datagram" => {
            expect(offenders.len() == 1, "expected one offender link");
            for o in offenders {
                expect(
                    protocol_close(o),
                    "the offender's link did not close as Protocol",
                );
                // The valid message sent first may be delivered, or discarded with the close: never the rest.
                expect(
                    o["reliable"].as_u64() <= Some(1)
                        && o["reliable_bytes"].as_u64() <= Some(5)
                        && o["datagrams"] == 0,
                    "offending bytes were delivered",
                );
            }
            expect(
                peer["closed_by_host"].is_string(),
                "the host left the connection open",
            );
        }
        "stalled-mid-message" => {
            expect(offenders.len() == 1, "expected one offender link");
            for o in offenders {
                expect(o["reliable"] == 1, "the partial message was delivered");
                let events: Vec<&str> = o["events"]
                    .as_array()
                    .map(|e| e.iter().map(|e| e[1].as_str().unwrap_or("")).collect())
                    .unwrap_or_default();
                expect(
                    events == ["Open", "Closed(Peer(9))"],
                    "the stall did not end with the offender's close",
                );
            }
            expect(
                peer["closed_by_host"].is_null(),
                "the host closed a live, stalled link",
            );
        }
        "flood-reliable" | "flood-datagram" => {
            expect(offenders.len() == 1, "expected one offender link");
            for o in offenders {
                let delivered = if case == "flood-reliable" {
                    &o["reliable"]
                } else {
                    &o["datagrams"]
                };
                expect(
                    delivered.as_u64().unwrap_or(0) > 0,
                    "the flood delivered nothing",
                );
            }
        }
        "over-cap" => {
            let opened = peer["clients"]
                .as_array()
                .map_or(0, |c| c.iter().filter(|c| *c == "opened").count());
            expect(
                opened <= 1,
                "more clients opened than the cap allows beside three healthy links",
            );
            expect(
                count("refused_full") >= 5,
                "the host did not refuse the clients over its cap",
            );
        }
        "unfinished-handshakes" => {
            expect(
                offenders.is_empty() && count("opened") == 0,
                "an unfinished handshake opened a link",
            );
            expect(
                count("startup_timed_out") + count("refused_full") >= 1,
                "the host reported no unfinished handshake",
            );
        }
        "no-hello" => {
            expect(
                offenders.is_empty() && count("opened") == 0,
                "a link opened without a hello",
            );
            expect(
                count("startup_timed_out") == 2,
                "the host did not drop both silent clients at the startup timeout",
            );
            let dropped = peer["clients"]
                .as_array()
                .is_some_and(|clients| clients.iter().all(|c| c["closed_by_host"].is_string()));
            expect(dropped, "a silent client was left connected");
        }
        _ => expect(false, "unknown case"),
    }
    failures
}

/// The hostile run: every case in turn beside three healthy links.
pub(crate) fn hostile() -> Result<Value> {
    let mut arena = Arena::start(3)?;
    let address: SocketAddr = arena.ticket.address.parse().map_err(text)?;
    let mut cases = Vec::new();
    let mut failures = Vec::new();
    arena.run_for(Duration::from_secs(2));
    let baseline = arena.quiesce();
    arena.take_peaks();
    for case in CASES {
        arena.case = case.into();
        arena.phase = case.into();
        let before = arena.host_counters();
        let began = arena.now_ms();
        let hash = arena.ticket.hash;
        let name = case.to_owned();
        let offender = std::thread::spawn(move || -> Result<Value> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(text)?;
            runtime.block_on(offend(
                &name,
                Peer {
                    host: address,
                    hash,
                },
            ))
        });
        while !offender.is_finished() {
            arena.tick();
            std::thread::sleep(Duration::from_millis(1));
        }
        let peer = offender
            .join()
            .map_err(|_| format!("{case}: offender panicked"))?;
        let peer = peer.unwrap_or_else(|error| json!({"offender_error": error}));
        // Let the host finish with the offender: startup deadlines and closes.
        arena
            .run_until(Duration::from_secs(10), |a| a.host_counters().active == 3)
            .ok();
        arena.run_for(Duration::from_millis(500));
        let healthy = arena.quiesce();
        let counters = delta(arena.host_counters(), before);
        let offenders: Vec<Value> = arena
            .offenders
            .iter()
            .filter(|o| o.case == case)
            .map(|o| {
                json!({
                    "host_link_id": o.link.id(), "opened_ms_after_case_start": o.opened_ms.saturating_sub(began),
                    "events": o.events.iter().map(|(at, e)| json!([at.saturating_sub(began), e])).collect::<Vec<_>>(),
                    "reliable": o.reliable, "reliable_bytes": o.reliable_bytes, "datagrams": o.datagrams,
                    "open_at_end": o.link.is_open(),
                })
            })
            .collect();
        let case_failures = judge(case, &counters, &offenders, &peer);
        failures.extend(case_failures.iter().cloned());
        cases.push(json!({
            "case": case, "duration_ms": arena.now_ms() - began,
            "offender": peer, "host_counters": counters, "offender_links_at_host": offenders,
            "healthy_after": healthy, "host_queue_peaks": arena.take_peaks(),
            "healthy_datagram_rtt": arena.rtt_summary(case),
            "host_failed": arena.host.failed(), "failures": case_failures,
        }));
        if arena.host.failed() {
            failures.push(format!("{case}: the host's I/O failed"));
            break;
        }
    }
    // The host still takes a new link after every case.
    let late =
        thrum_transport::native::connect(&arena.ticket, arena.profile.clone()).map_err(text)?;
    let late_host = arena.wait_for_link()?;
    arena
        .run_until(Duration::from_secs(10), |_| late.is_open())
        .map_err(|e| format!("a new client could not open: {e}"))?;
    let accepts_after = late_host.is_open();
    late.close(1);
    let final_healthy = arena.quiesce();
    failures.extend(arena.errors.iter().cloned());
    Ok(json!({
        "pass": failures.is_empty(), "failures": failures,
        "profile_caps": arena.caps(), "healthy_links": 3,
        "baseline_healthy": baseline, "cases": cases,
        "new_link_after_all_cases": accepts_after, "final_healthy": final_healthy,
        "host_counters_total": format!("{:?}", arena.host_counters()),
    }))
}

/// The overload run: a client that stops taking messages while the host keeps sending to it.
pub(crate) fn overload() -> Result<Value> {
    let mut arena = Arena::start(3)?;
    arena.case = "overload".into();
    arena.phase = "without_stalled_link".into();
    arena.run_for(Duration::from_secs(10));
    let before = arena.quiesce();
    let peaks_before = arena.take_peaks();
    // The stalled client opens, then its game never takes another message.
    let stalled =
        thrum_transport::native::connect(&arena.ticket, arena.profile.clone()).map_err(text)?;
    let host_link = arena.wait_for_link()?;
    arena.run_until(Duration::from_secs(10), |_| stalled.is_open())?;
    arena.claimed.push(host_link.clone());
    arena.phase = "with_stalled_link".into();
    let began = Instant::now();
    let chunk = vec![0xab; RELIABLE_LIMIT];
    let (mut accepted, mut full, mut first_full_ms) = (0_u64, 0_u64, None);
    let mut datagrams = 0_u64;
    let mut next_datagram = 0;
    let mut samples = Vec::new();
    let mut longest_stalled_send = Duration::ZERO;
    while began.elapsed() < Duration::from_secs(20) {
        let now = arena.now_ms();
        // The host's sends to the stalled link come from the same thread as everything else.
        loop {
            let sent = Instant::now();
            let result = host_link.send(Lane::Reliable, &chunk, now);
            longest_stalled_send = longest_stalled_send.max(sent.elapsed());
            match result {
                Ok(()) => accepted += 1,
                Err(LinkError::Full) => {
                    full += 1;
                    first_full_ms
                        .get_or_insert(u64::try_from(began.elapsed().as_millis()).unwrap_or(0));
                    break;
                }
                Err(error) => return Err(format!("stalled link send: {error:?}")),
            }
        }
        if now >= next_datagram {
            let sent = Instant::now();
            host_link
                .send(Lane::Datagram, &[1; 189], now)
                .map_err(|e| format!("{e:?}"))?;
            longest_stalled_send = longest_stalled_send.max(sent.elapsed());
            datagrams += 1;
            next_datagram = now + 1_000 / 60;
        }
        if samples.len() < (began.elapsed().as_millis() / 500) as usize {
            let counts = host_link.counters(now);
            samples.push(json!({
                "elapsed_ms": began.elapsed().as_millis(),
                "reliable_queued_bytes": counts.lanes[0].queued_bytes, "datagram_queued_bytes": counts.lanes[1].queued_bytes,
                "refused_full": counts.refused[2], "expired": counts.expired, "dropped_cap": counts.dropped_cap,
                "client_inbound_bytes": stalled.counters(now).lanes[0].inbound_bytes,
            }));
        }
        arena.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    let with = arena.quiesce();
    let peaks_with = arena.take_peaks();
    let host_counts = host_link.counters(arena.now_ms());
    let stalled_open = stalled.is_open() && host_link.is_open();
    let mut failures = arena.errors.clone();
    if full == 0 {
        failures.push("the stalled link never returned Full".into());
    }
    if !stalled_open {
        failures.push("the stalled link closed".into());
    }
    if host_counts.lanes[0].queued_bytes > arena.profile.reliable_bytes
        || host_counts.lanes[1].queued_bytes > arena.profile.datagram_bytes
    {
        failures.push("the stalled link's queues passed their caps".into());
    }
    let rtt_without = arena.rtt_summary("without_stalled_link");
    let rtt_with = arena.rtt_summary("with_stalled_link");
    Ok(json!({
        "pass": failures.is_empty(), "failures": failures,
        "profile_caps": arena.caps(),
        "host_sends_from_one_thread": "every host and client send and drain in this run is made from the program's main thread, in one tick loop",
        "healthy_without": before, "healthy_with": with,
        "healthy_datagram_rtt": {"without_stalled_link": rtt_without, "with_stalled_link": rtt_with},
        "host_queue_peaks": {"without_stalled_link": peaks_before, "with_stalled_link": peaks_with},
        "stalled_link": {
            "host_link_id": host_link.id(), "client_link_id": stalled.id(), "open_throughout": stalled_open,
            "reliable_accepted": accepted, "reliable_accepted_bytes": accepted * RELIABLE_LIMIT as u64,
            "full_returned": full, "first_full_ms": first_full_ms, "datagrams_sent": datagrams,
            "longest_send_call_ms": longest_stalled_send.as_secs_f64() * 1_000.0,
            "final_host_counters": format!("{host_counts:?}"),
            "client_messages_taken": 0, "samples": samples,
        },
    }))
}
