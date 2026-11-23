//! M3.3, ruling 2 for falls: a fall follows the gauge. In the lake's valley the channel's fall
//! into the tarn runs in every sample, and the second way out's fall onto the terrace, with the
//! terrace's own fall into the tarn, only in the highest.
//! - Where both samples run a part of a lip, its discharge, launch depth and speed and its path
//!   are the mix of the two records, worked out here from the records themselves, so the sheet's
//!   thickness, the plunge's power and its impulses grow with the gauge.
//! - A part only the upper sample runs gives no sheet, no births and no impulses up to its lip
//!   cell's wetting gauge; from there its discharge and launch depth rise from nothing, with no
//!   step, to the upper sample's.
//! - At every gauge tried no running part lands on a dry cell of a reach, and the terrace, fed
//!   only by the second way out's fall, is dry while that fall's lip is dry.
#![expect(
    clippy::unwrap_used,
    clippy::float_cmp,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::fall_samples::{FOOT, prepared, water};
use support::lake::{RANGE, lower_sample, unjoined};
use thrum_river_model::Grid;
use thrum_water_model::{Below, Fall, FallSegment, Map, PlungeProfile, RiverFile, SheetProfile};

/// Sample `k`'s falls.
fn falls(file: &RiverFile, k: usize) -> &[Fall] {
    if k == 0 {
        &file.falls
    } else {
        &file.upper[k - 1].falls
    }
}

fn segment(file: &RiverFile, k: usize, id: u64) -> Option<&FallSegment> {
    falls(file, k)
        .iter()
        .flat_map(|f| &f.segments)
        .find(|s| s.id == id)
}

/// The chart body the map gives a reach.
fn body_of(file: &RiverFile, name: &str) -> usize {
    2 + file.reaches.iter().position(|r| r.name == name).unwrap()
}

/// The water's level at a point, where there is water.
fn level_at(map: &Map) -> impl Fn([f64; 2]) -> Option<f64> + '_ {
    |p| map.sample(p).filter(|s| s.depth > 0.0).map(|s| s.row.level)
}

/// The lip cell of a segment: of the two cells either side of its face's middle, the higher.
fn lip_cell(grid: &Grid, s: &FallSegment) -> usize {
    let [a, b] = s.lip;
    let c = s.centre();
    let along = [b[0] - a[0], b[1] - a[1]];
    let w = along[0].hypot(along[1]);
    let normal = [along[1] / w, -along[0] / w];
    let h = 0.5 * grid.spacing();
    let either = [
        grid.cell([c[0] + h * normal[0], c[1] + h * normal[1]])
            .unwrap(),
        grid.cell([c[0] - h * normal[0], c[1] - h * normal[1]])
            .unwrap(),
    ];
    let z = grid.heights();
    if z[either[0]] >= z[either[1]] {
        either[0]
    } else {
        either[1]
    }
}

#[test]
fn a_part_both_samples_run_is_their_mix_and_grows_with_the_gauge() {
    let (grid, file) = prepared();
    let gauges = file.gauges();
    let (mut map, _) = water(&grid, &file, true);
    let (sheet, plunge) = (SheetProfile::default(), PlungeProfile::default());
    // The parts every sample runs.
    let ids: Vec<u64> = file.falls[0]
        .segments
        .iter()
        .map(|s| s.id)
        .filter(|&id| (1..gauges.len()).all(|k| segment(&file, k, id).is_some()))
        .collect();
    assert_ne!(ids.len(), 0);
    let mut last: Option<Vec<[f64; 4]>> = None;
    let mut worst = 0.0f64;
    for step in 0..=24 {
        let g = RANGE[0] + 0.025 * f64::from(step);
        map.set_stage(0, g).unwrap();
        let k = lower_sample(&gauges, g);
        let w = (g - gauges[k]) / (gauges[k + 1] - gauges[k]);
        let lerp = |a: f64, b: f64| a + (b - a) * w;
        let now_falls = file.falls_at(g);
        let mut now = Vec::with_capacity(ids.len());
        for &id in &ids {
            let s = now_falls
                .iter()
                .flat_map(|f| &f.segments)
                .find(|s| s.id == id)
                .unwrap();
            let (a, b) = (
                segment(&file, k, id).unwrap(),
                segment(&file, k + 1, id).unwrap(),
            );
            let (pa, pb) = (a.path[a.path.len() - 1], b.path[b.path.len() - 1]);
            let end = s.path[s.path.len() - 1];
            for (got, want) in [
                (s.discharge, lerp(a.discharge, b.discharge)),
                (s.launch_depth, lerp(a.launch_depth, b.launch_depth)),
                (s.launch_speed, lerp(a.launch_speed, b.launch_speed)),
                (
                    s.path[0].place[0],
                    lerp(a.path[0].place[0], b.path[0].place[0]),
                ),
                (s.path[0].height, lerp(a.path[0].height, b.path[0].height)),
                (end.place[0], lerp(pa.place[0], pb.place[0])),
                (end.place[1], lerp(pa.place[1], pb.place[1])),
                (end.height, lerp(pa.height, pb.height)),
                (end.time, lerp(pa.time, pb.time)),
            ] {
                worst = worst.max((got - want).abs());
            }
            let p = s.plunge(&sheet, level_at(&map)).unwrap();
            let births = s.births(&plunge, &p);
            let middle = s.path.len() / 2;
            assert!(
                middle > 0 && middle + 1 < s.path.len(),
                "part {id} at {g} has no interior sheet probe"
            );
            assert!(
                s.path[0].height > s.path[middle].height && s.path[middle].height > end.height,
                "part {id} at {g}"
            );
            now.push([
                s.discharge,
                s.sheet(&sheet, middle).thickness,
                births.foam + births.air,
                s.impulse_rms(&plunge, &p),
            ]);
        }
        if let Some(prev) = &last {
            for (a, b) in prev.iter().zip(&now) {
                for q in 0..4 {
                    assert!(b[q] > a[q], "at {g}: {a:?} then {b:?}");
                }
            }
        }
        if step % 6 == 0 {
            let m = &now[ids.len() / 2];
            println!(
                "gauge {g:.3} m: middle part {:.4} m²/s, sheet {:.1} mm thick at the interior point, \
                 plunge {:.0} W, impulse rms {:.3} N·s",
                m[0],
                m[1] * 1e3,
                m[2],
                m[3]
            );
        }
        last = Some(now);
    }
    println!(
        "{} parts run in every sample; their records at each gauge are the mix of the samples' \
         to {worst:e}",
        ids.len()
    );
    assert!(worst < 1e-12);
}

