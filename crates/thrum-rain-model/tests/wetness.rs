//! M2.2: the wetness function at a point, from prepared fields and the rain state alone:
//! it reads the stores by exposure, nothing under a roof, a quarter of the ground's on a
//! wall and none on an underside; sloping relief holds less, to none at the class's
//! steepest slope; a pattern's lows take the surface water first. And part 5's bound:
//! ground that 0.4 of the rain reaches, read between the stores, against that ground
//! stepped by itself.
use crate::support;
use support::mm_h;
use thrum_rain_model::{
    Canopy, GroundClass, GroundStores, Profile, Rain, Shelter, WetPoint, Wetness, wetness,
    wetness_at,
};

fn class(soak: f64, pores: f64, relief: f64) -> GroundClass {
    GroundClass {
        soak: mm_h(soak),
        percolation: 0.0,
        pores: pores / 1000.0,
        relief: relief / 1000.0,
        steepest: 0.5,
        share: 0.47,
        porosity: 1.0,
        film_roughness: 0.05,
    }
}

const UP: [f64; 3] = [0.0, 1.0, 0.0];

/// Part 5's run: ten minutes of 10 mm/h, then drying at 6 mm/h, nothing drying while it
/// rains; one store stepped by the test at a reach, each second.
fn own_run(c: &GroundClass, reach: f64) -> Vec<(f64, f64)> {
    let (mut surface, mut fill) = (0.0_f64, 0.0_f64);
    let cap = c.pores;
    (0..3600)
        .map(|t| {
            let (rain, dry) = if t < 600 {
                (10.0 / 3.6e6, 0.0)
            } else {
                (0.0, 6.0 / 3.6e6)
            };
            let mut water = surface + reach * rain;
            let take = water.min(c.soak / 1000.0).min((1.0 - fill) * cap);
            fill = if take >= (1.0 - fill) * cap {
                1.0
            } else {
                fill + take / cap
            };
            water -= take;
            surface = water.min(c.relief);
            let d = surface.min(dry);
            surface -= d;
            if surface <= 0.0 && dry > 0.0 {
                fill *= (-dry / cap).exp();
            }
            (surface, fill)
        })
        .collect()
}

fn showered(classes: &[GroundClass], seconds: usize) -> GroundStores {
    let mut g = GroundStores::new(classes).expect("valid test case");
    for t in 0..seconds {
        let rain = if t < 600 {
            Rain {
                flux: mm_h(10.0),
                drying: 0.0,
            }
        } else {
            Rain {
                flux: 0.0,
                drying: mm_h(6.0),
            }
        };
        g.step(rain, 1.0).expect("valid test case");
    }
    g
}

#[test]
fn ground_part_way_under_shelter_reads_between_the_stores_within_part_5_s_bound() {
    let classes = [class(30.0, 0.3, 1.0), class(15.0, 3.0, 0.5)];
    let mut g = GroundStores::new(&classes).expect("valid test case");
    let own: Vec<_> = classes.iter().map(|c| own_run(c, 0.4)).collect();
    let mut worst = [(0.0_f64, 0.0_f64); 2];
    for t in 0..3600 {
        let rain = if t < 600 {
            Rain {
                flux: mm_h(10.0),
                drying: 0.0,
            }
        } else {
            Rain {
                flux: 0.0,
                drying: mm_h(6.0),
            }
        };
        g.step(rain, 1.0).expect("valid test case");
        for (k, run) in own.iter().enumerate() {
            let w = wetness(
                &g,
                k,
                WetPoint {
                    exposure: 0.4,
                    normal: UP,
                    pattern: None,
                },
            );
            let (surface, fill) = run[t];
            worst[k].0 = worst[k].0.max((w.pores - fill).abs());
            worst[k].1 = worst[k].1.max((w.surface - surface).abs());
        }
    }
    println!(
        "paving: off by {:.3} of the pores and {:.3} mm; earth: {:.3} and {:.3} mm",
        worst[0].0,
        worst[0].1 * 1e3,
        worst[1].0,
        worst[1].1 * 1e3
    );
    for (pores, surface) in worst {
        assert!(pores <= 0.3 && surface <= 0.15e-3);
    }
    assert!(
        worst[0].0 > 0.0,
        "the read is between stores, not the ground stepped by itself"
    );
}

