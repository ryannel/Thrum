mod driver;

use driver::Match;
use serde_json::{Value, json};
use thrum_network::session::{ClientState, Close, Disposition, Input, Refusal, Settings, World};
use thrum_transport::{CloseReason, Lane, Profile};

use super::world::{Time, fault_world::Fault};

fn base(count: u32, players: usize, settings: Settings, measured: bool) -> Match {
    let mut game = Match::new(count, settings, measured);
    for _ in 0..players {
        game.add(Profile::default(), None);
    }
    game.until(180, |game| (0..players).all(|index| game.joined(index)));
    game
}

fn finish_and_compare(game: &mut Match, peers: &[usize], after: u64) -> Vec<usize> {
    let through = game.tick();
    game.finish();
    peers
        .iter()
        .map(|&index| game.compare(index, after, through, None))
        .collect()
}

fn join(count: u32, at: u64) -> Value {
    let mut game = base(count, 3, Settings::default(), false);
    game.advance_to(at);
    let peer = game.add(Profile::default(), None);
    while !game.joined(peer) {
        let prior = game.tick();
        game.advance();
        let slot = 3;
        let joined_now = game.host.roster().occupied & (1 << slot) != 0;
        assert!(
            joined_now || !game.host.admissions()[slot],
            "no input from reserved slot at {} before its join",
            prior + 1
        );
        assert!(game.tick() < at + 240, "late join missed the proof window");
    }
    game.advance_to(game.tick() + 90);
    let through = game.tick();
    game.finish();
    let arrival = game.event(peer, at, |r| r.joined);
    let (baseline, digest, bytes) = game.players[peer].world.borrow().restores[0];
    let matches = game.compare(peer, baseline, through, None);
    assert!(baseline > at && arrival > baseline && matches > 0);
    let first = game.players[peer].world.borrow().crowd.digests.borrow()[0];
    let others: Vec<_> = (0..peer)
        .map(|index| {
            let baseline = game.players[index].world.borrow().restores[0].0;
            game.compare(index, baseline, through, None)
        })
        .collect();
    json!({"schedule": game.scheduled(), "bodies": count, "added_at": at, "join_tick": arrival,
        "baseline_tick": baseline, "baseline_digest": digest, "image_bytes": bytes,
        "catch_up_ticks_through_join": arrival - baseline, "first_digest_tick": first,
        "early_admissions": 0, "matching_digests": matches, "other_peers": others})
}

