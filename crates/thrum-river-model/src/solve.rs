//! A steady solve: the declaration checked against the ground, pools filled to their sills,
//! the march to the stopping rule, and what the solve writes: depth, current, the energy account
//! and the narrow stretches.
use crate::brink::{Brink, brinks};
use crate::fall::{Ends, Ground, launch};
use crate::scheme::{DRY, Fixed, G, Landing};
use crate::{Declaration, Grid, Profile, RiverError, State, Stepper};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::time::Instant;
use thrum_water_model::{Bed, Below, Fall, FallSegment};

/// Density of water, kg/m³.
pub(crate) const RHO: f64 = 1000.0;

/// What a solve did.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Pseudo-time steps.
    pub steps: u64,
    /// Cell updates over all steps.
    pub cell_updates: u64,
    /// Wall-clock seconds of the march.
    pub seconds: f64,
    /// The largest level rate at the stop, metres a second.
    pub level_rate: f64,
    /// The largest imbalance of discharge between sections at the stop, as a share of the inflow.
    pub imbalance: f64,
    /// Cells wet in the chart at the stop.
    pub wet_cells: usize,
    /// The river's inflow: its springs and what leaves its source bodies, m³/s.
    pub discharge: f64,
}

/// A stretch of channel narrower than the profile's least width.
#[derive(Clone, Debug, PartialEq)]
pub struct Narrow {
    /// World x/z corners of the box round its cells.
    pub extent: [[f64; 2]; 2],
    /// Cells in the stretch.
    pub cells: usize,
    /// Its least wet width across the current, in cells.
    pub least_width: usize,
}

/// Which body holds a cell's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Held {
    Receiver(usize),
    Source(usize),
}

/// The steady flow one solve wrote.
#[derive(Clone, Debug)]
pub struct Solution {
    pub(crate) grid: Grid,
    pub(crate) declaration: Declaration,
    pub(crate) profile: Profile,
    pub(crate) held: Vec<Option<Held>>,
    /// Depth and discharge in every cell. A held cell has its body's depth and no discharge.
    pub state: State,
    /// Cells of the solve, not held by a body, deeper than the profile's wet depth.
    pub wet: Vec<bool>,
    /// Discharge through each x-face, `(nx + 1)` to a row, left of each cell, m³/s; positive in +x.
    pub x_flux: Vec<f64>,
    /// Discharge through each z-face, `nx` to a row, under each cell, m³/s; positive in +z.
    pub z_flux: Vec<f64>,
    /// Power the flow gives up in each cell, W/m²: the head each inflow loses entering it,
    /// times its discharge, over the cell's area.
    pub lost: Vec<f64>,
    /// Bed friction's power by Manning's relation, `ρ g |q| S`, W/m².
    pub friction: Vec<f64>,
    /// The power that breaks the surface, W/m²: head lost beyond friction, and friction's
    /// power beyond the chop's cap.
    pub loss: Vec<f64>,
    /// The solve's report.
    pub report: Report,
    /// Stretches narrower than the profile's least width.
    pub narrows: Vec<Narrow>,
    /// The bed's brinks, wet or dry.
    pub brinks: Vec<Brink>,
    /// The falls: the brinks the water leaves over, with their paths.
    pub falls: Vec<Fall>,
    /// The reach each wet cell belongs to. Reaches are cut at the falls.
    pub reach_of: Vec<Option<usize>>,
    /// Each reach's name.
    pub reach_names: Vec<String>,
    /// Whether each x-face, as `x_flux` is laid out, is a brink.
    pub(crate) x_brink: Vec<bool>,
    /// Whether each z-face, as `z_flux` is laid out, is a brink.
    pub(crate) z_brink: Vec<bool>,
}

