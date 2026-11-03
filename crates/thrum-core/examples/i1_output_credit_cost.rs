//! Local complete output-payload callback fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_output_credit_cost -- protected`

use std::{
    hint::black_box,
    time::{Duration, Instant},
};
use thrum_core::{
    ActorId, Authority, AuthorityId, ClockConfig, ClosedGameLimits, ClosedValueLimits, GameRules,
    Handle, HeadlessHost, HostBudget, HostPublication, Input, InputSchedule, MatchId, Origin,
    PeerId, PreparedCommit, PreparedEdits, PublicationBytes, PublicationPool, PublicationRecord,
    ResultDisposition, RuleContext, RulePayloadError, SnapshotWriteBounds, StableId, World,
    WorldEdit, WorldEpoch,
};

const WARMUP: u64 = 1_000;
const SAMPLES: u64 = 10_000;
type State = PublicationRecord<1, 1>;

struct Rule {
    pool: PublicationPool,
}

impl GameRules for Rule {
    type Entity = State;
    type Action = u8;
    type Change = (Handle, u8);
    type Rejection = ();
    type Event = ();

    fn decide(
        &self,
        world: &World<State>,
        _actor: ActorId,
        action: &u8,
        _context: &mut RuleContext<'_>,
    ) -> Result<Self::Change, ()> {
        Ok((world.handle_for(StableId(1)).unwrap(), *action))
    }

    fn prepare(
        &self,
        _world: &World<State>,
        (handle, value): Self::Change,
        context: &mut RuleContext<'_>,
    ) -> PreparedCommit<State, ()> {
        let source = [value; 32];
        let payload = match context.try_copy_payload(&source) {
            Ok(payload) => payload,
            Err(RulePayloadError::Unqualified) => {
                PublicationBytes::try_copy_from_slice(&self.pool, &source).unwrap()
            }
            Err(RulePayloadError::Limit) => panic!("declared output allowance exceeded"),
        };
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(
                handle,
                State::try_new(&self.pool, [value], [Some(payload)]).unwrap(),
            )),
            event: None,
        }
    }
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn main() {
    let protected = match std::env::args().nth(1).as_deref() {
        Some("protected") => true,
        Some("ordinary") => false,
        _ => panic!("choose ordinary or protected"),
    };
    let pool = PublicationPool::new(1_000_000);
    let mut world = World::new(WorldEpoch(300));
    world
        .spawn(StableId(1), State::try_new(&pool, [0], [None]).unwrap())
        .unwrap();
    let inline = |bytes| ClosedValueLimits {
        max_inline_bytes: bytes,
        max_payload_slots: 0,
        max_payload_bytes: 0,
    };
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(300)),
        Rule { pool: pool.clone() },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: inline(1),
            entity: ClosedValueLimits {
                max_inline_bytes: 1,
                max_payload_slots: 1,
                max_payload_bytes: 32,
            },
            event: inline(0),
            rejection: inline(0),
        },
    )
    .unwrap();
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    let origin = Instant::now();
    let budget = HostBudget {
        callback_cpu: Duration::from_millis(5),
        other_work: Duration::from_millis(1),
        tick_cost_bound: Duration::from_millis(1),
    };
    let (mut host, initial) = if protected {
        HeadlessHost::with_protected_output_replica(
            origin,
            ClockConfig::default(),
            budget,
            authority,
            &pool,
            1,
            1,
            PublicationBytes::required_charge(32).unwrap(),
        )
        .unwrap()
    } else {
        HeadlessHost::with_charged_replica_with_peer_limit(
            origin,
            ClockConfig::default(),
            budget,
            authority,
            &pool,
            1,
            1,
        )
        .unwrap()
    };
    let mut samples = Vec::with_capacity(SAMPLES as usize);
    for sequence in 1..=WARMUP + SAMPLES {
        let started = Instant::now();
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence,
            },
            actor: ActorId(1),
            schedule: InputSchedule::NextTick {
                observed_tick: sequence - 1,
            },
            action: sequence as u8,
        })
        .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_nanos(16_666_667 * sequence))
            .unwrap();
        let completed = report.clock.completed;
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged publication")
        };
        let committed = admitted.results().len() == 1
            && matches!(
                admitted.results()[0].disposition,
                ResultDisposition::Committed
            );
        black_box(&report);
        host.acknowledge_results(PeerId(1), sequence).unwrap();
        drop(report);
        let elapsed = started.elapsed().as_nanos();
        assert_eq!(completed, 1);
        assert!(committed);
        assert_eq!(
            host.replica()
                .unwrap()
                .get(StableId(1))
                .unwrap()
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[sequence as u8; 32]
        );
        if sequence > WARMUP {
            samples.push(elapsed);
        }
    }
    println!(
        "I1 output callback host={}/{} mode={} samples={} submit_callback_ack_ns[p50/p95/p99/max]={}/{}/{}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        if protected { "protected" } else { "ordinary" },
        SAMPLES,
        percentile(&mut samples, 50),
        percentile(&mut samples, 95),
        percentile(&mut samples, 99),
        samples[SAMPLES as usize - 1],
    );
    drop(initial);
    drop(host);
    assert_eq!(pool.used_bytes(), 0);
}
