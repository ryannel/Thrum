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
        let extent = match index {
            0 => [320, 320],
            1 => [3456, 2234],
            _ => [320 + rng.get(3137), 320 + rng.get(1915)],
        };
        let mut viewport = view(extent);
        viewport.insets = if index < 2 {
            [64.0; 4]
        } else {
            std::array::from_fn(|_| f64::from(rng.get(65)))
        };
        viewport.scale = 1 + index % 3;
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
    ];
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
