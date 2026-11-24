mod allocation;
mod faults;
use crate::world;
pub use faults::{run_fault, run_prediction};

use std::time::Instant;

use rayon::prelude::*;
use serde_json::{Value, json};
use thrum_core::profile::StateBytes;
use thrum_network::{
    envelope::{self, HEADER_BYTES, Kind},
    input_batch,
    session::{Client, ClientState, Controls, Host, PROTOCOL, PeerStream, SLOTS, Settings, World},
};
use thrum_transport::{
    Link, Profile,
    memory::{Conditioner, DatagramEvent, Memory},
};
use world::{CrowdWorld, NoActions, Time};

/// Parameters shared by the short workspace check and release match command.
#[derive(Clone, Copy)]
pub struct MemoryRun<'a> {
    /// local, a0, harsh, changing, or pinned.
    pub profile: &'a str,
    /// Authority endpoint.
    pub ticks: u64,
    /// Reference crowd size.
    pub bodies: u32,
    /// Number of repeated frames on the datagram lane.
    pub repeats: u8,
    /// Script and carrier seed.
    pub seed: u64,
    /// Turn on wall-clock and allocation measurements.
    pub costs: bool,
    /// Stall every carrier for two virtual seconds.
    pub stall: bool,
    /// Reserve the authority's slot for an in-process local player.
    pub listen: bool,
}

#[derive(Default)]
struct Probe {
    poll: Time,
    engine: Time,
    allocation_calls: Time,
    allocation_bytes: Time,
}

impl Probe {
    fn add(&mut self, start: Instant, counts: (u64, u64), engine_ns: u64) {
        self.poll
            .add(u64::try_from(start.elapsed().as_nanos()).expect("short measurement"));
        self.engine.add(engine_ns);
        self.allocation_calls.add(counts.0);
        self.allocation_bytes.add(counts.1);
    }
    fn json(&self) -> Value {
        json!({
            "advance_ns": self.poll.json(),
            "engine_tick_ns": self.engine.json(),
            "session_ns": {
                "samples": self.poll.count,
                "total": self.poll.total.saturating_sub(self.engine.total),
            },
            "allocations": self.allocation_calls.json(),
            "allocated_bytes": self.allocation_bytes.json(),
        })
    }
}

#[derive(Default)]
struct HostProbe {
    warmed: Probe,
    image_tick: Option<Value>,
}

impl HostProbe {
    fn advance(
        &mut self,
        host: &mut Host<CrowdWorld, NoActions>,
        time: u64,
        run: MemoryRun<'_>,
    ) -> Result<(), String> {
        let before = host.world().tick();
        let engine = host.world().engine.total;
        let measured = (run.costs && before >= 600 && before.is_multiple_of(16)).then(Instant::now);
        let image_tick_start = (run.costs && before == 1_800).then(Instant::now);
        if measured.is_some() {
            allocation::begin();
        }
        host.advance(time, run.ticks);
        let tick = host.world().tick();
        let engine_ns = host.world().engine.total - engine;
        if let Some(start) = measured {
            let counts = allocation::finish();
            if tick == before + 1 {
                self.warmed.add(start, counts, engine_ns);
            }
        }
        if let Some(error) = host.take_step_error() {
            return Err(error);
        }
        // On a warmed tick, run the same full image writer used in negotiation.
        // The clock covers this advance's normal barrier, world step and sends.
        if let Some(start) = image_tick_start
            && tick == before + 1
        {
            let mut image = StateBytes::new(Settings::default().image_limit as usize);
            let write_start = Instant::now();
            host.world().image(&mut image)?;
            let write_ns = u64::try_from(write_start.elapsed().as_nanos()).expect("short write");
            let total_ns = u64::try_from(start.elapsed().as_nanos()).expect("short advance");
            self.image_tick = Some(json!({
                "tick": tick, "total_ns": total_ns, "write_ns": write_ns,
                "engine_tick_ns": engine_ns, "image_bytes": image.len(),
            }));
        }
        Ok(())
    }
}

#[derive(Default)]
struct Path {
    latest: u64,
    lost_sets: u64,
    reordered_sets: u64,
    duplicated_sets: u64,
}

