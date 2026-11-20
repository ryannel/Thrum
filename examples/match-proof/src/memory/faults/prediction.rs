use std::{cell::Cell, collections::BTreeSet, rc::Rc, time::Instant};

use crowd_2d::PlayerController;
use serde_json::{Value, json};
use thrum_network::{
    envelope::{self, Kind},
    input_batch,
    session::{
        ClientState, Correction, CorrectionCause, PREDICTION_TICKS, PredictionEvent, Settings,
        World,
    },
};
use thrum_transport::{CloseReason, Lane, Profile, memory::DatagramEvent};

use super::{
    super::{
        condition,
        world::{
            Time,
            fault_world::{Fault, Witness, controller_bits},
        },
    },
    driver::{Match, Replica, nanos},
};

pub(super) struct Trace {
    pub(super) enabled: bool,
    pub(super) timing: Option<Rc<Cell<Time>>>,
    pub(super) events: Vec<PredictionEvent<PlayerController>>,
    witness: WitnessKind,
    datagrams: Vec<(u64, DatagramEvent)>,
    transitions: Vec<Value>,
    pages: Vec<Option<(u64, u64)>>,
    live: bool,
    lead: Time,
    advance: Time,
}

enum WitnessKind {
    LinkLoss,
    Isolation,
    Match,
}

impl Trace {
    fn new(enabled: bool, timing: bool, witness: WitnessKind) -> Self {
        Self {
            enabled,
            timing: timing.then(|| Rc::new(Cell::new(Time::default()))),
            events: Vec::new(),
            witness,
            datagrams: Vec::new(),
            transitions: Vec::new(),
            pages: Vec::new(),
            live: false,
            lead: Time::default(),
            advance: Time::default(),
        }
    }

    pub(super) fn datagram(&mut self, event: DatagramEvent, bytes: &[u8]) {
        if !matches!(self.witness, WitnessKind::LinkLoss) {
            return;
        }
        let packet = envelope::decode(bytes).expect("conditioned session envelope");
        if packet.header.kind == Kind::InputBatch && packet.payload.len() > 16 {
            // Two u64 progress fields precede the input batch in a report.
            let batch = input_batch::decode(&packet.payload[16..]).expect("input batch");
            self.datagrams
                .extend(batch.records().map(|record| (record.target_tick, event)));
        }
    }

    pub(super) fn sample(
        &mut self,
        client: &Replica,
        world: &Witness,
        host_tick: u64,
        now: u64,
        timing: Option<Instant>,
    ) {
        if let Some(timing) = timing {
            self.advance.add(nanos(timing));
        }
        if matches!(self.witness, WitnessKind::Isolation) {
            self.pages.push(world.page());
        }
        let Some(view) = client.controller(now) else {
            assert!(!self.live);
            return;
        };
        let confirmed = world.crowd.tick();
        let target = client.last_frame().0;
        let span = target.saturating_sub(confirmed);
        if view.predicted {
            assert_eq!(view.tick, target);
            assert!(span <= PREDICTION_TICKS as u64);
            self.lead.add(span);
        } else if view.tick == confirmed {
            assert_eq!(
                controller_bits(view.current),
                controller_bits(
                    world
                        .controller(client.slot().expect("assigned slot"))
                        .expect("confirmed controller")
                )
            );
        }
        if view.predicted != self.live {
            self.transitions
                .push(json!({"host_tick": host_tick, "confirmed": confirmed,
                "authored": target, "span": span, "predicted": view.predicted,
                "view_tick": view.tick, "state": format!("{:?}", client.state())}));
            self.live = view.predicted;
        }
    }

    fn revisions(&self) -> impl Iterator<Item = &Correction<PlayerController>> {
        self.events.iter().filter_map(|event| match event {
            PredictionEvent::Corrected(correction) => Some(correction),
            PredictionEvent::Compared { .. } => None,
        })
    }

