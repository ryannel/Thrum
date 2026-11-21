//! Reliable commands are ordered authority input even when timed actions miss.
/// The shared integer rule set and carrier fixture.
use crate::support;

use support::{Match, local, shaped};
use thrum_network::session::{Controls, Disposition, Input, World};

#[test]
fn commands_survive_loss_stall_and_an_in_flight_baseline() {
    let mut fixture = Match::new(shaped(), 4, None, true);
    for _ in 0..300 {
        fixture.neutral(0);
        if fixture
            .links
            .iter()
            .all(|links| links[1].counters(fixture.now / 1_000).lanes[0].sent > 1)
        {
            break;
        }
    }
    for (slot, client) in fixture.clients.iter_mut().enumerate() {
        assert!(client.baseline_progress().is_some() || client.settings().is_none());
        assert_eq!(
            client
                .command(&(10 + u16::try_from(slot).expect("four players")))
                .unwrap(),
            1
        );
    }
    fixture.ready();
    let mut issues = [false; 2];
    let mut release = None;
    let mut loss_until = None;
    let mut timed = [None; 4];
    let mut authored = [0; 4];
    let mut results: [Vec<_>; 4] = std::array::from_fn(|_| Vec::new());
    while fixture.host.world().tick() < 240 {
        let tick = fixture.host.world().tick();
        if tick >= 50 && !issues[0] {
            issues[0] = true;
            fixture
                .memory
                .recondition(thrum_transport::memory::Conditioner {
                    loss_ppm: 1_000_000,
                    ..shaped()
                })
                .unwrap();
            loss_until = Some(fixture.now + 15_000);
            for (slot, client) in fixture.clients.iter_mut().enumerate() {
                assert_eq!(
                    client
                        .command(&(20 + u16::try_from(slot).expect("four players")))
                        .unwrap(),
                    2
                );
            }
        }
        if loss_until.is_some_and(|end| fixture.now >= end) {
            fixture.memory.recondition(shaped()).unwrap();
            loss_until = None;
        }
        if tick >= 115 && !issues[1] {
            issues[1] = true;
            fixture.memory.stall(true);
            release = Some(fixture.now + 400_000);
            for (slot, client) in fixture.clients.iter_mut().enumerate() {
                assert_eq!(
                    client
                        .command(&(30 + u16::try_from(slot).expect("four players")))
                        .unwrap(),
                    3
                );
            }
        }
        if release.is_some_and(|deadline| fixture.now >= deadline) {
            fixture.memory.stall(false);
        }
        fixture.poll(240, |slot, target| {
            authored[slot] += 1;
            let mut controls = Controls::axes([0.0; 3]);
            if issues[1] && timed[slot].is_none() {
                controls
                    .actions
                    .push(50 + u16::try_from(slot).expect("four players"));
                timed[slot] = Some((target, authored[slot]));
            }
            controls
        });
        for (slot, client) in fixture.clients.iter_mut().enumerate() {
            while let Some(event) = client.result() {
                results[slot].push(event);
            }
        }
    }

    for links in &fixture.links {
        assert!(fixture.host.stream(links[1].id()).unwrap().late_frames > 0);
    }
    fixture.host.stop();
    fixture.finish(240);
    let host_log = fixture.host.world().log();
    for (slot, client) in fixture.clients.iter_mut().enumerate() {
        while let Some(event) = client.result() {
            results[slot].push(event);
        }
        assert_eq!(client.world().log(), host_log);
        assert_eq!(client.world().digest(), fixture.host.world().digest());
        let own = host_log
            .iter()
            .filter(|(_, owner, _)| Some(*owner) == client.slot())
            .collect::<Vec<_>>();
        assert_eq!(
            own.iter().map(|(_, _, value)| *value).collect::<Vec<_>>(),
            [
                10 + u16::try_from(slot).expect("four players"),
                20 + u16::try_from(slot).expect("four players"),
                30 + u16::try_from(slot).expect("four players")
            ]
        );
        let resolved = results[slot]
            .iter()
            .filter_map(|event| {
                if let Input::Command(sequence) = event.input {
                    Some((sequence, event.disposition))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            resolved,
            own.into_iter()
                .enumerate()
                .map(|(index, (tick, _, _))| (index as u64 + 1, Disposition::Committed(*tick)))
                .collect::<Vec<_>>()
        );
        let (target, sequence) = timed[slot].expect("authored during the stall");
        assert!((116..140).contains(&target));
        assert!(
            results[slot]
                .iter()
                .any(|event| event.input == Input::Frame(sequence)
                    && event.disposition == Disposition::Refused)
        );
    }
}

#[test]
fn a_full_timed_frame_defers_commands_to_the_next_barrier_in_order() {
    let mut fixture = Match::new(local(), 1, None, false);
    fixture.ready();
    let target = 40;
    let mut issued = false;
    while fixture.host.world().tick() < target + 4 {
        if !issued && fixture.host.world().tick() == target - 1 {
            fixture.clients[0].command(&901).unwrap();
            fixture.clients[0].command(&902).unwrap();
            issued = true;
        }
        fixture.poll(target + 4, |_, tick| {
            let actions = if tick == target {
                (100..108).collect()
            } else {
                Vec::new()
            };
            Controls {
                axes: [0.0; 3],
                actions,
            }
        });
    }
    fixture.host.stop();
    fixture.finish(target + 4);
    let expected = (100..108)
        .map(|action| (target, 0, action))
        .chain([(target + 1, 0, 901), (target + 1, 0, 902)])
        .collect::<Vec<_>>();
    assert_eq!(fixture.host.world().log(), expected);
    assert_eq!(fixture.clients[0].world().log(), expected);
}
