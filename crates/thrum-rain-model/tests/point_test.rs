//! M1.2: the point test is exact. On 200 seeded beds and fills, the cells where a hollow's
//! level stands above the cell's bed are the cells a flood from the pits reaches under the
//! same levels. The flood is the test's own and starts from pits only.
use crate::support;
use support::{Lcg, flood_from_pits, mm_h, pits, prepare, random_bed};
use thrum_rain_model::{GroundClass, Rain, RainState, TickReport};
use thrum_water_model::NO_HOLLOW;

#[test]
fn the_point_test_matches_a_flood_from_the_pits() {
    let (mut multi, mut checked) = (0, 0);
    for seed in 0..200 {
        let mut r = Lcg(1000 + seed);
        let bed = random_bed(seed, 24 + r.below(24));
        let tree = prepare(&bed);
        let g = tree.ground();
        let z = g.heights();
        let mut state = RainState::new(&tree);
        let mut report = TickReport::default();
        let wet_ticks = 50 + r.below(400);
        let dry_ticks = r.below(300);
        let rain = Rain {
            flux: mm_h(10.0 + r.uniform() * 90.0),
            drying: 0.0,
        };
        let drying = Rain {
            flux: 0.0,
            drying: mm_h(20.0 + r.uniform() * 200.0),
        };
        let class = GroundClass::bare(mm_h(r.uniform() * 5.0));
        for t in 0..wet_ticks + dry_ticks {
            let input = if t < wet_ticks { rain } else { drying };
            state
                .step(&tree, input, class, 1.0, 1, &mut report)
                .expect("valid test case");
        }
        let levels = state.levels(&tree);
        // The point test: a cell is wet where its hollow's level stands above its bed.
        let point: Vec<Option<f64>> = (0..z.len())
            .map(|i| {
                let l = tree.labels()[i];
                (l != NO_HOLLOW && levels[l as usize] > z[i]).then(|| levels[l as usize])
            })
            .collect();
        // The flood: from each pit of the bed, under the level of the water over that pit.
        let seeds: Vec<(usize, f64)> = pits(g)
            .into_iter()
            .map(|p| (p, levels[tree.labels()[p] as usize]))
            .collect();
        let flood = flood_from_pits(g, &seeds);
        let differ = (0..z.len()).filter(|&i| point[i] != flood[i]).count();
        assert_eq!(differ, 0, "seed {seed}: {differ} cells differ");
        // Count the cases whose puddles span several hollows and stand at several levels.
        let mut wet_levels: Vec<u64> = point.iter().flatten().map(|l| l.to_bits()).collect();
        wet_levels.sort_unstable();
        wet_levels.dedup();
        let spans = state
            .puddles()
            .any(|(k, v)| v > 0.0 && tree.node(k).children.is_some());
        if spans && wet_levels.len() > 1 {
            multi += 1;
        }
        checked += 1;
    }
    assert_eq!(checked, 200);
    assert!(
        multi >= 40,
        "{multi} cases with joined puddles beside others"
    );
}

#[test]
fn one_level_for_the_whole_body_fails_the_flood() {
    // The same comparison catches a point test that reads one level for every hollow.
    let mut caught = 0;
    for seed in 0..20 {
        let tree = prepare(&random_bed(seed, 40));
        let g = tree.ground();
        let z = g.heights();
        let mut state = RainState::new(&tree);
        support::run(
            &tree,
            &mut state,
            Rain {
                flux: mm_h(30.0),
                drying: 0.0,
            },
            GroundClass::bare(0.0),
            1.0,
            300,
        );
        let levels = state.levels(&tree);
        let one = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let wrong: Vec<Option<f64>> = (0..z.len())
            .map(|i| (tree.labels()[i] != NO_HOLLOW && one > z[i]).then_some(one))
            .collect();
        let seeds: Vec<(usize, f64)> = pits(g)
            .into_iter()
            .map(|p| (p, levels[tree.labels()[p] as usize]))
            .collect();
        let flood = flood_from_pits(g, &seeds);
        if (0..z.len()).any(|i| wrong[i] != flood[i]) {
            caught += 1;
        }
    }
    assert!(caught >= 15, "one level caught on {caught} of 20 beds");
}