    fn audit(&self, host: &[[bool; 4]], slot: usize) -> (usize, usize) {
        let mut compared = 0;
        let mut admitted_comparisons = 0;
        for event in &self.events {
            if let PredictionEvent::Compared {
                tick,
                admitted,
                equal,
            } = *event
            {
                let host_admitted = admitted_at(host, tick, slot);
                assert_eq!(
                    admitted, host_admitted,
                    "authority and replica admission at {tick}"
                );
                if host_admitted {
                    assert!(equal, "admitted frame disagreed at {tick}");
                    admitted_comparisons += 1;
                }
                compared += 1;
            }
        }
        for correction in self.revisions() {
            assert!(!admitted_at(host, correction.tick, slot));
            assert!(matches!(
                correction.cause,
                CorrectionCause::HeldFrame | CorrectionCause::AuthoringGap
            ));
            assert_eq!(
                correction.horizon - correction.tick,
                correction.replayed as u64
            );
        }
        (compared, admitted_comparisons)
    }

    fn fallback_admissions(&self, world: &Witness, host: &[[bool; 4]], slot: usize) -> Vec<u64> {
        let compared: BTreeSet<_> = self
            .events
            .iter()
            .filter_map(|event| match event {
                PredictionEvent::Compared { tick, .. } => Some(*tick),
                PredictionEvent::Corrected(_) => None,
            })
            .collect();
        let mut intervals = Vec::new();
        let mut dropped_at = Some(0);
        for transition in &self.transitions {
            let tick = transition["confirmed"]
                .as_u64()
                .expect("confirmed tick at transition");
            if transition["predicted"] == true {
                intervals.push(dropped_at.take().expect("prediction was absent")..=tick);
            } else {
                dropped_at = Some(tick);
            }
        }
        if let Some(tick) = dropped_at {
            intervals.push(tick..=u64::MAX);
        }
        world
            .steps
            .iter()
            .filter_map(|row| {
                let tick = row.tick;
                if !admitted_at(host, tick, slot) || compared.contains(&tick) {
                    return None;
                }
                assert!(
                    intervals.iter().any(|range| range.contains(&tick)),
                    "admitted tick {tick} was missed while prediction was live"
                );
                Some(tick)
            })
            .collect()
    }
}

fn admitted_at(host: &[[bool; 4]], tick: u64, slot: usize) -> bool {
    host[usize::try_from(tick - 1).expect("proof tick fits an index")][slot]
}

fn held() -> Value {
    let mut game = Match::new(64, Settings::default(), false);
    game.admitted = Some(Vec::new());
    let clean = condition("local", 29_721).expect("valid proof conditioner");
    let peer = game.add_conditioned(
        Profile::default(),
        None,
        clean,
        Some(5),
        Some(Trace::new(true, false, WitnessKind::LinkLoss)),
    );
    game.advance_to(90);
    assert!(game.joined(peer));
    let lost_tick = game.players[peer].client.last_frame().0;
    game.players[peer]
        .carrier
        .recondition(thrum_transport::memory::Conditioner {
            loss_ppm: 1_000_000,
            ..clean
        })
        .expect("valid conditioned prediction fixture");
    // Both reports carrying the frame leave the sender while the carrier is losing datagrams.
    game.advance_to(92);
    game.players[peer]
        .carrier
        .recondition(clean)
        .expect("valid proof conditioner");
    game.advance_to(150);
    let player = &game.players[peer];
    let trace = player
        .prediction
        .as_ref()
        .expect("prediction witness attached");
    let admissions = game
        .admitted
        .as_ref()
        .expect("authority admissions recorded");
    let (compared, admitted) = trace.audit(admissions, player.slot());
    assert!(compared > 100 && admitted > 100);
    assert!(!admitted_at(
        game.admitted.as_ref().expect("authority admissions"),
        lost_tick,
        player.slot()
    ));
    let copies: Vec<_> = trace
        .datagrams
        .iter()
        .filter(|(tick, _)| *tick == lost_tick)
        .collect();
    assert_eq!(copies.len(), 2);
    assert!(
        copies
            .iter()
            .all(|(_, event)| *event == DatagramEvent::Lost)
    );
    let fixes: Vec<_> = trace
        .revisions()
        .filter(|row| row.tick >= lost_tick - 1)
        .collect();
    let [correction] = fixes.as_slice() else {
        panic!("expected one correction: {fixes:?}")
    };
    assert_eq!(correction.tick, lost_tick);
    assert_eq!(correction.cause, CorrectionCause::HeldFrame);
    assert!(correction.replayed > 0);
    assert_ne!(
        controller_bits(correction.before),
        controller_bits(correction.after)
    );
    assert!(
        trace.events.iter().any(|event| matches!(event,
        PredictionEvent::Compared { tick, admitted: true, equal: true } if *tick == lost_tick + 1))
    );
    json!({"held_tick": lost_tick, "lost_copies": copies.len(), "delivered_copies": 0,
        "compared": compared, "admitted_equal": admitted, "correction": correction_row(correction)})
}

