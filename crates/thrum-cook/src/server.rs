//! Development HTTP server for browsers and remote devices that cannot map the store directly.
//!
//! Endpoints (HTTP/1.1, one request per connection):
//! - `GET /head` returns the 64-byte `HEAD` record, or `204` before the first publish;
//! - `GET /head?after=N` long-polls (up to 25 s) until a generation after `N` is published;
//! - `GET /cas/<64 hex>` returns immutable product bytes with a year-long immutable cache policy.
//!
//! It binds where it is told; callers should use loopback unless they mean to expose the store.
//! Clients verify product bytes against their content address before use.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use thrum_asset::{AssetStore, ContentHash};

const LONG_POLL: Duration = Duration::from_secs(25);

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
        std::thread::Builder::new()
            .name("thrum-dev-server".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopping.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let store = store.clone();
                    let publications = Arc::clone(&publications);
                    // A failed connection only affects that client.
                    std::thread::spawn(move || handle(stream, &store, &publications).ok());
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

fn handle(
    stream: TcpStream,
    store: &AssetStore,
    publications: &Publications,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.by_ref().take(4096).read_line(&mut request)?;
    let mut parts = request.split_ascii_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut stream = stream;
    if method != "GET" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            &[],
            false,
        );
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
            Ok(bytes) => respond(
                &mut stream,
                "200 OK",
                "application/octet-stream",
                &bytes,
                true,
            ),
            Err(_) => respond(&mut stream, "404 Not Found", "text/plain", &[], false),
        };
    }
    respond(&mut stream, "404 Not Found", "text/plain", &[], false)
}

fn send_head(stream: &mut TcpStream, store: &AssetStore) -> std::io::Result<()> {
    match store.read_head() {
        Ok(Some(head)) => respond(
            stream,
            "200 OK",
            "application/octet-stream",
            &head.encode(),
            false,
        ),
        Ok(None) => respond(stream, "204 No Content", "text/plain", &[], false),
        Err(_) => respond(stream, "503 Service Unavailable", "text/plain", &[], false),
    }
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    immutable: bool,
) -> std::io::Result<()> {
    let cache = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
