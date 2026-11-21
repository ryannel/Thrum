//! M3.8, ruling 2's wetting: between two samples a cell holds water only from its wetting gauge,
//! and wetting spreads from the lower sample's water and from where an inflow enters. In the
//! lake's valley with its ridge and second way out, as parts 8 and 10 of the design proof judge
//! them, at gauges either side of each wetting:
//! - the hollow behind the ridge is dry, with the ridge, below the ridge's wetting and fills from
//!   its bed once the notch is wet;
//! - the terrace below the second way out's fall, dry in the lower samples, is dry below the
//!   lip's wetting, then wet from its landing to its lower end, rising from nothing with no step,
//!   and at the highest sample its row is that sample's;
//! - no wet cell is cut off from both the lower sample's water and a running inflow, and a dry
//!   cell has no wave support and no shore margin.
//!
//! It also shows that a dry cell put at the nearest wet level would wet the hollow, and that a
//! flood seeded only from the lower sample's wet cells never reaches the terrace.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use std::collections::VecDeque;
use support::lake::{
    BANKS, CENTRE, FOOT, HOLLOW, NOTCH, RANGE, RIDGE, lower_sample, sampled, stored, unjoined,
    water,
};
use thrum_river_model::Grid;
use thrum_water_model::{Below, Field, Map, RiverFile};

/// Whether cell `i` lies above the tarn's cliff, not in the tarn, where a chart's footprint
/// also takes a strip of the tarn beyond its brink.
fn above_tarn(grid: &Grid, i: usize) -> bool {
    grid.point(i)[0] < FOOT
}

/// The valley's charts: (reach index, body index) of the terrace's reach, the one the lowest
/// sample leaves dry, and the channel's.
fn charts(grid: &Grid, file: &RiverFile) -> [(usize, usize); 2] {
    let z = grid.heights();
    let terrace = file
        .reaches
        .iter()
        .position(|r| {
            r.levels
                .iter()
                .zip(z)
                .zip(&r.footprint)
                .enumerate()
                .all(|(i, ((l, h), &f))| !f || !above_tarn(grid, i) || l <= h)
        })
        .unwrap();
    assert_eq!(file.reaches.len(), 2);
    [(terrace, 2 + terrace), (1 - terrace, 3 - terrace)]
}

/// The cells the charts wet now, and the level there.
fn water_now(
    map: &Map,
    grid: &Grid,
    file: &RiverFile,
    charts: [(usize, usize); 2],
) -> (Vec<bool>, Vec<f64>) {
    let z = grid.heights();
    let mut wet = vec![false; z.len()];
    let mut level = vec![f64::NEG_INFINITY; z.len()];
    for (r, b) in charts {
        for i in 0..z.len() {
            let l = map.row(b, grid.point(i)).level;
            if file.reaches[r].footprint[i] && l > z[i] {
                wet[i] = true;
                level[i] = level[i].max(l);
            }
        }
    }
    (wet, level)
}

fn still() -> Field {
    Field {
        layers: std::array::from_fn(|_| thrum_sea_model::LongWaves::new(vec![], 0.0).unwrap()),
    }
}

/// The cells of the falls running at `g` that land in a reach, by their path's last point.
fn landings(grid: &Grid, file: &RiverFile, g: f64) -> Vec<(String, usize)> {
    file.falls_at(g)
        .iter()
        .flat_map(|f| &f.segments)
        .filter_map(|s| match &s.below {
            Below::Reach(name) => Some((
                name.clone(),
                grid.cell(s.path[s.path.len() - 1].place).unwrap(),
            )),
            Below::Body(_) => None,
        })
        .collect()
}