fn isolated(enabled: bool) -> Value {
    let mut game = Match::new(
        128,
        Settings {
            digest_interval: 1,
            ..Settings::default()
        },
        false,
    );
    for slot in 0..4 {
        game.add_conditioned(
            Profile::default(),
            None,
            condition("a0", 29_721 + slot).expect("valid proof conditioner"),
            None,
            Some(Trace::new(
                enabled && slot == 0,
                false,
                WitnessKind::Isolation,
            )),
        );
    }
    game.advance_to(240);
    game.finish();
    let authority = game.world.borrow();
    let rows: Vec<_> = game
        .players
        .iter()
        .map(|player| {
            let world = player.world.borrow();
            let trace = player
                .prediction
                .as_ref()
                .expect("prediction witness attached");
            assert!(trace.pages.iter().flatten().count() > 200);
            assert_eq!(trace.events.is_empty(), !trace.enabled);
            json!({"digests": *world.crowd.digests.borrow(), "pages": trace.pages})
        })
        .collect();
    json!({"authority": *authority.crowd.digests.borrow(), "replicas": rows})
}

fn correction_row(correction: &Correction<PlayerController>) -> Value {
    let before = controller_bits(correction.before);
    let after = controller_bits(correction.after);
    let axis =
        |i: usize| f64::from(f32::from_bits(before[i])) - f64::from(f32::from_bits(after[i]));
    json!({"tick": correction.tick, "horizon": correction.horizon, "replayed": correction.replayed,
        "cause": format!("{:?}", correction.cause), "before_bits": before, "after_bits": after,
        "distance": axis(0).hypot(axis(1))})
}

fn summary(game: &Match, peer: usize) -> Value {
    let player = &game.players[peer];
    let trace = player
        .prediction
        .as_ref()
        .expect("prediction witness attached");
    let admissions = game
        .admitted
        .as_ref()
        .expect("authority admissions recorded");
    let (compared, admitted) = trace.audit(admissions, player.slot());
    let world = player.world.borrow();
    let fallback = trace.fallback_admissions(&world, admissions, player.slot());
    let lead = trace.lead.json();
    let period = f64::from(game.settings.tick_us) / 1_000.0;
    let engine = world.crowd.engine.json();
    json!({"slot": player.slot(), "epoch": player.client.epoch(), "compared": compared,
        "admitted_equal": admitted, "admitted_with_confirmed_fallback": fallback,
        "transitions": trace.transitions,
        "lead_ticks": lead, "lead_ms": {"mean": trace.lead.total as f64 / trace.lead.count as f64 * period,
            "max": trace.lead.max as f64 * period},
        "rule_tick_ns": trace.timing.as_ref().map(|time| time.get().json()),
        "session_advance_ns": trace.advance.json(), "engine_tick_ns": engine,
        "corrections": trace.revisions().map(correction_row).collect::<Vec<_>>(),
        "baselines": world.restores})
}

