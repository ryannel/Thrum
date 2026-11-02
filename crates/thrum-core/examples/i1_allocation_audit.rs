//! Standalone diagnostic of Rust allocator requests around a protected I1
//! host. Run this example in its own release process; it is not an RSS bound.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    num::NonZeroU32,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use thrum_core::{
    ActorId, Authority, AuthorityId, ClockConfig, ClosedGameLimits, ClosedValueLimits, GameRules,
    Handle, HeadlessHost, HostAdmissionError, HostBudget, HostPublication, Input, InputSchedule,
    MatchId, Origin, PeerId, PreparedCommit, PreparedEdits, PublicationBytes, PublicationPool,
    PublicationRecord, Replica, ResultDisposition, RuleContext, RuleScratchHandle, SessionId,
    SnapshotWriteBounds, StableId, World, WorldEdit, WorldEpoch,
};

struct CountedSystem;
static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

fn added(bytes: usize) {
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
}

// SAFETY: Every operation delegates to System with the original pointer and
// layout contract. Counters use only static atomics and never allocate, log,
// panic or unwind in an allocator hook. A failed request changes no counter.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the caller's valid layout unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            REQUESTS.fetch_add(1, Ordering::Relaxed);
            REQUESTED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            added(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the caller's valid layout unchanged.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            REQUESTS.fetch_add(1, Ordering::Relaxed);
            REQUESTED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            added(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: Forward the original live pointer and layout unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: Forward the original live pointer, layout and new size.
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            REQUESTS.fetch_add(1, Ordering::Relaxed);
            REQUESTED_BYTES.fetch_add(size, Ordering::Relaxed);
            if size >= layout.size() {
                added(size - layout.size());
            } else {
                LIVE_BYTES.fetch_sub(layout.size() - size, Ordering::Relaxed);
            }
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: CountedSystem = CountedSystem;

#[derive(Clone, Copy)]
struct HeapSample {
    requests: usize,
    requested_bytes: usize,
    live_bytes: usize,
    peak_bytes: usize,
}

fn heap_sample() -> HeapSample {
    HeapSample {
        requests: REQUESTS.load(Ordering::Relaxed),
        requested_bytes: REQUESTED_BYTES.load(Ordering::Relaxed),
        live_bytes: LIVE_BYTES.load(Ordering::Relaxed),
        peak_bytes: PEAK_BYTES.load(Ordering::Relaxed),
    }
}

struct Increment {
    private_allocation: bool,
}

impl GameRules for Increment {
    type Entity = u64;
    type Action = u8;
    type Change = (Handle, RuleScratchHandle);
    type Rejection = ();
    type Event = u8;

    fn decide(
        &self,
        world: &World<u64>,
        _actor: ActorId,
        _action: &u8,
        context: &mut RuleContext<'_>,
    ) -> Result<Self::Change, ()> {
        if self.private_allocation {
            let private = vec![7_u8; 257];
            black_box(&private);
        }
        let scratch = context.try_scratch_allocate(4).expect("bounded scratch");
        context
            .scratch_view_mut(scratch)
            .expect("same action")
            .copy_from_slice(&[1, 2, 3, 4]);
        Ok((world.handle_for(StableId(1)).expect("one object"), scratch))
    }

    fn prepare(
        &self,
        world: &World<u64>,
        (handle, scratch): Self::Change,
        context: &mut RuleContext<'_>,
    ) -> PreparedCommit<u64, u8> {
        assert_eq!(context.scratch_view(scratch), Ok(&[1, 2, 3, 4][..]));
        let next = world.get(handle).expect("live object").state + 1;
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(handle, next)),
            event: Some(1),
        }
    }
}

struct RunSample {
    admission_requests: usize,
    admission_bytes: usize,
    callback_requests: usize,
    callback_bytes: usize,
    peak_extra_rust_bytes: usize,
    pool_before: usize,
    pool_after_admission: usize,
    pool_after_callback: usize,
}