impl Solution {
    /// The grid the solve ran on.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }
    /// Level at a cell: bed plus depth.
    pub fn level(&self, cell: usize) -> f64 {
        self.grid.heights()[cell] + self.state.depth[cell]
    }
    /// Current at a cell, m/s; none where the cell is dry.
    pub fn current(&self, cell: usize) -> [f64; 2] {
        let h = self.state.depth[cell];
        if self.held[cell].is_some() || h <= DRY {
            return [0.0; 2];
        }
        [self.state.qx[cell] / h, self.state.qz[cell] / h]
    }
    /// Total head `level + |u|² / 2g` at a cell, metres.
    pub fn head(&self, cell: usize) -> f64 {
        let [u, v] = self.current(cell);
        self.level(cell) + (u * u + v * v) / (2.0 * G)
    }
    /// Whether a body holds the cell's level.
    pub fn is_held(&self, cell: usize) -> bool {
        self.held[cell].is_some()
    }
}

/// Where a cell's water leaves the rectangle: into a body, or over the edge.
#[derive(Clone, Copy, PartialEq)]
enum Out {
    Body,
    Edge,
}

struct Setup {
    held: Vec<Option<Held>>,
    state: State,
    springs: Vec<Vec<usize>>,
}

/// Solve a river's steady flow.
/// # Errors
/// Refuses, by name and place, invalid input, water that reaches the rectangle's edge, a
/// source with no way down to a declared body, a source body whose range reaches under its
/// outlet's sill, and a march that does not reach the stopping rule inside its step budget.
pub fn solve(
    grid: &Grid,
    declaration: &Declaration,
    profile: &Profile,
) -> Result<Solution, RiverError> {
    validate(grid, declaration, profile)?;
    let setup = setup(grid, declaration)?;
    let mut stepper = Stepper::new(grid, &declaration.manning, profile.courant, profile.threads)?;
    // The water leaves freely over every brink whose lip is not a body's, except where a body's
    // water below stands over the lip, at the low end of its range: that brink is drowned.
    let all_brinks = brinks(grid, profile);
    let level = |h: Held| match h {
        Held::Receiver(k) => declaration.receivers[k].level,
        Held::Source(k) => declaration.sources[k].range[0],
    };
    let falling: Vec<Brink> = all_brinks
        .iter()
        .filter(|b| {
            setup.held[b.cell].is_none()
                && setup.held[b.below(grid)].is_none_or(|h| level(h) < grid.heights()[b.cell])
        })
        .copied()
        .collect();
    stepper.set_brinks(&falling);
    let ground = Ground {
        grid,
        bed: Bed::new(
            grid.origin(),
            grid.spacing(),
            grid.size(),
            grid.heights().to_vec(),
        )
        .map_err(|e| RiverError::Invalid {
            name: e.name,
            reason: e.reason,
        })?,
        held: &setup.held,
        levels: declaration.receivers.iter().map(|r| r.level).collect(),
        boxes: declaration.receivers.iter().map(|r| r.extent).collect(),
        profile,
        name: &declaration.name,
    };
    for (i, held) in setup.held.iter().enumerate() {
        match held {
            Some(Held::Receiver(r)) => stepper.hold(
                i,
                Fixed {
                    level: declaration.receivers[*r].level,
                    receiver: true,
                },
            ),
            Some(Held::Source(s)) => stepper.hold(
                i,
                Fixed {
                    level: declaration.sources[*s].level,
                    receiver: false,
                },
            ),
            None => {}
        }
    }
    for (spring, cells) in declaration.springs.iter().zip(&setup.springs) {
        for &c in cells {
            stepper.add_source(c, spring.discharge / cells.len() as f64);
        }
    }
    stepper.finish_holds();
    let mut state = setup.state;
    let start = Instant::now();
    let mut steps = 0;
    let mut updates = 0;
    let (level_rate, imbalance, discharge) = loop {
        let mut rate = 0.0_f64;
        // The last batch stops at the budget, so no step is ever taken beyond it.
        let batch = profile.check_every.min(profile.step_budget - steps);
        if !falling.is_empty() {
            let landings = falling
                .iter()
                .map(|b| landing(&ground, &stepper, &state, b))
                .collect::<Result<Vec<_>, _>>()?;
            stepper.set_landings(&landings);
        }
        for _ in 0..batch {
            let s = stepper.step(&mut state);
            steps += 1;
            updates += s.updates;
            rate = s.level_rate;
        }
        let (imbalance, discharge) = imbalance(&stepper, grid, &setup.held);
        if rate < profile.level_rate && imbalance < profile.imbalance {
            break (rate, imbalance, discharge);
        }
        if steps / 1000 != (steps - batch) / 1000 {
            edge(grid, &setup.held, &state, profile, &declaration.name)?;
        }
        if steps >= profile.step_budget {
            return Err(RiverError::NotConverged {
                name: declaration.name.clone(),
                steps,
                level_rate: rate,
                imbalance,
            });
        }
    };
    let seconds = start.elapsed().as_secs_f64();
    edge(grid, &setup.held, &state, profile, &declaration.name)?;
    let [nx, nz] = grid.size();
    let dx = grid.spacing();
    let x_flux = (0..nz)
        .flat_map(|z| (0..=nx).map(move |x| (x, z)))
        .map(|(x, z)| stepper.x_face(x, z) * dx)
        .collect();
    let z_flux = (0..=nz)
        .flat_map(|z| (0..nx).map(move |x| (x, z)))
        .map(|(x, z)| stepper.z_face(x, z) * dx)
        .collect();
    let wet: Vec<bool> = (0..state.depth.len())
        .map(|i| setup.held[i].is_none() && state.depth[i] > profile.wet_depth)
        .collect();
    let mut x_brink = vec![false; (nx + 1) * nz];
    let mut z_brink = vec![false; nx * (nz + 1)];
    // A brink cuts the river only where water leaves over it.
    for b in falling.iter().filter(|b| wet[b.cell]) {
        let (x, z) = (b.cell % nx, b.cell / nx);
        match b.side {
            crate::Side::PlusX => x_brink[z * (nx + 1) + x + 1] = true,
            crate::Side::MinusX => x_brink[z * (nx + 1) + x] = true,
            crate::Side::PlusZ => z_brink[(z + 1) * nx + x] = true,
            crate::Side::MinusZ => z_brink[z * nx + x] = true,
        }
    }
    let (reach_of, reach_names) = reaches(grid, &wet, &x_brink, &z_brink, &declaration.name);
    let falls = falls(
        &ground,
        &stepper,
        &state,
        &falling,
        &wet,
        &reach_of,
        &reach_names,
        declaration,
    )?;
    let mut solution = Solution {
        grid: grid.clone(),
        declaration: declaration.clone(),
        profile: profile.clone(),
        held: setup.held,
        state,
        report: Report {
            steps,
            cell_updates: updates,
            seconds,
            level_rate,
            imbalance,
            wet_cells: wet.iter().filter(|&&w| w).count(),
            discharge,
        },
        wet,
        x_flux,
        z_flux,
        lost: Vec::new(),
        friction: Vec::new(),
        loss: Vec::new(),
        narrows: Vec::new(),
        brinks: all_brinks,
        falls,
        reach_of,
        reach_names,
        x_brink,
        z_brink,
    };
    account(&mut solution, &stepper);
    solution.narrows = narrows(&solution);
    Ok(solution)
}

