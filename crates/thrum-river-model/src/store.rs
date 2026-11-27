//! The river file's key and writer. The reader is `thrum_water_model::RiverFile::read`, so a
//! game that ships solved rivers does not link the solver.
use crate::{Declaration, Grid, Manning, Profile};
use thrum_water_model::{Below, EndKind, Fall, RIVER_FORMAT, RIVER_MAGIC, RiverFile};

/// The solver's version. Any change that can move a solve's output changes it, and so every key.
pub const SOLVER_VERSION: u32 = 4;

/// The key of a solve's inputs: a hash of the bed inside the rectangle, the declaration, the
/// solve's profile and the solver's version.
pub fn key(grid: &Grid, declaration: &Declaration, profile: &Profile) -> [u8; 32] {
    key_for_version(grid, declaration, profile, SOLVER_VERSION)
}

/// The key for a given solver version.
pub fn key_for_version(grid: &Grid, d: &Declaration, p: &Profile, version: u32) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    let f = |h: &mut blake3::Hasher, v: f64| {
        h.update(&v.to_le_bytes());
    };
    let s = |h: &mut blake3::Hasher, v: &str| {
        h.update(&(v.len() as u64).to_le_bytes());
        h.update(v.as_bytes());
    };
    let u = |h: &mut blake3::Hasher, v: u64| {
        h.update(&v.to_le_bytes());
    };
    h.update(b"thrum river key");
    u(&mut h, u64::from(version));
    for v in grid.origin() {
        f(&mut h, v);
    }
    f(&mut h, grid.spacing());
    for v in grid.size() {
        u(&mut h, v as u64);
    }
    for &v in grid.heights() {
        f(&mut h, v);
    }
    s(&mut h, &d.name);
    u(&mut h, d.springs.len() as u64);
    for sp in &d.springs {
        s(&mut h, &sp.name);
        for v in [sp.point[0], sp.point[1], sp.discharge, sp.radius] {
            f(&mut h, v);
        }
    }
    u(&mut h, d.sources.len() as u64);
    for so in &d.sources {
        s(&mut h, &so.name);
        for v in [so.seed[0], so.seed[1], so.level, so.range[0], so.range[1]] {
            f(&mut h, v);
        }
        for v in so.extent.iter().flatten() {
            f(&mut h, *v);
        }
    }
    u(&mut h, d.receivers.len() as u64);
    for r in &d.receivers {
        s(&mut h, &r.name);
        for v in [r.seed[0], r.seed[1], r.level] {
            f(&mut h, v);
        }
        for v in r.extent.iter().flatten() {
            f(&mut h, *v);
        }
    }
    match &d.manning {
        Manning::Uniform(n) => {
            u(&mut h, 0);
            f(&mut h, *n);
        }
        Manning::Grid(n) => {
            u(&mut h, 1);
            for &v in n {
                f(&mut h, v);
            }
        }
    }
    // Every profile value but the thread count, which never changes a result.
    for v in [
        p.courant,
        p.level_rate,
        p.imbalance,
        p.wet_depth,
        p.dry_margin,
        p.join_length,
        p.chop,
        p.depth_limit,
        p.least_fall,
        p.steepest_slope,
        p.path_spacing,
        p.band_rms[0],
        p.band_rms[1],
        p.band_depths[0],
        p.band_depths[1],
        p.sample_spacing,
    ] {
        f(&mut h, v);
    }
    for v in [
        p.step_budget,
        p.check_every,
        p.margin_cells as u64,
        p.least_width as u64,
    ] {
        u(&mut h, v);
    }
    *h.finalize().as_bytes()
}

/// The key of a river solved at several flow samples: the key of its inputs, and each spring
/// sample's discharges. A river sampled by its source body's levels takes those from its
/// declared range and the profile's sample spacing, which the inputs' key holds.
pub fn key_for_samples(
    grid: &Grid,
    declaration: &Declaration,
    profile: &Profile,
    springs: &[Vec<f64>],
) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"thrum river samples");
    h.update(&key(grid, declaration, profile));
    h.update(&(springs.len() as u64).to_le_bytes());
    for flows in springs {
        h.update(&(flows.len() as u64).to_le_bytes());
        for q in flows {
            h.update(&q.to_le_bytes());
        }
    }
    *h.finalize().as_bytes()
}

