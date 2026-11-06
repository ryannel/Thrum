//! Development HTTP server for browsers and remote devices that cannot map the store directly.
//!
//! Endpoints (HTTP/1.1, one request per connection):
//! - `GET /head` returns the 64-byte `HEAD` record, or `204` before the first publish;
//! - `GET /head?after=N` long-polls (up to 25 s) until a generation after `N` is published;
//! - `GET /cas/<64 hex>` returns immutable product bytes with a year-long immutable cache policy.
//!
//! It binds where it is told; callers should use loopback unless they mean to expose the store.
//! Clients verify product bytes against their content address before use.
//!
//! Web pages cannot read it: a request whose `Host` is not a loopback name or the bound address
//! is refused (DNS rebinding), and a request carrying an `Origin` other than a loopback origin is
//! refused; a loopback origin is echoed rather than allowed with a wildcard. At most
//! 32 requests are served at once.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use thrum_asset::{AssetStore, ContentHash};

const LONG_POLL: Duration = Duration::from_secs(25);
/// Concurrent connections served; further connections are closed at once.
const MAX_CONNECTIONS: usize = 32;
const MAX_HEADER_LINES: usize = 64;

/// Publication signal shared between the cook loop and the server.
#[derive(Debug, Default)]
pub struct Publications {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl Publications {
    /// Record a new publication and wake waiting clients.
    pub fn announce(&self, generation: u64) {
        let mut current = self
            .generation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = (*current).max(generation);
        self.changed.notify_all();
    }

    fn wait_after(&self, after: u64, timeout: Duration) {
        let current = self
            .generation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Timing out is a normal long-poll outcome; the client re-asks.
        drop(
            self.changed
                .wait_timeout_while(current, timeout, |generation| *generation <= after),
        );
    }
}

/// A running dev server.
#[derive(Debug)]
pub struct DevServer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl DevServer {
    /// Bind and serve `store` on background threads.
    ///
    /// # Errors
    /// Bind failure.
    pub fn start(
        address: &str,
        store: AssetStore,
        publications: Arc<Publications>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let active = Arc::new(AtomicUsize::new(0));
        std::thread::Builder::new()
            .name("thrum-dev-server".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopping.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    if active.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
                        active.fetch_sub(1, Ordering::AcqRel);
                        drop(stream);
                        continue;
                    }
                    let store = store.clone();
                    let publications = Arc::clone(&publications);
                    let active = Arc::clone(&active);
                    // A failed connection only affects that client.
                    std::thread::spawn(move || {
                        handle(stream, address, &store, &publications).ok();
                        active.fetch_sub(1, Ordering::AcqRel);
                    });
                }
            })?;
        Ok(Self { address, stop })
    }

    /// Bound address (useful when binding port zero).
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the accept loop so it observes the stop flag.
        TcpStream::connect(self.address).ok();
    }
}

/// Request line and the two headers the server's access rule reads.
struct Request {
    method: String,
    target: String,
    host: Option<String>,
    origin: Option<String>,
}

fn read_request(reader: &mut impl BufRead) -> std::io::Result<Request> {
    let mut line = String::new();
    reader.take(4096).read_line(&mut line)?;
    let mut parts = line.split_ascii_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let target = parts.next().unwrap_or("").to_owned();
    let (mut host, mut origin) = (None, None);
    for _ in 0..MAX_HEADER_LINES {
        line.clear();
        if reader.take(4096).read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let value = Some(value.trim().to_owned());
            if name.eq_ignore_ascii_case("host") {
                host = value;
            } else if name.eq_ignore_ascii_case("origin") {
                origin = value;
            }
        }
    }
    Ok(Request {
        method,
        target,
        host,
        origin,
    })
}

/// Whether an authority (`host[:port]`) names this machine's loopback or the bound address.
fn local_authority(authority: &str, bound: SocketAddr) -> bool {
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split_once(']').map_or("", |(host, _)| host)
    } else {
        authority
            .rsplit_once(':')
            .map_or(authority, |(host, _)| host)
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback() || ip == bound.ip())
}

/// A loopback page origin (`http://localhost:5173`), the only kind allowed to read responses.
fn loopback_origin(origin: &str, bound: SocketAddr) -> bool {
    origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .is_some_and(|authority| !authority.contains('/') && local_authority(authority, bound))
}

fn handle(
    stream: TcpStream,
    bound: SocketAddr,
    store: &AssetStore,
    publications: &Publications,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request = read_request(&mut BufReader::new(stream.try_clone()?))?;
    let mut stream = stream;
    let refuse =
        |stream: &mut TcpStream, status| respond(stream, status, None, "text/plain", &[], false);
    // DNS rebinding: a page on evil.example resolved to 127.0.0.1 still sends its own Host.
    if !request
        .host
        .as_deref()
        .is_some_and(|host| local_authority(host, bound))
    {
        return refuse(&mut stream, "421 Misdirected Request");
    }
    let origin = match request.origin.as_deref() {
        None => None,
        Some(origin) if loopback_origin(origin, bound) => Some(origin),
        Some(_) => return refuse(&mut stream, "403 Forbidden"),
    };
    let target = request.target.as_str();
    let mut stream = Reply {
        stream: &mut stream,
        origin,
    };
    if request.method != "GET" {
        return stream.send("405 Method Not Allowed", "text/plain", &[], false);
    }
    if let Some(after) = target
        .strip_prefix("/head?after=")
        .and_then(|value| value.parse().ok())
    {
        publications.wait_after(after, LONG_POLL);
        return send_head(&mut stream, store);
    }
    if target == "/head" {
        return send_head(&mut stream, store);
    }
    if let Some(hash) = target.strip_prefix("/cas/").and_then(ContentHash::from_hex) {
        return match std::fs::read(store.product_path(&hash)) {
            Ok(bytes) => stream.send("200 OK", "application/octet-stream", &bytes, true),
            Err(_) => stream.send("404 Not Found", "text/plain", &[], false),
        };
    }
    stream.send("404 Not Found", "text/plain", &[], false)
}

struct Reply<'a> {
    stream: &'a mut TcpStream,
    origin: Option<&'a str>,
}

impl Reply<'_> {
    fn send(
        &mut self,
        status: &str,
        content_type: &str,
        body: &[u8],
        immutable: bool,
    ) -> std::io::Result<()> {
        respond(
            self.stream,
            status,
            self.origin,
            content_type,
            body,
            immutable,
        )
    }
}

fn send_head(stream: &mut Reply<'_>, store: &AssetStore) -> std::io::Result<()> {
    match store.read_head() {
        Ok(Some(head)) => stream.send("200 OK", "application/octet-stream", &head.encode(), false),
        Ok(None) => stream.send("204 No Content", "text/plain", &[], false),
        Err(_) => stream.send("503 Service Unavailable", "text/plain", &[], false),
    }
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    origin: Option<&str>,
    content_type: &str,
    body: &[u8],
    immutable: bool,
) -> std::io::Result<()> {
    let cache = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    };
    let cors = origin.map_or_else(String::new, |origin| {
        format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n")
    });
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\n{cors}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