#[test]
fn a_part_only_the_upper_sample_runs_rises_from_nothing_at_its_lip_s_wetting() {
    let (grid, file) = prepared();
    let gauges = file.gauges();
    let z = grid.heights();
    let (mut map, _) = water(&grid, &file, true);
    let (sheet, plunge) = (SheetProfile::default(), PlungeProfile::default());
    // The parts only the highest sample runs, with their lips' wetting gauges.
    let upper: Vec<(&FallSegment, f64)> = file.upper[1]
        .falls
        .iter()
        .flat_map(|f| &f.segments)
        .filter(|s| segment(&file, 1, s.id).is_none())
        .map(|s| {
            let wg = file.upper[1]
                .lip_wetting
                .iter()
                .find(|(id, _)| *id == s.id)
                .unwrap()
                .1;
            (s, wg)
        })
        .collect();
    assert_ne!(upper.len(), 0);
    let mut checked = 0;
    for &(b, wg) in &upper {
        assert!(wg > gauges[1] && wg < gauges[2]);
        let midpoint = f64::midpoint(gauges[1], gauges[2]);
        assert!((wg - midpoint).abs() > 1e-3, "part {}", b.id);
        let find = |g: f64| {
            file.falls_at(g)
                .into_iter()
                .flat_map(|f| f.segments)
                .find(|s| s.id == b.id)
        };
        // Nothing up to the wetting: no record, so no sheet, births or impulses.
        for g in [gauges[1], f64::midpoint(gauges[1], wg), wg - 1e-3, wg] {
            assert!(
                find(g).is_none(),
                "part {} runs before wetting at {g}",
                b.id
            );
        }
        assert_eq!(find(midpoint).is_some(), midpoint > wg, "part {}", b.id);
        // From there it rises from nothing: over a tenth of the gauge above the wetting it holds
        // a tenth of the discharge and the launch depth, and each grows at every step.
        let near = find(wg + 1e-3).unwrap();
        let nearer = find(wg + 1e-4).unwrap();
        assert!(nearer.discharge <= 0.15 * near.discharge && near.discharge <= 0.05 * b.discharge);
        assert!(nearer.launch_depth <= 0.15 * near.launch_depth);
        let mut before = (0.0, 0.0);
        let mut first_plunge = None;
        for step in 1..=20 {
            let g = wg + (gauges[2] - wg) * f64::from(step) / 20.0;
            map.set_stage(0, g).unwrap();
            let s = find(g).unwrap();
            assert!(s.discharge > before.0 && s.launch_depth > before.1);
            before = (s.discharge, s.launch_depth);
            assert!(s.sheet(&sheet, 1).thickness > 0.0);
            // Once the water below covers the ground where the path meets it, it plunges, and
            // from then on at every step. (Onto a reach that wets only as this part runs, the
            // first few millimetres of water there lie under the ground's interpolated height.)
            match s.plunge(&sheet, level_at(&map)) {
                Some(p) => {
                    assert!(s.births(&plunge, &p).foam > 0.0 && s.impulse_rms(&plunge, &p) > 0.0);
                    first_plunge.get_or_insert(g);
                }
                None => assert!(first_plunge.is_none(), "{} stops plunging at {g}", b.id),
            }
        }
        let first_plunge = first_plunge.unwrap();
        assert!(first_plunge <= f64::midpoint(wg, gauges[2]), "{}", b.id);
        // At the upper sample it is the upper's.
        let top = find(gauges[2]).unwrap();
        assert_eq!(
            (top.discharge, top.launch_depth),
            (b.discharge, b.launch_depth)
        );
        // Its lip cell holds water only above the wetting.
        let lip = lip_cell(&grid, b);
        let chart = file
            .reaches
            .iter()
            .position(|r| r.footprint[lip] && r.upper[1].wetting[lip].is_some())
            .unwrap();
        // (Within a join's blend a row is mixed toward the body joined.)
        if unjoined(&file, chart, grid.point(lip)) {
            for (g, wet) in [(wg - 1e-3, false), (wg, false), (wg + 1e-3, true)] {
                map.set_stage(0, g).unwrap();
                let row = map.row(2 + chart, grid.point(lip));
                assert_eq!(row.level > z[lip], wet, "{} at {g}", b.id);
            }
        }
        checked += 1;
        if checked == 1 || checked == upper.len() {
            println!(
                "part {} ({:?} below): none up to {wg:.4} m; {:.2e} and {:.2e} m²/s 1 mm and 0.1 mm \
                 above; {:.4} m²/s at the top; plunging from {first_plunge:.4} m",
                b.id, b.below, near.discharge, nearer.discharge, b.discharge
            );
        }
    }
    println!("{checked} parts only the highest sample runs");
}