fn reconnect(count: u32) -> Value {
    let settings = Settings {
        grace: 200,
        held_expiry: 300,
        ..Settings::default()
    };
    let mut game = base(count, 2, settings, false);
    game.advance_to(90);
    let player = 1;
    let slot = game.players[player].slot();
    let old_host = game.players[player].host_link();
    let old_client = game.players[player].links[0].id();
    let old_epoch = game.players[player]
        .client
        .epoch()
        .expect("negotiated epoch");
    let old_input = game.players[player].input.clone();
    let old_sets = game.players[player].sets.clone();
    assert!(!old_input.is_empty() && !old_sets.is_empty());
    let token = game.players[player].client.token().expect("issued token");
    let cut_at = game.tick();
    game.players[player].carrier.cut(true);
    let command = game.players[player]
        .client
        .command(&17)
        .expect("fault scenario witness");
    game.until(310, |game| {
        matches!(
            game.players[player].client.state(),
            ClientState::Closed(CloseReason::TimedOut)
        )
    });
    let away = game.event(player, cut_at, |r| r.away);
    assert_eq!(
        game.world.borrow().steps[usize::try_from(away - 1).expect("short trace")].axes[slot]
            .map(f32::to_bits),
        [0; 3]
    );
    assert_ne!(
        game.world.borrow().steps[usize::try_from(away - 2).expect("short trace")].axes[slot]
            .map(f32::to_bits),
        [0; 3]
    );
    assert!(
        game.players[player]
            .commands
            .iter()
            .any(|event| event.input == Input::Command(command)
                && event.disposition == Disposition::Uncertain)
    );
    let returned = game.add(Profile::default(), Some(token));
    game.until(150, |game| game.joined(returned));
    let back = game.event(returned, away, |r| r.back);
    let new_host = game.players[returned].host_link();
    let new_client = game.players[returned].links[0].id();
    let new_epoch = game.players[returned]
        .client
        .epoch()
        .expect("negotiated epoch");
    assert_eq!(game.players[returned].slot(), slot);
    assert_ne!((new_host, new_client), (old_host, old_client));
    assert_ne!(new_epoch, old_epoch);
    assert_eq!(game.players[returned].commands, []);
    assert_eq!(
        game.players[returned].links[0]
            .counters(game.now / 1_000)
            .lanes[0]
            .sent,
        1,
        "the replacement sent its hello only"
    );
    let returning_started = game.players[returned].world.borrow().restores[0];

    // Closed transport links discard both queues by contract. Replay the exact
    // bytes captured at the old transport boundary into the replacement
    // carrier's stalled stack, and release the stack after the epoch changes.
    let stalled = game.tick();
    let now = game.now / 1_000;
    let peer = &mut game.players[returned];
    peer.carrier.stall(true);
    peer.links[0]
        .send(Lane::Datagram, &old_input, now)
        .expect("fault scenario witness");
    peer.links[1]
        .send(Lane::Datagram, &old_sets, now)
        .expect("fault scenario witness");
    peer.carrier.advance(now);
    assert!(
        peer.links
            .iter()
            .all(|link| link.counters(now).stack_bytes.unwrap_or(0) > 0)
    );
    game.advance_to(stalled + 30);
    assert_eq!(game.players[returned].client.stale_epoch(), 0);
    assert_eq!(
        game.host
            .stream(new_host)
            .expect("fault scenario witness")
            .stale_epoch,
        0
    );
    game.players[returned].carrier.stall(false);
    game.until(30, |game| {
        game.players[returned].client.stale_epoch() > 0
            && game
                .host
                .stream(new_host)
                .expect("fault scenario witness")
                .stale_epoch
                > 0
    });
    let stale = json!({"stalled_at": stalled, "released_at": stalled + 30,
        "received_by_tick": game.tick(), "input": game.host.stream(new_host).expect("fault scenario witness").stale_epoch,
        "sets": game.players[returned].client.stale_epoch()});

    let drop_again = game.tick() + 60;
    game.advance_to(drop_again);
    game.players[returned].links[0].close(89);
    game.until(20, |game| game.host.roster().away & (1 << slot) != 0);
    let without = game.add(Profile::default(), None);
    game.until(150, |game| game.joined(without));
    assert_ne!(
        game.players[without].slot(),
        slot,
        "old player still has the slot"
    );
    let fresh_join = game.event(without, drop_again, |r| r.joined);
    game.until(u64::from(settings.grace) + 20, |game| {
        game.host.roster().occupied & (1 << slot) == 0
    });
    let left = game.event(returned, drop_again, |r| r.left);
    let expired = game.add(Profile::default(), Some(token));
    game.until(150, |game| game.joined(expired));
    assert_eq!(
        game.players[expired].slot(),
        slot,
        "the slot is free for any new player"
    );
    assert!(game.players[expired].client.token() != Some(token));
    let expired_join = game.event(expired, left, |r| r.joined);
    game.advance_to(game.tick() + 45);
    let through = game.tick();
    game.finish();
    assert_eq!(game.event(player, cut_at, |r| r.away), away);
    game.neutral_at(slot, away);
    let steady = game.compare(0, 100, through, None);
    let returned_digests = game.compare(returned, returning_started.0, drop_again - 15, None);
    assert!(returned_digests > 0);
    json!({"schedule": game.scheduled(), "slot": slot, "cut_tick": cut_at,
        "away_tick": away, "neutral_tick": away, "back_tick": back, "returned_baseline": returning_started,
        "links": {"old": [old_client, old_host], "new": [new_client, new_host]},
        "epochs": [old_epoch, new_epoch], "old_epoch_refusals": stale,
        "uncertain_command": command, "resent_commands": game.players[returned].commands.len(),
        "without_token": {"slot": game.players[without].slot(), "join": fresh_join},
        "expired_token": {"slot": game.players[expired].slot(), "join": expired_join, "left": left},
        "steady_digests": steady, "returning_digests": returned_digests})
}

