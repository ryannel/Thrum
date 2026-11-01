//! Real monotonic headless host callback fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_host_cost`

use std::{
    thread,
    time::{Duration, Instant},
};

use thrum_core::{
    ActorId, Authority, AuthorityId, ClockConfig, ClockState, GameRules, Handle, HeadlessHost,
    HostBudget, HostPublication, Input, InputSchedule, MatchId, Origin, PeerId, PreparedCommit,
    PreparedEdits, StableId, World, WorldEdit, WorldEpoch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter(u64);

struct Rule {
    delay: Duration,
}

impl GameRules for Rule {
    type Entity = Counter;
    type Action = StableId;
    type Change = Handle;
    type Rejection = ();
    type Event = ();

    fn decide(&self, world: &World<Counter>, _: ActorId, id: &StableId) -> Result<Handle, ()> {
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }
        world.handle_for(*id).ok_or(())
    }

    fn prepare(&self, world: &World<Counter>, handle: Handle) -> PreparedCommit<Counter, ()> {
        let mut state = world.get(handle).expect("validated object").state.clone();
        state.0 += 1;
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(handle, state)),
            event: None,
        }
    }
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn report(label: &str, samples: &mut [u128]) {
    let p50 = percentile(samples, 50);
    let p95 = percentile(samples, 95);
    let p99 = percentile(samples, 99);
    println!(
        "{label}: count={} us[p50/p95/p99/max]={p50}/{p95}/{p99}/{}",
        samples.len(),
        samples[samples.len() - 1]
    );
}

fn authority(delay: Duration, match_id: u128) -> Authority<Rule> {
    let mut world = World::new(WorldEpoch(match_id + 1));
    world.spawn(StableId(1), Counter(0)).unwrap();
    world.spawn(StableId(2), Counter(0)).unwrap();
    let mut authority =
        Authority::new(AuthorityId::Match(MatchId(match_id)), Rule { delay }, world);
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    authority.register_peer(PeerId(2), 7, ActorId(2)).unwrap();
    authority
}

fn submit(
    host: &mut HeadlessHost<Rule>,
    peer: PeerId,
    actor: ActorId,
    epoch: u64,
    id: StableId,
    sequence: u64,
) {
    host.submit(Input {
        origin: Origin {
            peer,
            connection_epoch: epoch,
            sequence,
        },
        actor,
        schedule: InputSchedule::NextTick {
            observed_tick: host.authority().tick_index(),
        },
        action: id,
    })
    .unwrap();
}

fn consume(
    host: &mut HeadlessHost<Rule>,
    publications: Vec<HostPublication<Counter, (), ()>>,
    submitted_at: &[Vec<Instant>; 2],
    result_latency: &mut Vec<u128>,
) {
    for publication in publications {
        let HostPublication::Replicated { admitted, .. } = publication else {
            panic!("match host returned local-only publication")
        };
        for result in &admitted.results {
            let peer_index = usize::from(result.origin.peer == PeerId(2));
            let sent = submitted_at[peer_index][(result.origin.sequence - 1) as usize];
            result_latency.push(sent.elapsed().as_micros());
        }
        for (peer, through) in admitted.resolved_through {
            host.acknowledge_results(peer, through).unwrap();
        }
    }
}

fn normal_run() {
    const CALLBACKS: u64 = 480;
    let origin = Instant::now();
    let (mut host, initial) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(2),
            other_work: Duration::from_micros(500),
            tick_cost_bound: Duration::from_micros(200),
        },
        authority(Duration::ZERO, 10),
    )
    .unwrap();
    assert!(initial.results.is_empty());
    let mut callback_elapsed = Vec::with_capacity(CALLBACKS as usize);
    let mut service_elapsed = Vec::with_capacity(CALLBACKS as usize);
    let mut callback_gap = Vec::with_capacity(CALLBACKS as usize);
    let mut result_latency = Vec::with_capacity((CALLBACKS * 2) as usize);
    let mut submitted_at = [
        Vec::with_capacity(CALLBACKS as usize),
        Vec::with_capacity(CALLBACKS as usize),
    ];
    let mut prior = origin;
    let mut completed = 0;
    let mut overrun = 0;
    for index in 1..=CALLBACKS {
        let deadline = origin + Duration::from_nanos(index * 1_000_000_000 / 120);
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
        let full_path_start = Instant::now();
        submitted_at[0].push(Instant::now());
        submit(&mut host, PeerId(1), ActorId(1), 1, StableId(1), index);
        submitted_at[1].push(Instant::now());
        submit(&mut host, PeerId(2), ActorId(2), 7, StableId(2), index);
        let now = Instant::now();
        let report = host.on_callback(now).unwrap();
        callback_gap.push((now - prior).as_micros());
        prior = now;
        callback_elapsed.push(report.observed_elapsed.as_micros());
        completed += u64::from(report.clock.completed);
        overrun += u64::from(report.step_budget_overrun);
        assert_eq!(report.clock.state, ClockState::Active);
        assert!(report.clock.step_error.is_none());
        consume(
            &mut host,
            report.publications,
            &submitted_at,
            &mut result_latency,
        );
        service_elapsed.push(full_path_start.elapsed().as_micros());
    }
    let tail_deadline = origin + Duration::from_secs(4) + Duration::from_millis(20);
    thread::sleep(tail_deadline.saturating_duration_since(Instant::now()));
    let tail = host.on_callback(Instant::now()).unwrap();
    completed += u64::from(tail.clock.completed);
    consume(
        &mut host,
        tail.publications,
        &submitted_at,
        &mut result_latency,
    );
    assert_eq!(result_latency.len(), (CALLBACKS * 2) as usize);
    assert_eq!(
        host.replica().unwrap().get(StableId(1)),
        Some(&Counter(CALLBACKS))
    );
    assert_eq!(
        host.replica().unwrap().get(StableId(2)),
        Some(&Counter(CALLBACKS))
    );
    println!(
        "normal: callbacks={CALLBACKS} committed_ticks={completed} step_budget_overruns={overrun} final_authority_tick={} final_replica_tick={}",
        host.authority().tick_index(),
        host.replica().unwrap().tick()
    );
    report("real callback interval", &mut callback_gap);
    report("host callback elapsed", &mut callback_elapsed);
    report(
        "two submissions + callback service elapsed",
        &mut service_elapsed,
    );
    report("input-to-confirmed-result latency", &mut result_latency);
}

fn slow_run() {
    let origin = Instant::now();
    let (mut host, initial) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(10),
            other_work: Duration::from_millis(8),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority(Duration::from_millis(3), 20),
    )
    .unwrap();
    assert!(initial.results.is_empty());
    submit(&mut host, PeerId(1), ActorId(1), 1, StableId(1), 1);
    let deadline = origin + Duration::from_millis(17);
    thread::sleep(deadline.saturating_duration_since(Instant::now()));
    let report = host.on_callback(Instant::now()).unwrap();
    assert!(report.step_budget_overrun);
    assert_eq!(report.clock.completed, 1);
    assert_eq!(report.clock.state, ClockState::Overloaded);
    assert_eq!(host.replica().unwrap().get(StableId(1)), Some(&Counter(1)));
    let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
        panic!("missing slow callback result")
    };
    assert_eq!(admitted.results.len(), 1);
    println!(
        "slow callback: observed_elapsed_us={} total_budget_us=10000 reserved_other_us=8000 step_budget_us=2000 committed_ticks={} state={:?} new_input_admitted=false",
        report.observed_elapsed.as_micros(),
        report.clock.completed,
        report.clock.state
    );
}

fn main() {
    normal_run();
    slow_run();
}
