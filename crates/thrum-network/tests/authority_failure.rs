//! Runtime refusal terminates authority without publishing a set for that tick.
/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Failure, Match, local};
use thrum_core::sim::TickError;
use thrum_network::session::{
    ClientState, Controls, Disposition, EndReason, HostState, Input, World,
};

const FAILURE: u64 = 111;

#[test]
fn runtime_refusal_cannot_publish_a_false_commit() {
    let successful = run(false);
    let refused = run(true);
    assert_eq!(
        successful, refused,
        "each client authored the same target and sequence before refusal"
    );
}

fn run(fail: bool) -> [Option<u64>; 4] {
    let profile = thrum_transport::memory::Conditioner {
        delay_ms: 2,
        loss_ppm: 20_000,
        ..local()
    };
    let mut fixture = Match::new(profile, 4, fail.then_some(FAILURE), false);
    fixture.ready();
    let mut command_issued = false;
    let mut timed = [None; 4];
    let mut authored = [0; 4];
    while fixture.host.world().tick() < FAILURE
        && !matches!(fixture.host.state(), HostState::Ended(_))
    {
        if fixture.host.world().tick() == FAILURE - 1 && !command_issued {
            for (slot, client) in fixture.clients.iter_mut().enumerate() {
                let tag = u16::try_from(slot).expect("four players");
                assert_eq!(client.command(&(100 + tag)).expect("one command"), 1);
            }
            command_issued = true;
        }
        fixture.poll(FAILURE, |slot, tick| {
            authored[slot] += 1;
            let actions = if tick == FAILURE {
                timed[slot] = Some(authored[slot]);
                vec![200 + u16::try_from(slot).expect("four players")]
            } else {
                Vec::new()
            };
            Controls {
                axes: [0.02, 0.11, 0.0],
                actions,
            }
        });
    }
    assert!(command_issued);
    assert!(timed.iter().all(Option::is_some));
    let expected = FAILURE - u64::from(fail);
    if fail {
        let Some(Failure::Tick(error)) = fixture.host.take_step_error() else {
            panic!("expected the runtime's own step refusal");
        };
        assert!(matches!(error, TickError::Game(_)), "{error:?}");
        assert_eq!(
            fixture.host.state(),
            HostState::Ended(EndReason::AuthorityStep)
        );
    } else {
        fixture.host.stop();
    }
    fixture.finish(FAILURE);
    assert_eq!(fixture.host.world().tick(), expected);
    for (slot, client) in fixture.clients.iter_mut().enumerate() {
        assert_eq!(client.world().tick(), expected);
        assert_eq!(client.world().digest(), fixture.host.world().digest());
        assert_eq!(client.world().log(), fixture.host.world().log());
        let reason = if fail {
            EndReason::AuthorityStep
        } else {
            EndReason::Stopped
        };
        assert_eq!(client.state(), &ClientState::Ended(reason, expected));
        let mut answer = None;
        let mut timed_answer = None;
        let mut pending = 0;
        while let Some(event) = client.result() {
            if event.input == Input::Command(1) {
                answer = Some(event.disposition);
            }
            if event.input == Input::Frame(timed[slot].expect("target authored")) {
                timed_answer = Some(event.disposition);
            }
            if matches!(event.input, Input::Frame(_))
                && event.disposition == Disposition::NotCommitted
            {
                pending += 1;
            }
            if fail {
                assert_ne!(event.disposition, Disposition::Committed(FAILURE));
            }
        }
        assert!(pending > 0);
        assert_eq!(
            answer,
            Some(if fail {
                Disposition::NotCommitted
            } else {
                Disposition::Committed(FAILURE)
            })
        );
        assert_eq!(timed_answer, answer);
        let tag = u16::try_from(slot).expect("four players");
        let assigned = client.slot().expect("welcomed slot");
        let own = client
            .world()
            .log()
            .iter()
            .filter(|(_, owner, _)| *owner == assigned)
            .collect::<Vec<_>>();
        if fail {
            assert_eq!(own.first(), None);
        } else {
            assert_eq!(
                own,
                [
                    &(FAILURE, assigned, 200 + tag),
                    &(FAILURE, assigned, 100 + tag)
                ]
            );
        }
    }
    timed
}