fn validate(grid: &Grid, d: &Declaration, p: &Profile) -> Result<(), RiverError> {
    let bad = |v: f64| !v.is_finite();
    if d.name.is_empty() {
        return Err(RiverError::invalid("river", "empty name"));
    }
    if d.springs.is_empty() && d.sources.is_empty() {
        return Err(RiverError::invalid(&d.name, "no spring or source body"));
    }
    if d.receivers.is_empty() {
        return Err(RiverError::invalid(&d.name, "no receiving body"));
    }
    for s in &d.springs {
        if s.name.is_empty()
            || bad(s.discharge)
            || s.discharge <= 0.0
            || bad(s.radius)
            || s.radius < 0.0
        {
            return Err(RiverError::invalid(&s.name, "invalid spring"));
        }
    }
    for s in &d.sources {
        if s.name.is_empty()
            || [s.level, s.range[0], s.range[1]].iter().any(|&v| bad(v))
            || s.range[0] > s.range[1]
            || !(s.range[0]..=s.range[1]).contains(&s.level)
            || s.extent.iter().flatten().any(|&v| bad(v))
        {
            return Err(RiverError::invalid(&s.name, "invalid source body"));
        }
    }
    for r in &d.receivers {
        if r.name.is_empty() || bad(r.level) || r.extent.iter().flatten().any(|&v| bad(v)) {
            return Err(RiverError::invalid(&r.name, "invalid receiving body"));
        }
    }
    if let crate::Manning::Grid(n) = &d.manning
        && n.len() != grid.heights().len()
    {
        return Err(RiverError::invalid(
            "manning",
            "roughness grid does not match the bed",
        ));
    }
    let positive = [
        p.least_fall,
        p.steepest_slope,
        p.path_spacing,
        p.level_rate,
        p.imbalance,
        p.wet_depth,
        p.join_length,
        p.chop,
        p.depth_limit,
    ];
    if positive.iter().any(|&v| bad(v) || v <= 0.0)
        || bad(p.dry_margin)
        || p.dry_margin <= 0.0
        || p.step_budget == 0
        || p.check_every == 0
        || p.least_width == 0
    {
        return Err(RiverError::invalid(
            "profile",
            "invalid tolerance or constant",
        ));
    }
    Ok(())
}

