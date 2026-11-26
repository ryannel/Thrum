//! Seeded layout and invalidation invariants, independent of the paint backend.

mod layout_support;

use std::{fmt::Write as _, sync::Arc};

use layout_support::{fill, font, inputs, text, view};
use thrum_ui::{Anchor, Axis, Box2, Node, NodeKind, Size, SizeRule, Tree};

struct Random(u32);

impl Random {
    fn get(&mut self, range: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) % range
    }
}

fn generated(rng: &mut Random, depth: u32, id: &mut u32, parent: [Size; 2]) -> Node {
    *id += 1;
    let mut node = match rng.get(if depth == 0 { 3 } else { 5 }) {
        0 => text(*id, "A1 A2 A3", true),
        1 => fill(*id, [10.0; 2]),
        2 | 3 => Node::new(*id, NodeKind::Overlay),
        _ => Node::new(
            *id,
            NodeKind::Stack {
                axis: if rng.get(2) == 0 {
                    Axis::Row
                } else {
                    Axis::Column
                },
                gap: f64::from(rng.get(12)),
                align: Anchor::Centre,
            },
        ),
    };
    for (axis, parent) in parent.into_iter().enumerate() {
        node.size[axis] = Size {
            least: f64::from(rng.get(20)),
            most: f64::from(rng.get(180) + 20),
            rule: match rng.get(4) {
                0 => SizeRule::Fixed(f64::from(rng.get(400))),
                1 => SizeRule::Fit,
                2 if parent.rule != SizeRule::Fit => {
                    SizeRule::Fraction(f64::from(rng.get(11)) / 10.0)
                }
                _ => SizeRule::Share(f64::from(rng.get(12) + 1)),
            },
        };
        node.anchor[axis] = match rng.get(4) {
            0 => Anchor::Start,
            1 => Anchor::End,
            2 => Anchor::Centre,
            _ => Anchor::Point(f64::from(rng.get(120)) - 20.0),
        };
    }
    if matches!(node.kind, NodeKind::Stack { .. } | NodeKind::Overlay) && depth > 0 {
        node.padding = std::array::from_fn(|_| f64::from(rng.get(20)));
        node.children = (0..2 + rng.get(3))
            .map(|_| generated(rng, depth - 1, id, node.size))
            .collect();
    }
    node
}

fn within(child: Box2, parent: Box2) -> bool {
    (0..2).all(|axis| {
        child.size[axis] >= 0.0
            && child.origin[axis] + 1e-8 >= parent.origin[axis]
            && child.end()[axis] <= parent.end()[axis] + 1e-8
    })
}

fn verify(a: &Tree, b: &Tree, node: &Node, safe: Box2) {
    let Some(parent) = a.bounds(node.id) else {
        return;
    };
    assert_eq!(Some(parent), b.bounds(node.id));
    assert!(
        within(parent, safe),
        "unsafe node {} {parent:?} in {safe:?}",
        node.id
    );
    let mut last: Option<Box2> = None;
    for child in &node.children {
        let Some(rect) = a.bounds(child.id) else {
            continue;
        };
        assert!(within(rect, parent) || a.overflows().any(|id| id == child.id));
        if let Some(previous) = last
            && let NodeKind::Stack { axis, .. } = node.kind
        {
            let axis = usize::from(axis == Axis::Column);
            assert!(previous.end()[axis] <= rect.origin[axis] + 1e-8);
        }
        last = Some(rect);
        verify(a, b, child, safe);
    }
}

#[test]
fn nested_boxes_are_deterministic_contained_and_never_overlap_in_stacks() {
    let fonts = font();
    let mut rng = Random(73);
    for index in 0..256 {
        let mut root = Node::new(0, NodeKind::Overlay);
        root.size = [Size::fixed(300.0); 2];
        let mut id = 0;
        root.children = (0..2)
            .map(|_| generated(&mut rng, 4, &mut id, root.size))
            .collect();
        let points = match index {
            0..=2 => [320, 320],
            3..=5 => [3456, 2234],
            _ => [320 + rng.get(3137), 320 + rng.get(1915)],
        };
        let scale = 1 + index % 3;
        let mut viewport = view(points.map(|side| side * scale));
        viewport.insets = if index <= 5 {
            [64.0; 4]
        } else {
            std::array::from_fn(|_| f64::from(rng.get(65)))
        };
        viewport.scale = scale;
        let logical = viewport.full().size;
        assert_eq!(logical, points.map(f64::from));
        if index <= 5 {
            assert_eq!(viewport.safe().size, logical.map(|side| side - 128.0));
        }
        let mut a = Tree::new(root.clone()).expect("valid declaration");
        let mut b = Tree::new(root.clone()).expect("same declaration");
        let a_paint = a.frame(viewport, inputs(&fonts)).expect("layout");
        let b_paint = b.frame(viewport, inputs(&fonts)).expect("layout again");
        assert_eq!(a.bounds(0), Some(viewport.safe()));
        assert_eq!(a_paint.rows, b_paint.rows);
        verify(&a, &b, &root, viewport.safe());
    }
}

