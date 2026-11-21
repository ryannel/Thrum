//! Results come from the committed set stream and are retained through receipt.
/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Match, shaped};
use thrum_network::session::{Controls, Disposition, Input, World};

#[test]
fn each_authored_input_resolves_and_each_ack_releases_its_sets() {
    let mut fixture = Match::new(shaped(), 4, None, false);
    fixture.ready();
    let endpoint = 200;
    let mut targets: [Vec<u64>; 4] = std::array::from_fn(|_| Vec::new());
    let mut results: [Vec<_>; 4] = std::array::from_fn(|_| Vec::new());
    let mut peak = [0; 4];
    let mut stalled = None;
    let mut acknowledged_ahead_of_playout = false;
    for _ in 0..2_700 {
        let tick = fixture.host.world().tick();
        if tick == 70 && stalled.is_none() {
            fixture.memory.stall(true);
            stalled = Some(fixture.now);
            for (slot, client) in fixture.clients.iter_mut().enumerate() {
                assert_eq!(
                    client
                        .command(&(100 + u16::try_from(slot).unwrap()))
                        .unwrap(),
                    1
                );
            }
        }
        if stalled.is_some_and(|start| fixture.now >= start + 350_000) {
            fixture.memory.stall(false);
        }
        fixture.poll(endpoint, |slot, target| {
            assert!(targets[slot].last().is_none_or(|last| target > *last));
            targets[slot].push(target);
            Controls::axes([0.21, -0.19, 0.0])
        });
        for (slot, client) in fixture.clients.iter_mut().enumerate() {
            peak[slot] = peak[slot].max(fixture.host.retained(fixture.links[slot][1].id()));
            while let Some(result) = client.result() {
                results[slot].push(result);
            }
            let progress = fixture.host.stream(fixture.links[slot][1].id()).unwrap();
            let acknowledged = targets[slot].partition_point(|tick| *tick <= progress.held);
            let resolved = results[slot]
                .iter()
                .filter(|event| matches!(event.input, Input::Frame(_)))
                .count();
            assert!(resolved >= acknowledged);
            acknowledged_ahead_of_playout |=
                progress.held > client.world().tick() && acknowledged > 0;
        }
        if fixture
            .clients
            .iter()
            .all(|client| client.world().tick() == endpoint)
            && fixture
                .links
                .iter()
                .all(|links| fixture.host.retained(links[1].id()) == 0)
        {
            break;
        }
        // When paced sends stop, a reliable fallback continues to repair an overfull datagram.
    }
    assert_eq!(fixture.host.world().tick(), endpoint);
    assert!(acknowledged_ahead_of_playout);
    let mut counts = [0; 4];
    for slot in 0..4 {
        assert_eq!(fixture.clients[slot].world().tick(), endpoint);
        assert_eq!(
            fixture
                .host
                .stream(fixture.links[slot][1].id())
                .unwrap()
                .held,
            endpoint
        );
        assert!(peak[slot] > 10);
        assert_eq!(fixture.host.retained(fixture.links[slot][1].id()), 0);
        let committed_command = fixture
            .host
            .world()
            .log()
            .iter()
            .find(|(_, owner, _)| Some(*owner) == fixture.clients[slot].slot())
            .unwrap()
            .0;
        assert_eq!(
            results[slot]
                .iter()
                .filter(|event| event.input == Input::Command(1))
                .map(|event| event.disposition)
                .collect::<Vec<_>>(),
            [Disposition::Committed(committed_command)]
        );
        let target = &targets[slot];
        let seen = results[slot]
            .iter()
            .filter_map(|event| {
                if let Input::Frame(sequence) = event.input {
                    Some((sequence, event.disposition))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(seen.len(), target.partition_point(|tick| *tick <= endpoint));
        for (index, (sequence, disposition)) in seen.into_iter().enumerate() {
            assert_eq!(sequence, index as u64 + 1);
            match disposition {
                Disposition::Committed(tick) => assert_eq!(tick, target[index]),
                Disposition::Refused => counts[slot] += 1,
                other => panic!("unexpected frame result {other:?}"),
            }
        }
    }
    assert!(counts.into_iter().all(|count| count > 0), "{counts:?}");
}
