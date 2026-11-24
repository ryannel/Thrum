//! One real socket session peer, shared by the native and Wasm proof entry points.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
mod scheduling;

use crowd_2d::PlayerMovement;
use serde_json::{Value, json};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thrum_network::session::{Client, ClientState, Controls, PlayerToken, PredictionEvent, World};
use thrum_transport::{Link, Profile, Ticket};
#[cfg(target_arch = "wasm32")]
use web_time::{Instant, SystemTime, UNIX_EPOCH};

use crate::world::{
    CrowdWorld,
    fault_world::{CrowdController, Fault, FaultWire, PlayerRule, Shared, controller_bits},
};

pub(super) type ProofResult<T> = Result<T, Box<dyn std::error::Error>>;

fn micros(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).expect("proof duration in range")
}

fn unix_us() -> u64 {
    micros(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wall clock"),
    )
}

fn controls(tick: u64) -> [f32; 3] {
    let direction = if (tick / 180).is_multiple_of(2) {
        0.75
    } else {
        -0.75
    };
    [direction, 0.375, 0.0]
}

fn timestamp(mut row: Value) -> Value {
    row["wall_us"] = json!(unix_us());
    row
}

fn counters(link: &Link, now: u64) -> Value {
    let counts = link.counters(now / 1_000);
    json!({
        "application_sent": counts.lanes.iter().map(|lane| lane.sent_bytes).sum::<u64>(),
        "application_received": counts.lanes.iter().map(|lane| lane.received_bytes).sum::<u64>(),
        "outbound": counts.lanes.map(|lane| lane.queued_bytes),
        "expired_datagrams": counts.expired,
        "datagrams_dropped_at_cap": counts.dropped_cap,
        "wire_sent": counts.wire_bytes,
    })
}

type Replica = Client<Shared, FaultWire, CrowdController>;

struct Player {
    client: Replica,
    link: Link,
    clock: Instant,
    prior: ClientState,
    restore_count: usize,
    hello: Option<u64>,
    welcomed: Option<u64>,
    restored: Option<u64>,
    playing: Option<u64>,
    token_in: Option<PlayerToken>,
    token_out: Option<PlayerToken>,
    next_sample: u64,
    corrections: u64,
    axes: [[f32; 3]; thrum_network::session::SLOTS],
    record_axes: bool,
    trace_authored_frames: bool,
}