#[test]
fn editing_hidden_intrinsic_text_reuses_the_visible_frame() {
    let fonts = font();
    let viewport = view([320, 320]);
    let mut root = Node::new(0, NodeKind::Overlay);
    let mut group = Node::new(1, NodeKind::Overlay);
    group.children.push(text(2, "00", false));
    root.children = vec![group, fill(3, [10.0; 2])];
    let mut tree = Tree::new(root.clone()).expect("tree");
    tree.frame(viewport, inputs(&fonts)).expect("visible");
    tree.set_shown(1, false);
    let hidden = tree.frame(viewport, inputs(&fonts)).expect("hidden");
    let stats = tree.stats();
    tree.set_text(2, "1234");
    let edited = tree.frame(viewport, inputs(&fonts)).expect("edited");
    assert!(Arc::ptr_eq(&hidden.rows, &edited.rows));
    assert_eq!(tree.stats(), stats);
    assert!(tree.bounds(2).is_none());

    tree.set_shown(1, true);
    let shown = tree.frame(viewport, inputs(&fonts)).expect("shown again");
    let NodeKind::Text(label) = &mut root.children[0].children[0].kind else {
        unreachable!()
    };
    label.value = "1234".into();
    let mut fresh = Tree::new(root.clone()).expect("fresh tree");
    let expected = fresh.frame(viewport, inputs(&fonts)).expect("fresh frame");
    assert_eq!(shown.rows, expected.rows);
    assert_eq!(tree.stats().layouts, stats.layouts + 1);
    assert!(tree.elisions().eq(fresh.elisions()));
    verify(&tree, &fresh, &root, viewport.safe());
}

#[test]
fn hidden_geometry_and_paint_edits_take_effect_only_on_reveal() {
    let viewport = view([320, 320]);
    let mut root = Node::new(0, NodeKind::Overlay);
    let mut group = Node::new(1, NodeKind::Overlay);
    group.shown = false;
    let mut nested = Node::new(2, NodeKind::Overlay);
    nested.children.push(fill(3, [20.0; 2]));
    let mut bar = fill(4, [20.0; 2]);
    let track = [0.0, 0.0, 0.0, 1.0];
    let tint = [1.0, 0.0, 0.0, 1.0];
    bar.kind = NodeKind::Bar {
        fraction: 0.0,
        track,
        fill: tint,
    };
    nested.children.push(bar);
    group.children.push(nested);
    root.children = vec![group, fill(5, [50.0; 2])];
    let mut tree = Tree::new(root.clone()).expect("tree");
    let baseline = tree.frame(viewport, inputs(&[])).expect("visible sibling");
    let stats = tree.stats();
    let sibling = tree.bounds(5);
    let anchor = [Anchor::Point(4.0); 2];
    let check_hidden = |tree: &mut Tree| {
        let frame = tree.frame(viewport, inputs(&[])).expect("hidden group");
        assert_eq!(tree.stats(), stats);
        assert!(Arc::ptr_eq(&baseline.rows, &frame.rows));
        assert!(tree.bounds(3).is_none());
        assert_eq!(tree.bounds(5), sibling);
    };
    tree.set_anchor(3, anchor);
    check_hidden(&mut tree);
    tree.set_shown(3, false);
    check_hidden(&mut tree);
    tree.set_shown(3, true);
    check_hidden(&mut tree);
    tree.set_tint(3, tint);
    check_hidden(&mut tree);
    tree.set_fraction(4, 0.5);
    check_hidden(&mut tree);

    root.children[0].shown = true;
    let children = &mut root.children[0].children[0].children;
    children[0].anchor = anchor;
    children[0].kind = NodeKind::Fill(tint);
    children[1].kind = NodeKind::Bar {
        fraction: 0.5,
        track,
        fill: tint,
    };
    tree.set_shown(1, true);
    let revealed = tree.frame(viewport, inputs(&[])).expect("reveal");
    assert_eq!(tree.stats().layouts, stats.layouts + 1);
    assert_eq!(tree.stats().lists, stats.lists + 1);
    assert_eq!(tree.bounds(3).expect("shown fill").origin, [4.0; 2]);
    let mut fresh = Tree::new(root.clone()).expect("fresh tree");
    let expected = fresh.frame(viewport, inputs(&[])).expect("fresh frame");
    assert_eq!(revealed.rows, expected.rows);
    verify(&tree, &fresh, &root, viewport.safe());
}