fn repair_one(count: u32, interval: u16, fault: Fault) -> Value {
    let mut game = base(
        count,
        3,
        Settings {
            digest_interval: interval,
            ..Settings::default()
        },
        false,
    );
    game.advance_to(85);
    let player = 1;
    let slot = game.players[player].slot();
    let host_link = game.players[player].host_link();
    let link = game.players[player].links[0].id();
    let epoch = game.players[player]
        .client
        .epoch()
        .expect("negotiated epoch");
    let injected = game.players[player].client.world().tick();
    let expected_detection = if fault.part().is_some() {
        (injected / u64::from(interval) + 1) * u64::from(interval)
    } else {
        injected + 1
    };
    game.players[player]
        .world
        .borrow_mut()
        .change(fault)
        .expect("replica world changes");
    game.until(45, |game| {
        game.players[player].client.state() == &ClientState::Repairing
    });
    let stopped = game.players[player].client.world().tick();
    assert_eq!(
        stopped,
        expected_detection - u64::from(fault.part().is_none())
    );
    let admissions = game.players[player].admissions;
    let authored = game.players[player].client.last_frame().1;
    let position = game.world.borrow().position(slot);
    game.until(140, |game| {
        game.players[player].world.borrow().restores.len() == 2
    });
    let admitted_during_repair = game.players[player].admissions - admissions;
    let authored_during_repair = game.players[player].client.last_frame().1 - authored;
    assert!(authored_during_repair > 0);
    assert!(
        game.players[player].admissions > admissions,
        "host admitted input during repair"
    );
    assert_ne!(
        game.world.borrow().position(slot),
        position,
        "hero moved during transfer"
    );
    assert_eq!(game.players[player].slot(), slot);
    assert_eq!(game.players[player].client.epoch(), Some(epoch));
    assert_eq!(game.players[player].links[0].id(), link);
    assert_eq!(game.players[player].host_link(), host_link);
    let (baseline, digest, bytes) = game.players[player].world.borrow().restores[1];
    assert!(baseline > expected_detection && bytes > 0);
    game.advance_to(game.tick() + 70);
    let through = game.tick();
    game.finish();
    let comparisons = [
        game.compare(0, 50, through, None),
        game.compare(player, baseline, through, None),
        game.compare(2, 50, through, None),
    ];
    let [report] = game.diagnostics.as_slice() else {
        panic!("one divergence report")
    };
    assert_eq!(
        (report.link, report.slot, report.tick),
        (host_link, slot, expected_detection)
    );
    if let Some(part) = fault.part() {
        assert!(report.expected.is_some() && report.actual.is_some());
        assert_ne!(report.expected, report.actual);
        assert_eq!(report.parts, [part]);
        let observed = game.players[player]
            .world
            .borrow()
            .crowd
            .digests
            .borrow()
            .iter()
            .find(|row| row.0 == expected_detection)
            .expect("fault scenario witness")
            .1;
        assert_eq!(Some(observed), report.actual);
    } else {
        assert_eq!(report.actual, None);
    }
    json!({"schedule": game.scheduled(), "bodies": count, "digest_interval": interval,
        "fault": format!("{fault:?}"), "injected_after": injected, "stopped": stopped,
        "link": link, "host_link": host_link, "slot": slot, "epoch": epoch, "detection_tick": report.tick,
        "authority": report.expected, "replica": report.actual,
        "differing_parts": report.parts.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "restored": [baseline, digest, bytes as u64], "admissions_before_repair": admissions,
        "admissions_while_repairing": admitted_during_repair,
        "authored_while_repairing": authored_during_repair,
        "moved_during_repair": true, "digest_comparisons": comparisons})
}

fn repair_limit(count: u32) -> Value {
    let settings = Settings {
        digest_interval: 1,
        repairs: 2,
        ..Settings::default()
    };
    let mut game = base(count, 2, settings, false);
    game.advance_to(75);
    for repairs in 0..=settings.repairs {
        game.players[1]
            .world
            .borrow_mut()
            .change(Fault::Resource)
            .expect("fault scenario witness");
        if repairs < settings.repairs {
            game.until(180, |game| {
                game.players[1].world.borrow().restores.len() == usize::from(repairs) + 2
            });
        }
    }
    game.until(90, |game| closed(game, 1, Close::RepairLimit));
    assert_eq!(game.diagnostics.len(), usize::from(settings.repairs) + 1);
    let close = game.tick();
    game.advance_to(close + 60);
    let others = finish_and_compare(&mut game, &[0], 45);
    json!({"close": "repair_limit", "tick": close, "repairs": settings.repairs,
        "reports": game.diagnostics.iter().map(|report| report.tick).collect::<Vec<_>>(),
        "other_peer_comparisons": others})
}