fn run(match_play: bool, private_allocation: bool) -> RunSample {
    let pool = PublicationPool::new(2_000_000);
    let mut world = World::new(WorldEpoch(1));
    world.spawn(StableId(1), 0_u64).unwrap();
    let mut authority = Authority::try_new_closed_snapshot_charged(
        if match_play {
            AuthorityId::Match(MatchId(1))
        } else {
            AuthorityId::Standalone(SessionId(1))
        },
        Increment { private_allocation },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: ClosedValueLimits {
                max_inline_bytes: 1,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            entity: ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            event: ClosedValueLimits {
                max_inline_bytes: 1,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            rejection: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
        },
    )
    .unwrap();
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    if match_play {
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
    }
    authority.try_enable_rule_scratch(4).unwrap();
    let origin = Instant::now();
    let (mut host, baseline) = HeadlessHost::with_protected_output_replica(
        origin,
        ClockConfig {
            max_ticks_per_call: NonZeroU32::new(1).unwrap(),
            ..ClockConfig::default()
        },
        HostBudget {
            callback_cpu: Duration::from_secs(1),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
        &pool,
        1,
        if match_play { 2 } else { 1 },
        0,
    )
    .unwrap();
    let old_snapshot = host.authority().snapshot().clone();
    let pool_before = pool.used_bytes();
    let before = heap_sample();
    PEAK_BYTES.store(before.live_bytes, Ordering::Relaxed);
    let peers = if match_play { 2 } else { 1 };
    for peer in 1..=peers {
        for sequence in 1..=8 {
            host.submit(Input {
                origin: Origin {
                    peer: PeerId(peer),
                    connection_epoch: 1,
                    sequence,
                },
                actor: ActorId(peer),
                schedule: InputSchedule::NextTick { observed_tick: 0 },
                action: 1,
            })
            .unwrap();
        }
    }
    let after_admission = heap_sample();
    let pool_after_admission = pool.used_bytes();
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .unwrap();
    let report = host
        .on_callback_charged(origin + Duration::from_millis(17))
        .unwrap();
    assert_eq!(report.clock.completed, 1);
    let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
        panic!("expected charged replica");
    };
    assert_eq!(admitted.results().len(), 8 * peers as usize);
    assert_eq!(admitted.events().len(), 8 * peers as usize);
    assert_eq!(host.replica().unwrap().get(StableId(1)), Some(&(8 * peers)));
    assert_eq!(old_snapshot.records().next().unwrap().state, 0);
    let after_callback = heap_sample();
    drop(occupied);
    let pool_after_callback = pool.used_bytes();
    drop(report);
    for peer in 1..=peers {
        host.acknowledge_results(PeerId(peer), 8).unwrap();
    }
    drop(old_snapshot);
    drop(baseline);
    drop(host);
    assert_eq!(pool.used_bytes(), 0);
    RunSample {
        admission_requests: after_admission.requests - before.requests,
        admission_bytes: after_admission.requested_bytes - before.requested_bytes,
        callback_requests: after_callback.requests - after_admission.requests,
        callback_bytes: after_callback.requested_bytes - after_admission.requested_bytes,
        peak_extra_rust_bytes: after_callback.peak_bytes - before.live_bytes,
        pool_before,
        pool_after_admission,
        pool_after_callback,
    }
}

type CounterRecord = PublicationRecord<8, 1>;
type NumberRecord = PublicationRecord<8, 0>;

// The game-facing projection borrows its entire representation. Decoding the
// inline number makes no second owner, and the stamp remains in charged bytes.
struct CounterView<'a> {
    count: u64,
    stamp: Option<&'a [u8]>,
}

impl<'a> From<&'a CounterRecord> for CounterView<'a> {
    fn from(record: &'a CounterRecord) -> Self {
        Self {
            count: u64::from_le_bytes(*record.inline()),
            stamp: record.payload_owner(0).map(PublicationBytes::as_slice),
        }
    }
}

struct TypedIncrement {
    pool: PublicationPool,
}

impl GameRules for TypedIncrement {
    type Entity = CounterRecord;
    type Action = NumberRecord;
    type Change = (Handle, RuleScratchHandle);
    type Rejection = ();
    type Event = NumberRecord;