/// The wet cells water reaches from `seeds`: from a reached cell into a wet neighbour of the
/// same reach whose bed lies under the reached cell's level. Reaches part at brinks, which only
/// a fall crosses.
fn flood(
    grid: &Grid,
    file: &RiverFile,
    wet: &[bool],
    level: &[f64],
    seeds: impl IntoIterator<Item = usize>,
) -> Vec<bool> {
    let bed = grid.heights();
    let [nx, nz] = grid.size();
    let mut seen = vec![false; wet.len()];
    let mut queue: VecDeque<usize> = seeds.into_iter().filter(|&i| wet[i]).collect();
    for &i in &queue {
        seen[i] = true;
    }
    while let Some(i) = queue.pop_front() {
        let (x, z) = (i % nx, i / nx);
        let mut next = Vec::with_capacity(4);
        if x > 0 {
            next.push(i - 1);
        }
        if x + 1 < nx {
            next.push(i + 1);
        }
        if z > 0 {
            next.push(i - nx);
        }
        if z + 1 < nz {
            next.push(i + nx);
        }
        for j in next {
            let together = file
                .reaches
                .iter()
                .any(|r| r.footprint[i] && r.footprint[j]);
            if wet[j] && !seen[j] && together && level[i] > bed[j] {
                seen[j] = true;
                queue.push_back(j);
            }
        }
    }
    seen
}

