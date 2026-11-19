//! The second rule set observes slot identity and lifecycle only through committed sets.

/// Shared test rules and carrier.
pub mod support;

use support::{Match, Rule, local};
use thrum_network::session::{ClientState, Controls, PROTOCOL, Settings, World};

#[test]
fn heroes_respond_to_the_same_four_roster_events_on_authority_and_replicas() {
    let settings = Settings {
        grace: 42,
        held_expiry: 30,
        ..Settings::default()
    };
    let mut game = Match::with_settings(local(), 2, None, false, settings);
    game.ready();
    drive(&mut game, 34);
    let slot = game.clients[1].slot().expect("joined slot");
    let bit = 1 << slot;
    let token = game.clients[1].token().expect("issued token");
    let old_position = game
        .host
        .world()
        .roster_log()
        .last()
        .expect("committed tick")
        .positions[slot];
    assert!(old_position > 0);
    game.links[1][0].close(90);
    drive(&mut game, 52);
    assert_eq!(game.host.roster().away, bit);
    game.add_token(Rule::Add, PROTOCOL, Some(token));
    drive(&mut game, 94);
    assert_eq!(game.clients[2].slot(), Some(slot));
    assert_eq!(game.host.roster().away, 0);
    let new_position = game
        .host
        .world()
        .roster_log()
        .last()
        .expect("committed tick")
        .positions[slot];
    assert!(new_position > old_position);
    game.links[2][0].close(91);
    drive(&mut game, 155);
    game.add_token(Rule::Add, PROTOCOL, Some(token));
    drive(&mut game, 190);
    assert_eq!(game.clients[3].slot(), Some(slot));
    assert!(game.clients[3].token() != Some(token));
    game.host.stop();
    for _ in 0..500 {
        game.neutral(190);
        if matches!(game.clients[0].state(), ClientState::Ended(_, 190)) {
            break;
        }
    }
    assert_eq!(game.clients[0].world().tick(), 190);
    assert_eq!(game.host.world().digest(), game.clients[0].world().digest());

    let authority = game.host.world().roster_log();
    let observer = game.clients[0].world().roster_log();
    let observer_start = observer
        .iter()
        .find(|row| row.tick > 0)
        .expect("event in committed trace")
        .tick;
    assert!(
        authority
            .iter()
            .filter(|row| row.tick >= observer_start)
            .eq(observer.iter().filter(|row| row.tick >= observer_start))
    );
    let join = authority
        .iter()
        .find(|row| row.events.joined & bit != 0)
        .expect("event in committed trace");
    let away = authority
        .iter()
        .find(|row| row.events.away & bit != 0)
        .expect("event in committed trace");
    let back = authority
        .iter()
        .find(|row| row.events.back & bit != 0)
        .expect("event in committed trace");
    let left = authority
        .iter()
        .find(|row| row.events.left & bit != 0)
        .expect("event in committed trace");
    let hero = join.heroes[slot];
    assert_eq!(hero, join.tick);
    assert_eq!(away.heroes[slot], hero);
    assert_eq!(back.heroes[slot], hero);
    assert_eq!(left.heroes[slot], 0);
    assert_eq!(left.positions[slot], 0);
    let new_character = authority
        .iter()
        .find(|row| row.tick > left.tick && row.events.joined & bit != 0)
        .expect("new player joined on the released slot");
    assert_ne!(new_character.heroes[slot], hero);
    assert_eq!(new_character.heroes[slot], new_character.tick);
    assert!(game.clients[3].world().roster_log().contains(new_character));
    let pre_away = authority
        .iter()
        .find(|row| row.tick + 1 == away.tick)
        .expect("event in committed trace");
    assert!(
        authority
            .iter()
            .filter(|row| (away.tick..back.tick).contains(&row.tick))
            .all(|row| row.positions[slot] == pre_away.positions[slot])
    );
    assert!(
        authority
            .iter()
            .any(|row| row.tick > back.tick && row.positions[slot] > pre_away.positions[slot])
    );
    assert!(game.clients[2].world().roster_log().contains(back));
    assert!(game.clients[1].world().roster_log().contains(join));
    println!(
        "slot {slot}, character {hero}, joined {}, away {}, back {}, left {}, new character at {}",
        join.tick, away.tick, back.tick, left.tick, new_character.tick
    );
}

fn drive(game: &mut Match, tick: u64) {
    while game.host.world().tick() < tick {
        game.poll(tick, |_, _| Controls::axes([1.0, 0.0, 0.0]));
        for client in &mut game.clients {
            while client.result().is_some() {}
        }
    }
}
