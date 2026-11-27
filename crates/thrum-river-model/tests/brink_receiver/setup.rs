//! CPU declarations for the waterline and drowned-fall tests.
use thrum_water_model::{BRINK, Bed, Body, BodyKind, Chart, End, Map, Profile, Stage};

pub(super) use thrum_fixtures::brink_receiver::*;

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