#[test]
fn wetting_spreads_from_the_lower_sample_and_the_inflows_and_waits_for_its_gauge() {
    let (grid, samples, file) = sampled(BANKS);
    let gauges = samples.gauges.clone();
    let z = grid.heights();
    let n = z.len();
    let [(tr, tb), (cr, cb)] = charts(&grid, &file);
    let terrace_name = file.reaches[tr].name.clone();
    let (mut map, _) = water(&grid, &file, RANGE, RANGE[0], true);
    let field = still();

    // The hollow and the ridge's notch, on the channel's chart.
    let in_hollow = |p: [f64; 2]| {
        let off = p[1] - CENTRE;
        (HOLLOW[0]..HOLLOW[1]).contains(&off) && (13.0..17.0).contains(&p[0])
    };
    let in_notch = |p: [f64; 2]| {
        let off = p[1] - CENTRE;
        (RIDGE[0]..RIDGE[1]).contains(&off) && (NOTCH[0]..NOTCH[1]).contains(&p[0])
    };
    let hollow: Vec<usize> = (0..n).filter(|&i| in_hollow(grid.point(i))).collect();
    let notch: Vec<usize> = (0..n).filter(|&i| in_notch(grid.point(i))).collect();
    let channel = &file.reaches[cr];
    // The hollow is dry in the lowest sample and wet in the middle one; the ridge's wetting is
    // its lowest point's, the notch's.
    assert!(hollow.iter().all(|&i| stored(&file, cr, 0, i).0 <= z[i]));
    assert!(hollow.iter().all(|&i| stored(&file, cr, 1, i).0 > z[i]));
    let ridge = notch
        .iter()
        .filter_map(|&i| channel.upper[0].wetting[i])
        .fold(f64::INFINITY, f64::min);
    assert!(ridge > gauges[0] && ridge < gauges[1], "{ridge}");
    // The terrace: dry in the two lower samples, wet in the highest; fed by one fall whose lip
    // only the highest sample runs.
    let terrace_wet: Vec<usize> = (0..n)
        .filter(|&i| {
            above_tarn(&grid, i)
                && file.reaches[tr].footprint[i]
                && stored(&file, tr, 2, i).0 > z[i]
        })
        .collect();
    assert_ne!(terrace_wet.len(), 0);
    assert!((0..n).all(|i| !above_tarn(&grid, i)
        || stored(&file, tr, 0, i).0 <= z[i] && stored(&file, tr, 1, i).0 <= z[i]));
    let feeding: Vec<_> = file.upper[1]
        .falls
        .iter()
        .flat_map(|f| &f.segments)
        .filter(|s| s.below == Below::Reach(terrace_name.clone()))
        .collect();
    assert_ne!(feeding.len(), 0);
    let lip = feeding
        .iter()
        .map(|s| {
            file.upper[1]
                .lip_wetting
                .iter()
                .find(|(id, _)| *id == s.id)
                .unwrap()
                .1
        })
        .fold(f64::INFINITY, f64::min);
    assert!(lip > gauges[1] && lip < gauges[2], "{lip}");
    let landing_x = feeding
        .iter()
        .map(|s| s.path[s.path.len() - 1].place[0])
        .fold(f64::INFINITY, f64::min);
    println!(
        "samples at {gauges:?}; the ridge's notch wets at {ridge:.4} m, the second way out's lip at \
         {lip:.4} m; {} hollow cells, {} terrace cells wet at the top, landing from x = {landing_x:.2} m",
        hollow.len(),
        terrace_wet.len()
    );

    // Gauges either side of each wetting, the samples and between.
    let mut tried = vec![
        gauges[0],
        f64::midpoint(gauges[0], ridge),
        ridge - 1e-3,
        ridge,
        ridge + 1e-3,
        ridge + 0.02,
        gauges[1],
        f64::midpoint(gauges[1], lip),
        lip - 1e-3,
        lip,
        lip + 1e-3,
        lip + 0.02,
        f64::midpoint(lip, gauges[2]),
        gauges[2],
    ];
    tried.sort_by(f64::total_cmp);
    for &g in &tried {
        map.set_stage(0, g).unwrap();
        let (wet, level) = water_now(&map, &grid, &file, [(tr, tb), (cr, cb)]);
        // The hollow and the notch.
        let hollow_wet = hollow.iter().filter(|&&i| wet[i]).count();
        let notch_wet = notch.iter().filter(|&&i| wet[i]).count();
        if g <= ridge {
            assert_eq!((hollow_wet, notch_wet), (0, 0), "at {g}");
        } else {
            assert_eq!(hollow_wet, hollow.len(), "at {g}");
        }
        // The terrace.
        let terrace_now = terrace_wet.iter().filter(|&&i| wet[i]).count();
        let landed = landings(&grid, &file, g);
        let feeding_now = landed.iter().filter(|(r, _)| *r == terrace_name).count();
        if g <= lip {
            assert_eq!((terrace_now, feeding_now), (0, 0), "at {g}");
        } else {
            assert!(feeding_now > 0, "at {g}");
            // From the landing to the lower end, outside any join's blend.
            let below_landing = terrace_wet
                .iter()
                .filter(|&&i| grid.point(i)[0] >= landing_x && unjoined(&file, tr, grid.point(i)))
                .collect::<Vec<_>>();
            let dry: Vec<_> = below_landing
                .iter()
                .filter(|&&&i| !wet[i])
                .map(|&&i| {
                    (
                        grid.point(i),
                        map.row(tb, grid.point(i)).level - z[i],
                        file.reaches[tr].upper[1].wetting[i],
                    )
                })
                .collect();
            assert!(dry.is_empty(), "at {g}: {dry:?}");
        }
        // Every running fall lands on water, and no wet cell is cut off from both the lower
        // sample's water and a running inflow.
        let k = lower_sample(&gauges, g);
        assert!(landed.iter().all(|&(_, c)| wet[c]), "at {g}: {landed:?}");
        let lower_water = (0..n).filter(|&i| {
            (0..2).any(|r| stored(&file, r, k, i).0 > z[i] && file.reaches[r].footprint[i])
        });
        let reached = flood(
            &grid,
            &file,
            &wet,
            &level,
            lower_water.chain(landed.iter().map(|&(_, c)| c)),
        );
        let loose = (0..n).filter(|&i| wet[i] && !reached[i]).count();
        assert_eq!(loose, 0, "at {g}");
        // A cell wet in the upper sample that is dry now has no wave support and no margin.
        let upper = (k + 1).min(gauges.len() - 1);
        let mut waiting = 0;
        for i in 0..n {
            if (0..2).any(|r| file.reaches[r].footprint[i] && stored(&file, r, upper, i).0 > z[i])
                && !wet[i]
            {
                waiting += 1;
                let p = grid.point(i);
                assert!(
                    map.sample(p).is_none(),
                    "at {g}, cell {i}: {:?}",
                    map.sample(p)
                );
                assert!(map.wave_support(&field, p).is_none());
            }
        }
        println!(
            "gauge {g:.4} m: hollow {hollow_wet}/{} wet, notch {notch_wet}/{}, terrace {terrace_now}/{} wet, \
             {feeding_now} running parts land on it; {waiting} cells wait for their wetting, none \
             with water or margin; no loose wet cell",
            hollow.len(),
            notch.len(),
            terrace_wet.len()
        );
    }
}