#[test]
fn anchors_ignored_by_layout_reuse_the_visible_frame() {
    let viewport = view([320, 320]);
    let mut root = Node::new(0, NodeKind::Overlay);
    let mut stack = Node::new(
        1,
        NodeKind::Stack {
            axis: Axis::Column,
            gap: 0.0,
            align: Anchor::Start,
        },
    );
    stack.children.push(fill(2, [20.0; 2]));
    let mut bleed = fill(3, [20.0; 2]);
    bleed.full_bleed = true;
    root.children = vec![stack, bleed];
    let mut tree = Tree::new(root.clone()).expect("tree");
    let baseline = tree.frame(viewport, inputs(&[])).expect("visible nodes");
    let stats = tree.stats();
    let anchor = [Anchor::Point(4.0); 2];
    for id in [0, 2, 3] {
        tree.set_anchor(id, anchor);
        let frame = tree.frame(viewport, inputs(&[])).expect("ignored anchor");
        assert_eq!(tree.stats(), stats);
        assert!(Arc::ptr_eq(&baseline.rows, &frame.rows));
    }
    root.anchor = anchor;
    root.children[0].children[0].anchor = anchor;
    root.children[1].anchor = anchor;
    let mut fresh = Tree::new(root.clone()).expect("fresh tree");
    let expected = fresh.frame(viewport, inputs(&[])).expect("fresh frame");
    assert_eq!(baseline.rows, expected.rows);
    verify(&tree, &fresh, &root, viewport.safe());
}

#[test]
fn random_edits_match_fresh_layout_and_rebuild_only_what_is_dirty() {
    let fonts = font();
    let mut rng = Random(114);
    let mut root = Node::new(
        0,
        NodeKind::Stack {
            axis: Axis::Row,
            gap: 6.0,
            align: Anchor::Start,
        },
    );
    root.children = vec![
        text(1, "00", false),
        fill(2, [25.0, 8.0]),
        fill(3, [10.0; 2]),
        text(4, "A", true),
    ];
    root.children[3].size = [Size::fixed(80.0), Size::fixed(24.0)];
    root.children[2].kind = NodeKind::Bar {
        fraction: 0.0,
        track: [0.0, 0.0, 0.0, 1.0],
        fill: [1.0; 4],
    };
    let mut incremental = Tree::new(root.clone()).expect("tree");
    let viewport = view([320, 320]);
    for turn in 0..128 {
        let stats = incremental.stats();
        let paint = incremental.frame(viewport, inputs(&fonts)).expect("live");
        let updated = incremental.stats();
        if turn > 0 {
            assert_eq!(
                updated.layouts - stats.layouts,
                u64::from((turn - 1) % 7 == 0)
            );
        }
        for _ in 0..4 {
            let same = incremental
                .frame(viewport, inputs(&fonts))
                .expect("unchanged");
            assert!(Arc::ptr_eq(&same.rows, &paint.rows));
            assert_eq!(updated, incremental.stats());
        }
        let mut fresh = Tree::new(root.clone()).expect("fresh tree");
        let expected = fresh.frame(viewport, inputs(&fonts)).expect("fresh layout");
        assert_eq!(paint.rows, expected.rows);
        assert!(incremental.elisions().eq(fresh.elisions()));
        verify(&incremental, &fresh, &root, viewport.safe());

        let NodeKind::Text(label) = &mut root.children[0].kind else {
            unreachable!()
        };
        if turn % 7 == 0 {
            // Vary the literal prefix length so this also changes a sibling's box.
            label.value = if label.value.starts_with('A') {
                "00".into()
            } else {
                "A00".into()
            };
        } else {
            let offset = usize::from(label.value.starts_with('A'));
            label.value.truncate(offset);
            write!(label.value, "{:02}", rng.get(100)).expect("format digits");
        }
        incremental.set_text(1, &label.value);
        let fixed = ["A", "AB", "A1 A2 A3", "A1 A2 A3 A4 A5"]
            [usize::try_from(rng.get(4)).expect("valid index")];
        let NodeKind::Text(label) = &mut root.children[3].kind else {
            unreachable!()
        };
        label.value = fixed.into();
        incremental.set_text(4, fixed);
        let tint = [
            f32::from(u8::try_from(rng.get(255)).expect("channel")) / 255.0,
            0.2,
            0.4,
            1.0,
        ];
        root.children[1].kind = NodeKind::Fill(tint);
        incremental.set_tint(2, tint);
        let fraction = f64::from(rng.get(101)) / 100.0;
        if let NodeKind::Bar {
            fraction: value, ..
        } = &mut root.children[2].kind
        {
            *value = fraction;
        }
        incremental.set_fraction(3, fraction);
    }
}