fn flood(
    grid: &Grid,
    seed: usize,
    level: f64,
    extent: [[f64; 2]; 2],
    held: &[Option<Held>],
) -> Vec<usize> {
    let z = grid.heights();
    let [nx, nz] = grid.size();
    let mut seen = vec![false; z.len()];
    let mut out = Vec::new();
    let mut q = VecDeque::from([seed]);
    seen[seed] = true;
    while let Some(i) = q.pop_front() {
        out.push(i);
        let (x, y) = (i % nx, i / nx);
        let nb = [
            (x > 0).then(|| i - 1),
            (x + 1 < nx).then_some(i + 1),
            (y > 0).then(|| i - nx),
            (y + 1 < nz).then_some(i + nx),
        ];
        for j in nb.into_iter().flatten() {
            if !seen[j] && z[j] < level && grid.inside(j, extent) && held[j].is_none() {
                seen[j] = true;
                q.push_back(j);
            }
        }
    }
    out
}

fn neighbours(grid: &Grid, i: usize) -> impl Iterator<Item = usize> + use<> {
    let [nx, nz] = grid.size();
    let (x, y) = (i % nx, i / nx);
    [
        (x > 0).then(|| i - 1),
        (x + 1 < nx).then_some(i + 1),
        (y > 0).then(|| i - nx),
        (y + 1 < nz).then_some(i + nx),
    ]
    .into_iter()
    .flatten()
}

fn is_edge(grid: &Grid, i: usize) -> bool {
    let [nx, nz] = grid.size();
    i.is_multiple_of(nx) || i % nx + 1 == nx || i / nx == 0 || i / nx + 1 == nz
}

