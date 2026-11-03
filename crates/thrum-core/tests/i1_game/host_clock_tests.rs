//! Hosted clock overload and admission integration tests.

use super::*;

#[test]
fn slow_hosted_callback_reports_overload_and_stops_new_input() {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    struct SlowRule;
    impl GameRules for SlowRule {
        type Entity = Object;
        type Action = Action;
        type Change = Change;
        type Rejection = Rejection;
        type Event = Event;

        fn decide(
            &self,
            world: &World<Object>,
            actor: ActorId,
            action: &Action,
            context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Change, Rejection> {
            thread::sleep(Duration::from_millis(3));
            Rule.decide(world, actor, action, context)
        }
        fn prepare(
            &self,
            world: &World<Object>,
            change: Change,
            context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Object, Event> {
            Rule.prepare(world, change, context)
        }
    }

    let origin = Instant::now();
    let mut authority = Authority::new(AuthorityId::Match(MatchId(110)), SlowRule, game_world(111));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let (mut host, initial) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(10),
            other_work: Duration::from_millis(8),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .expect("test setup or assertion requires a value");
    assert_eq!(initial.results, []);
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .expect("test setup or assertion requires a value");
    let report = host
        .on_callback(origin + Duration::from_millis(34))
        .expect("test setup or assertion requires a value");
    assert_eq!(report.clock.completed, 1);
    assert!(report.step_budget_overrun);
    assert_eq!(report.clock.state, ClockState::Overloaded);
    assert_eq!(report.clock.queued, 1);
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        1
    );
    let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
        panic!("expected admitted replica group")
    };
    assert_eq!(admitted.results.len(), 1);
    host.acknowledge_results(PEER, admitted.resolved_through[0].1)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        host.submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        )),
        Err(HostAdmissionError::Unavailable)
    );
    let drained = host
        .on_callback(origin + Duration::from_millis(35))
        .expect("test setup or assertion requires a value");
    assert_eq!(drained.clock.completed, 1);
    assert_eq!(drained.clock.queued, 0);
    host.resume(origin + Duration::from_millis(36))
        .expect("test setup or assertion requires a value");
    assert_eq!(host.clock_state(), ClockState::Active);
    assert_eq!(host.authority().baseline().event_cursor, 1);
}

#[test]
fn local_host_delivers_results_and_acknowledgement_restores_capacity() {
    use std::time::{Duration, Instant};

    let origin = Instant::now();
    let mut authority = Authority::new(
        AuthorityId::Standalone(SessionId(120)),
        Rule,
        game_world(121),
    );
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let mut host = HeadlessHost::local(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .expect("test setup or assertion requires a value");
    for sequence in 1..=256_u64 {
        let observed_tick = host.authority().tick_index();
        host.submit(input(
            1,
            sequence,
            InputSchedule::NextTick { observed_tick },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
        let report = host
            .on_callback(origin + Duration::from_millis(sequence * 20))
            .expect("test setup or assertion requires a value");
        let observed = report.publications.iter().any(|publication| {
            let HostPublication::Local(committed) = publication else {
                panic!("local host returned replica output")
            };
            committed
                .group
                .results
                .iter()
                .any(|result| result.origin.sequence == sequence)
        });
        assert!(observed);
    }
    let next = input(
        1,
        257,
        InputSchedule::NextTick {
            observed_tick: host.authority().tick_index(),
        },
        Action::Open(DOOR),
    );
    assert_eq!(
        host.submit(next.clone()),
        Err(HostAdmissionError::Authority(AdmissionError::Busy))
    );
    host.acknowledge_results(PEER, 256)
        .expect("test setup or assertion requires a value");
    assert_eq!(host.submit(next), Ok(Admission::Queued));
}