    fn decide(
        &self,
        world: &World<CounterRecord>,
        _actor: ActorId,
        action: &NumberRecord,
        context: &mut RuleContext<'_>,
    ) -> Result<Self::Change, ()> {
        let amount = u64::from_le_bytes(*action.inline());
        let handle = world.handle_for(StableId(1)).expect("one object");
        let state = CounterView::from(&world.get(handle).expect("live object").state);
        if let Some(stamp) = state.stamp {
            assert_eq!(stamp, &[state.count as u8; 16]);
        }
        let scratch = context.try_scratch_allocate(8).expect("charged scratch");
        context
            .scratch_view_mut(scratch)
            .expect("same action")
            .copy_from_slice(&amount.to_le_bytes());
        Ok((handle, scratch))
    }

    fn prepare(
        &self,
        world: &World<CounterRecord>,
        (handle, scratch): Self::Change,
        context: &mut RuleContext<'_>,
    ) -> PreparedCommit<CounterRecord, NumberRecord> {
        let amount = u64::from_le_bytes(context.scratch_view(scratch).unwrap().try_into().unwrap());
        let previous = CounterView::from(&world.get(handle).expect("live object").state);
        let next = previous.count + amount;
        let stamp = [next as u8; 16];
        let payload = context.try_copy_payload(&stamp).expect("admitted output");
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(
                handle,
                CounterRecord::try_new(&self.pool, next.to_le_bytes(), [Some(payload)]).unwrap(),
            )),
            event: Some(NumberRecord::try_new(&self.pool, next.to_le_bytes(), []).unwrap()),
        }
    }
}

fn number(pool: &PublicationPool, value: u64) -> NumberRecord {
    NumberRecord::try_new(pool, value.to_le_bytes(), []).unwrap()
}