fn setup(grid: &Grid, d: &Declaration) -> Result<Setup, RiverError> {
    let z = grid.heights();
    let n = z.len();
    let mut held: Vec<Option<Held>> = vec![None; n];
    let mut hold = |name: &str, seed: [f64; 2], level: f64, extent, what: Held| {
        let seed = grid
            .cell(seed)
            .filter(|&c| z[c] < level && held[c].is_none())
            .ok_or_else(|| {
                RiverError::invalid(name, "body's seed is dry, taken or off the grid")
            })?;
        for c in flood(grid, seed, level, extent, &held) {
            held[c] = Some(what);
        }
        Ok::<_, RiverError>(())
    };
    for (k, r) in d.receivers.iter().enumerate() {
        hold(&r.name, r.seed, r.level, r.extent, Held::Receiver(k))?;
    }
    for (k, s) in d.sources.iter().enumerate() {
        hold(&s.name, s.seed, s.level, s.extent, Held::Source(k))?;
    }
    // Spill levels by a priority flood from the receiving bodies' water and from the
    // rectangle's edge. Each cell learns the least level at which its water leaves, where it
    // leaves (a body or the edge) and the next cell on the way.
    let mut spill = vec![f64::INFINITY; n];
    let mut out = vec![Out::Edge; n];
    let mut parent = vec![usize::MAX; n];
    let mut done = vec![false; n];
    let mut heap = BinaryHeap::new();
    let key =
        |v: f64, i: usize| Reverse((v.to_bits() ^ if v < 0.0 { u64::MAX } else { 1 << 63 }, i));
    for i in 0..n {
        if let Some(Held::Receiver(r)) = held[i] {
            spill[i] = d.receivers[r].level;
            out[i] = Out::Body;
            heap.push(key(spill[i], i));
        } else if held[i].is_none() && is_edge(grid, i) {
            spill[i] = z[i];
            out[i] = Out::Edge;
            heap.push(key(spill[i], i));
        }
    }
    while let Some(Reverse((_, i))) = heap.pop() {
        if done[i] {
            continue;
        }
        done[i] = true;
        for j in neighbours(grid, i) {
            if done[j] || matches!(held[j], Some(Held::Receiver(_))) {
                continue;
            }
            let s = z[j].max(spill[i]);
            if s < spill[j] {
                spill[j] = s;
                out[j] = out[i];
                parent[j] = i;
                heap.push(key(s, j));
            }
        }
    }
    let mut fill = vec![false; n];
    let fill_from = |start: usize, fill: &mut Vec<bool>| {
        let mut c = start;
        while parent[c] != usize::MAX {
            if !fill[c] && held[c].is_none() && spill[c] > z[c] {
                // The whole hollow this cell lies in, to its spill level.
                let level = spill[c];
                let mut q = VecDeque::from([c]);
                fill[c] = true;
                while let Some(i) = q.pop_front() {
                    for j in neighbours(grid, i) {
                        if !fill[j]
                            && held[j].is_none()
                            && spill[j] > z[j]
                            && spill[j].to_bits() == level.to_bits()
                        {
                            fill[j] = true;
                            q.push_back(j);
                        }
                    }
                }
            }
            c = parent[c];
        }
    };
    let mut springs = Vec::with_capacity(d.springs.len());
    for s in &d.springs {
        let mut cells: Vec<usize> = (0..n)
            .filter(|&i| {
                let p = grid.point(i);
                (p[0] - s.point[0]).hypot(p[1] - s.point[1]) <= s.radius
            })
            .collect();
        if cells.is_empty() {
            cells.extend(grid.cell(s.point));
        }
        if cells.is_empty() || cells.iter().any(|&c| held[c].is_some() || is_edge(grid, c)) {
            return Err(RiverError::invalid(
                &s.name,
                "spring is off the grid, in a body or at its edge",
            ));
        }
        for &c in &cells {
            if out[c] == Out::Edge {
                return Err(RiverError::NoWayDown {
                    name: s.name.clone(),
                    place: s.point,
                });
            }
            fill_from(c, &mut fill);
        }
        springs.push(cells);
    }
    for (k, s) in d.sources.iter().enumerate() {
        let cells: Vec<usize> = (0..n)
            .filter(|&i| held[i] == Some(Held::Source(k)))
            .collect();
        // The outlet: the body's way out at the least spill level.
        let outlet = cells
            .iter()
            .flat_map(|&c| neighbours(grid, c))
            .filter(|&j| held[j].is_none())
            .min_by(|&a, &b| spill[a].total_cmp(&spill[b]).then(a.cmp(&b)));
        let Some(outlet) = outlet.filter(|&o| out[o] == Out::Body) else {
            return Err(RiverError::NoWayDown {
                name: s.name.clone(),
                place: s.seed,
            });
        };
        let sill = spill[outlet];
        if s.range[0] <= sill {
            let mut place = outlet;
            let mut c = outlet;
            while parent[c] != usize::MAX {
                if z[c] > z[place] {
                    place = c;
                }
                c = parent[c];
            }
            return Err(RiverError::BelowSill {
                name: s.name.clone(),
                place: grid.point(place),
                sill,
                level: s.range[0],
            });
        }
        fill_from(outlet, &mut fill);
    }
    let mut state = State::at_rest(grid, |i| if fill[i] { spill[i] } else { f64::NEG_INFINITY });
    for (i, h) in held.iter().enumerate() {
        state.depth[i] = match h {
            Some(Held::Receiver(r)) => (d.receivers[*r].level - z[i]).max(0.0),
            Some(Held::Source(s)) => (d.sources[*s].level - z[i]).max(0.0),
            None => state.depth[i],
        };
    }
    Ok(Setup {
        held,
        state,
        springs,
    })
}

