//! Two-controlled-peer headless cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_two_peer_cost`

use std::{hint::black_box, time::Instant};

use thrum_core::{
    ActorId, Authority, AuthorityId, GameRules, Handle, Input, InputSchedule, MatchId, Origin,
    PeerId, Replica, StableId, World, WorldEpoch, WorldWork,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter(u64);

struct Rule;
impl GameRules for Rule {
    type Entity = Counter;
    type Action = StableId;
    type Change = Handle;
    type Rejection = ();
    type Event = ();

    fn decide(&self, world: &World<Counter>, _: ActorId, id: &StableId) -> Result<Handle, ()> {
        world.handle_for(*id).ok_or(())
    }
    fn commit(&self, world: &mut World<Counter>, handle: Handle) -> Option<()> {
        world.get_mut(handle).expect("validated object").0 += 1;
        None
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
        "{label}: ns/tick p50={p50} p95={p95} p99={p99} max={}",
        samples[samples.len() - 1]
    );
}

fn main() {
    const WARMUP: usize = 1_000;
    const TICKS: usize = 10_000;
    let mut world = World::new(WorldEpoch(1));
    for id in 1..=64_u128 {
        world.spawn(StableId(id), Counter(0)).unwrap();
    }
    let mut authority = Authority::new(AuthorityId::Match(MatchId(1)), Rule, world);
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    authority.register_peer(PeerId(2), 7, ActorId(2)).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let mut end_to_end = Vec::with_capacity(TICKS);
    let mut step_cost = Vec::with_capacity(TICKS);
    let mut total_work = WorldWork::default();
    for i in 0..WARMUP + TICKS {
        let start = Instant::now();
        for (peer, actor, epoch, id) in [
            (PeerId(1), ActorId(1), 1, StableId(1)),
            (PeerId(2), ActorId(2), 7, StableId(2)),
        ] {
            let input = Input {
                origin: Origin {
                    peer,
                    connection_epoch: epoch,
                    sequence: (i + 1) as u64,
                },
                actor,
                schedule: InputSchedule::NextTick {
                    observed_tick: authority.tick_index(),
                },
                action: id,
            };
            black_box(authority.submit(input).unwrap());
        }
        let step_start = Instant::now();
        let publication = authority.step().unwrap();
        let stepped = step_start.elapsed().as_nanos();
        assert_eq!(publication.group.results.len(), 2);
        let admitted = replica.apply(publication.group).unwrap();
        assert_eq!(admitted.resolved_through.len(), 2);
        for (peer, through) in admitted.resolved_through {
            authority.acknowledge_results(peer, through).unwrap();
        }
        let elapsed = start.elapsed().as_nanos();
        if i >= WARMUP {
            step_cost.push(stepped);
            end_to_end.push(elapsed);
            total_work.page_references += publication.world_work.page_references;
            total_work.copied_pages += publication.world_work.copied_pages;
            total_work.copied_objects += publication.world_work.copied_objects;
            total_work.group_objects += publication.world_work.group_objects;
        }
    }
    assert_eq!(replica.tick(), (WARMUP + TICKS) as u64);
    assert_eq!(
        replica.get(StableId(1)),
        Some(&Counter((WARMUP + TICKS) as u64))
    );
    assert_eq!(
        replica.get(StableId(2)),
        Some(&Counter((WARMUP + TICKS) as u64))
    );
    report("two-peer/authority-step", &mut step_cost);
    report("two-peer/admit-to-replica+ack", &mut end_to_end);
    println!(
        "per-tick work: page_refs={} copied_pages={} copied_objects={} group_objects={} results=2",
        total_work.page_references / TICKS,
        total_work.copied_pages / TICKS,
        total_work.copied_objects / TICKS,
        total_work.group_objects / TICKS
    );
}
