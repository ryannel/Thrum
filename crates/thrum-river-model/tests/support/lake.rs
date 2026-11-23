//! A lake's outlet, made by these tests and named nowhere else. A lake held as a source body
//! spills over a broad sill 2 m wide into a channel on a slope of 2%. Part way down, the channel
//! deepens into a pool. On one side of the pool a ridge with a notch shuts off a hollow; on the
//! other a low second way out, a short channel through the bank, leads to a cliff and a terrace below it that
//! runs down beside the channel. At its foot the channel drops over a cliff into a tarn, which
//! the terrace also reaches. x runs down the valley; the channel's centre line is z = 8 m.
use thrum_river_model::{Declaration, Grid, Manning, Receiver, Source};
use thrum_water_model::{
    Bed, Body, BodyKind, EndKind, Gauge, Map, Profile as WaterProfile, RiverFile, Stage,
};

pub(crate) const CENTRE: f64 = 8.0;
/// The sill's crest.
pub(crate) const SILL: f64 = 3.0;
/// The tarn's level.
pub(crate) const TARN: f64 = -1.0;
/// The channel's half width.
pub(crate) const HALF: f64 = 1.0;
/// Where the pool lies along the channel.
pub(crate) const POOL: [f64; 2] = [12.0, 18.0];
/// Where the notches lie along the pool, the ridge's and the second way out's.
pub(crate) const NOTCH: [f64; 2] = [14.5, 15.5];
/// Across the ridge (+z of the channel) and the hollow behind it, offsets from the centre.
pub(crate) const RIDGE: [f64; 2] = [1.25, 1.75];
pub(crate) const HOLLOW: [f64; 2] = [1.75, 3.25];
/// The second way out (-z of the channel): its sill runs to the cliff at this offset.
pub(crate) const BRINK: f64 = 2.5;
/// The terrace below that cliff, offsets from the centre.
pub(crate) const TERRACE: [f64; 2] = [2.75, 5.25];
/// Where the channel's cliff into the tarn stands.
pub(crate) const FOOT: f64 = 26.0;
pub(crate) const MANNING: f64 = 0.04;
const PLATEAU: f64 = 8.0;

/// What sets the heights round the pool.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Banks {
    /// The ridge's crest and its notch's.
    pub(crate) ridge: f64,
    pub(crate) notch: f64,
    /// The hollow's floor.
    pub(crate) hollow: f64,
    /// The second way out's sill.
    pub(crate) outlet: f64,
}

/// The channel's floor on its centre line.
pub(crate) fn floor(x: f64) -> f64 {
    let line = if x < 9.0 {
        SILL
    } else {
        SILL - 0.02 * (x - 9.0)
    };
    if (POOL[0]..POOL[1]).contains(&x) {
        line - 0.4
            * (std::f64::consts::PI * (x - POOL[0]) / (POOL[1] - POOL[0]))
                .sin()
                .powi(2)
    } else {
        line
    }
}

/// The bed at a world point.
pub(crate) fn bed(banks: Banks, x: f64, z: f64) -> f64 {
    let off = z - CENTRE;
    let a = off.abs();
    // The tarn below both cliffs.
    if (FOOT..33.0).contains(&x) && (1.0..15.0).contains(&z) {
        return TARN - 1.0;
    }
    if x >= FOOT {
        return PLATEAU;
    }
    // The terrace, below the second way out's cliff, runs on to the tarn.
    let terrace = banks.outlet - 2.0 - 0.02 * (x - NOTCH[0]);
    if (13.0..FOOT).contains(&x) && (-TERRACE[1]..-TERRACE[0]).contains(&off) {
        return terrace;
    }
    if x < 6.0 {
        // The lake, in a basin.
        return if (1.0..6.0).contains(&x) && (3.0..13.0).contains(&z) {
            2.0
        } else {
            PLATEAU
        };
    }
    let f = floor(x);
    let bank = (f + 6.0 * (a - HALF).max(0.0)).min(PLATEAU);
    let in_pool = (POOL[0] + 0.5..POOL[1] - 0.5).contains(&x);
    let notch = (NOTCH[0]..NOTCH[1]).contains(&x);
    if in_pool && off > 0.0 {
        if (RIDGE[0]..RIDGE[1]).contains(&off) {
            return if notch { banks.notch } else { banks.ridge };
        }
        if (HOLLOW[0]..HOLLOW[1]).contains(&off) && (13.0..17.0).contains(&x) {
            return banks.hollow;
        }
        if (HALF..RIDGE[0]).contains(&off) {
            return bank.min(banks.notch);
        }
    }
    if notch && off < 0.0 && (HALF..BRINK).contains(&a) {
        return banks.outlet;
    }
    if (13.0..FOOT).contains(&x) && (-TERRACE[0]..-BRINK).contains(&off) {
        // The cliff's top edge: plateau between the outlet's cliff and the terrace.
        return if notch { terrace } else { bank };
    }
    bank
}

