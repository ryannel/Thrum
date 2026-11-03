//! Complete 4,096-change sealed group and replica admission cost.
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_changed_group_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ActorId, Authority, AuthorityId, ClosedGameLimits, ClosedValueLimits, GameRules, Input,
    InputSchedule, MatchId, Origin, PeerId, PreparedCommit, PreparedEdits, PublicationPool,
    PublicationRecord, Replica, SnapshotWriteBounds, StableId, World, WorldEdit, WorldEpoch,
};

type State = PublicationRecord<8, 0>;

struct ChangeAll {
    pool: PublicationPool,
    count: usize,
}

impl GameRules for ChangeAll {
    type Entity = State;
    type Action = ();
    type Change = ();
    type Rejection = ();
    type Event = ();

    fn decide(
        &self,
        _world: &World<State>,
        _actor: ActorId,
        _action: &(),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> Result<(), ()> {
        Ok(())
    }

    fn prepare(
        &self,
        world: &World<State>,
        _change: (),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<State, ()> {
        let mut edits = Vec::with_capacity(self.count);
        for id in 1..=self.count {
            let handle = world.handle_for(StableId(id as u128)).unwrap();
            let old = world.get(handle).unwrap();
            let next = u64::from_le_bytes(*old.state.inline()) + 1;
            edits.push(WorldEdit::Replace(
                handle,
                State::try_new(&self.pool, next.to_le_bytes(), []).unwrap(),
            ));
        }
        PreparedCommit {
            edits: PreparedEdits::Many(edits),
            event: None,
        }
    }
}

fn summary(times: &mut [u128]) -> (u128, u128, u128, u128) {
    times.sort_unstable();
    let last = times.len() - 1;
    (
        times[last / 2],
        times[last * 95 / 100],
        times[last * 99 / 100],
        times[last],
    )
}

fn main() {
    const COUNT: usize = 4_096;
    const WARMUP: usize = 100;
    const SAMPLES: usize = 1_000;
    let pool = PublicationPool::new(usize::MAX);
    let mut world = World::new(WorldEpoch(411));
    for id in 1..=COUNT {
        world
            .spawn(
                StableId(id as u128),
                State::try_new(&pool, 0_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
    }
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(411)),
        ChangeAll {
            pool: pool.clone(),
            count: COUNT,
        },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: COUNT,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            entity: ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            event: ClosedValueLimits {
                max_inline_bytes: 0,
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
    let (mut replica, _) =
        Replica::from_charged_baseline(authority.try_baseline_charged().unwrap(), &pool, COUNT)
            .unwrap();
    let mut replica_times = Vec::with_capacity(SAMPLES);
    let mut complete_times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = authority.tick_index();
        let start = Instant::now();
        authority
            .submit(Input {
                origin: Origin {
                    peer: PeerId(1),
                    connection_epoch: 1,
                    sequence: tick + 1,
                },
                actor: ActorId(1),
                schedule: InputSchedule::NextTick {
                    observed_tick: tick,
                },
                action: (),
            })
            .unwrap();
        let published = authority.step_charged().unwrap();
        assert_eq!(published.group.changes().len(), COUNT);
        let replica_start = Instant::now();
        let admitted = replica.apply_charged(published.group).unwrap();
        authority
            .acknowledge_results(PeerId(1), admitted.resolved_through()[0].1)
            .unwrap();
        if sample >= WARMUP {
            replica_times.push(replica_start.elapsed().as_nanos());
            complete_times.push(start.elapsed().as_nanos());
        }
    }
    let final_state = replica.get(StableId(COUNT as u128)).unwrap();
    assert_eq!(
        u64::from_le_bytes(*final_state.inline()),
        (WARMUP + SAMPLES) as u64
    );
    black_box(final_state);
    println!(
        "changed_group/4096: replica_ack_ns[p50/p95/p99/max]={:?} complete_ns[p50/p95/p99/max]={:?} host={}/{} profile=release",
        summary(&mut replica_times),
        summary(&mut complete_times),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
}