#[test]
fn the_hollow_and_the_terrace_rise_from_nothing_with_no_step() {
    let (grid, samples, file) = sampled(BANKS);
    let gauges = samples.gauges.clone();
    let z = grid.heights();
    let n = z.len();
    let [(tr, tb), (cr, cb)] = charts(&grid, &file);
    let (mut map, _) = water(&grid, &file, RANGE, RANGE[0], true);
    let hollow: Vec<usize> = (0..n)
        .filter(|&i| {
            let p = grid.point(i);
            let off = p[1] - CENTRE;
            (HOLLOW[0]..HOLLOW[1]).contains(&off) && (13.0..17.0).contains(&p[0])
        })
        .collect();
    // The terrace's wet cells at the top, short of the tarn's join.
    let terrace: Vec<usize> = (0..n)
        .filter(|&i| {
            above_tarn(&grid, i)
                && unjoined(&file, tr, grid.point(i))
                && file.reaches[tr].footprint[i]
                && stored(&file, tr, 2, i).0 > z[i]
        })
        .collect();
    for (what, cells, r, b, k) in [
        ("hollow", &hollow, cr, cb, 0),
        ("terrace", &terrace, tr, tb, 1),
    ] {
        let wg = cells
            .iter()
            .filter_map(|&i| file.reaches[r].upper[k].wetting[i])
            .fold(f64::INFINITY, f64::min);
        // Depth and speed in each cell, swept from 2 mm below the first wetting to 2 cm above
        // in steps of `h`: the largest rise over one step. None is wet up to the wetting.
        let mut sweep = |h: f64| {
            let (mut rise, mut faster) = (0.0f64, 0.0f64);
            let mut last: Option<Vec<(f64, f64)>> = None;
            let mut g = wg - 2e-3;
            while g <= wg + 0.02 {
                map.set_stage(0, g).unwrap();
                let now: Vec<(f64, f64)> = cells
                    .iter()
                    .map(|&i| {
                        let row = map.row(b, grid.point(i));
                        (
                            (row.level - z[i]).max(0.0),
                            row.current[0].hypot(row.current[1]),
                        )
                    })
                    .collect();
                if g <= wg {
                    assert!(now.iter().all(|&(d, _)| d == 0.0), "{what} wet at {g}");
                }
                if let Some(prev) = &last {
                    for (a, c) in prev.iter().zip(&now) {
                        assert!(c.0 >= a.0, "{what} falls at {g}");
                        rise = rise.max(c.0 - a.0);
                        faster = faster.max(c.1 - a.1);
                    }
                }
                last = Some(now);
                g += h;
            }
            (rise, faster)
        };
        let coarse = sweep(1e-3);
        let fine = sweep(1e-4);
        let mut deepest = |g: f64| {
            map.set_stage(0, g).unwrap();
            cells
                .iter()
                .map(|&i| map.row(b, grid.point(i)).level - z[i])
                .fold(0.0, f64::max)
        };
        let (near, nearer) = (deepest(wg + 1e-3), deepest(wg + 1e-4));
        println!(
            "{what}: wets from {wg:.4} m; over 1 mm of gauge its depth rises by at most {:.3} mm and \
             its speed by {:.2} mm/s, over 0.1 mm by {:.4} mm and {:.3} mm/s; 1 mm and 0.1 mm \
             above its wetting the deepest cell holds {:.4} mm and {:.4} mm",
            coarse.0 * 1e3,
            coarse.1 * 1e3,
            fine.0 * 1e3,
            fine.1 * 1e3,
            near * 1e3,
            nearer * 1e3
        );
        // No step: the rise over a step shrinks with the step.
        assert!(coarse.0 > 0.0);
        assert!(fine.0 <= 0.15 * coarse.0, "{what}: {fine:?} {coarse:?}");
        assert!(
            fine.1 <= 0.15 * coarse.1 + 1e-12,
            "{what}: {fine:?} {coarse:?}"
        );
        assert!(
            near > 0.0 && nearer <= 0.15 * near,
            "{what}: {near} {nearer}"
        );
    }
    // At the highest sample the terrace's row is that sample's.
    map.set_stage(0, gauges[2]).unwrap();
    let (mut worst_level, mut worst_current) = (0.0f64, 0.0f64);
    for &i in &terrace {
        let row = map.row(tb, grid.point(i));
        let (level, current) = stored(&file, tr, 2, i);
        worst_level = worst_level.max((row.level - level).abs());
        worst_current =
            worst_current.max((row.current[0] - current[0]).hypot(row.current[1] - current[1]));
    }
    println!(
        "at {:.2} m the terrace's row is the highest sample's: off by {worst_level:e} m and \
         {worst_current:e} m/s in {} cells",
        gauges[2],
        terrace.len()
    );
    assert!(worst_level == 0.0 && worst_current == 0.0);
}