/// The valley at `dx` cells, its lake ranging over `range`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell counts of a positive extent"
)]
pub(crate) fn lake(dx: f64, banks: Banks, range: [f64; 2]) -> (Grid, Declaration) {
    let (nx, nz) = ((34.0 / dx).round() as usize, (16.0 / dx).round() as usize);
    let origin = [0.5 * dx, 0.5 * dx];
    let heights = (0..nx * nz)
        .map(|i| {
            bed(
                banks,
                origin[0] + (i % nx) as f64 * dx,
                origin[1] + (i / nx) as f64 * dx,
            )
        })
        .collect();
    let grid = Grid::new(origin, dx, [nx, nz], heights).expect("lake grid");
    let declaration = Declaration {
        name: "outlet".into(),
        springs: vec![],
        sources: vec![Source {
            name: "lake".into(),
            seed: [3.5, CENTRE],
            extent: [[0.0, 0.0], [6.0, 16.0]],
            level: range[0],
            range,
        }],
        receivers: vec![Receiver {
            name: "tarn".into(),
            seed: [30.0, CENTRE],
            extent: [[FOOT, 0.0], [34.0, 16.0]],
            level: TARN,
        }],
        manning: Manning::Uniform(MANNING),
    };
    (grid, declaration)
}

/// Banks that no sample overtops: a plain channel with a pool.
pub(crate) const HIGH: Banks = Banks {
    ridge: 7.0,
    notch: 7.0,
    hollow: 7.0,
    outlet: 7.0,
};

/// Banks the samples of a lake ranging from 3.1 m to 3.7 m reach in turn: the pool tops the
/// ridge's notch between the lowest sample and the middle one (where it stands at 2.89 m and
/// 3.08 m), and the second way out's sill between the middle one and the highest (3.26 m).
pub(crate) const BANKS: Banks = Banks {
    ridge: 3.5,
    notch: 2.98,
    hollow: 2.7,
    outlet: 3.17,
};

pub(crate) const WATER: WaterProfile = WaterProfile {
    margin_height: 0.3,
    swash_ratio: 0.7,
};

/// The bed the map takes, the solve's own grid.
pub(crate) fn bed_of(grid: &Grid) -> Bed {
    Bed::new(
        grid.origin(),
        grid.spacing(),
        grid.size(),
        grid.heights().to_vec(),
    )
    .expect("bed")
}

pub(crate) fn basin(name: &str, seed: [f64; 2], level: f64, stage: Stage) -> Body {
    Body {
        name: name.into(),
        kind: BodyKind::Basin { seed },
        level,
        stage,
        fetch: 1e6,
        gains: [0.0; 4],
    }
}

pub(crate) fn chart_body(name: &str, chart: thrum_water_model::Chart) -> Body {
    Body {
        name: name.into(),
        kind: BodyKind::Chart(chart),
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    }
}