fn closed(game: &Match, player: usize, reason: Close) -> bool {
    matches!(game.players[player].client.state(), ClientState::Closed(CloseReason::Peer(code)) if *code == reason as u32)
}

fn stalled(count: u32) -> Value {
    let settings = Settings {
        backlog: 80,
        ..Settings::default()
    };
    let mut game = base(count, 3, settings, false);
    game.advance_to(75);
    let player = 1;
    let stalled_at = game.tick();
    game.players[player].carrier.stall(true);
    game.advance_to(stalled_at + 120);
    assert!(game.players[player].links[0].is_open());
    assert_eq!(
        game.players[player].most_sets,
        usize::from(settings.backlog)
    );
    let healed = game.tick();
    game.players[player].carrier.stall(false);
    game.until(170, |game| {
        game.players[player].world.borrow().restores.len() == 2
    });
    let baseline = game.players[player].world.borrow().restores[1].0;
    let recovered = game.tick();
    assert!(baseline >= healed);
    game.advance_to(recovered + 75);
    let through = game.tick();
    let others = finish_and_compare(&mut game, &[0, 2], 50);
    let repaired = game.compare(player, baseline, through, None);
    json!({"schedule": game.scheduled(), "stalled_at": stalled_at, "healed_at": healed,
        "reason": "ring_overrun", "repair_baseline": baseline, "matching_digests": repaired,
        "other_peer_comparisons": others, "most_sets": game.players[player].most_sets,
        "most_bytes": game.players[player].most_bytes, "ring_cap": settings.backlog})
}

fn quiet(count: u32, nonreader: bool) -> Value {
    let settings = Settings {
        response_ms: 500,
        join_ms: 1_800,
        held_expiry: 22,
        absence: 220,
        ..Settings::default()
    };
    let mut game = base(count, 2, settings, false);
    let player = game.add(
        Profile {
            reliable_bytes: 65_536,
            ..Profile::default()
        },
        None,
    );
    game.until(130, |game| game.joined(player));
    game.advance_to(80.max(game.tick() + 12));
    let slot = game.players[player].slot();
    let quiet_at = game.tick();
    if nonreader {
        // A real replica first requests a repair. Leave its link open as a
        // receiver that no longer drains the bounded reliable lane.
        game.players[player]
            .world
            .borrow_mut()
            .change(Fault::Resource)
            .expect("fault scenario witness");
        game.until(75, |game| !game.diagnostics.is_empty());
        game.players[player].poll = false;
        game.until(160, |game| !game.players[player].links[1].is_open());
    } else {
        game.players[player].send = false;
        game.until(u64::from(settings.absence) + 30, |game| {
            !game.players[player].links[1].is_open()
        });
    }
    let peer = &game.players[player];
    let counters = peer.links[1].counters(game.now / 1_000);
    let reason = if nonreader {
        assert!(counters.refused[2] > 0);
        Close::Synchronization
    } else {
        Close::Absent
    };
    let close_at = game.tick();
    game.players[player].poll = true;
    game.players[player].send = true;
    game.until(10, |game| closed(game, player, reason));
    game.advance_to(close_at + 50);
    let others = finish_and_compare(&mut game, &[0, 1], 45);
    let away = game.event(player, quiet_at, |r| r.away);
    let world = game.world.borrow();
    let rows = &world.steps;
    let neutral = rows
        .iter()
        .find(|row| row.tick > quiet_at && row.axes[slot].map(f32::to_bits) == [0; 3])
        .expect("fault scenario witness")
        .tick;
    assert_eq!(away, close_at);
    game.neutral_at(slot, neutral);
    if !nonreader {
        assert_eq!(
            away - neutral,
            u64::from(settings.absence - settings.held_expiry)
        );
        assert!(
            rows.iter()
                .filter(|row| (neutral..=away).contains(&row.tick))
                .all(|row| row.axes[slot].map(f32::to_bits) == [0; 3])
        );
    }
    json!({"schedule": game.scheduled(), "reason": format!("{reason:?}"), "quiet_at": quiet_at,
        "closed_at": close_at, "neutral_at": neutral, "away_at": away,
        "reliable_full": counters.refused[2], "most_bytes": game.players[player].most_bytes,
        "most_sets": game.players[player].most_sets, "ring_cap": settings.backlog,
        "other_peer_comparisons": others})
}

