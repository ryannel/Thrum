//! Opt-in Quinn carrier. Public calls only touch bounded application queues.
use crate::{
    CloseReason, Controller, HELLO, Lane, Link, LinkError, Profile, RELIABLE_LIMIT, Ticket,
};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use web_transport_quinn::{RecvStream, SendStream, Session};

const ALPN: &[u8] = b"thrum-1";
const PATH: &str = "/thrum/1";
type IoResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
struct HostState {
    ticket: Option<Ticket>,
    accepted: VecDeque<Link>,
    failed: bool,
}
/// A host owns one network thread. Dropping it closes all links without joining that thread.
#[derive(Debug)]
pub struct Host {
    state: Arc<Mutex<HostState>>,
    stop: Arc<AtomicBool>,
}
impl Host {
    /// Start a host asynchronously, making its key and certificate in the I/O thread.
    /// # Errors
    /// Returns a profile error or `StartFailed` if the thread cannot start.
    /// # Panics
    /// The I/O thread panics if its internal state was poisoned.
    pub fn start(address: SocketAddr, profile: Profile) -> Result<Self, LinkError> {
        profile.validate()?;
        let state = Arc::new(Mutex::new(HostState {
            ticket: None,
            accepted: VecDeque::new(),
            failed: false,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&state);
        let stopped = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("transport host".into())
            .spawn(move || {
                let result =
                    runtime().and_then(|rt| rt.block_on(serve(address, profile, &shared, stopped)));
                if result.is_err() {
                    shared.lock().expect("host state poisoned").failed = true;
                }
            })
            .map_err(|_| LinkError::StartFailed)?;
        Ok(Self { state, stop })
    }
    /// The public join ticket once local startup completes; no network is waited on.
    /// # Panics
    /// Panics if the host's state was poisoned by an I/O panic.
    pub fn ticket(&self) -> Option<Ticket> {
        self.state
            .lock()
            .expect("host state poisoned")
            .ticket
            .clone()
    }
    /// Whether local startup or the host's I/O runtime failed.
    /// # Panics
    /// Panics if the host's state was poisoned by an I/O panic.
    pub fn failed(&self) -> bool {
        self.state.lock().expect("host state poisoned").failed
    }
    /// Take an opened link. The accepted queue counts against the host's link cap.
    /// # Panics
    /// Panics if the host's state was poisoned by an I/O panic.
    pub fn accept(&self) -> Option<Link> {
        self.state
            .lock()
            .expect("host state poisoned")
            .accepted
            .pop_front()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Connect using a mandatory certificate hash and a new process-unique identifier.
/// # Errors
/// Returns a profile error, invalid address or thread creation failure immediately.
/// # Panics
/// The I/O thread panics if its internal state was poisoned.
pub fn connect(ticket: &Ticket, profile: Profile) -> Result<Link, LinkError> {
    profile.validate()?;
    let address: SocketAddr = ticket
        .address
        .parse()
        .map_err(|_| LinkError::InvalidTicket)?;
    let hash = ticket.hash;
    let link = Link::new(profile.clone());
    link.state.lock().expect("link state poisoned").clock = Some(Instant::now());
    let io = link.clone();
    std::thread::Builder::new()
        .name("transport client".into())
        .spawn(move || {
            let result =
                runtime().and_then(|rt| rt.block_on(client(address, hash, profile, io.clone())));
            if let Err(error) = result {
                let reason = if error
                    .downcast_ref::<tokio::time::error::Elapsed>()
                    .is_some()
                    || matches!(
                        error.downcast_ref::<quinn::ConnectionError>(),
                        Some(quinn::ConnectionError::TimedOut)
                    ) {
                    CloseReason::TimedOut
                } else {
                    CloseReason::Refused
                };
                io.state.lock().expect("link state poisoned").close(reason);
            }
        })
        .map_err(|_| LinkError::StartFailed)?;
    Ok(link)
}
fn runtime() -> IoResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}
fn transport(profile: &Profile) -> IoResult<Arc<quinn::TransportConfig>> {
    let mut config = quinn::TransportConfig::default();
    config.max_idle_timeout(Some(Duration::from_millis(profile.idle_ms).try_into()?));
    config.keep_alive_interval(Some(Duration::from_millis((profile.idle_ms / 4).max(1))));
    config.datagram_send_buffer_size(profile.stack_bytes);
    config.datagram_receive_buffer_size(Some(profile.datagram_bytes));
    config.stream_receive_window(u32::try_from(profile.reliable_bytes)?.into());
    config.receive_window(u32::try_from(profile.reliable_bytes)?.into());
    config.send_window(profile.reliable_bytes as u64);
    // H3's control/QPACK streams need three uni streams. A second game stream is rejected by the driver.
    config.max_concurrent_uni_streams(3_u32.into());
    config.max_concurrent_bidi_streams(2_u32.into());
    if profile.controller == Controller::Bbr {
        config.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
    }
    Ok(Arc::new(config))
}
fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}
async fn serve(
    address: SocketAddr,
    profile: Profile,
    shared: &Arc<Mutex<HostState>>,
    stop: Arc<AtomicBool>,
) -> IoResult<()> {
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)?;
    let mut params = rcgen::CertificateParams::new(vec!["localhost".into()])?;
    params.not_before = time::OffsetDateTime::now_utc() - time::Duration::minutes(1);
    params.not_after = params.not_before + time::Duration::days(13);
    let certificate = params.self_signed(&key)?;
    let hash = Sha256::digest(certificate.der()).into();
    let mut tls = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )?;
    tls.max_early_data_size = 0;
    tls.alpn_protocols = vec![ALPN.to_vec(), b"h3".to_vec()];
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls)?,
    ));
    config.transport_config(transport(&profile)?);
    let endpoint = quinn::Endpoint::server(config, address)?;
    shared.lock().expect("host state poisoned").ticket = Some(Ticket {
        address: endpoint.local_addr()?.to_string(),
        hash,
    });
    let active = Arc::new(AtomicUsize::new(0));
    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else { break; };
                if active.load(Ordering::Relaxed) >= profile.links { incoming.refuse(); continue; }
                active.fetch_add(1, Ordering::Relaxed);
                let active = Arc::clone(&active);
                let shared = Arc::clone(shared);
                let profile = profile.clone();
                tokio::spawn(async move {
                    let link = Link::new(profile.clone());
                    link.state.lock().expect("link state poisoned").clock = Some(Instant::now());
                    let result = async {
                        let connection = incoming.await?;
                        let session = if connection.handshake_data().and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok()).and_then(|d| d.protocol).as_deref() == Some(b"h3") {
                            let request = web_transport_quinn::Request::accept(connection.clone()).await?;
                            if request.connect().url.path() != PATH { return Err("wrong WebTransport path".into()); }
                            request.ok().await?
                        } else { Session::raw(connection.clone()) };
                        let (mut send, mut recv) = session.accept_bi().await?;
                        read_hello(&mut recv).await?;
                        send.write_all(HELLO).await?;
                        if session.max_datagram_size() < crate::DATAGRAM_LIMIT { return Err("datagram ceiling too small".into()); }
                        link.state.lock().expect("link state poisoned").opened();
                        {
                            let mut host = shared.lock().expect("host state poisoned");
                            host.accepted.retain(Link::is_open);
                            if host.accepted.len() >= profile.links { return Err("accepted link queue full".into()); }
                            host.accepted.push_back(link.clone());
                        }
                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((session, connection, send, recv))
                    };
                    match tokio::time::timeout(Duration::from_millis(profile.startup_ms), result).await {
                        Ok(Ok((session, connection, send, recv))) => { drive(link.clone(), session, connection, send, recv).await; }
                        _ => { link.state.lock().expect("link state poisoned").close(CloseReason::TimedOut); }
                    }
                    active.fetch_sub(1, Ordering::Relaxed);
                });
            }
            () = tokio::time::sleep(Duration::from_millis(10)) => { if stop.load(Ordering::Relaxed) { break; } }
        }
    }
    endpoint.close(0_u32.into(), b"host dropped");
    Ok(())
}
#[derive(Debug)]
struct Pinned([u8; 32]);
impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if Sha256::digest(end).as_slice() != self.0 {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ));
        }
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            signature,
            &provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            signature,
            &provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