impl Player {
    fn new(
        ticket: &Ticket,
        profile: Profile,
        bodies: u32,
        token: Option<PlayerToken>,
        record_axes: bool,
        trace_authored_frames: bool,
    ) -> ProofResult<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        let link = thrum_transport::native::connect(ticket, profile)?;
        #[cfg(target_arch = "wasm32")]
        let link = thrum_transport::browser::connect(ticket, profile)?;
        let world = Shared::new(CrowdWorld::new(bodies, 2, false, false).without_publication());
        let mut client = Replica::with_controller(
            world,
            FaultWire,
            link.clone(),
            0,
            None,
            Some(PlayerRule {
                movement: PlayerMovement::default(),
                timing: None,
            }),
        );
        if let Some(token) = token {
            client = client.with_token(token);
        }
        Ok(Self {
            client,
            link,
            clock: Instant::now(),
            prior: ClientState::Connecting,
            restore_count: 0,
            hello: None,
            welcomed: None,
            restored: None,
            playing: None,
            token_in: token,
            token_out: None,
            next_sample: 0,
            corrections: 0,
            axes: [[0.0; 3]; thrum_network::session::SLOTS],
            record_axes,
            trace_authored_frames,
        })
    }

    fn perturb(&self) -> ProofResult<Value> {
        self.client.world().0.borrow_mut().change(Fault::Resource)?;
        Ok(timestamp(
            json!({"event": "perturbed", "tick": self.client.world().tick(),
                            "epoch": self.client.epoch(), "link": self.link.id()}),
        ))
    }

    fn advance(&mut self) -> ProofResult<Vec<Value>> {
        let now = micros(self.clock.elapsed());
        let mut rows = Vec::new();
        // Re-poll native input between individual world steps so replica
        // catch-up cannot keep the latest controls behind a long compute batch.
        // The page amortizes its browser task dispatch across a short batch.
        let step_budget = if cfg!(target_arch = "wasm32") { 4 } else { 1 };
        let trace_authored_frames = self.trace_authored_frames;
        self.client.advance_with_prediction_events(
            now,
            step_budget,
            |tick| {
                if trace_authored_frames {
                    rows.push(timestamp(json!({"event": "authored_frame", "tick": tick})));
                }
                Controls::axes(controls(tick))
            },
            |event| {
                if matches!(event, PredictionEvent::Corrected(_)) {
                    self.corrections += 1;
                }
            },
        )?;
        while self.client.result().is_some() {}
        if self.client.world().0.borrow().restores.len() > self.restore_count {
            self.restored.get_or_insert(now);
        }
        let state = self.client.state();
        if *state != self.prior {
            if *state == ClientState::Negotiating {
                self.hello = Some(now);
            }
            if *state == ClientState::Synchronizing {
                self.welcomed.get_or_insert(now);
            }
            if *state == ClientState::Playing {
                self.playing.get_or_insert(now);
            }
            rows.push(timestamp(json!({
                "event": "state", "state": format!("{state:?}"), "tick": self.client.world().tick(),
                "epoch": self.client.epoch(), "slot": self.client.slot(), "link": self.link.id(),
                "used_saved_token": self.token_in.is_some(),
                "same_token": self.token_in.zip(self.client.token()).map(|(a,b)| a == b),
                "hello_to_playing_us": self.playing.zip(self.hello).map(|(p,h)| p-h),
                "catch_up_us": self.playing.zip(self.restored).map(|(p,r)| p-r),
                "corrections": self.corrections,
                "counters": counters(&self.link, now),
            })));
            self.prior = state.clone();
        }
        self.token_out = self.client.token().or(self.token_out);
        let mut world = self.client.world().0.borrow_mut();
        for &(tick, digest, bytes) in &world.restores[self.restore_count..] {
            self.restored.get_or_insert(now);
            rows.push(timestamp(json!({
                "event": "baseline", "tick": tick, "digest": format!("{digest:016x}"),
                "bytes": bytes, "hello_to_restore_us": self.hello.map(|h| now-h),
                "welcome_to_restore_us": self.welcomed.map(|w| now-w),
                "link": self.link.id(), "epoch": self.client.epoch(),
            })));
        }
        self.restore_count = world.restores.len();
        for &(tick, digest) in world.crowd.digests.borrow().iter() {
            rows.push(timestamp(
                json!({"event": "digest", "tick": tick, "digest": format!("{digest:016x}"),
                                       "link": self.link.id()}),
            ));
        }
        world.crowd.digests.borrow_mut().clear();
        for step in world.steps.drain(..) {
            if self.record_axes
                && (self.axes != step.axes
                    || step.roster.joined | step.roster.away | step.roster.back | step.roster.left
                        != 0)
            {
                rows.push(timestamp(json!({"event": "axes", "tick": step.tick, "axes": step.axes,
                    "roster": [step.roster.joined, step.roster.away, step.roster.back, step.roster.left]})));
            }
            self.axes = step.axes;
        }
        drop(world);
        if let Some(stall) = self.client.take_stall() {
            rows.push(timestamp(
                json!({"event": "stall", "position": stall.position,
                "duration_us": stall.duration_us, "work_cap": stall.work_cap}),
            ));
        }
        if now >= self.next_sample {
            self.next_sample = now + 250_000;
            rows.push(timestamp(json!({
                "event": "sample", "tick": self.client.world().tick(),
                "playout": self.client.playout(now).map(|view| view.position),
                "prediction": self.client.controller(now).map(|view| json!({
                    "tick": view.tick, "predicted": view.predicted, "state": controller_bits(view.current),
                })),
                "lead": self.client.reported_lead(), "corrections": self.corrections,
                "link": self.link.id(), "counters": counters(&self.link, now),
            })));
        }
        Ok(rows)
    }
}

fn token_from_text(text: &str) -> ProofResult<PlayerToken> {
    let mut bytes = [0; 16];
    if text.len() != 32 || !text.is_ascii() {
        return Err("invalid persisted token".into());
    }
    for (index, value) in bytes.iter_mut().enumerate() {
        *value = u8::from_str_radix(&text[2 * index..2 * index + 2], 16)?;
    }
    PlayerToken::from_bytes(bytes).ok_or_else(|| "invalid empty token".into())
}