fn lag(count: u32) -> Value {
    let settings = Settings {
        catch_up_deadline: 110,
        ..Settings::default()
    };
    let mut game = base(count, 3, settings, false);
    game.advance_to(75);
    let slow = 1;
    let since = game.tick();
    let first_replica = game.players[slow].client.world().tick();
    let first_authored = game.players[slow].client.last_frame().1;
    while game.players[slow].links[1].is_open() {
        assert!(game.tick() < since + 310);
        game.players[slow].budget = u8::from(game.tick().is_multiple_of(2));
        game.advance();
    }
    let closed_at = game.tick();
    game.until(10, |game| closed(game, slow, Close::UnableToKeepUp));
    let peer = &game.players[slow];
    let start = peer
        .backlog
        .iter()
        .find(|&&(tick, held, stepped, _)| {
            tick >= since && held - stepped > u64::from(settings.behind)
        })
        .expect("fault scenario witness")
        .0;
    let window: Vec<_> = peer
        .backlog
        .iter()
        .copied()
        .filter(|row| row.0 >= start)
        .collect();
    assert!(closed_at >= start + u64::from(settings.catch_up_deadline) - 1);
    assert!(closed_at <= start + u64::from(settings.catch_up_deadline) + 4);
    assert!(window.iter().all(|&(_, held, stepped, _)| held - stepped
        > u64::from(settings.behind)
        && held - stepped < u64::from(settings.backlog)));
    let ordinary_ring = game.players[0].most_sets.max(game.players[2].most_sets);
    assert!(
        window
            .iter()
            .all(|&(_, _, _, ring)| ring <= ordinary_ring + 4)
    );
    let authored = peer.client.last_frame().1 - first_authored;
    assert!(authored * 10 >= (closed_at - since) * 9);
    let stepped = peer.client.world().tick() - first_replica;
    assert!(stepped * 2 < closed_at - since + 8 && stepped * 2 > closed_at - since - 8);
    let slow_proof = json!({"since": since, "behind_at": start, "closed_at": closed_at,
        "reason": "unable_to_keep_up", "authored": authored, "stepped": stepped,
        "ordinary_ring_high_water": ordinary_ring, "progress": window});

    let recovering = 2;
    let stall = game.tick();
    game.players[recovering].carrier.stall(true);
    game.advance_to(stall + 84);
    game.players[recovering].carrier.stall(false);
    game.players[recovering].budget = 0;
    let floor = u64::from(settings.behind);
    game.until(40, |game| {
        game.players[recovering]
            .backlog
            .last()
            .is_some_and(|&(_, held, stepped, _)| held - stepped > floor)
    });
    let above = game.tick();
    game.players[recovering].budget = settings.catch_up;
    game.until(u64::from(settings.catch_up_deadline) - 1, |game| {
        game.players[recovering]
            .backlog
            .last()
            .is_some_and(|&(_, held, stepped, _)| held - stepped <= floor)
    });
    let below = game.tick();
    let recovering_proof = json!({"stalled_at": stall, "above_at": above, "below_at": below,
        "progress": game.players[recovering].backlog.iter().filter(|row| row.0 >= stall).collect::<Vec<_>>()});
    assert!(below > above);

    let waiting = game.add(
        Profile {
            reliable_bytes: 65_536,
            ..Profile::default()
        },
        None,
    );
    game.players[waiting]
        .carrier
        .recondition(thrum_transport::memory::Conditioner {
            delay_ms: 170,
            jitter_ms: 5,
            ..thrum_transport::memory::Conditioner::default()
        })
        .expect("fault scenario witness");
    game.players[waiting].budget = 0;
    game.until(250, |game| {
        game.players[waiting]
            .backlog
            .last()
            .is_some_and(|&(_, held, stepped, _)| held - stepped > floor)
    });
    let waiting_at = game.tick();
    game.advance_to(waiting_at + u64::from(settings.catch_up_deadline) + 10);
    assert!(!game.joined(waiting) && game.players[waiting].links[1].is_open());
    let waited = game.tick() - waiting_at;
    game.players[waiting].budget = settings.catch_up;
    game.until(150, |game| game.joined(waiting));
    game.advance_to(game.tick() + 55);
    let through = game.tick();
    let others = finish_and_compare(&mut game, &[0, recovering], 45);
    let baseline = game.players[waiting].world.borrow().restores[0].0;
    let joiner_checks = game.compare(waiting, baseline, through, None);
    json!({"schedule": game.scheduled(), "settings": {"threshold": floor,
            "catch_up_deadline": settings.catch_up_deadline, "backlog_cap": settings.backlog},
        "half_pace": slow_proof, "stall_recovery": recovering_proof,
        "joiner": {"above_since": waiting_at, "allowed_ticks": waited,
            "digest_comparisons": joiner_checks},
        "other_peer_comparisons": others})
}

