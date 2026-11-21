//! M2.5: the plunge. For a body below at any level in its stage's range, each path meets that
//! level where the free path does, and the plunge moves as the level moves. Births follow the
//! fall's power `rho g q z` for each metre of lip, split by the profile's air share. Each tick's
//! impulse is the wave share times the momentum flux into the surface times the sheet's pattern
//! arriving there, with its mean over the loop removed: over the loop its mean is under 1% of its
//! rms, its rms is `alpha rho q v_n w dt` within 5%, and a tick gives the same impulse every
//! time. A path that lands on bare ground gives no births and no impulses.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it measured"
)]
use crate::support;
use support::fall::{Channel, Cliff, Foot, channel};
use thrum_river_model::{Grid, Profile, Solution, solve};
use thrum_water_model::{
    Bed, Body, BodyKind, FallSegment, Map, PlungeProfile, Profile as WaterProfile, SheetProfile,
    Stage, loop_ticks,
};

const G: f64 = 9.81;
const RHO: f64 = 1000.0;
const DX: f64 = 0.25;
/// The pool's stage range above its level at the low end, metres.
const RISE: f64 = 0.4;

const WATER: WaterProfile = WaterProfile {
    margin_height: 0.3,
    swash_ratio: 0.7,
};

fn bed(c: &Channel) -> Bed {
    Bed::new(
        c.grid.origin(),
        c.grid.spacing(),
        c.grid.size(),
        c.grid.heights().to_vec(),
    )
    .unwrap()
}

fn pool_fall() -> (Channel, Solution, Map) {
    let c = channel(
        DX,
        40,
        0.005,
        0.15,
        0.04,
        Cliff {
            height: 3.0,
            face_cells: 0,
        },
        Foot::Pool {
            cells: 16,
            depth: 0.5,
        },
    );
    let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
    let pool = Body {
        name: "pool".into(),
        kind: BodyKind::Basin {
            seed: c.declaration.receivers[0].seed,
        },
        level: c.level,
        stage: Stage {
            min: 0.0,
            max: RISE,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    };
    let map = Map::prepare(bed(&c), vec![pool], WATER).unwrap();
    (c, s, map)
}

/// The water's level at a point, from the map: none where there is no water.
fn water(map: &Map) -> impl Fn([f64; 2]) -> Option<f64> + '_ {
    |p| map.sample(p).filter(|m| m.depth > 0.0).map(|m| m.row.level)
}

/// The test's sheet pattern: two streaks down the sheet and a change along the lip, with a mean
/// that is not zero.
fn pattern(along: f64, down: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    0.5 + 0.3 * (tau * down / 0.37 + 3.0 * along).sin() + 0.2 * (tau * down / 1.13 + along).sin()
}

#[test]
fn the_plunge_is_where_each_path_meets_the_level_and_moves_as_the_level_moves() {
    let (_, s, mut map) = pool_fall();
    let sheet = SheetProfile::default();
    let mut rows = Vec::new();
    for g in &s.falls[0].segments {
        let top = g.path[0].height;
        let mut last = f64::INFINITY;
        for k in 0..=4 {
            let stage = RISE * f64::from(k) / 4.0;
            map.set_stage(0, stage).unwrap();
            let level = map.row(0, g.centre()).level;
            let plunge = g.plunge(&sheet, water(&map)).unwrap();
            // The free path's closed form to that level.
            let t = (2.0 * (top - level) / G).sqrt();
            let out = g.launch_speed * t;
            let p = plunge.place;
            let start = g.path[0].place;
            let measured = (p[0] - start[0]).hypot(p[1] - start[1]);
            assert!((plunge.level - level).abs() < 1e-9);
            assert!(
                (measured / out - 1.0).abs() < 0.01,
                "{measured} against {out}"
            );
            assert!((plunge.time / t - 1.0).abs() < 0.01);
            assert!((plunge.normal_speed / (G * t) - 1.0).abs() < 0.01);
            assert!(measured < last, "the plunge moves in as the level rises");
            last = measured;
            if g.id == s.falls[0].segments[2].id {
                rows.push((level, measured, out));
            }
        }
    }
    for (level, measured, out) in rows {
        println!("level {level:.2} m: plunge {measured:.4} m out, free path {out:.4} m");
    }
}