#[test]
fn the_rules_the_wetting_replaces_would_fail() {
    let (grid, samples, file) = sampled(BANKS);
    let z = grid.heights();
    let n = z.len();
    let [(tr, tb), (cr, cb)] = charts(&grid, &file);
    // A dry cell put at its nearest wet cell's level: at the lowest sample the hollow would take
    // the pool's level, above its floor, and be wet behind a ridge that holds it dry.
    let hollow: Vec<usize> = (0..n)
        .filter(|&i| {
            let p = grid.point(i);
            let off = p[1] - CENTRE;
            (HOLLOW[0]..HOLLOW[1]).contains(&off) && (13.0..17.0).contains(&p[0])
        })
        .collect();
    let wet0: Vec<usize> = (0..n)
        .filter(|&i| file.reaches[cr].footprint[i] && stored(&file, cr, 0, i).0 > z[i])
        .collect();
    let nearest_wet = |i: usize| {
        let p = grid.point(i);
        let j = *wet0
            .iter()
            .min_by(|&&a, &&b| {
                let d = |c: usize| {
                    let q = grid.point(c);
                    (q[0] - p[0]).hypot(q[1] - p[1])
                };
                d(a).total_cmp(&d(b))
            })
            .unwrap();
        stored(&file, cr, 0, j).0
    };
    let flooded = hollow.iter().filter(|&&i| nearest_wet(i) > z[i]).count();
    println!(
        "at the lowest sample a nearest-wet-level rule wets {flooded} of the {} hollow cells",
        hollow.len()
    );
    assert_eq!(flooded, hollow.len());
    // A flood seeded only from the lower sample's wet cells, above the lip's wetting: it never
    // reaches the terrace the fall now feeds.
    let (mut map, _) = water(&grid, &file, RANGE, RANGE[0], true);
    let g = f64::midpoint(samples.gauges[1], samples.gauges[2]) + 0.1;
    map.set_stage(0, g).unwrap();
    let (wet, level) = water_now(&map, &grid, &file, [(tr, tb), (cr, cb)]);
    let lower = (0..n).filter(|&i| {
        (0..2).any(|r| file.reaches[r].footprint[i] && stored(&file, r, 1, i).0 > z[i])
    });
    let reached = flood(&grid, &file, &wet, &level, lower);
    let terrace_wet = (0..n)
        .filter(|&i| above_tarn(&grid, i) && file.reaches[tr].footprint[i] && wet[i])
        .count();
    let terrace_reached = (0..n)
        .filter(|&i| above_tarn(&grid, i) && file.reaches[tr].footprint[i] && reached[i])
        .count();
    println!(
        "at {g:.2} m the terrace has {terrace_wet} wet cells; a flood from the lower sample's water \
         alone reaches {terrace_reached}"
    );
    assert!(terrace_wet > 0);
    assert_eq!(terrace_reached, 0);
}
