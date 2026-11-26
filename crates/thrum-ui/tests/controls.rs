//! Interface controls proof on the smallest usable tree.

mod input_support;

use input_support::{NOW, SCOPE, button, layout, press, tree, viewport};
use thrum_ui::{ButtonState, Contact, Event, Routed, Router, Scope};

#[test]
fn matching_contact_activates_on_release_and_cannot_reenter_after_sliding_off() {
    let mut tree = tree(vec![
        button(10, [60.0; 2], [50.0; 2]),
        button(11, [120.0, 60.0], [50.0; 2]),
    ]);
    let mut router = Router::default();
    router.submitted(tree.input_layout());
    router.open(SCOPE, Scope::default(), NOW);
    let a = Contact::Pointer(0);
    let b = Contact::Touch(1);
    let p = [85.0; 2];
    router.route(Event::Hover([140.0, 85.0]), NOW);
    assert_eq!(router.button_state(11), ButtonState::Focused);
    assert_eq!(router.route(press(b, Some(p), None), NOW), None);
    assert_eq!(router.button_state(10), ButtonState::Pressed);
    assert_eq!(router.button_state(11), ButtonState::Idle);
    assert_eq!(router.route(Event::Release(a, Some(p)), NOW), None);
    assert_eq!(
        router.route(Event::Release(b, Some(p)), NOW),
        Some(Routed::Activated(10))
    );
    assert_eq!(router.button_state(10), ButtonState::Idle);
    assert_eq!(router.route(Event::Release(b, Some(p)), NOW), None);
    router.route(press(a, Some(p), None), NOW);
    router.route(Event::Move(a, [15.0; 2]), NOW);
    router.route(Event::Move(a, p), NOW);
    assert_eq!(router.route(Event::Release(a, Some(p)), NOW), None);
    for removed in [false, true] {
        router.route(press(a, Some(p), None), NOW);
        if removed {
            tree.set_shown(10, false);
        } else {
            tree.set_enabled(10, false);
        }
        layout(&mut tree, viewport());
        router.submitted(tree.input_layout());
        assert_eq!(router.route(Event::Release(a, Some(p)), NOW), None);
        tree.set_enabled(10, true);
        layout(&mut tree, viewport());
        router.submitted(tree.input_layout());
    }
}

#[test]
fn hit_boxes_grow_to_44_and_overlap_uses_drawn_distance_on_the_displayed_layout() {
    let mut tree = tree(vec![
        button(10, [55.0, 60.0], [10.0; 2]),
        button(11, [75.0, 60.0], [10.0; 2]),
    ]);
    let mut router = Router::default();
    router.submitted(tree.input_layout());
    router.open(SCOPE, Scope::default(), NOW);
    tree.set_anchor(10, [thrum_ui::Anchor::Point(180.0); 2]);
    layout(
        &mut tree,
        thrum_ui::Viewport {
            scale: 2,
            ..viewport()
        },
    );
    let mouse = Contact::Pointer(0);
    for (point, id) in [([38.0, 43.0], 10), ([69.0, 63.0], 10), ([70.0, 63.0], 11)] {
        router.route(press(mouse, Some(point), None), NOW);
        assert_eq!(
            router.route(Event::Release(mouse, Some(point)), NOW),
            Some(Routed::Activated(id))
        );
    }
    router.submitted(tree.input_layout());
    let old = [56.0, 64.0];
    router.route(press(mouse, Some(old), None), NOW);
    assert_eq!(router.route(Event::Release(mouse, Some(old)), NOW), None);
    let new = [183.0; 2];
    router.route(press(mouse, Some(new), None), NOW);
    assert_eq!(
        router.route(Event::Release(mouse, Some(new)), NOW),
        Some(Routed::Activated(10))
    );
}
