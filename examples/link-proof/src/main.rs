//! Explicit socket runner; default tests never execute this program.
#[cfg(not(target_arch = "wasm32"))]
mod host_drop;
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use link_proof::Actor;
    use std::{
        io::{self, Write},
        time::{Duration, Instant},
    };
    use thrum_transport::{Controller, Link, Profile, native};
    let args: Vec<_> = std::env::args().collect();
    let role = args.get(1).ok_or("host or client required")?;
    if role == "host-drop" {
        let record = host_drop::run()?;
        writeln!(io::stdout(), "{record}")?;
        return if record["pass"] == true {
            Ok(())
        } else {
            Err("host-drop expectations failed".into())
        };
    }
    let seconds: u64 = args.get(3).map_or("32", String::as_str).parse()?;
    let case = args.get(5).map_or("exchange", String::as_str);
    let control = args.get(6);
    let mut last_command = String::new();
    let mut next_progress = 1_000;
    let client_ticket = if role == "host" {
        None
    } else {
        Some(
            args.get(2)
                .ok_or("ticket required")?
                .parse::<thrum_transport::Ticket>()?,
        )
    };
    let profile = Profile {
        controller: if args.get(4).is_some_and(|v| v == "cubic") {
            Controller::Cubic
        } else {
            Controller::Bbr
        },
        ..Profile::default()
    };
    let host = if role == "host" {
        Some(native::Host::start(
            "127.0.0.1:0".parse()?,
            profile.clone(),
        )?)
    } else {
        None
    };
    let epoch = Instant::now();
    let epoch_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let mut ever_opened = false;
    let mut links: Vec<(Link, Actor, Instant)> = Vec::new();
    if let Some(host) = &host {
        loop {
            if let Some(ticket) = host.ticket() {
                writeln!(
                    io::stdout(),
                    "{}",
                    serde_json::json!({"ticket":ticket.to_string(), "pid":std::process::id(),"epoch_unix_ms":epoch_unix_ms})
                )?;
                io::stdout().flush()?;
                break;
            }
            if host.failed() || epoch.elapsed().as_secs() > 5 {
                return Err("host startup failed".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    } else {
        let ticket = client_ticket.as_ref().ok_or("ticket required")?;
        writeln!(
            io::stdout(),
            "{}",
            serde_json::json!({"client_pid":std::process::id(), "platform":std::env::consts::OS, "arch":std::env::consts::ARCH})
        )?;
        io::stdout().flush()?;
        links.push((
            native::connect(ticket, profile.clone())?,
            Actor::new(0, case),
            Instant::now(),
        ));
    }
    loop {
        let command = control
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let command = command.trim();
        if command != last_command {
            if command == "reconnect" || command == "close" {
                for (link, actor, _) in &mut links {
                    link.close(88);
                    writeln!(
                        io::stdout(),
                        "{}",
                        serde_json::json!({"closed_result":actor.record(link,u64::try_from(epoch.elapsed().as_millis())?),"control":command})
                    )?;
                }
                if command == "reconnect" {
                    links.clear();
                    links.push((
                        native::connect(
                            client_ticket.as_ref().ok_or("client ticket required")?,
                            profile.clone(),
                        )?,
                        Actor::new(0, case),
                        Instant::now(),
                    ));
                }
            }
            if command == "drain" {
                for (_, actor, _) in &mut links {
                    actor.drain_only();
                }
            }
            last_command = command.into();
            io::stdout().flush()?;
        }
        if let Some(host) = &host {
            while let Some(link) = host.accept() {
                ever_opened = true;
                links.push((link, Actor::new(1, case), Instant::now()));
            }
        }
        for (link, script, started) in &mut links {
            let now = u64::try_from(epoch.elapsed().as_millis())?;
            script
                .tick(link, now)
                .map_err(|e| format!("script failed: {e:?}"))?;
            let mut events = Vec::new();
            while let Some(event) = link.event() {
                events.push(format!("{event:?}"));
            }
            if !events.is_empty() {
                writeln!(
                    io::stdout(),
                    "{}",
                    serde_json::json!({"id":link.id(), "elapsed_ms":now, "events":events})
                )?;
                io::stdout().flush()?;
                if events.iter().any(|e| e.starts_with("Closed(")) {
                    writeln!(
                        io::stdout(),
                        "{}",
                        serde_json::json!({"closed_result":script.record(link,now),"elapsed_ms":now})
                    )?;
                    io::stdout().flush()?;
                }
            }
            if started.elapsed().as_secs() >= seconds || command == "finish" {
                writeln!(
                    io::stdout(),
                    "{}",
                    serde_json::json!({"result":script.record(link,now), "elapsed_ms":now})
                )?;
                io::stdout().flush()?;
                link.close(77);
                let mut final_events = Vec::new();
                while let Some(event) = link.event() {
                    final_events.push(format!("{event:?}"));
                }
                if !final_events.is_empty() {
                    writeln!(
                        io::stdout(),
                        "{}",
                        serde_json::json!({"id":link.id(),"elapsed_ms":now,"events":final_events,"after_drain":true})
                    )?;
                }
            }
        }
        let now = u64::try_from(epoch.elapsed().as_millis())?;
        if case == "session" && now >= next_progress {
            writeln!(
                io::stdout(),
                "{}",
                serde_json::json!({"elapsed_ms":now,"progress":links.iter().map(|(l,a,_)|a.record(l,now)).collect::<Vec<_>>()})
            )?;
            io::stdout().flush()?;
            next_progress += 1_000;
        }
        links.retain(|(_, _, start)| start.elapsed().as_secs() < seconds);
        if command == "finish" {
            break;
        }
        if links.is_empty() && (host.is_none() || ever_opened || epoch.elapsed().as_secs() > 60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    // The game-facing close is immediate; leave the I/O runtime alive for its bounded close flight.
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}
#[cfg(target_arch = "wasm32")]
fn main() {}