impl Path {
    fn observe(&mut self, direction: usize, event: DatagramEvent, bytes: &[u8]) {
        if direction == 0 {
            return;
        }
        let message = envelope::decode(bytes).expect("host's own envelope");
        if message.header.kind != Kind::AdmittedSets {
            return;
        }
        let mut contents = &message.payload[10..];
        let mut sets = 0;
        while !contents.is_empty() {
            let length = usize::from(u16::from_le_bytes(
                contents[..2].try_into().expect("framed set"),
            ));
            contents = &contents[length + 2..];
            sets += 1;
        }
        let sequence = message.header.message_sequence;
        match event {
            DatagramEvent::Lost => self.lost_sets += sets,
            DatagramEvent::Duplicate => self.duplicated_sets += sets,
            DatagramEvent::Delivered => {
                if sequence < self.latest {
                    self.reordered_sets += sets;
                }
                self.latest = self.latest.max(sequence);
            }
        }
    }

    fn json(&self) -> Value {
        json!({"lost": self.lost_sets, "reordered": self.reordered_sets, "duplicated": self.duplicated_sets})
    }
}

struct Player {
    client: Client<CrowdWorld, NoActions>,
    link: Link,
    authority: Link,
    memory: Memory,
    path: Path,
    probe: Probe,
    index: usize,
    changes: u64,
    lead_observed: u64,
    lead_reported: i64,
    lead_actual: i64,
    delay: Time,
    stalls: Time,
    skips: u64,
    off_grid_axes: u64,
    axes_changes: u64,
    previous_axes: [u32; 2],
    active: bool,
    stream: Option<PeerStream>,
    repeats: u8,
    welcomed_at: Option<u64>,
    restored_at: Option<u64>,
}

impl Player {
    fn new(
        host: &mut Host<CrowdWorld, NoActions>,
        index: usize,
        run: MemoryRun<'_>,
        profile: Conditioner,
    ) -> Result<Self, String> {
        let mut memory =
            Memory::new(Profile::default(), profile).map_err(|error| error.to_string())?;
        let [link, authority] = memory.connect().map_err(|error| error.to_string())?;
        host.connect(authority.clone(), 0)
            .map_err(|error| error.to_string())?;
        let world = memory_world(run, 100 + index as u128, false);
        let repeats = if run.costs {
            [1, 2, 4, 8][index]
        } else {
            run.repeats
        };
        let client = Client::new(
            world,
            NoActions,
            link.clone(),
            player_time(index, 0),
            PROTOCOL,
            (run.profile == "pinned").then_some(5),
        )
        .with_link_origin(player_time(index, 0))
        .with_repeat_limit(repeats);
        Ok(Self {
            client,
            link,
            authority,
            memory,
            path: Path::default(),
            probe: Probe::default(),
            index,
            changes: 0,
            lead_observed: 0,
            lead_reported: 0,
            lead_actual: 0,
            delay: Time::default(),
            stalls: Time::default(),
            skips: 0,
            off_grid_axes: 0,
            axes_changes: 0,
            previous_axes: [0; 2],
            active: true,
            stream: None,
            repeats,
            welcomed_at: None,
            restored_at: None,
        })
    }