fn reconnect(
    game: &mut Match,
    lost: usize,
    profile: &str,
    seed: u64,
    costs: bool,
) -> (usize, Value) {
    let original = &game.players[lost];
    let slot = original.slot();
    let old_epoch = original
        .client
        .epoch()
        .expect("negotiated connection epoch");
    let token = original
        .client
        .token()
        .expect("negotiated connection epoch");
    let stale_input = original.input.clone();
    let stale_set = original.sets.clone();
    game.players[lost].carrier.cut(true);
    game.until(300, |game| {
        game.players[lost].client.state() == &ClientState::Closed(CloseReason::TimedOut)
    });
    let returned = game.add_conditioned(
        Profile::default(),
        Some(token),
        condition(profile, seed).expect("valid proof conditioner"),
        None,
        Some(Trace::new(true, costs, WitnessKind::Match)),
    );
    assert!(
        game.players[returned].client.controller(game.now).is_none(),
        "no old epoch state before baseline"
    );
    game.until(300, |game| game.joined(returned));
    let new = &mut game.players[returned];
    let new_epoch = new.client.epoch().expect("negotiated connection epoch");
    assert_eq!(new.slot(), slot);
    assert_ne!(old_epoch, new_epoch);
    let now = game.now / 1_000;
    let stale_before = new.client.stale_epoch();
    let host_link = new.host_link();
    let host_stale_before = game
        .host
        .stream(host_link)
        .expect("rejoined stream")
        .stale_epoch;
    for packet in [&stale_input, &stale_set] {
        assert_eq!(
            envelope::decode(packet)
                .expect("old session packet")
                .header
                .connection_epoch,
            old_epoch
        );
    }
    let conditioned = condition(profile, seed).expect("valid proof conditioner");
    new.carrier
        .recondition(thrum_transport::memory::Conditioner {
            loss_ppm: 0,
            ..conditioned
        })
        .expect("delivery control for both old epoch packets");
    new.links[0]
        .send(Lane::Datagram, &stale_input, now)
        .expect("open link accepts the stale datagram");
    new.links[1]
        .send(Lane::Datagram, &stale_set, now)
        .expect("open link accepts the stale datagram");
    new.carrier.advance(now);
    new.carrier
        .recondition(conditioned)
        .expect("restore the original loss profile");
    game.advance_to(game.tick() + 60);
    let new = &game.players[returned];
    let stale_seen = new.client.stale_epoch() - stale_before;
    let host_stale = game
        .host
        .stream(host_link)
        .expect("rejoined stream")
        .stale_epoch
        - host_stale_before;
    assert!(
        stale_seen > 0 && host_stale > 0,
        "both directions rejected old epoch bytes: client {stale_seen}, host {host_stale}"
    );
    assert!(
        new.client
            .controller(game.now)
            .expect("controller endpoint available")
            .predicted
    );
    (
        returned,
        json!({"old_epoch": old_epoch, "new_epoch": new_epoch, "slot": slot,
        "old_set_injected": true, "old_input_injected": true, "client_stale_sets": stale_seen,
        "host_stale_inputs": host_stale,
        "new_baseline": new.world.borrow().restores[0].0}),
    )
}