#[test]
fn births_follow_the_falls_power_split_by_the_air_share() {
    let (_, s, mut map) = pool_fall();
    let sheet = SheetProfile::default();
    let profile = PlungeProfile::default();
    for stage in [0.0, RISE] {
        map.set_stage(0, stage).unwrap();
        for g in &s.falls[0].segments {
            let plunge = g.plunge(&sheet, water(&map)).unwrap();
            let b = g.births(&profile, &plunge);
            let z = g.path[0].height - plunge.level;
            let power = RHO * G * g.discharge * z * g.width();
            assert!(((b.foam + b.air) / power - 1.0).abs() < 1e-12);
            assert!((b.air / power - profile.air_share).abs() < 1e-12);
            assert_eq!(b.length, plunge.sheet.thickness.max(profile.foam_cell));
            assert_eq!(b.width, g.width());
        }
    }
    let g = &s.falls[0].segments[0];
    let plunge = g.plunge(&sheet, water(&map)).unwrap();
    let b = g.births(&profile, &plunge);
    println!(
        "births at the high stage: {:.1} W of foam and {:.1} W of air from {:.3} m2/s over {:.2} m",
        b.foam,
        b.air,
        g.discharge,
        g.path[0].height - plunge.level
    );
}

/// The impulses over one loop: (mean, rms).
fn over_loop(g: &FallSegment, profile: &PlungeProfile, map: &Map) -> (f64, f64, f64) {
    let plunge = g.plunge(&SheetProfile::default(), water(map)).unwrap();
    let stats = g.pattern_loop(profile, &pattern);
    let ticks = loop_ticks(profile.tick);
    let (mut sum, mut squares) = (0.0, 0.0);
    for n in 0..ticks {
        let i = g.impulse(profile, &pattern, &stats, &plunge, n).vertical;
        sum += i;
        squares += i * i;
    }
    let count = ticks as f64;
    let design =
        profile.wave_share * RHO * g.discharge * plunge.normal_speed * g.width() * profile.tick;
    // The design's figure, worked out here from the free path's speed into the surface.
    let t = plunge.time;
    let figure = profile.wave_share * RHO * g.discharge * (G * t) * g.width() * profile.tick;
    assert!((design / figure - 1.0).abs() < 0.01);
    (sum / count, (squares / count).sqrt(), figure)
}

#[test]
fn impulses_carry_the_sheets_pattern_with_its_mean_removed_at_the_designs_rms() {
    let (_, s, mut map) = pool_fall();
    let profile = PlungeProfile::default();
    let segments = &s.falls[0].segments;
    for stage in [0.0, 0.5 * RISE] {
        map.set_stage(0, stage).unwrap();
        for g in [&segments[0], &segments[3]] {
            let (mean, rms, figure) = over_loop(g, &profile, &map);
            assert!(mean.abs() < 0.01 * rms, "mean {mean} against rms {rms}");
            assert!(
                (rms / figure - 1.0).abs() < 0.05,
                "rms {rms} against {figure}"
            );
            println!(
                "segment {:x}, stage {stage}: mean {:.2e} of rms, rms {:.4} N s against {:.4}",
                g.id,
                mean.abs() / rms,
                rms,
                figure
            );
        }
    }
    // The same tick gives the same impulse, from a fresh plunge and fresh loop statistics.
    let g = &segments[1];
    let once = || {
        let plunge = g.plunge(&SheetProfile::default(), water(&map)).unwrap();
        let stats = g.pattern_loop(&profile, &pattern);
        g.impulse(&profile, &pattern, &stats, &plunge, 12_345)
    };
    let (a, b) = (once(), once());
    assert_eq!(a.vertical.to_bits(), b.vertical.to_bits());
    assert_eq!(a, b);
    assert_eq!(a.id, g.id);
}

#[test]
fn a_path_that_lands_on_bare_ground_gives_no_births_and_no_impulses() {
    let c = channel(
        DX,
        40,
        0.005,
        0.15,
        0.04,
        Cliff {
            height: 3.0,
            face_cells: 0,
        },
        Foot::Ground {
            slope: 0.02,
            cells: 30,
        },
    );
    let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
    // The map holds the reach above and nothing below the fall: the ground there is bare.
    let bed = bed(&c);
    let upper = s.reaches().unwrap()[0].chart(&bed, |_| None).unwrap();
    let map = Map::prepare(
        bed,
        vec![Body {
            name: "upper".into(),
            kind: BodyKind::Chart(upper),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1e6,
            gains: [0.0; 4],
        }],
        WATER,
    )
    .unwrap();
    for g in &s.falls[0].segments {
        assert!(g.plunge(&SheetProfile::default(), water(&map)).is_none());
        // With no water below, the record gives a plunge only where the caller has one.
        assert!(g.plunge(&SheetProfile::default(), |_| None).is_none());
    }
    println!(
        "{} segments land on bare ground: no plunge",
        s.falls[0].segments.len()
    );
}

