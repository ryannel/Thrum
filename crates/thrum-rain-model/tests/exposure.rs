//! M1.5: shelter is content. Exposure from shelters the test gives, and the hollows under a
//! roof that fill only by what drains to them from open ground.
use crate::support;
use support::{ground, mm_h};
use thrum_rain_model::{
    Canopy, GroundClass, Profile, Rain, RainError, RainState, Shelter, TickReport, Tree,
};

/// Two triangles: a level rectangle at height `y` from x0,z0 to x1,z1.
fn quad(x0: f64, z0: f64, x1: f64, z1: f64, y: f64) -> Vec<[[f64; 3]; 3]> {
    vec![
        [[x0, y, z0], [x1, y, z0], [x1, y, z1]],
        [[x0, y, z0], [x1, y, z1], [x0, y, z1]],
    ]
}

fn shelter(name: &str, triangles: Vec<[[f64; 3]; 3]>, share: f64) -> Shelter {
    Shelter {
        name: name.into(),
        triangles,
        share,
    }
}

/// A roof slab from 3.0 m to 3.2 m with a crown under it, and two crowns that overlap.
fn shelters() -> Vec<Shelter> {
    let mut roof = quad(1.0, 1.0, 3.0, 3.0, 3.0);
    roof.extend(quad(1.0, 1.0, 3.0, 3.0, 3.2));
    vec![
        shelter("roof", roof, 0.0),
        shelter("crown under roof", quad(1.5, 1.5, 2.5, 2.5, 2.0), 0.5),
        shelter("crown", quad(4.0, 1.0, 6.0, 3.0, 2.0), 0.5),
        shelter("second crown", quad(5.0, 1.0, 7.0, 3.0, 2.5), 0.6),
    ]
}

fn canopy(list: &[Shelter]) -> Result<Canopy, RainError> {
    Canopy::prepare([0.0, 0.0], [100, 50], list, &Profile::default())
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

#[test]
fn exposure_under_roofs_crowns_and_open_sky() {
    let c = canopy(&shelters()).expect("valid test case");
    close(c.exposure([1.5, 0.0, 1.2]), 0.0); // under the roof
    close(c.exposure([2.0, 0.0, 2.0]), 0.0); // under the crown under the roof, at the ground
    close(c.exposure([2.0, 2.5, 2.0]), 0.0); // between the crown and the roof
    close(c.exposure([2.0, 3.5, 2.0]), 1.0); // above the roof
    close(c.exposure([1.5, 3.2, 1.2]), 1.0); // on the roof's own top
    close(c.exposure([4.5, 0.0, 2.0]), 0.5); // under one crown: its share
    close(c.exposure([5.5, 0.0, 2.0]), 0.3); // under two crowns: the product
    close(c.exposure([6.5, 0.0, 2.0]), 0.6);
    close(c.exposure([5.5, 2.2, 2.0]), 0.6); // above the first crown, under the second
    close(c.exposure([8.0, 0.0, 2.0]), 1.0); // open sky
    close(c.exposure([-5.0, 0.0, 2.0]), 1.0); // outside the grid
    close(c.exposure([20.0, 0.0, 20.0]), 1.0);
    // The roof's edge at x = 1: open sky to roof over the profile's edge width, 0.3 m.
    let w = Profile::default().edge_width;
    close(c.exposure([1.0 - 0.5 * w, 0.0, 1.5]), 1.0);
    close(c.exposure([1.0, 0.0, 1.5]), 0.5);
    close(c.exposure([1.0 + 0.25 * w, 0.0, 1.5]), 0.25);
    close(c.exposure([1.0 + 0.5 * w, 0.0, 1.5]), 0.0);
    let mut last = 1.0;
    for i in 0..=30 {
        let e = c.exposure([0.8 + 0.4 * f64::from(i) / 30.0, 0.0, 1.5]);
        assert!(e <= last + 1e-12);
        last = e;
    }
}

#[test]
fn a_shelter_counts_once_and_a_third_is_refused() {
    // Six triangles over every column, at three heights: one layer, at the lowest.
    let mut stacked = quad(0.0, 0.0, 2.0, 2.0, 1.0);
    stacked.extend(quad(0.0, 0.0, 2.0, 2.0, 1.5));
    stacked.extend(quad(0.0, 0.0, 2.0, 2.0, 2.0));
    let c = canopy(&[shelter("stack", stacked.clone(), 0.5)]).expect("valid test case");
    close(c.exposure([1.0, 0.0, 1.0]), 0.5);
    close(c.exposure([1.0, 1.2, 1.0]), 1.0);
    // With a crown beneath it is still two layers, not refused.
    let two = canopy(&[
        shelter("stack", stacked, 0.5),
        shelter("crown", quad(0.0, 0.0, 1.0, 1.0, 0.5), 0.5),
    ])
    .expect("valid test case");
    close(two.exposure([0.5, 0.0, 0.5]), 0.25);
    // A third shelter over the roof and the crown under it is refused by name and place.
    let mut list = shelters();
    list.push(shelter("awning", quad(1.8, 1.8, 2.2, 2.2, 2.8), 0.0));
    match canopy(&list) {
        Err(RainError::Shelter { name, place, .. }) => {
            assert_eq!(name, "awning");
            assert!(
                (1.8..2.2).contains(&place[0]) && (1.8..2.2).contains(&place[1]),
                "{place:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_hollow_under_a_roof_fills_only_from_open_ground() {
    // Two bowls with a divide at x = 2 m. A roof covers the left bowl's pit but not all of
    // its catchment; another covers the right bowl's whole catchment and the soft edge.
    let g = ground([80, 40], 0.05, |x, y| {
        let cx = if x < 2.0 { 1.0 } else { 3.0 };
        0.05 * ((x - cx).powi(2) + (y - 1.0).powi(2)) - 0.02
    });
    let roofs = [
        shelter("left roof", quad(0.3, 0.3, 1.7, 1.7, 2.5), 0.0),
        shelter("right roof", quad(1.8, -0.3, 4.3, 2.3, 2.5), 0.0),
    ];
    let c = Canopy::prepare([-1.0, -1.0], [70, 50], &roofs, &Profile::default())
        .expect("valid test case");
    let tree = Tree::prepare(&g, &c, &Profile::default()).expect("valid test case");
    assert_eq!(tree.hollow_count(), 2);
    let w = 80;
    let (left, right) = (tree.labels()[20 * w + 20], tree.labels()[20 * w + 60]);
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let rain = Rain {
        flux: mm_h(30.0),
        drying: 0.0,
    };
    for _ in 0..300 {
        state
            .step(&tree, rain, GroundClass::bare(0.0), 1.0, 1, &mut report)
            .expect("valid test case");
        let (l, r) = (
            report.hollows[left as usize],
            report.hollows[right as usize],
        );
        assert_eq!(l.on_water, 0.0, "no rain reaches the water under the roof");
        assert!(l.runoff > 0.0, "the open ground drains to it");
        assert_eq!((r.on_water, r.runoff), (0.0, 0.0));
    }
    let held = |h: u32| {
        state
            .puddles()
            .find(|p| p.0 == h)
            .expect("valid test case")
            .1
    };
    assert!(held(left) > 0.0, "the left hollow fills from open ground");
    assert_eq!(
        held(right),
        0.0,
        "the right hollow, with no open ground, stays dry"
    );
}