/// The largest imbalance of discharge between sections, as a share of the inflow: for every
/// line of faces across the rectangle in x and in z, the water still being stored on one side.
fn imbalance(stepper: &Stepper, grid: &Grid, held: &[Option<Held>]) -> (f64, f64) {
    let [nx, nz] = grid.size();
    let area = grid.spacing() * grid.spacing();
    let rates = stepper.rates();
    let mut cols = vec![0.0; nx];
    let mut rows = vec![0.0; nz];
    let mut inflow = 0.0;
    for (z, span) in stepper.spans().iter().enumerate() {
        for (x, col) in cols.iter_mut().enumerate().take(span[1]).skip(span[0]) {
            let i = z * nx + x;
            if held[i].is_some() {
                if matches!(held[i], Some(Held::Source(_))) {
                    let out = stepper.x_face(x + 1, z) - stepper.x_face(x, z)
                        + stepper.z_face(x, z + 1)
                        - stepper.z_face(x, z);
                    inflow += out * grid.spacing();
                }
                continue;
            }
            inflow += stepper.source_rate(i) * area;
            *col += rates[i] * area;
            rows[z] += rates[i] * area;
        }
    }
    let worst = |v: &[f64]| {
        let mut acc = 0.0_f64;
        let mut w = 0.0_f64;
        for s in v {
            acc += s;
            w = w.max(acc.abs());
        }
        w
    };
    let w = worst(&cols).max(worst(&rows));
    (w / inflow.max(1e-12), inflow)
}

fn edge(
    grid: &Grid,
    held: &[Option<Held>],
    state: &State,
    p: &Profile,
    name: &str,
) -> Result<(), RiverError> {
    for (i, h) in held.iter().enumerate() {
        if h.is_none() && is_edge(grid, i) && state.depth[i] > p.wet_depth {
            return Err(RiverError::ReachesEdge {
                name: name.into(),
                place: grid.point(i),
            });
        }
    }
    Ok(())
}

/// The energy account. In each cell the head each inflow loses entering it, times its
/// discharge, is the power the flow gives up there; summed over the cells it is the discharge
/// times the head between the sources and the mouths. Friction's share is Manning's.
fn account(s: &mut Solution, stepper: &Stepper) {
    let [nx, _] = s.grid.size();
    let n = s.state.depth.len();
    let area = s.grid.spacing() * s.grid.spacing();
    let cap = s.profile.slope_cap();
    let head = |i: usize| match s.held[i] {
        Some(Held::Receiver(r)) => s.declaration.receivers[r].level,
        Some(Held::Source(k)) => s.declaration.sources[k].level,
        None => s.head(i),
    };
    let mut lost = vec![0.0; n];
    let mut friction = vec![0.0; n];
    let mut loss = vec![0.0; n];
    for i in 0..n {
        if s.held[i].is_some() || s.state.depth[i] <= DRY {
            continue;
        }
        let (x, z) = (i % nx, i / nx);
        let hi = head(i);
        // (neighbour, discharge into this cell)
        // A brink's water leaves the cell above and does not enter the cell below.
        let faces = [
            (x > 0 && !s.x_brink[z * (nx + 1) + x]).then(|| (i - 1, s.x_flux[z * (nx + 1) + x])),
            (x + 1 < nx && !s.x_brink[z * (nx + 1) + x + 1])
                .then(|| (i + 1, -s.x_flux[z * (nx + 1) + x + 1])),
            (z > 0 && !s.z_brink[z * nx + x]).then(|| (i - nx, s.z_flux[z * nx + x])),
            (z + 1 < s.grid.size()[1] && !s.z_brink[(z + 1) * nx + x])
                .then(|| (i + nx, -s.z_flux[(z + 1) * nx + x])),
        ];
        let mut p = 0.0;
        for (j, q) in faces.into_iter().flatten() {
            if q > 0.0 {
                p += q * (head(j) - hi);
            }
        }
        p += stepper.source_rate(i) * area * (s.level(i) - hi);
        let h = s.state.depth[i];
        let q = s.state.qx[i].hypot(s.state.qz[i]);
        let n_i = s.declaration.manning.at(i);
        let slope = n_i * n_i * (q / h).powi(2) / h.powf(4.0 / 3.0);
        lost[i] = RHO * G * p / area;
        friction[i] = RHO * G * q * slope;
        loss[i] = RHO * G * ((p / area - q * slope).max(0.0) + q * (slope - cap).max(0.0));
    }
    s.lost = lost;
    s.friction = friction;
    s.loss = loss;
}

