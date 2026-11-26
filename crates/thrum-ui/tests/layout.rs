//! Readable hand calculations for point-space layout and cooked-text fitting.

mod layout_support;

use layout_support::{fill, font, inputs, text, view};
use thrum_ui::{Anchor, Axis, Box2, Node, NodeKind, Size, SizeRule, Tree, TreeError};

fn assert_box(tree: &Tree, id: u32, origin: [f64; 2], size: [f64; 2]) {
    let Box2 {
        origin: actual,
        size: actual_size,
    } = tree.bounds(id).expect("visible");
    for (actual, expected) in actual
        .into_iter()
        .chain(actual_size)
        .zip(origin.into_iter().chain(size))
    {
        assert!(
            (actual - expected).abs() < 1e-8,
            "node {id}: {actual} != {expected}"
        );
    }
}

#[test]
fn row_distributes_space_by_weight_and_pins_the_least() {
    let mut root = Node::new(
        0,
        NodeKind::Stack {
            axis: Axis::Row,
            gap: 5.0,
            align: Anchor::Centre,
        },
    );
    root.padding = [4.0, 6.0, 4.0, 6.0];
    root.children = vec![
        fill(1, [24.0, 10.0]),
        fill(2, [0.0, 12.0]),
        fill(3, [0.0, 14.0]),
    ];
    root.children[1].size[0] = Size {
        least: 30.0,
        ..Size::new(SizeRule::Share(1.0))
    };
    root.children[2].size[0] = Size::new(SizeRule::Share(3.0));
    let mut tree = Tree::new(root.clone()).expect("row");
    // 114 - 8 padding - 10 gaps - 24 fixed = 72. A quarter is 18,
    // so the first share claims 30 and leaves 42 for the other.
    tree.frame(view([114, 60]), inputs(&[])).expect("layout");
    assert_box(&tree, 1, [4.0, 25.0], [24.0, 10.0]);
    assert_box(&tree, 2, [33.0, 24.0], [30.0, 12.0]);
    assert_box(&tree, 3, [68.0, 23.0], [42.0, 14.0]);
    root.children[1].size[0].least = 0.0;
    root.children[2].size[0].most = 50.0;
    let mut tree = Tree::new(root).expect("capped row");
    tree.frame(view([114, 60]), inputs(&[])).expect("layout");
    assert_box(&tree, 2, [33.0, 24.0], [22.0, 12.0]);
    assert_box(&tree, 3, [60.0, 23.0], [50.0, 14.0]);
}

#[test]
fn capped_shares_release_space_to_lighter_siblings_in_either_order() {
    for (heavy, light) in [(3.0, 1.0), (f64::MAX, f64::from_bits(1))] {
        let mut root = Node::new(
            0,
            NodeKind::Stack {
                axis: Axis::Row,
                gap: 0.0,
                align: Anchor::Start,
            },
        );
        root.children = vec![fill(1, [0.0, 10.0]), fill(2, [0.0, 10.0])];
        root.children[0].size[0] = Size {
            most: 32.0,
            ..Size::new(SizeRule::Share(heavy))
        };
        root.children[1].size[0] = Size {
            most: 400.0,
            ..Size::new(SizeRule::Share(light))
        };
        for (heavy_x, light_x) in [(0.0, 32.0), (288.0, 0.0)] {
            let mut tree = Tree::new(root.clone()).expect("weighted row");
            tree.frame(view([320, 48]), inputs(&[])).expect("layout");
            assert_box(&tree, 1, [heavy_x, 0.0], [32.0, 10.0]);
            assert_box(&tree, 2, [light_x, 0.0], [288.0, 10.0]);
            assert_eq!(tree.overflows().count(), 0);
            root.children.reverse();
        }
        root.children[1].size[0].most = 64.0;
        let mut tree = Tree::new(root).expect("both shares capped");
        tree.frame(view([320, 48]), inputs(&[])).expect("layout");
        assert_box(&tree, 1, [0.0, 0.0], [32.0, 10.0]);
        assert_box(&tree, 2, [32.0, 0.0], [64.0, 10.0]);
    }
}

#[test]
fn overlay_anchors_nested_column_and_rejects_circular_fraction() {
    let mut root = Node::new(0, NodeKind::Overlay);
    root.size = [Size::fixed(120.0), Size::fixed(80.0)];
    let mut column = Node::new(
        1,
        NodeKind::Stack {
            axis: Axis::Column,
            gap: 4.0,
            align: Anchor::End,
        },
    );
    column.anchor = [Anchor::End, Anchor::Centre];
    column.padding = [3.0; 4];
    column.size = [Size::new(SizeRule::Fraction(0.5)), Size::new(SizeRule::Fit)];
    let mut nested = Node::new(2, NodeKind::Overlay);
    nested.size = [Size::fixed(16.0), Size::fixed(9.0)];
    let mut mark = fill(3, [5.0, 5.0]);
    mark.anchor = [Anchor::Point(1.0), Anchor::End];
    nested.children.push(mark);
    column.children = vec![nested, fill(4, [8.0, 11.0])];
    let mut edge = fill(5, [30.0, 10.0]);
    edge.anchor = [Anchor::Centre, Anchor::End];
    root.children = vec![column, edge];
    let mut tree = Tree::new(root.clone()).expect("overlay");
    tree.frame(view([120, 80]), inputs(&[])).expect("layout");
    assert_box(&tree, 1, [60.0, 25.0], [60.0, 30.0]);
    assert_box(&tree, 2, [101.0, 28.0], [16.0, 9.0]);
    assert_box(&tree, 3, [102.0, 32.0], [5.0, 5.0]);
    assert_box(&tree, 4, [109.0, 41.0], [8.0, 11.0]);
    assert_box(&tree, 5, [45.0, 70.0], [30.0, 10.0]);
    root.size[0] = Size::new(SizeRule::Fit);
    assert!(matches!(
        Tree::new(root),
        Err(TreeError::CircularFraction(1))
    ));
}

#[test]
fn wrapped_text_sets_its_parent_height() {
    let mut root = Node::new(0, NodeKind::Overlay);
    root.size[0] = Size::fixed(72.0);
    let mut parent = Node::new(1, NodeKind::Overlay);
    parent.size[0] = Size::new(SizeRule::Fraction(0.5));
    parent.padding = [2.0; 4];
    let mut label = text(2, "AA AB", true);
    label.size[0] = Size::new(SizeRule::Share(1.0));
    parent.children.push(label);
    root.children.push(parent);
    let mut tree = Tree::new(root).expect("tree");
    // The grid advances by 8 px. Five cells require 40, so the available
    // 32 px wraps into two lines of height 8 plus the four points of padding.
    tree.frame(view([72, 40]), inputs(&font()))
        .expect("wrapped");
    assert_box(&tree, 1, [0.0; 2], [36.0, 20.0]);
    assert_box(&tree, 2, [2.0; 2], [32.0, 16.0]);
    assert_eq!(tree.elisions().count(), 0);
    tree.set_text(2, "AA");
    tree.frame(view([72, 40]), inputs(&font()))
        .expect("single line");
    assert_box(&tree, 1, [0.0; 2], [36.0, 12.0]);
}