/// The lake (body 0, its stage the lake's level over `range`), the tarn (body 1) and one chart
/// for each of the file's reaches (bodies 2 and on). With `river`, the reaches are the river's
/// charts and its gauge is the lake's level; the river's index is returned.
pub(crate) fn water(
    grid: &Grid,
    file: &RiverFile,
    range: [f64; 2],
    gauge: f64,
    river: bool,
) -> (Map, Option<usize>) {
    water_at(
        grid,
        file,
        range,
        gauge,
        river,
        [[3.5, CENTRE], [30.0, CENTRE]],
    )
}

pub(crate) fn water_at(
    grid: &Grid,
    file: &RiverFile,
    range: [f64; 2],
    gauge: f64,
    river: bool,
    [lake_seed, tarn_seed]: [[f64; 2]; 2],
) -> (Map, Option<usize>) {
    let bed = bed_of(grid);
    let lookup = |name: &str| match name {
        "lake" => Some(0),
        "tarn" => Some(1),
        _ => None,
    };
    let mut bodies = vec![
        basin(
            "lake",
            lake_seed,
            0.0,
            Stage {
                min: range[0],
                max: range[1],
                value: gauge,
            },
        ),
        basin(
            "tarn",
            tarn_seed,
            TARN,
            Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
        ),
    ];
    for r in &file.reaches {
        bodies.push(chart_body(
            &r.name,
            r.chart(&bed, lookup).expect("reach chart"),
        ));
    }
    let charts: Vec<usize> = (2..bodies.len()).collect();
    let mut map = Map::prepare(bed.clone(), bodies, WATER).expect("map");
    let id = river.then(|| {
        map.add_river(
            file.river(&bed, "outlet", &charts, Gauge::Body(0))
                .expect("river"),
        )
        .expect("add river")
    });
    (map, id)
}

/// The range of the lake's level the samples cover.
pub(crate) const RANGE: [f64; 2] = [3.1, 3.7];

/// The valley at 0.25 m cells with `banks`, solved at its samples, and its river file.
pub(crate) fn sampled(banks: Banks) -> (Grid, thrum_river_model::Samples, RiverFile) {
    let (grid, declaration) = lake(0.25, banks, RANGE);
    let profile = thrum_river_model::Profile::default();
    let (samples, file) = solve_at_gauges(&grid, &declaration, &profile);
    (grid, samples, file)
}

pub(crate) fn solve_at_gauges(
    grid: &Grid,
    declaration: &Declaration,
    profile: &thrum_river_model::Profile,
) -> (thrum_river_model::Samples, RiverFile) {
    let s = thrum_river_model::solve_samples(grid, declaration, profile, &[]).expect("solve");
    let key = thrum_river_model::key_for_samples(grid, declaration, profile, &[]);
    let file = s.file(key).expect("river file");
    (s, file)
}

/// Reach `r`'s stored row in sample `k`: level and current at cell `i`.
pub(crate) fn stored(file: &RiverFile, r: usize, k: usize, i: usize) -> (f64, [f64; 2]) {
    let reach = &file.reaches[r];
    if k == 0 {
        (reach.levels[i], reach.currents[i])
    } else {
        (reach.upper[k - 1].levels[i], reach.upper[k - 1].currents[i])
    }
}

/// The lower of the two samples that bracket gauge `g`.
pub(crate) fn lower_sample(gauges: &[f64], g: f64) -> usize {
    gauges[..gauges.len() - 1]
        .iter()
        .rposition(|&a| a <= g)
        .unwrap_or(0)
}

/// Whether a point of reach `r` lies beyond every join's blend: farther from each end that
/// joins a body than the end's length. Within it the chart's row is mixed toward the body's.
pub(crate) fn unjoined(file: &RiverFile, r: usize, p: [f64; 2]) -> bool {
    file.reaches[r]
        .ends
        .iter()
        .filter(|e| e.kind != EndKind::Brink)
        .all(|e| {
            let [a, b] = e.segment;
            let d = [b[0] - a[0], b[1] - a[1]];
            let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
                .clamp(0.0, 1.0);
            (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1]) >= e.length
        })
}