    fn advance(&mut self, time: u64, run: MemoryRun<'_>) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }
        let now = player_time(self.index, time);
        let before = self.client.world().tick();
        let engine = self.client.world().engine.total;
        let measured = (run.costs && before >= 600 && before.is_multiple_of(16)).then(Instant::now);
        if measured.is_some() {
            allocation::begin();
        }
        let index = self.index;
        let off_grid = &mut self.off_grid_axes;
        let changes = &mut self.axes_changes;
        let previous = &mut self.previous_axes;
        let result = self.client.advance(now, |tick| {
            let axes = axes(run.seed, index, tick);
            let bits = [axes[0].to_bits(), axes[1].to_bits()];
            *changes += u64::from(bits != *previous);
            *previous = bits;
            if axes.map(f32::to_bits)
                != input_batch::canonical_axes(axes)
                    .expect("scripted normalized axes")
                    .map(f32::to_bits)
            {
                *off_grid += 1;
            }
            Controls::axes(axes)
        });
        if let Some(start) = measured {
            let counts = allocation::finish();
            if self.client.world().tick() == before + 1 {
                self.probe
                    .add(start, counts, self.client.world().engine.total - engine);
            }
        }
        result.map_err(|error| error.to_string())?;
        if self.client.settings().is_some() {
            self.welcomed_at.get_or_insert(time);
        }
        if self.client.world().restored.is_some() {
            self.restored_at.get_or_insert(time);
        }
        while self.client.result().is_some() {}
        if let Some(stall) = self.client.take_stall() {
            self.stalls.add(stall.duration_us);
        }
        if self.client.take_skip().is_some() {
            self.skips += 1;
        }
        match self.client.state() {
            ClientState::Ended(_, _) => self.active = false,
            ClientState::Refused(_) | ClientState::Closed(_) => {
                return Err(format!(
                    "player {index} at tick {before}: {:?}",
                    self.client.state()
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn sample_lead(&mut self, tick: u64, time: u64) {
        if let Some(reported) = self.client.reported_lead() {
            self.lead_observed += 1;
            self.lead_reported += reported;
            self.lead_actual += i64::try_from(self.client.last_frame().0)
                .expect("proof target fits i64")
                - i64::try_from(tick).expect("proof tick fits i64");
            if let Some(playout) = self.client.playout(player_time(self.index, time)) {
                let delay = (tick as f64 - playout.position).max(0.0);
                self.delay.add(
                    std::time::Duration::from_secs_f64(delay * 0.016_667)
                        .as_micros()
                        .try_into()
                        .expect("bounded delay"),
                );
            }
        }
    }

    fn stretch(&mut self, admitted: u64, total: u64) -> Value {
        let changes = self.client.lead_changes();
        let record = json!({
            "player": self.index,
            "slot": self.slot(),
            "own_frames": admitted,
            "ticks": total,
            "fraction": admitted as f64 / total as f64,
            "reported_lead": self.lead_reported as f64 / self.lead_observed as f64,
            "true_lead": self.lead_actual as f64 / self.lead_observed as f64,
            "changes": changes - self.changes,
        });
        self.changes = changes;
        self.lead_reported = 0;
        self.lead_actual = 0;
        self.lead_observed = 0;
        record
    }

    fn slot(&self) -> usize {
        self.client.slot().expect("negotiated player slot")
    }

    fn report(&self, now: u64, held_by_slot: &[u64; SLOTS]) -> Value {
        let stream = self.stream.as_ref();
        let histogram = stream.as_ref().map(|stream| stream.datagram_sizes);
        let restored = self.client.world().restored.expect("baseline installed");
        json!({
            "player": self.index,
            "world_epoch": self.client.world().epoch(),
            "restored": [restored.0, restored.1, restored.2 as u64],
            "slot": self.slot(),
            "off_grid_frames": self.off_grid_axes,
            "axes_changes": self.axes_changes,
            "clock_origin_us": player_time(self.index, 0),
            "clock_rate_ppm": if self.index == 2 { 1_000_100 } else { 1_000_000 },
            "to_host": bytes(&self.link, now),
            "from_host": bytes(&self.authority, now),
            "carrier_sets": self.path.json(),
            "datagram_sets": histogram.map(|hist| json!({
                "median": percentile(&hist, 50), "p99": percentile(&hist, 99),
                "max": hist.iter().rposition(|count| *count != 0).unwrap_or_default(),
            })),
            "reliable_sets": stream.as_ref().map(|stream| stream.reliable_sets),
            "repeat_depth": self.repeats,
            "ticks_held": held_by_slot[self.slot()],
            "baseline_transfer_us": self.restored_at.zip(self.welcomed_at)
                .map(|(restored, welcomed)| restored - welcomed),
            "join_us": self.restored_at,
            "late_frames": stream.as_ref().map(|stream| stream.late_frames),
            "obsolete_set_records": self.client.obsolete_sets(),
            "confirmed_delay_us": self.delay.json(),
            "stall_duration_us": self.stalls.json(),
            "skip_advances": self.skips,
            "warmed_tick": self.probe.json(),
            "digest_ticks": self.client.world().digests.borrow().len(),
        })
    }
}

fn percentile(histogram: &[u64], percentile: u64) -> usize {
    let rank = (histogram.iter().sum::<u64>() * percentile)
        .div_ceil(100)
        .max(1);
    let mut sum = 0;
    histogram
        .iter()
        .position(|count| {
            sum += count;
            sum >= rank
        })
        .unwrap_or_default()
}

fn bytes(link: &Link, now: u64) -> [Value; 2] {
    link.counters(now).lanes.map(|lane| {
        let header = lane.sent * HEADER_BYTES as u64;
        json!({"messages": lane.sent, "envelope": header, "payload": lane.sent_bytes - header})
    })
}

fn player_time(index: usize, time: u64) -> u64 {
    1_011_711 * (index as u64 + 1) + time + if index == 2 { time / 10_000 } else { 0 }
}

fn axes(seed: u64, slot: usize, tick: u64) -> [f32; 3] {
    // Hold most axes briefly so both axis encodings are represented in the cost profile.
    let value = seed ^ (tick / 11).wrapping_mul(0x9e37_79b9) ^ ((slot as u64 + 1) * 0x85eb_ca77);
    let value = value.wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    let axis = |shift| {
        let integer = u16::try_from((value >> shift) & 1023_u64).expect("ten-bit input");
        f32::from(integer) / 521.7 - 0.98
    };
    [axis(32), axis(49), 0.0]
}

fn condition(profile: &str, seed: u64) -> Result<Conditioner, String> {
    let (delay_ms, jitter_ms, loss_ppm) = match profile {
        "local" => (2, 0, 0),
        "a0" | "changing" => (50, 10, 20_000),
        "harsh" | "pinned" => (125, 50, 50_000),
        _ => return Err(format!("unknown memory profile: {profile}")),
    };
    Ok(Conditioner {
        delay_ms,
        jitter_ms,
        loss_ppm,
        seed,
        duplicate: true,
    })
}

/// Run independently restored worlds over the same portable production sessions.
///
/// # Errors
/// The match settings, carrier, synchronization, or digest comparison failed.
///
/// # Panics
/// A registered proof schema or bounded proof counter is invalid.
pub fn run_memory(run: MemoryRun<'_>) -> Result<Value, MemoryProofError> {
    execute(run).map_err(MemoryProofError)
}

/// A headless match could not establish its required endpoint and witnesses.
#[derive(Debug)]
pub struct MemoryProofError(String);

impl std::fmt::Display for MemoryProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for MemoryProofError {}

fn memory_world(run: MemoryRun<'_>, epoch: u128, authority: bool) -> CrowdWorld {
    let world = CrowdWorld::new(run.bodies, epoch, authority, run.costs);
    if run.costs {
        world
    } else {
        world.without_publication()
    }
}

fn execute(run: MemoryRun<'_>) -> Result<Value, String> {
    let settings = Settings {
        repeats: if run.costs { 8 } else { run.repeats },
        ..Settings::default()
    };
    let world = memory_world(run, 37, true);
    let baseline_digest = world.digest_value();
    let mut host = Host::new(
        world, NoActions, settings, [13; 16], run.seed, run.listen, 0,
    )
    .map_err(|error| error.to_string())?;
    let first = usize::from(run.listen);
    let mut players = (first..SLOTS)
        .map(|index| {
            Player::new(
                &mut host,
                index,
                run,
                condition(run.profile, run.seed + index as u64)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Each replica owns its world. Dispatch the 2,000-body deterministic
    // fixture independently too so the full five-world check fits the fast tier.
    let pool = (run.ticks >= 3_000 || run.bodies >= 2_000)
        .then(|| rayon::ThreadPoolBuilder::new().num_threads(4).build())
        .transpose()
        .map_err(|error| error.to_string())?;
    let mut host_probe = HostProbe::default();
    let mut stretch_start = 0;
    let mut stretch = Vec::new();
    let mut stretch_by_slot = Vec::new();
    let mut admissions = [0; SLOTS];
    let mut held_by_slot = [0; SLOTS];
    let mut local_admitted = 0;
    let mut state = Timing::default();
    let mut time = 0;
    let limit = (run.ticks + 4_000) * u64::from(settings.tick_us);
    while time < limit {
        let before = host.world().tick();
        for player in &mut players {
            player
                .memory
                .advance_observed(time / 1_000, |direction, event, bytes| {
                    player.path.observe(direction, event, bytes);
                });
        }
        if !state.started && (first..SLOTS).all(|slot| host.ready(slot)) {
            state.started = true;
            host.start(time);
        }
        if run.listen && state.started && before < run.ticks {
            let tick = before + 1;
            host.stage_local(tick, axes(run.seed, 0, tick), [])
                .map_err(|error| error.to_string())?;
        }
        host_probe.advance(&mut host, time, run)?;
        let tick = host.world().tick();
        if tick > before {
            local_admitted += u64::from(run.listen && host.admissions()[0]);
            for (slot, own) in host.admissions().into_iter().enumerate() {
                held_by_slot[slot] += u64::from(!own);
                if tick > stretch_start + 600 {
                    admissions[slot] += u64::from(own);
                }
            }
            for player in &mut players {
                if tick > stretch_start + 600 {
                    player.sample_lead(tick, time);
                }
            }
            if tick == run.ticks
                || run.profile == "changing" && (tick == run.ticks / 3 || tick == run.ticks * 2 / 3)
            {
                let span = tick.saturating_sub(stretch_start + 600);
                if span > 0 {
                    stretch_by_slot.push(admissions);
                    stretch.push(json!(
                        players
                            .iter_mut()
                            .map(|player| player.stretch(admissions[player.slot()], span))
                            .collect::<Vec<_>>()
                    ));
                }
                if tick != run.ticks {
                    let next = if stretch_start == 0 { "harsh" } else { "a0" };
                    for player in &mut players {
                        player
                            .memory
                            .recondition(condition(next, run.seed)?)
                            .map_err(|error| error.to_string())?;
                    }
                }
                if tick == run.ticks {
                    for player in &mut players {
                        player.stream = host.stream(player.authority.id());
                    }
                }
                stretch_start = tick;
                admissions = [0; SLOTS];
            }
        }

        if run.stall
            && state.stalled_at.is_none()
            && tick >= run.ticks / 2
            && state.stall_us.is_none()
        {
            for player in &mut players {
                player.memory.stall(true);
            }
            state.stalled_at = Some(time);
        }
        if let Some(start) = state.stalled_at
            && time >= start + 2_000_000
        {
            for player in &mut players {
                player.memory.stall(false);
            }
            state.stalled_at = None;
            state.stall_us = Some([start, time]);
        }

        if let Some(pool) = &pool {
            pool.install(|| {
                players
                    .par_iter_mut()
                    .try_for_each(|player| player.advance(time, run))
            })?;
        } else {
            players
                .iter_mut()
                .try_for_each(|player| player.advance(time, run))?;
        }
        if tick == run.ticks
            && players
                .iter()
                .all(|player| player.client.world().tick() == tick)
        {
            break;
        }
        // Once the endpoint is reached no fresh set datagram would repair a final loss.
        if tick == run.ticks {
            host.stop();
        }
        time += u64::from(settings.tick_us) / 2;
    }

    let checkpoint = host.world().digests.borrow();
    let expected_checkpoints = run.ticks / u64::from(settings.digest_interval);
    if host.world().tick() != run.ticks || checkpoint.len() as u64 != expected_checkpoints {
        return Err(format!(
            "authority reached tick {} and made {} digest checkpoints",
            host.world().tick(),
            checkpoint.len()
        ));
    }
    let final_digest = host.world().digest_value();
    for player in &players {
        let world = player.client.world();
        let actual = world.digests.borrow();
        if world.tick() != run.ticks
            || world.digest_value() != final_digest
            || !actual.iter().eq(checkpoint.iter())
            || world.restored.is_none_or(|(tick, digest, bytes)| {
                tick != 0 || digest != baseline_digest || bytes == 0
            })
        {
            return Err(format!(
                "player {} reached tick {}, {} digest checkpoints, baseline {:?}",
                player.index,
                world.tick(),
                actual.len(),
                world.restored
            ));
        }
    }
    let peers = players
        .iter()
        .map(|player| player.report(time / 1_000, &held_by_slot))
        .collect::<Vec<_>>();
    Ok(json!({
        "profile": run.profile, "seed": run.seed, "bodies": run.bodies, "ticks": run.ticks,
        "repeats": settings.repeats, "stall_us": state.stall_us,
        "rule_set": crate::identity(false),
        "final_digest": final_digest, "baseline_digest": baseline_digest, "checkpoints": *checkpoint,
        "listen_host": run.listen, "host_epoch": host.world().epoch(),
        "host_local_admitted": local_admitted,
        "stretch": stretch, "admitted_by_slot": stretch_by_slot,
        "peers": peers, "held_by_slot": held_by_slot,
        "host_tick": host_probe.warmed.json(),
        "host_barrier_to_send_ns": host.world().barrier_send.json(),
        "host_image_write_ns": host.world().image_time.borrow().json(),
        "host_image_tick": host_probe.image_tick,
        "image_bytes": host.world().image_bytes.get(),
    }))
}

#[derive(Default)]
struct Timing {
    started: bool,
    stalled_at: Option<u64>,
    stall_us: Option<[u64; 2]>,
}