async fn client(address: SocketAddr, hash: [u8; 32], profile: Profile, link: Link) -> IoResult<()> {
    let mut tls = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Pinned(hash)))
        .with_no_client_auth();
    tls.alpn_protocols = vec![ALPN.to_vec()];
    tls.enable_early_data = false;
    let mut config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    ));
    config.transport_config(transport(&profile)?);
    let bind = if address.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let mut endpoint = quinn::Endpoint::client(bind.parse()?)?;
    endpoint.set_default_client_config(config);
    let setup = async {
        let connection = endpoint.connect(address, "localhost")?.await?;
        let session = Session::raw(connection.clone());
        let (mut send, mut recv) = session.open_bi().await?;
        send.write_all(HELLO).await?;
        read_hello(&mut recv).await?;
        if session.max_datagram_size() < crate::DATAGRAM_LIMIT {
            return Err("datagram ceiling too small".into());
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((connection, session, send, recv))
    };
    let (connection, session, send, recv) =
        tokio::time::timeout(Duration::from_millis(profile.startup_ms), setup).await??;
    link.state.lock().expect("link state poisoned").opened();
    drive(link, session, connection, send, recv).await;
    endpoint.wait_idle().await;
    Ok(())
}
async fn read_hello(recv: &mut RecvStream) -> IoResult<()> {
    let mut hello = [0; 4];
    recv.read_exact(&mut hello).await?;
    if &hello != HELLO {
        return Err("invalid link hello".into());
    }
    Ok(())
}
async fn drive(
    link: Link,
    session: Session,
    connection: quinn::Connection,
    mut send: SendStream,
    mut recv: RecvStream,
) {
    // Each future owns at most one message outside the capped application queues.
    let empty_stack_space = session.datagram_send_buffer_space();
    let writer = async {
        loop {
            let bytes = link
                .state
                .lock()
                .expect("link state poisoned")
                .pop(Lane::Reliable);
            if let Some(bytes) = bytes {
                send.write_all(
                    &u32::try_from(bytes.len())
                        .expect("bounded message")
                        .to_le_bytes(),
                )
                .await
                .map_err(|_| CloseReason::Transport)?;
                send.write_all(&bytes)
                    .await
                    .map_err(|_| CloseReason::Transport)?;
            } else {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
        #[expect(unreachable_code, reason = "loop returns only through a failed write")]
        Ok::<(), CloseReason>(())
    };
    let reader = async {
        loop {
            let mut prefix = [0; 4];
            recv.read_exact(&mut prefix)
                .await
                .map_err(|_| CloseReason::Transport)?;
            let length = u32::from_le_bytes(prefix) as usize;
            if length == 0 || length > RELIABLE_LIMIT {
                return Err::<(), _>(CloseReason::Protocol);
            }
            let mut bytes = vec![0; length];
            recv.read_exact(&mut bytes)
                .await
                .map_err(|_| CloseReason::Transport)?;
            loop {
                let ready = {
                    let state = link.state.lock().expect("link state poisoned");
                    state.inbound_bytes[0] + length <= state.profile.reliable_bytes
                };
                if ready {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            link.state
                .lock()
                .expect("link state poisoned")
                .receive(Lane::Reliable, bytes);
        }
    };
    let datagrams_in = async {
        loop {
            let bytes = session
                .read_datagram()
                .await
                .map_err(|_| CloseReason::Transport)?;
            if bytes.is_empty() || bytes.len() > crate::DATAGRAM_LIMIT {
                return Err::<(), _>(CloseReason::Protocol);
            }
            link.state
                .lock()
                .expect("link state poisoned")
                .receive(Lane::Datagram, bytes.to_vec());
        }
    };
    let datagrams_out = async {
        loop {
            let bytes = {
                let mut state = link.state.lock().expect("link state poisoned");
                let now = u64::try_from(state.clock.expect("native clock").elapsed().as_millis())
                    .unwrap_or(u64::MAX);
                state.expire(now);
                if session.datagram_send_buffer_space()
                    >= state.outbound[1]
                        .front()
                        .map_or(usize::MAX, |m| m.bytes.len())
                {
                    state.pop(Lane::Datagram)
                } else {
                    None
                }
            };
            if let Some(bytes) = bytes {
                session
                    .send_datagram_wait(bytes.into())
                    .await
                    .map_err(|_| CloseReason::Transport)?;
            } else {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
        #[expect(unreachable_code, reason = "loop returns only through a failed send")]
        Ok::<(), CloseReason>(())
    };
    let monitor = async {
        loop {
            {
                let mut state = link.state.lock().expect("link state poisoned");
                let now = u64::try_from(state.clock.expect("native clock").elapsed().as_millis())
                    .unwrap_or(u64::MAX);
                state.expire(now);
                let stats = connection.stats();
                state.counts.stack_bytes =
                    Some(empty_stack_space.saturating_sub(session.datagram_send_buffer_space()));
                state.counts.rtt_us =
                    Some(u64::try_from(connection.rtt().as_micros()).unwrap_or(u64::MAX));
                state.counts.packets_sent = Some(stats.path.sent_packets);
                state.counts.packets_lost = Some(stats.path.lost_packets);
                state.counts.wire_bytes = Some(stats.udp_tx.bytes);
                if let Some(reason) = &state.closed {
                    return Err::<(), _>(reason.clone());
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    };
    let reason = tokio::select! {
        result = writer => result.err().unwrap_or(CloseReason::Transport),
        result = reader => result.err().unwrap_or(CloseReason::Transport),
        result = datagrams_in => result.err().unwrap_or(CloseReason::Transport),
        result = datagrams_out => result.err().unwrap_or(CloseReason::Transport),
        result = monitor => result.err().unwrap_or(CloseReason::Transport),
        result = session.accept_bi() => if result.is_ok() { CloseReason::Protocol } else { CloseReason::Transport },
        result = session.accept_uni() => if result.is_ok() { CloseReason::Protocol } else { CloseReason::Transport },
        error = connection.closed() => match error {
            quinn::ConnectionError::TimedOut => CloseReason::TimedOut,
            quinn::ConnectionError::ApplicationClosed(close) => CloseReason::Peer(u32::try_from(close.error_code.into_inner()).unwrap_or(u32::MAX)),
            _ => CloseReason::Transport,
        },
    };
    let reason = if matches!(
        connection.close_reason(),
        Some(quinn::ConnectionError::TimedOut)
    ) {
        // H3's capsule reader may wrap this as a read error. The QUIC endpoint owns idle timeouts.
        CloseReason::TimedOut
    } else {
        match session.close_reason() {
            Some(web_transport_quinn::SessionError::WebTransportError(
                web_transport_quinn::WebTransportError::Closed(code, _),
            )) => CloseReason::Peer(code),
            Some(web_transport_quinn::SessionError::ConnectionError(
                quinn::ConnectionError::TimedOut,
            )) => CloseReason::TimedOut,
            Some(web_transport_quinn::SessionError::ConnectionError(
                quinn::ConnectionError::ApplicationClosed(close),
            )) => {
                CloseReason::Peer(u32::try_from(close.error_code.into_inner()).unwrap_or(u32::MAX))
            }
            _ => reason,
        }
    };
    let code = if let CloseReason::Local(code) = reason {
        code
    } else {
        0
    };
    session.close(code, b"link closed");
    link.state
        .lock()
        .expect("link state poisoned")
        .close(reason);
    // Keep the H3 control streams alive until the CONNECT close capsule is delivered.
    // Dropping the last Session first turns a coded close into a critical-stream error in Chrome.
    drop(session.closed().await);
}
