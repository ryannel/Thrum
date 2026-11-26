//! Interface drops proof on the smallest usable tree.

#[expect(dead_code, reason = "drop layout needs no button or contact press")]
mod input_support;

#[path = "layout_support/font.rs"]
mod font;

use input_support::{NOW, SCOPE, tree};
use thrum_ui::{
    Alignment, Elision, Event, Node, NodeKind, Routed, Router, Scope, TextOptions, TextRun,
    TextSource,
};

#[test]
fn drops_are_whole_and_source_tagged_or_refused_and_counted() {
    let ticket = format!("start-{}-end", "abc".repeat(30));
    let mut router = Router::<u8>::default();
    let event = Event::Text(ticket.clone(), TextSource::Launch);
    assert_eq!(router.route(event.clone(), NOW), Some(Routed::Game(event)));
    router.submitted(tree(vec![Node::new(10, NodeKind::DropTarget(100))]).input_layout());
    router.open(SCOPE, Scope::default(), NOW);
    assert_eq!(ticket.chars().count(), 100);
    for source in [TextSource::Launch, TextSource::Link, TextSource::Paste] {
        let routed = router.route(Event::Text(ticket.clone(), source), NOW);
        assert_eq!(routed, Some(Routed::Text(10, ticket.clone(), source)));
    }
    assert_eq!(
        router.route(Event::Text(format!("{ticket}x"), TextSource::Paste), NOW),
        None
    );
    assert_eq!(
        router.route(Event::Text("A\u{7f}B".to_owned(), TextSource::Paste), NOW),
        None
    );
    assert_eq!(router.refused_drops(), 2);
}

#[test]
fn a_hundred_character_ticket_keeps_both_ends_and_copies_the_entire_value() {
    let ticket = format!("AB{}CD", "1234".repeat(24));
    let font = font::pixel();
    let options = TextOptions {
        points: 8.0,
        scale: 1,
        extent: [56, 8],
        alignment: Alignment::Left,
        wrap: false,
        elision: Elision::Middle,
        tabular_digits: false,
        outlined: false,
    };
    let mut router = Router::<u8>::default();
    router.submitted(tree(vec![Node::new(10, NodeKind::DropTarget(100))]).input_layout());
    router.open(SCOPE, Scope::default(), NOW);
    let Some(Routed::Text(_, received, TextSource::Paste)) =
        router.route(Event::Text(ticket.clone(), TextSource::Paste), NOW)
    else {
        panic!("ticket was not handed to the game");
    };
    let visible = TextRun::new(font, &received, options).expect("cooked pixel strike");
    let strike = font.strike(8.0, 1).expect("8-point strike");
    let frame = |character| strike.glyph(character).expect("cooked glyph").frame;
    let frames = visible
        .glyphs()
        .iter()
        .map(|glyph| glyph.frame)
        .collect::<Vec<_>>();
    assert!(visible.elided());
    assert!(frames.starts_with(&[frame('A'), frame('B')]));
    assert!(frames.ends_with(&[frame('C'), frame('D')]));
    assert!(frames.contains(&frame('…')));
    router.request_copy(received);
    assert_eq!(router.take_copy(), Some(ticket));
    assert_eq!(router.take_copy(), None);
}
