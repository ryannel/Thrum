//! Interface focus proof on the smallest usable tree.

mod input_support;

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use input_support::{NOW, SCOPE, button, layout, press, tree, viewport};
use thrum_ui::{Contact, Direction, Event, Intent, Node, NodeKind, Routed, Router, Scope, Size};

const DIRECTIONS: [(Direction, Direction); 4] = [
    (Direction::Left, Direction::Right),
    (Direction::Right, Direction::Left),
    (Direction::Up, Direction::Down),
    (Direction::Down, Direction::Up),
];

fn key(router: &mut Router<u8>, intent: Intent) -> Option<Routed<u8>> {
    let contact = Contact::Key(0);
    let result = router.route(press(contact, None, Some(intent)), NOW);
    assert_eq!(router.route(Event::Release(contact, None), NOW), None);
    result
}

#[test]
fn the_entire_grid_family_is_reachable_and_every_move_reverses() {
    for wide in 1..=6 {
        for tall in 1..=4 {
            for (size, gap) in [([20.0, 60.0], 1.0), ([52.0, 22.0], 17.0), ([44.0; 2], 23.0)] {
                let mut nodes = vec![];
                for y in 0..tall {
                    for x in 0..wide {
                        nodes.push(button(
                            10 + y * wide + x,
                            [
                                f64::from(x) * (size[0] + gap),
                                f64::from(y) * (size[1] + gap),
                            ],
                            size,
                        ));
                    }
                }
                let tree = tree(nodes);
                let snapshot = tree.input_layout();
                let mut pending = VecDeque::from([10]);
                let mut visited = BTreeSet::new();
                while let Some(from) = pending.pop_front() {
                    if !visited.insert(from) {
                        continue;
                    }
                    let mut router = Router::default();
                    router.submitted(snapshot.clone());
                    router.open(
                        SCOPE,
                        Scope {
                            initial: Some(from),
                            ..Scope::default()
                        },
                        NOW,
                    );
                    for (forward, back) in DIRECTIONS {
                        key(&mut router, Intent::Direction(forward));
                        let to = router.focus().expect("grid focus");
                        if to != from {
                            pending.push_back(to);
                            key(&mut router, Intent::Direction(back));
                            assert_eq!(
                                router.focus(),
                                Some(from),
                                "{wide}×{tall}, {from} via {to}"
                            );
                        }
                    }
                }
                assert_eq!(visited.len(), (wide * tall) as usize);
            }
        }
    }
}

#[test]
fn named_ragged_grid_neighbours_win_and_scope_restores_focus() {
    let mut nodes = vec![];
    for (id, x, y) in [
        (10, 0.0, 0.0),
        (11, 80.0, 0.0),
        (12, 160.0, 0.0),
        (13, 0.0, 80.0),
        (14, 80.0, 80.0),
    ] {
        nodes.push(button(id, [x, y], [64.0; 2]));
    }
    if let NodeKind::Button(button) = &mut nodes[2].kind {
        button.neighbours[3] = Some(13);
    }
    if let NodeKind::Button(button) = &mut nodes[3].kind {
        button.neighbours[2] = Some(12);
    }
    let mut nested = Node::new(2, NodeKind::Scope(Scope::default()));
    nested.size = [Size::fixed(100.0); 2];
    nested.children.push(button(20, [0.0; 2], [61.0; 2]));
    nodes.push(nested);
    let mut tree = tree(nodes);
    let mut router = Router::default();
    router.submitted(tree.input_layout());
    router.open(
        SCOPE,
        Scope {
            initial: Some(12),
            ..Scope::default()
        },
        NOW,
    );
    key(&mut router, Intent::Direction(Direction::Down));
    assert_eq!(router.focus(), Some(13));
    key(&mut router, Intent::Direction(Direction::Up));
    assert_eq!(router.focus(), Some(12));
    for id in [13, 14, 10, 11, 12] {
        key(&mut router, Intent::Next);
        assert_eq!(router.focus(), Some(id));
    }
    for id in [11, 10, 14, 13, 12] {
        key(&mut router, Intent::Previous);
        assert_eq!(router.focus(), Some(id));
    }
    assert_eq!(router.open(2, Scope::default(), NOW), None);
    assert_eq!(router.focus(), Some(20));
    key(&mut router, Intent::Next);
    assert_eq!(router.focus(), Some(20));
    assert_eq!(router.close(2), None);
    assert_eq!(router.focus(), Some(12));
    tree.set_enabled(12, false);
    tree.set_shown(10, false);
    layout(&mut tree, viewport());
    router.submitted(tree.input_layout());
    assert_eq!(router.focus(), Some(11));
}

#[test]
fn an_already_held_or_disarmed_key_does_not_act_and_activate_does_not_repeat() {
    let mut router = Router::default();
    router.submitted(
        tree(vec![
            button(10, [0.0; 2], [50.0; 2]),
            button(11, [60.0, 0.0], [50.0; 2]),
        ])
        .input_layout(),
    );
    let direction = Contact::Key(0);
    let activate = Contact::Key(1);
    let binding = Some(Intent::Direction(Direction::Right));
    router.route(press(direction, None, binding), Duration::ZERO);
    router.open(SCOPE, Scope::default(), NOW);
    for now in [Duration::ZERO, NOW] {
        assert_eq!(router.route(Event::Repeat(direction), now), None);
        assert_eq!(router.focus(), Some(10));
    }
    router.route(
        press(activate, None, Some(Intent::Activate)),
        Duration::ZERO,
    );
    assert_eq!(router.route(Event::Repeat(activate), NOW), None);
    router.route(Event::Release(activate, None), NOW);
    assert_eq!(
        router.route(press(activate, None, Some(Intent::Activate)), NOW),
        Some(Routed::Activated(10))
    );
    assert_eq!(router.route(Event::Repeat(activate), NOW), None);
    router.route(Event::Release(direction, None), NOW);
    router.route(press(direction, None, binding), NOW);
    assert_eq!(router.focus(), Some(11));
}
