//! One-event replica validation cost; groups are built before timing.
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_event_validation_cost`

use std::{hint::black_box, time::Instant};

use thrum_core::{
    ActionResult, AuthorityId, CommittedEvent, MatchId, Origin, PeerId, Replica, ResultDisposition,
    StateGroup,
};

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn main() {
    const WARMUP: usize = 1_000;
    const SAMPLES: usize = 10_000;
    let authority = AuthorityId::Match(MatchId(253));
    let peer = PeerId(1);
    let origin = Origin {
        peer,
        connection_epoch: 1,
        sequence: 0,
    };
    let baseline = StateGroup::<u8, (), ()> {
        authority,
        group_id: 0,
        commit_tick: 0,
        event_cursor: 0,
        changes: vec![],
        results: vec![],
        events: vec![],
        resolved_through: vec![(peer, 0)],
    };
    let (mut replica, _) = Replica::from_baseline(baseline).unwrap();
    let groups = (1..=(WARMUP + SAMPLES) as u64)
        .map(|tick| {
            let event_origin = Origin {
                sequence: tick,
                ..origin
            };
            StateGroup::<u8, (), ()> {
                authority,
                group_id: tick,
                commit_tick: tick,
                event_cursor: tick,
                changes: vec![],
                results: vec![ActionResult {
                    origin: event_origin,
                    resolved_tick: tick,
                    disposition: ResultDisposition::Committed,
                    event_id: Some(tick),
                }],
                events: vec![CommittedEvent {
                    id: tick,
                    tick,
                    origin: event_origin,
                    payload: (),
                }],
                resolved_through: vec![(peer, tick)],
            }
        })
        .collect::<Vec<_>>();
    let mut samples = Vec::with_capacity(SAMPLES);
    for (index, group) in groups.into_iter().enumerate() {
        let started = Instant::now();
        let admitted = replica.apply(group).unwrap();
        let elapsed = started.elapsed().as_nanos();
        black_box(admitted);
        if index >= WARMUP {
            samples.push(elapsed);
        }
    }
    assert_eq!(replica.tick(), (WARMUP + SAMPLES) as u64);
    println!(
        "one_event_replica_apply_ns[p50/p95/p99/max]=({}/{}/{}/{})",
        percentile(&mut samples, 50),
        percentile(&mut samples, 95),
        percentile(&mut samples, 99),
        samples[SAMPLES - 1]
    );
}