/// A 4 m face over eight cells into the pool: steady at slope 2, or steeper below or above.
fn rock_face(variant: usize) -> Channel {
    let mut c = channel(
        DX,
        40,
        0.005,
        0.15,
        0.04,
        Cliff {
            height: 4.0,
            face_cells: 7,
        },
        Foot::Pool {
            cells: 24,
            depth: 0.5,
        },
    );
    let drops = match variant {
        0 => return c,
        1 => [0.4, 0.4, 0.4, 0.4, 0.6, 0.6, 0.6, 0.6],
        _ => [0.6, 0.6, 0.6, 0.6, 0.4, 0.4, 0.4, 0.4],
    };
    let mut h = c.grid.heights().to_vec();
    let mut y = h[c.cell(c.lip, c.rows().start)];
    for (k, d) in drops.iter().enumerate() {
        y -= d;
        for z in c.rows() {
            h[c.cell(c.lip + k + 1, z)] = y;
        }
    }
    c.grid = Grid::new(c.grid.origin(), c.grid.spacing(), c.grid.size(), h).unwrap();
    c
}

const SCAN: u32 = 1000;

/// The first place along the stored path where the map has water and the path is at or under
/// its level, found by stepping each chord in thousandths: (place, level).
fn scanned(g: &FallSegment, map: &Map) -> Option<([f64; 2], f64)> {
    for w in g.path.windows(2) {
        let (a, b) = (w[0], w[1]);
        // Stored chords are under 0.1 m, so a thousand steps are under 0.1 mm.
        assert!((b.place[0] - a.place[0]).hypot(b.place[1] - a.place[1]) < 0.1);
        for j in 1..=SCAN {
            let f = f64::from(j) / f64::from(SCAN);
            let p = [
                a.place[0] + f * (b.place[0] - a.place[0]),
                a.place[1] + f * (b.place[1] - a.place[1]),
            ];
            let y = a.height + f * (b.height - a.height);
            if let Some(l) = water(map)(p).filter(|&l| y <= l) {
                return Some((p, l));
            }
        }
    }
    None
}

#[test]
fn a_path_down_the_rock_meets_the_water_at_the_shoreline_at_every_stage() {
    let sheet = SheetProfile::default();
    let profile = PlungeProfile::default();
    let mut worst = (0.0_f64, 0.0_f64);
    let mut on_rock = 0;
    for variant in 0..3 {
        let c = rock_face(variant);
        let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
        let bed = bed(&c);
        let pool = Body {
            name: "pool".into(),
            kind: BodyKind::Basin {
                seed: c.declaration.receivers[0].seed,
            },
            level: c.level,
            stage: Stage {
                min: 0.0,
                max: 2.5,
                value: 0.0,
            },
            fetch: 1e6,
            gains: [0.0; 4],
        };
        let mut map = Map::prepare(bed.clone(), vec![pool], WATER).unwrap();
        for stage in [0.0, 0.4, 1.0, 2.0, 2.5] {
            map.set_stage(0, stage).unwrap();
            let surface = c.level + stage;
            for g in &s.falls[0].segments {
                let plunge = g.plunge(&sheet, water(&map)).unwrap();
                let (place, level) = scanned(g, &map).unwrap();
                // The water's own level, the same as the scan's, and the place to the scan's step.
                assert!(
                    (plunge.level - surface).abs() < 1e-9,
                    "level {} against {surface}",
                    plunge.level
                );
                assert!((plunge.level - level).abs() < 1e-9);
                let off = (plunge.place[0] - place[0]).hypot(plunge.place[1] - place[1]);
                assert!(off < 1e-3, "plunge {off} m from the first wet point");
                assert!((plunge.drop - (g.path[0].height - surface)).abs() < 1e-9);
                let b = g.births(&profile, &plunge);
                let power = RHO * G * g.discharge * (g.path[0].height - surface) * g.width();
                assert!(((b.foam + b.air) / power - 1.0).abs() < 1e-12);
                // Where the impact is on the rock, the path there lies on the bed at the level.
                let ground = bed.height(plunge.place).unwrap();
                if (ground - surface).abs() < 1e-3 {
                    on_rock += 1;
                }
                worst = (
                    worst.0.max((plunge.level - surface).abs()),
                    worst.1.max(off),
                );
            }
        }
    }
    assert!(on_rock > 0, "some paths reach the water down the rock");
    println!(
        "{on_rock} plunges at the shoreline on the rock; worst level error {:.1e} m, \
         worst place {:.1e} m from the scanned first wet point",
        worst.0, worst.1
    );
}
