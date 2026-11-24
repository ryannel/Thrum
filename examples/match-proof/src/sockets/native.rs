//! Native headless authority and independently launched desktop or simulator replica.

use std::{
    cell::RefCell,
    fs,
    io::{self, BufWriter, Write},
    net::{Ipv4Addr, SocketAddr},
    path::Path,
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use thrum_network::session::{ClientState, Host, SLOTS, Settings, World};
use thrum_transport::{Controller, Link, Profile, native};

use super::{
    CONTROL_WITNESS_INTERVAL, CrowdWorld, FaultWire, Instant, Player, ProofResult, Shared, Trace,
    controls, counters, micros, timestamp, token_from_text,
};

fn emit(output: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")
}

fn read_control(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn profile(controller: &str) -> ProofResult<Profile> {
    let controller = match controller {
        "bbr" => Controller::Bbr,
        "cubic" => Controller::Cubic,
        _ => return Err("unknown Quinn controller".into()),
    };
    Ok(Profile {
        controller,
        ..Profile::default()
    })
}

fn configure_workers(bodies: u32) -> ProofResult<usize> {
    if !cfg!(target_os = "macos") && bodies == 10_000 {
        // Each peer is a separate process. A small pool keeps simultaneous
        // peers from each creating a machine-sized pool.
        rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build_global()?;
        return Ok(2);
    }
    Ok(1)
}

/// Run one fixed-clock session authority on a loopback socket.
///
/// # Errors
/// A socket, peer session, game step or evidence write failed.
///
/// # Panics
/// The proof world has lost its image observer.
pub fn host(
    bodies: u32,
    ticks: u64,
    controller: &str,
    listen: bool,
    trace: &str,
    join_ms: u32,
    control: &Path,
) -> ProofResult<()> {
    let trace = Trace::parse(trace)?;
    let thread_qos = super::scheduling::interactive()?;
    let rule_workers = configure_workers(bodies)?;
    let profile = profile(controller)?;
    let transport =
        native::Host::start(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)), profile.clone())?;
    // The cross-target campaign shares the machine with builds. Keep ordinary
    // held-control and absence deadlines while giving an active replica time
    // to drain its bounded backlog and the terminal endpoint.
    let settings = Settings {
        behind: 240,
        catch_up: 32,
        playout_catch_up: 1_250,
        catch_up_deadline: 3_600,
        repeats: 4,
        response_ms: join_ms.min(30_000),
        join_ms,
        ..Settings::default()
    };
    let world = Shared::new(CrowdWorld::new(bodies, 1, true, true).without_publication());
    world.0.borrow_mut().images = Some(RefCell::default());
    let seed = 71;
    let mut host = Host::new(world, FaultWire, settings, [9; 16], seed, listen, 0)?;
    let clock = Instant::now();
    let mut output = BufWriter::new(io::stdout().lock());
    let mut links = Vec::<Link>::new();
    let mut prior_axes = [[0.0; 3]; SLOTS];
    let mut started = false;
    let mut stopped = None;
    let mut offered_ticket = false;
    let mut next_poll = 0;
    let mut next_report = 0;
    let mut held = [0_u64; SLOTS];
    let mut local_staged = 0;
    let mut end_tick = ticks;
    // Sweep the authority's wall-clock debt without burning through input
    // periods faster than a real peer can replenish its negotiated lead.
    // The bound leaves every tick due on the session clock.
    let minimum_barrier_spacing_us = u64::from(settings.tick_us) * 97 / 100;
    let mut next_barrier_us = 0;
    loop {
        let now = micros(clock.elapsed());
        if !offered_ticket {
            if let Some(ticket) = transport.ticket() {
                emit(
                    &mut output,
                    &timestamp(json!({
                        "event": "host", "pid": std::process::id(), "ticket": ticket.to_string(),
                        "listen": listen, "bodies": bodies, "seed": seed, "controller": controller, "rule_workers": rule_workers, "thread_qos": thread_qos,
                        "tick_us": settings.tick_us, "minimum_barrier_spacing_us": minimum_barrier_spacing_us,
                        "digest_interval": settings.digest_interval,
                        "absence": settings.absence, "held_expiry": settings.held_expiry,
                        "idle_ms": profile.idle_ms, "staging": settings.staging,
                    "behind": settings.behind, "catch_up": settings.catch_up, "repeats": settings.repeats,
                    "catch_up_deadline": settings.catch_up_deadline, "playout_catch_up": settings.playout_catch_up,
                    "response_ms": settings.response_ms, "join_ms": settings.join_ms,
                        "backlog": settings.backlog, "image_limit": settings.image_limit,
                        "command_cap": settings.command_cap, "outbound_caps": [profile.reliable_bytes, profile.datagram_bytes],
                    })),
                )?;
                output.flush()?;
                offered_ticket = true;
            } else if transport.failed() || now > 5_000_000 {
                return Err("socket host failed to start".into());
            }
        }
        if now >= next_poll {
            next_poll = now + 10_000;
            let command = read_control(control);
            if !started && command == "run" {
                host.start(now);
                emit(
                    &mut output,
                    &timestamp(json!({"event": "start", "clock_us": now})),
                )?;
                started = true;
            }
            if let Some(end) = command.strip_prefix("end ")
                && let Ok(end) = end.parse::<u64>()
                && (host.world().tick()..=end_tick).contains(&end)
            {
                end_tick = end;
            }
        }
        while let Some(link) = transport.accept() {
            host.connect(link.clone(), now)?;
            emit(
                &mut output,
                &timestamp(json!({"event":"accept", "link": link.id()})),
            )?;
            links.push(link);
        }

        let prior = host.world().tick();
        if listen && started && prior < end_tick && local_staged <= prior {
            local_staged = prior + 1;
            host.stage_local(local_staged, controls(local_staged), [])?;
        }
        let advance = Instant::now();
        let through = prior + u64::from(now >= next_barrier_us);
        host.advance(now, end_tick.min(through));
        let advance_us = micros(advance.elapsed());
        if let Some(error) = host.take_step_error() {
            return Err(error.into());
        }
        let tick = host.world().tick();
        let admissions = host.admissions();
        if tick != prior {
            next_barrier_us = now + minimum_barrier_spacing_us;
            for (slot, admitted) in admissions.into_iter().enumerate() {
                if host.ready(slot) && !admitted {
                    held[slot] += 1;
                }
            }
            emit(
                &mut output,
                &timestamp(json!({"event": "commit", "tick": tick,
                    "clock_us": micros(clock.elapsed()), "advance_us": advance_us,
                    "admissions": admissions, "held": held})),
            )?;
        }
        emit_world(
            &mut output,
            host.world(),
            advance_us,
            trace,
            &mut prior_axes,
        )?;
        while let Some(closed) = host.take_disconnect() {
            let final_counters = links
                .iter()
                .find(|link| link.id() == closed.link)
                .map(|link| counters(link, now));
            emit(
                &mut output,
                &timestamp(json!({"event": "disconnect", "link": closed.link,
                "epoch": closed.epoch, "tick": closed.tick, "clock_us": now, "reason": format!("{:?}", closed.reason), "counters": final_counters})),
            )?;
            links.retain(|link| link.id() != closed.link);
        }
        while let Some(fault) = host.take_divergence() {
            emit(
                &mut output,
                &timestamp(json!({"event": "divergence", "link": fault.link,
                "slot": fault.slot, "tick": fault.tick, "expected": fault.expected.map(|d| format!("{d:016x}")),
                "actual": fault.actual.map(|d| format!("{d:016x}")), "parts": fault.parts})),
            )?;
        }
        let trace_commit = tick != prior
            && (trace == Trace::Axes
                || trace == Trace::Seeded && tick.is_multiple_of(CONTROL_WITNESS_INTERVAL));
        if now >= next_report || trace_commit {
            next_report = now + 100_000;
            for link in &links {
                if let Some(stream) = host.stream(link.id()) {
                    emit(
                        &mut output,
                        &timestamp(json!({
                            "event":"stream", "clock_us": now, "tick": tick, "link": link.id(), "slot": stream.slot,
                            "epoch": stream.epoch, "retained_bytes": stream.retained_bytes,
                        "held": stream.held, "stepped": stream.stepped,
                            "retained_sets": host.retained(link.id()), "staged_frames": stream.staged_frames,
                            "synchronizing": stream.synchronizing, "repairs": stream.repairs,
                            "last_input": stream.last_input, "last_staged": stream.last_staged,
                            "counters": counters(link, now),
                        })),
                    )?;
                }
            }
            output.flush()?;
        }
        if tick == end_tick && stopped.is_none() {
            host.stop();
            stopped = Some(now);
        }
        let drain_us = u64::from(settings.response_ms) * 1_000 + 500_000;
        if stopped.is_some_and(|finished| links.is_empty() || now - finished >= drain_us) {
            emit(
                &mut output,
                &timestamp(json!({"event":"finish","tick": tick, "held": held})),
            )?;
            output.flush()?;
            return Ok(());
        }
        if tick == prior {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn emit_world(
    output: &mut impl Write,
    world: &Shared,
    advance_us: u64,
    trace: Trace,
    prior_axes: &mut [[f32; 3]; SLOTS],
) -> io::Result<()> {
    let mut world = world.0.borrow_mut();
    for &(tick, digest) in world.crowd.digests.borrow().iter() {
        emit(
            output,
            &timestamp(
                json!({"event": "digest", "tick": tick, "digest": format!("{digest:016x}")}),
            ),
        )?;
    }
    world.crowd.digests.borrow_mut().clear();
    for (baseline_tick, digest, bytes) in world
        .images
        .as_ref()
        .expect("recording enabled")
        .borrow_mut()
        .drain(..)
    {
        emit(
            output,
            &timestamp(json!({"event": "image", "tick": baseline_tick,
            "digest": format!("{digest:016x}"), "bytes": bytes, "advance_us": advance_us})),
        )?;
    }
    for step in world.steps.drain(..) {
        if trace == Trace::Seeded && step.tick.is_multiple_of(CONTROL_WITNESS_INTERVAL) {
            emit(
                output,
                &timestamp(json!({"event": "admitted_axes", "tick": step.tick, "axes": step.axes})),
            )?;
        }
        if trace == Trace::Axes
            && (step.axes != *prior_axes
                || step.roster.joined | step.roster.away | step.roster.back | step.roster.left != 0)
        {
            emit(
                output,
                &timestamp(
                    json!({"event": "axes", "tick": step.tick, "axes": step.axes,
                "roster": [step.roster.joined, step.roster.away, step.roster.back, step.roster.left]}),
                ),
            )?;
        }
        *prior_axes = step.axes;
    }
    Ok(())
}

/// Run one native replica. Reconnects make a fresh session from the persisted token.
///
/// # Errors
/// A socket, peer session, token write or evidence write failed.
pub fn client(
    ticket: &str,
    bodies: u32,
    controller: &str,
    control: &Path,
    token_path: &Path,
    trace: &str,
    trace_authored_frames: bool,
) -> ProofResult<()> {
    let ticket = ticket.parse()?;
    let trace = Trace::parse(trace)?;
    let thread_qos = super::scheduling::interactive()?;
    let rule_workers = configure_workers(bodies)?;
    let profile = profile(controller)?;
    let mut output = BufWriter::new(io::stdout().lock());
    let clock = Instant::now();
    emit(
        &mut output,
        &timestamp(
            json!({"event": "client", "pid": std::process::id(), "bodies": bodies,
        "controller": controller, "rule_workers": rule_workers, "thread_qos": thread_qos,
        "os": std::env::consts::OS, "arch": std::env::consts::ARCH}),
        ),
    )?;
    output.flush()?;
    while read_control(control) == "hold" {
        thread::sleep(Duration::from_millis(10));
    }
    let load = || match fs::read_to_string(token_path) {
        Ok(text) => token_from_text(&text).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    };
    let player = || {
        Player::new(
            &ticket,
            profile.clone(),
            bodies,
            load()?,
            trace,
            trace_authored_frames,
        )
    };
    let mut active = player()?;
    let mut saved = load()?;
    let mut command = String::new();
    let mut next_poll = 0;
    loop {
        let now = micros(clock.elapsed());
        if now >= next_poll {
            next_poll = now + 10_000;
            let next = read_control(control);
            if next != command {
                match next.as_str() {
                    "exit" => return Ok(()),
                    "reconnect" => {
                        active = player()?;
                    }
                    "perturb" => emit(&mut output, &active.perturb()?)?,
                    _ => {}
                }
                command = next;
            }
        }
        let prior = active.client.world().tick();
        if command != "stall" {
            for row in active.advance()? {
                emit(&mut output, &row)?;
            }
            if active.token_out.is_some() && active.token_out != saved {
                let temporary = token_path.with_extension("partial");
                if let Some(token) = active.token_out {
                    fs::write(&temporary, crate::hex(&token.to_bytes()))?;
                    fs::rename(&temporary, token_path)?;
                    saved = Some(token);
                }
            }
            output.flush()?;
            if matches!(active.client.state(), ClientState::Ended(..)) {
                return Ok(());
            }
        }
        if active.client.world().tick() == prior {
            thread::sleep(Duration::from_millis(1));
        }
    }
}
