//! M2.4: a fall record gives, for each place along the lip, the water's path with its time, its
//! discharge for each metre of lip and its launch speed, with stable IDs; the f64 reference in
//! `thrum-water-model` gives the sheet's speed, thickness, white share and presence along it.
//! The expected values are part 4 of `tests/fixtures/rivers/river_proof.py`, worked
//! out here: `hc = (q^2/g)^(1/3)`, `hb = 0.715 hc`, `ub = q/hb`, `t = sqrt(2H/g)`, the throw
//! `ub t`, the speed `hypot(ub, g t)`, the thickness `q/v` and the breakup length `6 q^0.32`.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it measured"
)]
use crate::support;
use support::fall::{Channel, Cliff, Foot, channel};
use thrum_river_model::{Profile, Solution, key, solve, write};
use thrum_water_model::{Bed, FallSegment, RiverFile, SheetProfile};

const G: f64 = 9.81;
const DX: f64 = 0.25;

fn fall(unit: f64, cliff: Cliff) -> (Channel, Solution) {
    let c = channel(
        DX,
        40,
        0.005,
        unit,
        0.04,
        cliff,
        Foot::Pool {
            cells: 24,
            depth: 0.5,
        },
    );
    let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
    (c, s)
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Part 4's closed forms for a discharge `q` and a drop `h`: (launch speed, time, throw,
/// speed, thickness, breakup length).
fn part4(q: f64, h: f64) -> (f64, f64, f64, f64, f64, f64) {
    let hc = (q * q / G).cbrt();
    let hb = 0.715 * hc;
    let ub = q / hb;
    let t = (2.0 * h / G).sqrt();
    let v = ub.hypot(G * t);
    (ub, t, ub * t, v, q / v, 6.0 * q.powf(0.32))
}

fn horizontal(g: &FallSegment, k: usize) -> f64 {
    let (a, b) = (g.path[0].place, g.path[k].place);
    (b[0] - a[0]).hypot(b[1] - a[1])
}

#[test]
fn the_reference_gives_part_4s_table() {
    // Rows of part 4's printed table: q, height, speed at the foot, thickness there (mm).
    let rows = [
        (0.005, 1.0, 4.46, 1.1),
        (0.05, 3.0, 7.75, 6.5),
        (0.2, 8.0, 12.65, 15.8),
        (1.0, 12.0, 15.63, 64.0),
    ];
    let profile = SheetProfile::default();
    for (q, h, speed, thickness) in rows {
        let (ub, _, _, v, d, length) = part4(q, h);
        let s = profile.sample(q, ub, h, length);
        assert!((s.speed / v - 1.0).abs() < 1e-12);
        assert!(
            (s.speed - speed).abs() < 0.006,
            "{q} m2/s, {h} m: {} m/s",
            s.speed
        );
        assert!((1000.0 * s.thickness - thickness).abs() < 0.06);
        assert!((s.thickness / d - 1.0).abs() < 1e-12);
        assert_eq!(s.white, 1.0);
        assert_eq!(s.presence, (d / 0.005).min(1.0));
        assert_eq!(profile.sample(q, ub, 0.0, 0.0).white, 0.0);
    }
}

#[test]
fn clear_of_the_rock_the_path_is_the_free_path_and_the_sheet_follows_its_closed_forms() {
    let profile = SheetProfile::default();
    let cliff = Cliff {
        height: 6.0,
        face_cells: 0,
    };
    for unit in [0.005, 0.2, 1.0] {
        let (c, s) = fall(unit, cliff);
        let fall = &s.falls[0];
        let mut ids: Vec<u64> = fall.segments.iter().map(|g| g.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            c.rows().len(),
            "one segment, one ID, for each place"
        );
        let mut worst = 0.0_f64;
        for g in &fall.segments {
            let q = g.discharge;
            let (ub, _, _, _, _, length) = part4(q, 0.0);
            assert!((g.launch_speed / ub - 1.0).abs() < 1e-12);
            let top = g.path[0].height;
            for (k, p) in g.path.iter().enumerate() {
                let tau = p.time;
                let (_, _, throw, v, d, _) = part4(q, 0.5 * G * tau * tau);
                assert!((horizontal(g, k) - throw).abs() <= 1e-9 * (1.0 + throw));
                assert!((top - p.height - 0.5 * G * tau * tau).abs() < 1e-9);
                let sheet = g.sheet(&profile, k);
                assert!((sheet.speed / v - 1.0).abs() < 1e-9);
                assert!((sheet.thickness / d - 1.0).abs() < 1e-9);
                assert_eq!(sheet.white, smooth(p.distance / length));
                assert_eq!(sheet.presence, (sheet.thickness / 0.005).min(1.0));
            }
            assert_eq!(g.sheet(&profile, 0).white, 0.0);
            // The throw where the path meets the pool, against the free path to that drop.
            let end = g.path.len() - 1;
            let (_, _, throw, _, _, _) = part4(q, top - c.level);
            let error = horizontal(g, end) / throw - 1.0;
            worst = worst.max(error.abs());
            assert!(error.abs() < 0.01, "throw off by {error}");
        }
        let g = &fall.segments[2];
        let foot = g.sheet(&profile, g.path.len() - 1);
        println!(
            "{unit} m2/s: launch {:.2} m/s, throw {:.3} m to the pool (worst {:.2e} off), \
             {:.1} mm thick at {:.2} m/s, white {:.2}, presence {:.2}",
            g.launch_speed,
            horizontal(g, g.path.len() - 1),
            worst,
            1000.0 * foot.thickness,
            foot.speed,
            foot.white,
            foot.presence
        );
    }
}

#[test]
fn where_the_rock_stands_in_its_way_the_path_follows_its_face_and_never_passes_through_the_bed() {
    // A face falling 4 m over 2 m: the free path from the lip would pass through it.
    let (c, s) = fall(
        0.15,
        Cliff {
            height: 4.0,
            face_cells: 7,
        },
    );
    let bed = Bed::new(
        c.grid.origin(),
        c.grid.spacing(),
        c.grid.size(),
        c.grid.heights().to_vec(),
    )
    .unwrap();
    let profile = SheetProfile::default();
    for g in &s.falls[0].segments {
        let mut on_face = 0;
        let top = g.path[0].height;
        for (k, p) in g.path.iter().enumerate() {
            let b = bed.height(p.place).unwrap();
            assert!(
                p.height >= b - 1e-9,
                "point {k} under the bed by {}",
                b - p.height
            );
            if (p.height - b).abs() < 1e-6 && k > 0 {
                on_face += 1;
                // On the face the speed is still energy's.
                let v = g
                    .launch_speed
                    .mul_add(g.launch_speed, 2.0 * G * (top - p.height))
                    .sqrt();
                assert!((g.sheet(&profile, k).speed / v - 1.0).abs() < 1e-12);
            }
        }
        // Between stored points, on the chord, the path stays over the bed to a millimetre.
        for w in g.path.windows(2) {
            for j in 1..10 {
                let f = f64::from(j) / 10.0;
                let p = [
                    w[0].place[0] + f * (w[1].place[0] - w[0].place[0]),
                    w[0].place[1] + f * (w[1].place[1] - w[0].place[1]),
                ];
                let y = w[0].height + f * (w[1].height - w[0].height);
                assert!(y >= bed.height(p).unwrap() - 1e-3);
            }
        }
        assert!(
            on_face > 3,
            "the path follows the face: {on_face} points on it"
        );
        // The free path from the lip would have passed through the face.
        let free_end = g.launch_speed * (2.0 * (top - c.level) / G).sqrt();
        assert!(free_end < (c.foot - c.lip) as f64 * DX);
    }
    let g = &s.falls[0].segments[2];
    println!(
        "a face of 4 m over 2 m: {} points, landing {:.2} m out at {:.2} m/s",
        g.path.len(),
        horizontal(g, g.path.len() - 1),
        g.sheet(&profile, g.path.len() - 1).speed
    );
}

#[test]
fn the_river_file_keeps_the_fall_records_and_their_ids_are_stable() {
    let cliff = Cliff {
        height: 3.0,
        face_cells: 0,
    };
    let (c, s) = fall(0.15, cliff);
    let k = key(&c.grid, &c.declaration, &Profile::default());
    let read = RiverFile::read(&write(&s.file(k).unwrap()), &k).unwrap();
    assert_eq!(read.falls, s.falls);
    assert_eq!(read.reaches, s.reaches().unwrap());
    let (_, again) = fall(0.6, cliff);
    let ids = |s: &Solution| -> Vec<u64> { s.falls[0].segments.iter().map(|g| g.id).collect() };
    assert_eq!(ids(&s), ids(&again));
}
