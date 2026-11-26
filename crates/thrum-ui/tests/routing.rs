//! Interface routing proof on the smallest usable tree.

mod input_support;

use std::collections::BTreeMap;
use std::time::Duration;

use input_support::{NOW, SCOPE, button, press, tree};
use thrum_ui::{Contact, Direction, Event, Held, Intent, Routed, Router, Scope};

#[derive(Default)]
struct Game {
    held: BTreeMap<Contact<u8>, Option<[f64; 2]>>,
    interruptions: usize,
}

impl Game {
    fn accept(&mut self, routed: Option<Routed<u8>>, exclusive: bool) {
        match routed {
            Some(Routed::Game(event)) => {
                assert!(!exclusive, "exclusive scope leaked {event:?}");
                match event {
                    Event::Press {
                        contact, position, ..
                    } => {
                        assert!(self.held.insert(contact, position).is_none());
                    }
                    Event::Release(contact, _) | Event::Cancel(contact) => {
                        assert!(self.held.remove(&contact).is_some());
                    }
                    Event::Move(contact, position) => {
                        *self.held.get_mut(&contact).expect("move without press") = Some(position);
                    }
                    Event::Repeat(contact) => assert!(self.held.contains_key(&contact)),
                    event => panic!("unexpected routed event: {event:?}"),
                }
            }
            Some(Routed::Interrupted) => {
                self.interruptions += 1;
                self.held.clear();
            }
            Some(Routed::Resumed(held)) => {
                assert!(!exclusive);
                for Held { contact, position } in held {
                    assert!(self.held.insert(contact, position).is_none());
                }
            }
            _ => {}
        }
    }
}

#[test]
fn generated_interleavings_match_the_game_contacts_after_every_event() {
    let layout = tree(vec![button(10, [60.0; 2], [50.0; 2])]).input_layout();
    let mut random = 43_u64;
    // 20,000 independent sequences include duplicate presses, orphan releases,
    // cancellation, contact motion, focus loss and nested exclusive scopes.
    for _ in 0..20_000 {
        let mut router = Router::default();
        router.submitted(layout.clone());
        let mut game = Game::default();
        let mut down = BTreeMap::new();
        let mut depth = 0;
        let mut deadlines = [NOW; 2];
        let mut interrupts = 0;
        for step in 0..40 {
            let now = NOW + Duration::from_millis(step);
            assert_eq!(router.advance(now), None);
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let contact = match (random >> 32) % 5 {
                0 => Contact::Key(0),
                1 => Contact::Key(1),
                2 => Contact::Pointer(0),
                3 => Contact::Touch(7),
                _ => Contact::Touch(8),
            };
            let point = if random & (1 << 40) == 0 {
                [90.0; 2]
            } else {
                [12.0; 2]
            };
            let routed = match (random >> 48) % 10 {
                0..=2 => {
                    let intent = (contact == Contact::Key(1)).then_some(Intent::Activate);
                    let position = (!matches!(contact, Contact::Key(_))).then_some(point);
                    let used = depth > 0
                        && now >= deadlines[(depth - 1) as usize]
                        && (intent.is_some() || depth == 1 && position == Some([90.0; 2]));
                    down.entry(contact).or_insert((position, used));
                    router.route(press(contact, position, intent), now)
                }
                3 => {
                    down.remove(&contact);
                    router.route(Event::Release(contact, None), now)
                }
                4 if !matches!(contact, Contact::Key(_)) => {
                    if let Some((position, _)) = down.get_mut(&contact) {
                        *position = Some(point);
                    }
                    router.route(Event::Move(contact, point), now)
                }
                5 if depth < 2 => {
                    let id = SCOPE + depth;
                    let armed_at = now + Duration::from_millis((random >> 24) % 5);
                    deadlines[depth as usize] = armed_at;
                    interrupts += usize::from(depth == 0);
                    depth += 1;
                    router.open(id, Scope::default(), armed_at)
                }
                6 if depth > 0 => {
                    depth -= 1;
                    router.close(SCOPE + depth)
                }
                7 => {
                    down.clear();
                    interrupts += 1;
                    router.route(Event::Interrupted, now)
                }
                8 => {
                    down.remove(&contact);
                    router.route(Event::Cancel(contact), now)
                }
                _ => router.route(Event::Repeat(contact), now),
            };
            game.accept(routed, depth != 0);
            let expected = down
                .iter()
                .filter_map(|(&contact, &(position, used))| {
                    (depth == 0 && !used).then_some((contact, position))
                })
                .collect::<BTreeMap<_, _>>();
            assert_eq!(game.held, expected);
            assert_eq!(game.interruptions, interrupts);
        }
    }
}