/// Encode a river file in the layout `thrum_water_model::RiverFile::read` documents.
/// # Panics
/// If a name, a reach's size or a count exceeds `u32`, which no grid that fits in memory does.
pub fn write(file: &RiverFile) -> Vec<u8> {
    let mut out = Vec::new();
    let name = |out: &mut Vec<u8>, s: &str| {
        out.extend_from_slice(&u32::try_from(s.len()).expect("name fits u32").to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    };
    let count = |out: &mut Vec<u8>, n: usize| {
        out.extend_from_slice(&u32::try_from(n).expect("count fits u32").to_le_bytes());
    };
    let f = |out: &mut Vec<u8>, v: f64| out.extend_from_slice(&v.to_le_bytes());
    out.extend_from_slice(&RIVER_MAGIC);
    out.extend_from_slice(&RIVER_FORMAT.to_le_bytes());
    out.extend_from_slice(&file.key);
    f(&mut out, file.gauge_level);
    count(&mut out, file.upper.len());
    for u in &file.upper {
        f(&mut out, u.gauge_level);
    }
    let cell = |out: &mut Vec<u8>, level: f64, current: [f64; 2], loss: f64, gains: [f64; 2]| {
        for v in [level, current[0], current[1], loss, gains[0], gains[1]] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    };
    count(&mut out, file.reaches.len());
    for r in &file.reaches {
        name(&mut out, &r.name);
        f(&mut out, r.origin[0]);
        f(&mut out, r.origin[1]);
        f(&mut out, r.spacing);
        count(&mut out, r.size[0]);
        count(&mut out, r.size[1]);
        f(&mut out, r.dry_margin);
        for i in 0..r.levels.len() {
            cell(&mut out, r.levels[i], r.currents[i], r.loss[i], r.gains[i]);
            out.push(u8::from(r.footprint[i]));
        }
        for u in &r.upper {
            for i in 0..u.levels.len() {
                cell(&mut out, u.levels[i], u.currents[i], u.loss[i], u.gains[i]);
                match u.wetting[i] {
                    Some(g) => {
                        out.push(1);
                        f(&mut out, g);
                    }
                    None => out.push(0),
                }
            }
        }
        count(&mut out, r.ends.len());
        for e in &r.ends {
            name(&mut out, &e.body);
            for v in e.segment.iter().flatten() {
                f(&mut out, *v);
            }
            f(&mut out, e.length);
            out.push(match e.kind {
                EndKind::Source => 0,
                EndKind::Mouth => 1,
                EndKind::Brink => 2,
            });
        }
    }
    falls(&mut out, &file.falls);
    for u in &file.upper {
        falls(&mut out, &u.falls);
        count(&mut out, u.lip_wetting.len());
        for &(id, g) in &u.lip_wetting {
            out.extend_from_slice(&id.to_le_bytes());
            f(&mut out, g);
        }
    }
    out
}

fn falls(out: &mut Vec<u8>, falls: &[Fall]) {
    let name = |out: &mut Vec<u8>, s: &str| {
        out.extend_from_slice(&u32::try_from(s.len()).expect("name fits u32").to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    };
    let count = |out: &mut Vec<u8>, n: usize| {
        out.extend_from_slice(&u32::try_from(n).expect("count fits u32").to_le_bytes());
    };
    let f = |out: &mut Vec<u8>, v: f64| out.extend_from_slice(&v.to_le_bytes());
    count(out, falls.len());
    for fall in falls {
        name(out, &fall.name);
        name(out, &fall.above);
        count(out, fall.segments.len());
        for s in &fall.segments {
            out.extend_from_slice(&s.id.to_le_bytes());
            for v in s.lip.iter().flatten() {
                f(out, *v);
            }
            f(out, s.discharge);
            f(out, s.launch_depth);
            f(out, s.launch_speed);
            match &s.below {
                Below::Body(b) => {
                    out.push(0);
                    name(out, b);
                }
                Below::Reach(r) => {
                    out.push(1);
                    name(out, r);
                }
            }
            count(out, s.path.len());
            for p in &s.path {
                for v in [p.place[0], p.place[1], p.height, p.time, p.distance] {
                    f(out, v);
                }
            }
        }
    }
}