#[test]
fn no_running_part_lands_on_a_dry_reach_and_the_terrace_waits_for_its_lip() {
    let (grid, file) = prepared();
    let gauges = file.gauges();
    let z = grid.heights();
    let n = z.len();
    let (mut map, _) = water(&grid, &file, true);
    let sheet = SheetProfile::default();
    // The terrace's reach: the one the lowest sample leaves dry above the tarn.
    let terrace = file
        .reaches
        .iter()
        .position(|r| {
            (0..n).all(|i| !r.footprint[i] || grid.point(i)[0] >= FOOT || r.levels[i] <= z[i])
        })
        .unwrap();
    let name = &file.reaches[terrace].name;
    // The terrace is fed only by the fall over the second way out.
    let feeding: Vec<&FallSegment> = file
        .upper
        .iter()
        .flat_map(|u| &u.falls)
        .chain(&file.falls)
        .flat_map(|f| &f.segments)
        .filter(|s| s.below == Below::Reach(name.clone()))
        .collect();
    assert_ne!(feeding.len(), 0);
    let lip_wetting = file.upper[1]
        .lip_wetting
        .iter()
        .filter(|(id, _)| feeding.iter().any(|s| s.id == *id))
        .map(|&(_, g)| g)
        .fold(f64::INFINITY, f64::min);
    let lips: Vec<usize> = feeding.iter().map(|s| lip_cell(&grid, s)).collect();
    let mut tried: Vec<f64> = (0..=24).map(|s| RANGE[0] + 0.025 * f64::from(s)).collect();
    tried.extend([lip_wetting - 1e-3, lip_wetting, lip_wetting + 1e-3]);
    tried.sort_by(f64::total_cmp);
    let (mut landed, mut plunged) = (0, 0);
    for &g in &tried {
        map.set_stage(0, g).unwrap();
        let k = lower_sample(&gauges, g);
        for f in file.falls_at(g) {
            for s in &f.segments {
                if let Below::Reach(r) = &s.below {
                    // The cell the path ends in, on the ground below, holds the reach's water.
                    let body = body_of(&file, r);
                    let cell = grid.cell(s.path[s.path.len() - 1].place).unwrap();
                    assert!(
                        map.row(body, grid.point(cell)).level > z[cell],
                        "{} lands on a dry cell of {r} at {g}",
                        s.id
                    );
                    plunged += usize::from(s.plunge(&sheet, level_at(&map)).is_some());
                    landed += 1;
                } else {
                    // Into a body: its water is there at every gauge.
                    assert!(
                        s.plunge(&sheet, level_at(&map)).is_some(),
                        "{} at {g}",
                        s.id
                    );
                }
            }
        }
        // The terrace is dry while the lip is.
        let lip_wet = lips
            .iter()
            .any(|&i| (0..2).any(|r| map.row(2 + r, grid.point(i)).level > z[i] && r != terrace));
        let terrace_wet = (0..n)
            .filter(|&i| {
                file.reaches[terrace].footprint[i]
                    && grid.point(i)[0] < FOOT
                    && map.row(2 + terrace, grid.point(i)).level > z[i]
            })
            .count();
        assert_eq!(lip_wet, g > lip_wetting, "at {g}");
        if !lip_wet {
            assert_eq!(terrace_wet, 0, "at {g}");
        }
        if g == RANGE[1] {
            assert!(terrace_wet > 0);
        }
        if g == lip_wetting || g == lip_wetting + 1e-3 || g == RANGE[1] {
            println!(
                "gauge {g:.4} m (samples {k} and {}): lip wet {lip_wet}, {terrace_wet} terrace cells wet",
                k + 1
            );
        }
    }
    assert!(landed > 0 && plunged > 0);
    println!(
        "{} gauges tried; {landed} landings on a reach, each in a wet cell; {plunged} of them \
         meet the water before the ground",
        tried.len()
    );
}
