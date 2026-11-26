//! Point and density sweep of the exact status tree painted by the game.

mod status_store;

use status_store as store;
use v1_game::interface_status as status;

use thrum_test_support::Scratch;
use thrum_ui::{Box2, Font, Image, PaintInputs, Viewport};

fn contains(parent: Box2, child: Box2) -> bool {
    (0..2).all(|axis| {
        parent.origin[axis] <= child.origin[axis] + 1e-8
            && parent.end()[axis] + 1e-8 >= child.end()[axis]
    })
}

fn overlaps(a: Box2, b: Box2) -> bool {
    (0..2)
        .all(|axis| a.end()[axis] > b.origin[axis] + 1e-8 && b.end()[axis] > a.origin[axis] + 1e-8)
}

fn points(min: u32, max: u32) -> impl Iterator<Item = u32> {
    (min..max).step_by(16).chain([max])
}

fn check_extent(tree: &mut thrum_ui::Tree, inputs: PaintInputs<'_>, viewport: Viewport) {
    tree.frame(viewport, inputs).expect("status layout");
    assert!(
        tree.overflows().next().is_none(),
        "{viewport:?}: {:?}",
        tree.overflows().collect::<Vec<_>>()
    );
    assert!(
        tree.elisions().next().is_none(),
        "{viewport:?}: {:?}",
        tree.elisions().collect::<Vec<_>>()
    );
    let rect = |id| tree.bounds(id).expect("visible status element");
    let parents = [
        (1, 0),
        (2, 1),
        (3, 2),
        (4, 3),
        (5, 3),
        (7, 3),
        (6, 2),
        (8, 2),
        (18, 8),
        (19, 18),
        (20, 18),
        (21, 18),
        (22, 8),
        (23, 22),
        (24, 22),
        (25, 22),
        (26, 8),
        (27, 26),
        (28, 26),
        (29, 26),
    ];
    assert_eq!(rect(0), viewport.safe());
    for (child, parent) in parents {
        assert!(
            contains(viewport.safe(), rect(child)),
            "{child} outside safe area at {viewport:?}"
        );
        assert!(
            contains(rect(parent), rect(child)),
            "{child} outside {parent} at {viewport:?}"
        );
    }
    let elements = [4, 5, status::BAR, status::TIMER, 18, 22, 26];
    for (i, a) in elements.into_iter().enumerate() {
        for &b in &elements[i + 1..] {
            assert!(
                !overlaps(rect(a), rect(b)),
                "{a} covers {b} at {viewport:?}"
            );
        }
    }
}

#[test]
fn status_fits_point_grid_at_all_three_densities() {
    let scratch = Scratch::new("status-sweep");
    let products = store::cook(store::fixture(), scratch.join("store"));
    let (font, art) = status::proof::open(products.store());
    let badge = art.frame_at(0).expect("badge");
    let fonts = [Font {
        product: &font,
        sheet: 1,
    }];
    let inputs = PaintInputs {
        stamp: None,
        fonts: &fonts,
        white: status::WHITE_TILE,
    };
    let mut tree = status::tree(Image {
        sheet: 2,
        frame: 0,
        texels: badge.size,
    })
    .expect("status display");
    let mut cases = 0;
    for scale in 1..=3 {
        // All combinations of logical widths and heights in the range at 16-point
        // intervals, plus the upper endpoints. Maximum insets bound the fit.
        for width in points(320, 3456) {
            for height in points(320, 2234) {
                check_extent(
                    &mut tree,
                    inputs,
                    Viewport {
                        extent: [width * scale, height * scale],
                        insets: [64.0; 4],
                        scale,
                    },
                );
                cases += 1;
            }
        }
        for [width, height] in [[320, 568], [568, 320], [390, 844], [844, 390]] {
            for inset in 0..=64 {
                let i = f64::from(inset);
                for insets in [[i; 4], [i, 64.0 - i, 0.0, i]] {
                    check_extent(
                        &mut tree,
                        inputs,
                        Viewport {
                            extent: [width * scale, height * scale],
                            insets,
                            scale,
                        },
                    );
                    cases += 1;
                }
            }
        }
    }
    println!("{cases} extent, inset and scale combinations");
}
