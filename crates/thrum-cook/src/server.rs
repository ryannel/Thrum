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
//! Web pages cannot read it unless allowed: a request whose `Host` is not a loopback name or the
//! bound address is refused (DNS rebinding), and a request carrying an `Origin` is refused unless
//! that origin is loopback or listed in [`AllowedOrigins`] (empty by default). An allowed origin is
//! echoed with `Vary: Origin`, never allowed with a wildcard. At most 32 requests are served at
//! once.

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

/// Page origins beyond loopback that may read the server, for a page served from another device.
/// Empty by default, which refuses every page that is not on loopback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AllowedOrigins(Vec<String>);

impl AllowedOrigins {
    /// Exact page origins, each `http://host[:port]` or `https://host[:port]` as a browser sends
    /// it in `Origin`: no path, trailing slash, query, fragment, credentials or wildcard. Compared
    /// without regard to case.
    ///
    /// # Errors
    /// Names the first origin that is not of that form.
    pub fn parse<S: AsRef<str>>(origins: impl IntoIterator<Item = S>) -> Result<Self, String> {
        let mut allowed = Vec::new();
        for origin in origins {
            let origin = origin.as_ref();
            let authority = origin
                .strip_prefix("http://")
                .or_else(|| origin.strip_prefix("https://"))
                .unwrap_or("");
            // `[v6]:port`, `[v6]`, `name:port` or `name`.
            let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
                match rest.split_once(']') {
                    Some((v6, "")) => (v6, None),
                    Some((v6, tail)) => (v6, Some(tail.strip_prefix(':').unwrap_or(""))),
                    None => ("", None),
                }
            } else {
                match authority.split_once(':') {
                    Some((name, port)) => (name, Some(port)),
                    None => (authority, None),
                }
            };
            let v6 = authority.starts_with('[');
            let host_ok = !host.is_empty()
                && host
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || (v6 && c == ':'));
            // Digits only, no leading zero: browsers send the canonical port.
            let port_ok = port.is_none_or(|port| {
                port.bytes().all(|b| b.is_ascii_digit())
                    && !port.starts_with('0')
                    && port.parse::<u16>().is_ok()
            });
            if !host_ok || !port_ok {
                return Err(format!(
                    "allowed origin `{origin}` must be http://host[:port] or https://host[:port], \
                     exactly as the page's Origin header (no path or trailing slash)"
                ));
            }
            // Browsers omit the scheme's default port from `Origin`.
            let origin = origin.to_ascii_lowercase();
            let canonical = origin
                .strip_suffix(":80")
                .filter(|_| origin.starts_with("http://"))
                .or_else(|| {
                    origin
                        .strip_suffix(":443")
                        .filter(|_| origin.starts_with("https://"))
                })
                .unwrap_or(&origin);
            allowed.push(canonical.to_owned());
        }
        Ok(Self(allowed))
    }

    fn allows(&self, origin: &str) -> bool {
        self.0
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(origin))
    }
}

/// A running dev server.
#[derive(Debug)]
pub struct DevServer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl DevServer {
    /// Bind and serve `store` on background threads to native clients and loopback pages.
    ///
    /// # Errors
    /// Bind failure.
    pub fn start(
        address: &str,
        store: AssetStore,
        publications: Arc<Publications>,
    ) -> std::io::Result<Self> {
        Self::start_allowing(address, store, publications, AllowedOrigins::default())
    }

    /// As [`Self::start`], and also to pages from `origins`. Bind the address those pages reach
    /// (not a wildcard address), since `Host` must name it.
    ///
    /// # Errors
    /// Bind failure, or origins listed with a wildcard address (`InvalidInput`).
    pub fn start_allowing(
        address: &str,
        store: AssetStore,
        publications: Arc<Publications>,
        origins: AllowedOrigins,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;
        if !origins.0.is_empty() && address.ip().is_unspecified() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "allowed origins need the address their pages reach, not the wildcard {address}: \
                     requests must name it in Host"
                ),
            ));
        }
        let origins = Arc::new(origins);
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
                    let origins = Arc::clone(&origins);
                    let active = Arc::clone(&active);
                    // A failed connection only affects that client.
                    std::thread::spawn(move || {
                        handle(stream, address, &store, &publications, &origins).ok();
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

/// A loopback page origin (`http://localhost:5173`), always allowed to read responses.
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
    origins: &AllowedOrigins,
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
        Some(origin) if loopback_origin(origin, bound) || origins.allows(origin) => Some(origin),
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
    // Responses differ by Origin, so caches must key on it even when no Origin was sent.
    let cors = origin.map_or_else(String::new, |origin| {
        format!("Access-Control-Allow-Origin: {origin}\r\n")
    });
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nVary: Origin\r\n{cors}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::AllowedOrigins;

    #[test]
    fn allowed_origins_are_exact_page_origins() {
        let allowed = AllowedOrigins::parse([
            "http://192.168.1.20:5173",
            "https://Tablet.local",
            "http://[fd00::2]:8080",
            "https://default.local:443",
        ])
        .expect("valid origins");
        assert!(allowed.allows("http://192.168.1.20:5173"));
        assert!(allowed.allows("https://tablet.local"));
        assert!(allowed.allows("http://[fd00::2]:8080"));
        // A default port is dropped, as browsers send it.
        assert!(allowed.allows("https://default.local"));
        for near_miss in [
            "http://192.168.1.20:5174",
            "https://192.168.1.20:5173",
            "http://192.168.1.20",
            "http://192.168.1.20:5173/",
            "http://tablet.local",
            "null",
        ] {
            assert!(!allowed.allows(near_miss), "{near_miss}");
        }
        assert!(!AllowedOrigins::default().allows("http://192.168.1.20:5173"));
    }

    #[test]
    fn a_malformed_allowed_origin_is_refused_by_name() {
        for bad in [
            "192.168.1.20:5173",
            "http://host/",
            "http://host/page",
            "http://host:0",
            "http://host:99999",
            "http://host:+80",
            "http://host:05173",
            "http://*",
            "http://user@host",
            "ftp://host",
            "http://",
            "*",
        ] {
            let error = AllowedOrigins::parse([bad]).expect_err(bad);
            assert!(error.contains(bad), "{error}");
        }
    }
}
