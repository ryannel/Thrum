//! Shared CPU and Metal declarations for the waterline below a brink.
use thrum_water_model::{BRINK, Bed, Body, BodyKind, Chart, End, Map, Profile, Stage};

pub(super) const MARGIN: f64 = 0.65;
pub(super) const LIP: u32 = 2;
pub(super) const CENTRE: f64 = 3.0;
pub(super) const TOP: f64 = 4.1;
pub(super) const LEVEL: f64 = 0.2;
pub(super) const RISE: f64 = 0.25;
const SIZE: [usize; 2] = [14, 7];
const BOTTOM: f64 = -0.7;

pub(super) fn fixture(width: u32, basin: bool) -> (Map, usize) {
    let foot = LIP + width;
    let heights = bed_heights(width, 4.0);
    let upper = upper_chart(&heights, TOP);
    let kind = if basin {
        BodyKind::Basin {
            seed: [f64::from(foot + 2), CENTRE],
        }
    } else {
        BodyKind::Chart(chart(
            |x, z| (LIP + 1..=10).contains(&x) && (2..=4).contains(&z),
            LEVEL,
            vec![],
        ))
    };
    let level = if basin { LEVEL } else { 0.0 };
    let receiving = body("Plunge pool", kind, level, RISE);
    (prepare(heights, vec![upper, receiving]), 1)
}

pub(super) fn points(width: u32) -> impl Iterator<Item = [f64; 2]> {
    (0..=4 * width).map(|quarter| [f64::from(LIP) + f64::from(quarter) * 0.25, CENTRE])
}

pub(super) fn flanks() -> [[f64; 2]; 2] {
    [[2.953125, 1.953125], [2.953125, 4.046875]]
}

pub(super) fn rock(width: u32, x: f64) -> f64 {
    let heights = ramp(width, 4.0);
    let offset = x - f64::from(LIP);
    for (d, endpoints) in (0..width).zip(heights.windows(2)) {
        let t = offset - f64::from(d);
        if t <= 1.0 {
            return (endpoints[1] - endpoints[0]).mul_add(t, endpoints[0]);
        }
    }
    BOTTOM
}

fn ramp(width: u32, lip: f64) -> Vec<f64> {
    match width {
        1 => vec![lip, BOTTOM],
        4 => vec![lip, 2.6, 1.6, 0.9, BOTTOM],
        _ => unreachable!("fixture cliff widths"),
    }
}

pub(super) fn bed_heights(width: u32, lip: f64) -> Vec<f64> {
    let heights = ramp(width, lip);
    cells()
        .map(|(x, z)| {
            if x == 0 || x > 10 || !(2..=4).contains(&z) {
                10.0
            } else if x <= LIP {
                lip
            } else {
                heights.get((x - LIP) as usize).copied().unwrap_or(BOTTOM)
            }
        })
        .collect()
}

pub(super) fn upper_chart(heights: &[f64], level: f64) -> Body {
    let mut upper = chart(
        |x, z| (1..=LIP).contains(&x) && (2..=4).contains(&z),
        level,
        vec![end(BRINK, LIP, 1.0)],
    );
    for (i, own) in upper.levels.iter_mut().enumerate() {
        if !upper.footprint[i] {
            *own = heights[i] - 0.2;
        }
    }
    body("Lip", BodyKind::Chart(upper), 0.0, 0.0)
}

pub(super) fn chart(inside: impl Fn(u32, u32) -> bool, level: f64, ends: Vec<End>) -> Chart {
    Chart {
        footprint: cells().map(|(x, z)| inside(x, z)).collect(),
        levels: vec![level; SIZE[0] * SIZE[1]],
        currents: vec![[0.0; 2]; SIZE[0] * SIZE[1]],
        ends,
    }
}

pub(super) fn end(body: usize, x: u32, length: f64) -> End {
    let x = f64::from(x);
    End {
        body,
        segment: [[x, 2.0], [x, 4.0]],
        length,
        mouth: body != BRINK,
    }
}

pub(super) fn body(name: &str, kind: BodyKind, level: f64, rise: f64) -> Body {
    Body {
        name: name.to_owned(),
        kind,
        level,
        stage: Stage {
            min: 0.0,
            max: rise,
            value: 0.0,
        },
        fetch: 100.0,
        gains: [0.0; 4],
    }
}

pub(super) fn prepare(heights: Vec<f64>, bodies: Vec<Body>) -> Map {
    Map::prepare(
        Bed::new([0.0, 0.0], 1.0, SIZE, heights).expect("receiver bed"),
        bodies,
        Profile {
            margin_height: MARGIN,
            swash_ratio: 0.7,
        },
    )
    .expect("map with a fall")
}

fn cells() -> impl Iterator<Item = (u32, u32)> {
    (0..u32::try_from(SIZE[1]).expect("rows"))
        .flat_map(|z| (0..u32::try_from(SIZE[0]).expect("columns")).map(move |x| (x, z)))
}