fn join_pair(count: u32) -> Value {
    let settings = Settings {
        response_ms: 700,
        join_ms: 3_100,
        ..Settings::default()
    };
    let mut game = base(count, 2, settings, false);
    game.advance_to(75);
    let profile = Profile {
        reliable_bytes: 65_536,
        ..Profile::default()
    };
    let first = game.add(profile.clone(), None);
    game.until(45, |game| {
        game.players[first].client.baseline_progress().is_some()
    });
    let second = game.add(profile.clone(), None);
    let refused = game.add(Profile::default(), None);
    game.until(40, |game| {
        game.players[second].client.baseline_progress().is_some()
    });
    assert!(game.players[first].client.baseline_progress().is_some());
    let overlap = game.tick();
    let pending = game.players[first]
        .client
        .baseline_progress()
        .expect("fault scenario witness");
    assert!(pending.0 < pending.1);
    game.until(40, |game| {
        game.players[refused].client.state() == &ClientState::Refused(Refusal::Full)
    });
    let refused_at = game.tick();
    assert_eq!(game.host.roster().occupied.count_ones(), 2);
    game.until(170, |game| game.joined(first) && game.joined(second));
    game.advance_to(game.tick() + 65);
    let both_played = game.tick();
    let free_slot = game.players[first].slot();
    game.players[first]
        .client
        .goodbye(game.now)
        .expect("fault scenario witness");
    game.until(45, |game| {
        game.host.roster().occupied & (1 << free_slot) == 0
    });
    let late = game.add(profile, None);
    game.until(45, |game| {
        game.players[late].client.baseline_progress().is_some()
    });
    assert_eq!(game.players[late].slot(), free_slot);
    game.players[late].budget = 0;
    let expired_on = game.tick();
    game.until(240, |game| closed(game, late, Close::Synchronization));
    let expired = game.tick();
    let next = game.add(Profile::default(), None);
    game.until(160, |game| game.joined(next));
    assert_eq!(game.players[next].slot(), free_slot);
    game.advance_to(game.tick() + 55);
    let through = game.tick();
    game.finish();
    let comparable = [0, 1, first, second, next];
    let checked: Vec<_> = comparable
        .iter()
        .map(|&index| {
            let baseline = game.players[index].world.borrow().restores[0].0;
            game.compare(
                index,
                baseline,
                if index == first {
                    both_played - 18
                } else {
                    through
                },
                None,
            )
        })
        .collect();
    let joined: Vec<_> = [first, second]
        .iter()
        .map(|&index| {
            let tick = game.event(index, 75, |r| r.joined);
            let image = game.players[index].world.borrow().restores[0];
            json!({"slot": game.players[index].slot(), "join_tick": tick,
            "baseline_tick": image.0, "image_bytes": image.2,
            "restored_at_host_tick": game.players[index].restored_at[0]})
        })
        .collect();
    assert_ne!(joined[0]["baseline_tick"], joined[1]["baseline_tick"]);
    assert!(joined[1]["baseline_tick"].as_u64() < joined[0]["restored_at_host_tick"].as_u64());
    json!({"schedule": game.scheduled(), "bodies": count, "both_transfers_at": overlap,
        "first_progress_then": pending, "refused_at": refused_at, "refusal": "full",
        "held_slots_at_refusal": 2, "reserved_slots_at_refusal": 2, "joins": joined,
        "deadline_case": {"began_by": expired_on, "closed_at": expired,
            "reserved_slot": free_slot, "next_slot": game.players[next].slot()},
        "digest_comparisons": checked})
}