fn long(profile: &str, ticks: u64, count: u32, seed: u64, costs: bool) -> Value {
    let mut game = Match::new(count, Settings::default(), costs);
    game.admitted = Some(Vec::new());
    for slot in 0..4 {
        let trace = (slot == 0).then(|| Trace::new(true, costs, WitnessKind::Match));
        game.add_conditioned(
            Profile::default(),
            None,
            condition(profile, seed + slot).expect("valid proof conditioner"),
            None,
            trace,
        );
    }
    let hero = 0;
    // The full check spreads the events across the run. The cost run uses the same sequence.
    let change_at = ticks / 6;
    game.advance_to(change_at);
    let changed = if profile == "a0" { "harsh" } else { "a0" };
    game.players[hero]
        .carrier
        .recondition(condition(changed, seed).expect("valid proof conditioner"))
        .expect("valid proof conditioner");
    let restored_profile_at = change_at + 180;
    game.advance_to(restored_profile_at);
    game.players[hero]
        .carrier
        .recondition(condition(profile, seed).expect("valid proof conditioner"))
        .expect("valid proof conditioner");
    let stall_at = ticks / 3;
    game.advance_to(stall_at);
    game.players[hero].carrier.stall(true);
    let release_at = stall_at + 100;
    game.advance_to(release_at);
    {
        let player = &game.players[hero];
        let view = player
            .client
            .controller(game.now)
            .expect("controller endpoint available");
        assert!(!view.predicted);
        assert_eq!(view.tick, player.client.world().tick());
        assert!(player.client.last_frame().0 - view.tick > PREDICTION_TICKS as u64);
    }
    game.players[hero].carrier.stall(false);
    game.until(180, |game| {
        game.players[hero]
            .client
            .controller(game.now)
            .is_some_and(|view| view.predicted)
    });
    let resumed_at = game.tick();
    let repair_at = ticks / 2;
    game.advance_to(repair_at);
    game.players[hero]
        .world
        .borrow_mut()
        .change(Fault::Resource)
        .expect("valid conditioned prediction fixture");
    game.until(180, |game| {
        game.players[hero].client.state() == &ClientState::Repairing
    });
    assert!(
        !game.players[hero]
            .client
            .controller(game.now)
            .expect("controller endpoint available")
            .predicted
    );
    game.until(180, |game| {
        game.players[hero].world.borrow().restores.len() == 2
            && game.players[hero]
                .client
                .controller(game.now)
                .is_some_and(|view| view.predicted)
    });
    let repair_resumed = game.tick();
    let reconnect_at = ticks * 2 / 3;
    game.advance_to(reconnect_at);
    let (returned, rejoin) = reconnect(&mut game, hero, profile, seed, costs);
    game.advance_to(ticks);
    game.finish();
    assert_eq!(
        game.diagnostics.len(),
        1,
        "only the deliberately perturbed resource diverged"
    );
    let checked: Vec<_> = (0..game.players.len())
        .map(|peer| game.compare_played(peer, (peer == hero).then_some(game.diagnostics[0].tick)))
        .collect();
    let branches = [summary(&game, hero), summary(&game, returned)];
    let admitted: u64 = branches
        .iter()
        .map(|row| {
            row["admitted_equal"]
                .as_u64()
                .expect("admitted comparison count")
        })
        .sum();
    assert!(
        admitted > 0,
        "the two controller branches compared admitted ticks"
    );
    json!({"profile": profile, "ticks": ticks, "bodies": count, "seed": seed,
        "schedule": game.scheduled(), "rule_set": crate::identity(false), "branches": branches,
        "compared_digests_by_replica": checked,
        "link_change": {"tick": change_at, "to": changed, "restored_at": restored_profile_at},
        "stall": {"from": stall_at, "through": release_at, "resumed": resumed_at},
        "repair": {"injected": repair_at, "detected": game.diagnostics[0].tick, "resumed": repair_resumed},
        "reconnect": rejoin})
}

/// Run a headless controller witness through the production session and rule.
///
/// # Errors
/// The selected scenario or conditioner profile is unknown.
///
/// # Panics
/// The fixture failed an authority, prediction or lifecycle assertion.
pub fn run_prediction(
    scenario: &str,
    profile: &str,
    ticks: u64,
    bodies: u32,
    seed: u64,
) -> Result<Value, &'static str> {
    match scenario {
        "held" => Ok(held()),
        "isolated" => {
            let first = isolated(false);
            assert_eq!(
                first,
                isolated(true),
                "prediction changed authority or publication"
            );
            Ok(first)
        }
        "long" | "costs" if matches!(profile, "a0" | "harsh") && ticks >= 1_800 => {
            Ok(long(profile, ticks, bodies, seed, scenario == "costs"))
        }
        _ => Err("unknown prediction scenario or incomplete run configuration"),
    }
}