fn run_typed(match_play: bool) -> RunSample {
    let pool = PublicationPool::new(2_000_000);
    let mut world = World::new(WorldEpoch(2));
    world
        .spawn(
            StableId(1),
            CounterRecord::try_new(&pool, 0_u64.to_le_bytes(), [None]).unwrap(),
        )
        .unwrap();
    let inline = ClosedValueLimits {
        max_inline_bytes: 8,
        max_payload_slots: 0,
        max_payload_bytes: 0,
    };
    let mut authority = Authority::try_new_closed_snapshot_charged(
        if match_play {
            AuthorityId::Match(MatchId(2))
        } else {
            AuthorityId::Standalone(SessionId(2))
        },
        TypedIncrement { pool: pool.clone() },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: inline,
            entity: ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 1,
                max_payload_bytes: 16,
            },
            event: inline,
            rejection: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
        },
    )
    .unwrap();
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    if match_play {
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
    }
    authority.try_enable_rule_scratch(8).unwrap();
    let origin = Instant::now();
    let peers = if match_play { 2 } else { 1 };
    let (mut host, baseline) = HeadlessHost::with_protected_output_replica(
        origin,
        ClockConfig {
            max_ticks_per_call: NonZeroU32::new(1).unwrap(),
            ..ClockConfig::default()
        },
        HostBudget {
            callback_cpu: Duration::from_secs(1),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
        &pool,
        1,
        peers,
        PublicationBytes::required_charge(16).unwrap(),
    )
    .unwrap();
    let old_snapshot = host.authority().snapshot().clone();
    let pool_before = pool.used_bytes();
    let before = heap_sample();
    PEAK_BYTES.store(before.live_bytes, Ordering::Relaxed);
    for peer in 1..=peers {
        for sequence in 1..=8 {
            host.submit(Input {
                origin: Origin {
                    peer: PeerId(peer as u64),
                    connection_epoch: 1,
                    sequence,
                },
                actor: ActorId(peer as u64),
                schedule: InputSchedule::Timed { intended_tick: 2 },
                action: number(&pool, peer as u64),
            })
            .unwrap();
        }
    }
    let after_admission = heap_sample();
    let pool_after_admission = pool.used_bytes();
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .unwrap();
    let idle = host
        .on_callback_charged(origin + Duration::from_millis(17))
        .unwrap();
    assert_eq!(idle.clock.completed, 1);
    assert_eq!(
        host.replica()
            .unwrap()
            .get(StableId(1))
            .map(CounterView::from)
            .unwrap()
            .count,
        0
    );
    drop(idle);
    let report = host
        .on_callback_charged(origin + Duration::from_millis(34))
        .unwrap();
    assert_eq!(report.clock.completed, 1);
    let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
        panic!("expected charged replica");
    };
    assert_eq!(admitted.results().len(), 8 * peers);
    assert_eq!(admitted.events().len(), 8 * peers);
    assert!(
        admitted
            .results()
            .iter()
            .all(|result| matches!(result.disposition, ResultDisposition::Committed))
    );
    let expected = 8 * peers * (peers + 1) / 2;
    assert_eq!(
        u64::from_le_bytes(*admitted.events().last().unwrap().payload.inline()),
        expected as u64
    );
    assert!(
        admitted
            .events()
            .windows(2)
            .all(|pair| pair[0].id + 1 == pair[1].id)
    );
    let replica = CounterView::from(host.replica().unwrap().get(StableId(1)).unwrap());
    assert_eq!(replica.count, expected as u64);
    assert_eq!(replica.stamp, Some(&[expected as u8; 16][..]));
    assert_eq!(
        CounterView::from(&old_snapshot.records().next().unwrap().state).count,
        0
    );
    let after_callback = heap_sample();
    drop(occupied);
    let pool_after_callback = pool.used_bytes();
    drop(report);
    for peer in 1..=peers {
        host.acknowledge_results(PeerId(peer as u64), 8).unwrap();
    }
    assert_eq!(
        host.register_peer(PeerId(1), 2, ActorId(1)),
        Err(HostAdmissionError::RequiresBaseline)
    );
    let fresh_baseline = host.authority().try_baseline_charged().unwrap();
    let (fresh_replica, fresh_outcomes) =
        Replica::from_charged_baseline(fresh_baseline, &pool, 1).unwrap();
    assert!(fresh_outcomes.results().is_empty());
    assert_eq!(
        CounterView::from(fresh_replica.get(StableId(1)).unwrap()).count,
        expected as u64
    );
    drop(fresh_outcomes);
    drop(fresh_replica);
    drop(old_snapshot);
    drop(baseline);
    drop(host);
    assert_eq!(pool.used_bytes(), 0);
    RunSample {
        admission_requests: after_admission.requests - before.requests,
        admission_bytes: after_admission.requested_bytes - before.requested_bytes,
        callback_requests: after_callback.requests - after_admission.requests,
        callback_bytes: after_callback.requested_bytes - after_admission.requested_bytes,
        peak_extra_rust_bytes: after_callback.peak_bytes - before.live_bytes,
        pool_before,
        pool_after_admission,
        pool_after_callback,
    }
}

fn print_sample(name: &str, sample: RunSample) {
    println!(
        "{name}: admission requests={} bytes={}; callback requests={} bytes={}; rust_peak_extra={}B; pool before={} admitted={} callback={}B",
        sample.admission_requests,
        sample.admission_bytes,
        sample.callback_requests,
        sample.callback_bytes,
        sample.peak_extra_rust_bytes,
        sample.pool_before,
        sample.pool_after_admission,
        sample.pool_after_callback,
    );
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("typed_two_peer") => {
            print_sample("typed_two_peer", run_typed(true));
            return;
        }
        None => {}
        Some(_) => panic!("choose typed_two_peer or omit the mode"),
    }
    let local = run(false, false);
    let match_clean = run(true, false);
    let match_control = run(true, true);
    let typed_local = run_typed(false);
    let typed_match = run_typed(true);
    assert!(match_control.callback_requests >= match_clean.callback_requests + 16);
    assert!(match_control.callback_bytes >= match_clean.callback_bytes + 16 * 257);
    for (name, sample) in [
        ("standalone", local),
        ("two_peer", match_clean),
        ("private_allocation_control", match_control),
        ("typed_standalone", typed_local),
        ("typed_two_peer", typed_match),
    ] {
        print_sample(name, sample);
    }
    black_box(heap_sample().live_bytes);
}
