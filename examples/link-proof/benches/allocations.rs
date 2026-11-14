//! Explicit socket allocation proof. Excluded from default tests by a required feature.
#[global_allocator]
static ALLOCATOR: divan::AllocProfiler = divan::AllocProfiler::system();

const CONTROLLER: &str = "TRANSPORT_ALLOCATION_CONTROLLER";
struct Relay(std::process::Child, std::path::PathBuf);
impl Drop for Relay {
    fn drop(&mut self) {
        drop(self.0.kill());
        drop(self.0.wait());
        drop(std::fs::remove_file(&self.1));
    }
}
fn relay(ticket: &mut thrum_transport::Ticket) -> Relay {
    use std::io::{BufRead, BufReader};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::create_dir_all(root.join("tmp/transport")).expect("scratch");
    let output = root.join(format!(
        "tmp/transport/alloc-relay-{}.json",
        std::process::id()
    ));
    let mut child = std::process::Command::new("python3")
        .current_dir(root)
        .args([
            "scripts/transport_relay.py",
            "--host",
            &ticket.address,
            "--output",
        ])
        .arg(&output)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("relay");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("relay address");
    let value: serde_json::Value = serde_json::from_str(&line).expect("relay JSON");
    ticket.address = value["address"].as_str().expect("address").into();
    Relay(child, output)
}

fn main() {
    divan::main();
}

#[divan::bench(sample_count = 100, sample_size = 1)]
fn warmed_send_and_drain(bencher: divan::Bencher<'_, '_>) {
    use std::time::{Duration, Instant};
    use thrum_transport::{Controller, Lane, Profile, native};
    let profile = Profile {
        controller: if std::env::var(CONTROLLER).is_ok_and(|v| v == "cubic") {
            Controller::Cubic
        } else {
            Controller::Bbr
        },
        ..Profile::default()
    };
    let host = native::Host::start("127.0.0.1:0".parse().expect("address"), profile.clone())
        .expect("host");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut ticket = loop {
        if let Some(ticket) = host.ticket() {
            break ticket;
        }
        assert!(Instant::now() < deadline, "host startup");
        std::thread::yield_now();
    };
    let _relay = relay(&mut ticket);
    let client = native::connect(&ticket, profile).expect("client");
    let peer = loop {
        if let Some(peer) = host.accept() {
            break peer;
        }
        assert!(Instant::now() < deadline, "link startup");
        std::thread::yield_now();
    };
    while !client.is_open() {
        assert!(Instant::now() < deadline, "client startup");
        std::thread::yield_now();
    }
    let operation = || {
        client.send(Lane::Reliable, &[7; 189], 0).expect("send");
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Some(bytes) = peer.receive(Lane::Reliable) {
                assert_eq!(bytes, [7; 189]);
                break;
            }
            assert!(Instant::now() < deadline, "drain");
            std::thread::yield_now();
        }
    };
    for _ in 0..10 {
        operation();
    }
    bencher.bench_local(operation);
    client.close(0);
}
