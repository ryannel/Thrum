//! Prepared samples of a reach with two ways out: over its main lip into the tarn,
//! and, only above the middle gauge, over a side lip onto a terrace that also falls
//! to the tarn. The toe pool is beyond the upstream part of the terrace.
use super::lake::{RANGE, water_at};
use thrum_river_model::Grid;
use thrum_water_model::{
    Below, EndKind, Fall, FallSegment, GRAVITY, Map, PathPoint, ReachSample, RiverEnd, RiverFile,
    RiverReach, UpperSample,
};

const WIDTH: usize = 15;
const HEIGHT: usize = 12;
const SIZE: usize = WIDTH * HEIGHT;
const WETTING: f64 = 3.55;
const MAIN: &str = "main reach";
const TERRACE: &str = "terrace";
const LAKE_SEED: [f64; 2] = [2.5, 7.5];
const TARN_SEED: [f64; 2] = [12.5, 7.5];
pub(crate) const FOOT: f64 = 9.0;

fn cell(x: usize, z: usize) -> usize {
    z * WIDTH + x
}

fn brink(segment: [[f64; 2]; 2]) -> RiverEnd {
    RiverEnd {
        body: String::new(),
        segment,
        length: 0.5,
        kind: EndKind::Brink,
    }
}

fn reach(
    name: &str,
    grid: &Grid,
    cells: &[usize],
    ends: Vec<RiverEnd>,
    depth: impl Fn(usize, usize) -> f64,
) -> RiverReach {
    let sample = |k| {
        let mut row = ReachSample {
            levels: grid.heights().iter().map(|h| h - 0.3).collect(),
            currents: vec![[0.0; 2]; SIZE],
            loss: vec![0.0; SIZE],
            gains: vec![[0.0; 2]; SIZE],
            wetting: vec![None; SIZE],
        };
        for &i in cells {
            let d = depth(k, i);
            if d > 0.0 {
                row.levels[i] = grid.heights()[i] + d;
                row.currents[i] = [d, 0.0];
                if k > 0 && depth(k - 1, i) == 0.0 {
                    row.wetting[i] = Some(WETTING);
                }
            }
        }
        row
    };
    let base = sample(0);
    let mut footprint = vec![false; SIZE];
    for &i in cells {
        footprint[i] = true;
    }
    RiverReach {
        name: name.into(),
        origin: grid.origin(),
        spacing: grid.spacing(),
        size: grid.size(),
        dry_margin: 0.3,
        levels: base.levels,
        currents: base.currents,
        loss: base.loss,
        gains: base.gains,
        footprint,
        upper: vec![sample(1), sample(2)],
        ends,
    }
}

fn segment(
    id: u64,
    lip: [[f64; 2]; 2],
    below: Below,
    launch: f64,
    depth: f64,
    landing: ([f64; 2], f64),
) -> FallSegment {
    let place = [
        f64::midpoint(lip[0][0], lip[1][0]),
        f64::midpoint(lip[0][1], lip[1][1]),
    ];
    let (end, height) = landing;
    let drop = launch - height;
    let run = (end[0] - place[0]).hypot(end[1] - place[1]);
    let time = (2.0 * drop / GRAVITY).sqrt();
    let speed = run / time;
    FallSegment {
        id,
        lip,
        below,
        launch_depth: depth,
        launch_speed: speed,
        discharge: depth * speed,
        path: vec![
            PathPoint {
                place,
                height: launch,
                time: 0.0,
                distance: 0.0,
            },
            PathPoint {
                place: end,
                height,
                time,
                distance: run.hypot(drop),
            },
        ],
    }
}

pub(crate) fn prepared() -> (Grid, RiverFile) {
    let mut heights = vec![8.0; SIZE];
    for z in 6..9 {
        for x in 1..4 {
            heights[cell(x, z)] = 2.0;
        }
    }
    for z in 3..10 {
        for x in 11..14 {
            heights[cell(x, z)] = -2.0;
        }
    }
    let main: Vec<_> = (6..9)
        .flat_map(|z| (5..9).map(move |x| cell(x, z)))
        .collect();
    for &i in &main {
        heights[i] = 2.8;
    }
    for z in 7..9 {
        heights[cell(9, z)] = 0.0;
    }
    let terrace: Vec<_> = (4..6)
        .flat_map(|z| (6..10).map(move |x| cell(x, z)))
        .collect();
    for &i in &terrace {
        heights[i] = 0.0;
    }
    heights[cell(10, 4)] = -0.25;
    let side_lip = cell(7, 6);
    heights[side_lip] = 3.15;
    let grid = Grid::new([0.5; 2], 1.0, [WIDTH, HEIGHT], heights).expect("fall bed");
    let reaches = vec![
        reach(
            MAIN,
            &grid,
            &main,
            vec![
                brink([[8.5, 6.5], [8.5, 8.5]]),
                brink([[6.5, 6.5], [8.5, 6.5]]),
            ],
            |k, i| match (k, i == side_lip) {
                (2, true) => 0.15,
                (_, true) => 0.0,
                (_, false) => [0.10, 0.20, 0.32][k] + 0.01 * (8 - i % WIDTH) as f64,
            },
        ),
        reach(
            TERRACE,
            &grid,
            &terrace,
            vec![brink([[9.5, 4.0], [9.5, 5.0]])],
            |k, i| {
                if i == cell(9, 5) {
                    [0.05, 0.10, 0.2][k]
                } else if k == 2 {
                    0.15
                } else {
                    0.0
                }
            },
        ),
    ];
    let falls = |depth: f64, end_x: f64| {
        vec![Fall {
            name: "main fall".into(),
            above: MAIN.into(),
            segments: [7.5, 8.5]
                .into_iter()
                .zip([1, 2])
                .map(|(z, id)| {
                    segment(
                        id,
                        [[9.0, z - 0.5], [9.0, z + 0.5]],
                        Below::Body("tarn".into()),
                        2.8 + depth,
                        depth,
                        ([end_x, z], -2.0),
                    )
                })
                .collect(),
        }]
    };
    let mut high_falls = falls(0.32, 12.25);
    high_falls.extend([
        Fall {
            name: "side fall".into(),
            above: MAIN.into(),
            segments: vec![segment(
                3,
                [[7.0, 6.0], [8.0, 6.0]],
                Below::Reach(TERRACE.into()),
                3.30,
                0.15,
                ([7.5, 4.5], 0.0),
            )],
        },
        Fall {
            name: "terrace fall".into(),
            above: TERRACE.into(),
            segments: vec![segment(
                4,
                [[10.0, 4.0], [10.0, 5.0]],
                Below::Body("tarn".into()),
                0.15,
                0.15,
                ([11.5, 4.5], -2.0),
            )],
        },
    ]);
    let file = RiverFile {
        key: [0; 32],
        gauge_level: RANGE[0],
        reaches,
        falls: falls(0.10, 11.75),
        upper: vec![
            UpperSample {
                gauge_level: 3.4,
                falls: falls(0.20, 12.0),
                lip_wetting: vec![],
            },
            UpperSample {
                gauge_level: RANGE[1],
                falls: high_falls,
                lip_wetting: vec![(3, WETTING), (4, WETTING)],
            },
        ],
    };
    (grid, file)
}

pub(crate) fn water(grid: &Grid, file: &RiverFile, lake_gauge: bool) -> (Map, Option<usize>) {
    water_at(
        grid,
        file,
        RANGE,
        RANGE[0],
        lake_gauge,
        [LAKE_SEED, TARN_SEED],
    )
}