/// Stretches of wet channel narrower than the profile's least width, measured across the
/// current along the grid axis nearer to it.
fn narrows(s: &Solution) -> Vec<Narrow> {
    let [nx, nz] = s.grid.size();
    let n = s.wet.len();
    let run = |i: usize, along_x: bool| {
        let (x, z) = (i % nx, i / nx);
        let mut w = 1;
        if along_x {
            let mut k = x;
            while k > 0 && s.wet[z * nx + k - 1] {
                k -= 1;
                w += 1;
            }
            let mut k = x;
            while k + 1 < nx && s.wet[z * nx + k + 1] {
                k += 1;
                w += 1;
            }
        } else {
            let mut k = z;
            while k > 0 && s.wet[(k - 1) * nx + x] {
                k -= 1;
                w += 1;
            }
            let mut k = z;
            while k + 1 < nz && s.wet[(k + 1) * nx + x] {
                k += 1;
                w += 1;
            }
        }
        w
    };
    let width: Vec<usize> = (0..n)
        .map(|i| {
            if !s.wet[i] {
                return usize::MAX;
            }
            // Across the current: a current along x is measured along z.
            run(i, s.state.qx[i].abs() < s.state.qz[i].abs())
        })
        .collect();
    let narrow: Vec<bool> = width.iter().map(|&w| w < s.profile.least_width).collect();
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    for start in 0..n {
        if !narrow[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let mut q = VecDeque::from([start]);
        let mut cells = 0;
        let mut least = usize::MAX;
        let (mut lo, mut hi) = (s.grid.point(start), s.grid.point(start));
        while let Some(i) = q.pop_front() {
            cells += 1;
            least = least.min(width[i]);
            let p = s.grid.point(i);
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
            let (x, z) = (i % nx, i / nx);
            for zz in z.saturating_sub(1)..=(z + 1).min(nz - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(nx - 1) {
                    let j = zz * nx + xx;
                    if narrow[j] && !seen[j] {
                        seen[j] = true;
                        q.push_back(j);
                    }
                }
            }
        }
        out.push(Narrow {
            extent: [lo, hi],
            cells,
            least_width: least,
        });
    }
    out
}

/// Where the water leaving a brink lands, from the lip's state now.
fn landing(
    ground: &Ground<'_>,
    stepper: &Stepper,
    state: &State,
    brink: &Brink,
) -> Result<Landing, RiverError> {
    let none = Landing {
        cell: None,
        velocity: [0.0; 2],
    };
    let i = brink.cell;
    let q = stepper.brink_discharge(brink);
    if state.depth[i] <= DRY || q <= 0.0 {
        return Ok(none);
    }
    let l = launch(brink, state.depth[i], [state.qx[i], state.qz[i]], q);
    let face = brink.face(ground.grid);
    let start = [
        f64::midpoint(face[0][0], face[1][0]),
        f64::midpoint(face[0][1], face[1][1]),
    ];
    let path = ground.trace(start, ground.grid.heights()[i], &l)?;
    Ok(match path.ends {
        Ends::Body(_) => none,
        Ends::Ground(cell, velocity) => Landing {
            cell: Some(cell),
            velocity,
        },
    })
}

/// The wet cells' reaches: connected through faces that are not brinks, numbered in cell order.
fn reaches(
    grid: &Grid,
    wet: &[bool],
    x_brink: &[bool],
    z_brink: &[bool],
    name: &str,
) -> (Vec<Option<usize>>, Vec<String>) {
    let [nx, _] = grid.size();
    let mut of = vec![None; wet.len()];
    let mut count = 0;
    for start in 0..wet.len() {
        if !wet[start] || of[start].is_some() {
            continue;
        }
        of[start] = Some(count);
        let mut q = VecDeque::from([start]);
        while let Some(i) = q.pop_front() {
            for j in neighbours(grid, i) {
                let (x, z) = (i % nx, i / nx);
                let across = if j + 1 == i {
                    x_brink[z * (nx + 1) + x]
                } else if j == i + 1 {
                    x_brink[z * (nx + 1) + x + 1]
                } else if j + nx == i {
                    z_brink[z * nx + x]
                } else {
                    z_brink[(z + 1) * nx + x]
                };
                if wet[j] && of[j].is_none() && !across {
                    of[j] = Some(count);
                    q.push_back(j);
                }
            }
        }
        count += 1;
    }
    let names = if count == 1 {
        vec![name.to_string()]
    } else {
        (1..=count).map(|k| format!("{name} reach {k}")).collect()
    };
    (of, names)
}

/// The falls: runs of brinks, beside one another along their lips, that water leaves.
#[expect(
    clippy::too_many_arguments,
    reason = "the solve's parts a fall record is assembled from"
)]
fn falls(
    ground: &Ground<'_>,
    stepper: &Stepper,
    state: &State,
    falling: &[Brink],
    wet: &[bool],
    reach_of: &[Option<usize>],
    names: &[String],
    d: &Declaration,
) -> Result<Vec<Fall>, RiverError> {
    let grid = ground.grid;
    let active: Vec<&Brink> = falling
        .iter()
        .filter(|b| wet[b.cell] && stepper.brink_discharge(b) > 0.0)
        .collect();
    // Group by lips that are the same cell or beside one another.
    let mut group = vec![usize::MAX; active.len()];
    let mut count = 0;
    for a in 0..active.len() {
        if group[a] != usize::MAX {
            continue;
        }
        group[a] = count;
        let mut q = VecDeque::from([a]);
        while let Some(k) = q.pop_front() {
            for b in 0..active.len() {
                let (i, j) = (active[k].cell, active[b].cell);
                if group[b] == usize::MAX && (i == j || neighbours(grid, i).any(|n| n == j)) {
                    group[b] = count;
                    q.push_back(b);
                }
            }
        }
        count += 1;
    }
    let hash = blake3::hash(d.name.as_bytes());
    let prefix = u64::from(u32::from_le_bytes(
        hash.as_bytes()[..4].try_into().expect("four bytes"),
    )) << 32;
    let mut out: Vec<Fall> = (0..count)
        .map(|k| Fall {
            name: format!("{} fall {}", d.name, k + 1),
            above: String::new(),
            segments: Vec::new(),
        })
        .collect();
    for (k, b) in active.iter().enumerate() {
        let i = b.cell;
        let q = stepper.brink_discharge(b);
        let l = launch(b, state.depth[i], [state.qx[i], state.qz[i]], q);
        let face = b.face(grid);
        let start = [
            f64::midpoint(face[0][0], face[1][0]),
            f64::midpoint(face[0][1], face[1][1]),
        ];
        let path = ground.trace(start, grid.heights()[i], &l)?;
        let below = match path.ends {
            Ends::Body(r) => Below::Body(d.receivers[r].name.clone()),
            Ends::Ground(cell, _) => {
                let reach = reach_of[cell]
                    .or_else(|| neighbours(grid, cell).find_map(|j| reach_of[j]))
                    .ok_or_else(|| {
                        RiverError::invalid(&d.name, "a fall lands where no reach is")
                    })?;
                Below::Reach(names[reach].clone())
            }
        };
        let fall = &mut out[group[k]];
        if let Some(r) = reach_of[i] {
            fall.above.clone_from(&names[r]);
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a face's index is kept to 32 bits under the river's prefix"
        )]
        let face_id = (i as u64 * 4 + b.side.index()) as u32;
        fall.segments.push(FallSegment {
            id: prefix | u64::from(face_id),
            lip: face,
            discharge: q,
            launch_depth: l.depth,
            launch_speed: l.speed,
            below,
            path: path.points,
        });
    }
    Ok(out)
}