fn backlog(game: &Match, player: usize, first: u64, end: u64) -> Value {
    let rows = game.players[player]
        .backlog
        .iter()
        .filter(|row| (first..=end).contains(&row.0));
    let mut peak = 0;
    let mut above = 0;
    let mut longest = 0;
    let mut prior = None;
    let boundary = u64::from(game.settings.behind);
    for &(tick, held, stepped, _) in rows {
        let backlog = held - stepped;
        peak = peak.max(backlog);
        if backlog > boundary {
            above = if prior == Some(tick - 1) {
                above + 1
            } else {
                1
            };
        } else {
            above = 0;
        }
        longest = longest.max(above);
        prior = Some(tick);
    }
    json!({"peak": peak, "longest_above_ticks": longest,
        "longest_above_us": longest * u64::from(game.settings.tick_us)})
}

fn join_costs(count: u32) -> Value {
    let settings = Settings::default();
    let mut game = base(count, 1, settings, true);
    game.advance_to(50);
    let peer = game.add(
        Profile {
            reliable_bytes: 131_072,
            ..Profile::default()
        },
        None,
    );
    let added = game.tick();
    game.until(170, |game| game.joined(peer));
    let joined = game.tick();
    let (join_tick, _, bytes) = game.players[peer].world.borrow().restores[0];
    let join_image = game
        .images
        .iter()
        .find(|row| row["baseline_tick"] == join_tick)
        .expect("fault scenario witness")
        .clone();
    let join_catchup = game.players[peer].catchup_ns.json();
    let join_engine_ns = game.players[peer].catchup_engine_ns;
    let join_steps = game.players[peer].catchup_steps;

    game.advance_to(joined + 40);
    let cut = game.tick();
    game.players[peer].carrier.stall(true);
    game.advance_to(cut + 78);
    game.players[peer].carrier.stall(false);
    game.players[peer].budget = 0;
    game.until(35, |game| {
        game.players[peer]
            .backlog
            .last()
            .is_some_and(|&(_, held, stepped, _)| held - stepped > u64::from(settings.behind))
    });
    game.advance_to(game.tick() + 12);
    game.players[peer].budget = settings.catch_up;
    game.until(130, |game| {
        game.players[peer]
            .backlog
            .last()
            .is_some_and(|&(_, held, stepped, _)| held - stepped <= u64::from(settings.behind))
    });
    let caught = game.tick();

    game.advance_to(caught + 45);
    game.players[peer].catchup_ns = Time::default();
    game.players[peer].catchup_steps = 0;
    game.players[peer].catchup_engine_ns = 0;
    let injected = game.tick();
    game.players[peer]
        .world
        .borrow_mut()
        .change(Fault::Resource)
        .expect("replica resource changes");
    game.until(170, |game| {
        game.players[peer].world.borrow().restores.len() == 2
    });
    let restored_at = game.tick();
    game.until(100, |game| {
        game.players[peer].client.state() == &ClientState::Playing
    });
    let repaired = game.tick();
    let (repair_tick, _, repair_bytes) = game.players[peer].world.borrow().restores[1];
    let repair_image = game
        .images
        .iter()
        .find(|row| row["baseline_tick"] == repair_tick)
        .expect("fault scenario witness")
        .clone();
    let repair_catchup = game.players[peer].catchup_ns.json();
    let repair_engine_ns = game.players[peer].catchup_engine_ns;
    let repair_steps = game.players[peer].catchup_steps;
    let steady = game.host_times.json();
    let now = game.now / 1_000;
    let links = &game.players[peer].links;
    let volume: Vec<_> = links
        .iter()
        .map(|link| {
            link.counters(now)
                .lanes
                .iter()
                .map(|lane| lane.sent_bytes)
                .sum::<u64>()
        })
        .collect();
    game.advance_to(repaired + 60);
    let through = game.tick();
    game.finish();
    let matched = game.compare(peer, repair_tick, through, None);
    json!({"bodies": count, "schedule": game.scheduled(), "ordinary_host_ticks_ns": steady,
        "bytes_by_direction": volume, "measured_until_us": now * 1_000,
        "largest_retention_bytes": game.players[peer].most_bytes, "largest_retention_sets": game.players[peer].most_sets,
        "largest_staging_frames": game.players[peer].max_staged, "largest_link_queue_bytes": game.players[peer].max_link_bytes,
        "join": {"image_bytes": bytes, "write_tick": join_image, "catch_up_steps": join_steps,
            "catch_up_calls_ns": join_catchup, "catch_up_engine_ns": join_engine_ns,
            "restored_to_playing_us": (joined - game.players[peer].restored_at[0]) * u64::from(settings.tick_us),
            "hello_to_playing_us": (joined - added) * u64::from(settings.tick_us),
            "backlog": backlog(&game, peer, added, joined)},
        "repair": {"image_bytes": repair_bytes, "write_tick": repair_image, "catch_up_steps": repair_steps,
            "catch_up_calls_ns": repair_catchup, "catch_up_engine_ns": repair_engine_ns, "restore_to_playing_us": (repaired - restored_at) * u64::from(settings.tick_us),
            "injection_to_playing_us": (repaired - injected) * u64::from(settings.tick_us)},
        "stall_backlog": backlog(&game, peer, cut, caught),
        "matched_after_repair": matched})
}