#[test]
fn shelter_walls_undersides_slopes_and_patterns() {
    let c = class(30.0, 0.3, 1.0);
    let g = showered(&[c], 300);
    let open = wetness(
        &g,
        0,
        WetPoint {
            exposure: 1.0,
            normal: UP,
            pattern: None,
        },
    );
    assert!(open.pores > 0.99 && open.surface > 0.0, "{open:?}");
    // Read by exposure: the half store at a half.
    let half = wetness(
        &g,
        0,
        WetPoint {
            exposure: 0.5,
            normal: UP,
            pattern: None,
        },
    );
    assert_eq!(half.pores, g.store(0, 1).fill);
    assert_eq!(half.surface, g.store(0, 1).surface);
    // Under a roof: nothing, read from the canopy at the point.
    let roof = Shelter {
        name: "roof".into(),
        triangles: vec![
            [[0.0, 2.5, 0.0], [2.0, 2.5, 0.0], [2.0, 2.5, 2.0]],
            [[0.0, 2.5, 0.0], [2.0, 2.5, 2.0], [0.0, 2.5, 2.0]],
        ],
        share: 0.0,
    };
    let canopy = Canopy::prepare(
        [-1.0, -1.0],
        [40, 40],
        std::slice::from_ref(&roof),
        &Profile::default(),
    )
    .expect("valid test case");
    assert_eq!(
        wetness_at(&g, 0, &canopy, [1.0, 0.0, 1.0], UP, None),
        Wetness::default()
    );
    assert_eq!(
        wetness_at(&g, 0, &canopy, [1.0, 2.6, 1.0], UP, None),
        open,
        "the roof's own top is wet"
    );
    let soft_canopy = Canopy::prepare_sparse([-1.0, -1.0], [40, 40], &[roof], &Profile::default())
        .expect("valid test case")
        .with_material_half_width(0.25)
        .expect("material transition");
    let point = [1.0, 2.5, 1.0];
    assert_eq!(soft_canopy.exposure(point), canopy.exposure(point));
    assert_eq!(
        soft_canopy.exposure([1.0, 2.49, 1.0]),
        canopy.exposure([1.0, 2.49, 1.0])
    );
    let blended = wetness_at(&g, 0, &soft_canopy, point, UP, None);
    assert!((blended.pores - half.pores).abs() < 1e-12);
    assert!((blended.surface - half.surface).abs() < 1e-12);
    // A wall takes a quarter of the ground's rain and holds no standing water; an
    // underside takes none.
    let wall = wetness(
        &g,
        0,
        WetPoint {
            exposure: 1.0,
            normal: [1.0, 0.0, 0.0],
            pattern: None,
        },
    );
    assert_eq!(wall.pores, g.store(0, 2).fill);
    assert_eq!(wall.surface, 0.0);
    let under = wetness(
        &g,
        0,
        WetPoint {
            exposure: 1.0,
            normal: [0.0, -1.0, 0.0],
            pattern: None,
        },
    );
    assert_eq!(under, Wetness::default());
    // Relief on a slope holds less, to none at the class's steepest (a rise of 0.5).
    let mut last = open.surface;
    for rise in [0.1_f64, 0.2, 0.3, 0.4, 0.5, 0.6] {
        let n = [
            rise / (1.0 + rise * rise).sqrt(),
            1.0 / (1.0 + rise * rise).sqrt(),
            0.0,
        ];
        let w = wetness(
            &g,
            0,
            WetPoint {
                exposure: 1.0,
                normal: n,
                pattern: None,
            },
        );
        let hold = c.relief * (1.0 - rise / c.steepest).max(0.0);
        assert!(
            w.surface <= hold + 1e-18 && w.surface <= last,
            "rise {rise}: {w:?}"
        );
        if rise >= c.steepest {
            assert_eq!(w.surface, 0.0);
            assert_eq!(w.cover, 0.0);
        }
        last = w.surface;
    }
    // A pattern's lows take the water first: as the relief fills, the cover reaches
    // points of the pattern in order of their height.
    let mut stores = GroundStores::new(&[c]).expect("valid test case");
    let heights = [0.1, 0.35, 0.6, 0.85];
    let mut first_covered = [None; 4];
    for t in 0..400 {
        stores
            .step(
                Rain {
                    flux: mm_h(60.0),
                    drying: 0.0,
                },
                1.0,
            )
            .expect("valid test case");
        for (i, &h) in heights.iter().enumerate() {
            let w = wetness(
                &stores,
                0,
                WetPoint {
                    exposure: 1.0,
                    normal: UP,
                    pattern: Some(h),
                },
            );
            if w.cover >= 1.0 && first_covered[i].is_none() {
                first_covered[i] = Some(t);
            }
            // Below its height the water has not reached that point of the pattern.
            let share = w.surface / c.relief;
            assert_eq!(w.cover, f64::from(u8::from(share > h)));
        }
    }
    let order: Vec<usize> = first_covered
        .iter()
        .map(|t| t.expect("each point is covered"))
        .collect();
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "lows first: {order:?}"
    );
    // Without a pattern the cover is the relief's fill.
    let w = wetness(
        &g,
        0,
        WetPoint {
            exposure: 1.0,
            normal: UP,
            pattern: None,
        },
    );
    assert!((w.cover - w.surface / c.relief).abs() < 1e-15);
}
