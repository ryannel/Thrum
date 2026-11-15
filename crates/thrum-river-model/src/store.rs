//! The river file's key and writer. The reader is `thrum_water_model::RiverFile::read`, so a
//! game that ships solved rivers does not link the solver.
use crate::{Declaration, Grid, Manning, Profile};
use thrum_water_model::{RIVER_FORMAT, RIVER_MAGIC, RiverFile};

/// The solver's version. Any change that can move a solve's output changes it, and so every key.
pub const SOLVER_VERSION: u32 = 1;

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
            f(&mut out, r.levels[i]);
            f(&mut out, r.currents[i][0]);
            f(&mut out, r.currents[i][1]);
            f(&mut out, r.loss[i]);
            out.push(u8::from(r.footprint[i]));
        }
        count(&mut out, r.ends.len());
        for e in &r.ends {
            name(&mut out, &e.body);
            for v in e.segment.iter().flatten() {
                f(&mut out, *v);
            }
            f(&mut out, e.length);
            out.push(u8::from(e.mouth));
        }
    }
    out
}