fn short(count: u32) -> Value {
    let settings = Settings {
        backlog: 82,
        held_expiry: 60,
        grace: 100,
        ..Settings::default()
    };
    let mut game = base(count, 2, settings, false);
    game.advance_to(60);
    let late = game.add(
        Profile {
            idle_ms: 450,
            ..Profile::default()
        },
        None,
    );
    game.until(130, |game| game.joined(late));
    let joined = game.tick();
    game.advance_to(joined + 30);
    let token = game.players[late].client.token().expect("issued token");
    game.players[late].carrier.cut(true);
    game.until(45, |game| {
        matches!(
            game.players[late].client.state(),
            ClientState::Closed(CloseReason::TimedOut)
        )
    });
    let rejoined = game.add(Profile::default(), Some(token));
    game.until(130, |game| game.joined(rejoined));
    assert_eq!(game.players[late].slot(), game.players[rejoined].slot());

    game.advance_to(game.tick() + 30);
    game.players[rejoined]
        .world
        .borrow_mut()
        .change(Fault::Resource)
        .expect("replica resource changes");
    game.until(150, |game| {
        game.players[rejoined].world.borrow().restores.len() == 2
    });
    let bad_tick = game.diagnostics[0].tick;

    let stall = game.tick();
    game.players[1].carrier.stall(true);
    game.advance_to(stall + 110);
    game.players[1].carrier.stall(false);
    game.until(150, |game| {
        game.players[1].world.borrow().restores.len() == 2
    });
    game.advance_to(game.tick() + 65);
    game.finish();
    let compared: Vec<_> = (0..game.players.len())
        .map(|player| game.compare_played(player, (player == rejoined).then_some(bad_tick)))
        .collect();
    assert!(compared.iter().all(|&count| count > 0));
    json!({"schedule": game.scheduled(), "join_reported_by": joined, "repair_detection": bad_tick,
        "per_player_digest_comparisons": compared})
}

/// Run the named fault scenario over independent client worlds and a virtual carrier.
///
/// # Panics
/// A scenario deadline elapsed or an observed invariant failed.
///
/// # Errors
/// The named fault scenario does not exist.
pub fn run_fault(scenario: &str, count: u32) -> Result<Value, &'static str> {
    Ok(match scenario {
        "join" => join(count, 3_600),
        "reconnect" => reconnect(count),
        "repair" => {
            let cases: Vec<_> = [30, 1]
                .into_iter()
                .flat_map(|interval| {
                    [Fault::Component, Fault::FreeList, Fault::Resource]
                        .into_iter()
                        .map(move |fault| repair_one(count, interval, fault))
                })
                .collect();
            json!({"cases": cases, "step_refusal": repair_one(count, 30, Fault::StepRefused),
                "repair_limit": repair_limit(count)})
        }
        "slow" => {
            json!({"stalled": stalled(count), "nonreader": quiet(count, true), "silent": quiet(count, false)})
        }
        "lag" => lag(count),
        "join-pair" => join_pair(count),
        "join-costs" => join_costs(count),
        "short" => short(count),
        _ => return Err("unknown in-memory fault scenario"),
    })
}