#[test]
fn a_direction_pressed_before_arming_is_handed_back_if_still_down() {
    let mut router = Router::default();
    router.submitted(tree(vec![button(10, [60.0; 2], [50.0; 2])]).input_layout());
    let direction = Contact::Key(0);
    let before = Duration::ZERO;
    assert_eq!(
        router.open(SCOPE, Scope::default(), NOW),
        Some(Routed::Interrupted)
    );
    assert_eq!(
        router.route(
            press(direction, None, Some(Intent::Direction(Direction::Right))),
            before
        ),
        None
    );
    assert_eq!(router.advance(NOW), None);
    assert_eq!(router.route(Event::Repeat(direction), NOW), None);
    assert_eq!(
        router.close(SCOPE),
        Some(Routed::Resumed(vec![Held {
            contact: direction,
            position: None
        }]))
    );
    assert!(matches!(
        router.route(Event::Release(direction, None), NOW),
        Some(Routed::Game(_))
    ));
}

#[test]
fn direction_and_stick_resume_but_the_choice_contact_does_not() {
    for (held, start, chosen) in [
        (Contact::Key(0), None, Contact::Key(1)),
        (Contact::Touch(4), Some([9.0, 10.0]), Contact::Touch(5)),
    ] {
        let mut router = Router::default();
        router.submitted(tree(vec![button(10, [60.0; 2], [50.0; 2])]).input_layout());
        assert!(matches!(
            router.route(press(held, start, None), NOW),
            Some(Routed::Game(_))
        ));
        assert_eq!(
            router.open(SCOPE, Scope::default(), NOW),
            Some(Routed::Interrupted)
        );
        let position = start.map(|_| [12.0, 14.0]);
        if let Some(position) = position {
            assert_eq!(router.route(Event::Move(held, position), NOW), None);
            router.route(press(chosen, Some([80.0; 2]), None), NOW);
            assert_eq!(
                router.route(Event::Release(chosen, Some([80.0; 2])), NOW),
                Some(Routed::Activated(10))
            );
        } else {
            assert_eq!(router.route(Event::Repeat(held), NOW), None);
            assert_eq!(
                router.route(press(chosen, None, Some(Intent::Activate)), NOW),
                Some(Routed::Activated(10))
            );
        }
        assert_eq!(
            router.close(SCOPE),
            Some(Routed::Resumed(vec![Held {
                contact: held,
                position
            }]))
        );
        assert_eq!(router.route(Event::Release(chosen, None), NOW), None);
        assert!(matches!(
            router.route(Event::Release(held, position), NOW),
            Some(Routed::Game(_))
        ));
    }
}

#[test]
fn losing_focus_cancels_the_button_and_drops_late_releases() {
    let mut router = Router::default();
    router.submitted(tree(vec![button(10, [60.0; 2], [50.0; 2])]).input_layout());
    let touch = Contact::Touch(5);
    router.open(SCOPE, Scope::default(), NOW);
    router.route(press(touch, Some([80.0; 2]), None), NOW);
    assert_eq!(
        router.route(Event::Interrupted, NOW),
        Some(Routed::Interrupted)
    );
    assert_eq!(
        router.route(Event::Release(touch, Some([80.0; 2])), NOW),
        None
    );
    assert_eq!(router.close(SCOPE), Some(Routed::Resumed(vec![])));
}
