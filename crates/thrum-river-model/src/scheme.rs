//! The finite-volume march: an HLL flux through each face, Chen and Noelle's hydrostatic
//! reconstruction (so a thin sheet on steep ground keeps the slope's pull and water at rest
//! stays at rest round dry ground), friction linearised about the old discharge (so the steady
//! state does not move with the step), and each cell's own pseudo-time step. Every step works
//! on the active spans only: wet cells, the cells beside them and the cells water rises in.
//! Each step reads the old state only, so the result is the same for any thread count.
use crate::{Grid, Manning, RiverError};
use rayon::prelude::*;
use std::sync::Arc;

pub(crate) const G: f64 = 9.81;
/// Depth under which a cell holds no water and no current.
pub(crate) const DRY: f64 = 1e-6;
/// Depth under which a velocity is desingularised.
const THIN: f64 = 1e-4;
/// Least signal speed in a cell's step, so a dry cell's step stays bounded.
const MIN_SPEED: f64 = 0.3;
/// Active cells from which a step is shared among the workers.
const PARALLEL_CELLS: usize = 1024;

/// Depth and discharge in every cell of a grid.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Depth, metres.
    pub depth: Vec<f64>,
    /// Discharge in x for each metre of width, m²/s.
    pub qx: Vec<f64>,
    /// Discharge in z for each metre of width, m²/s.
    pub qz: Vec<f64>,
}

impl State {
    /// Water at rest at `level`, and none where the bed stands over it.
    pub fn at_rest(grid: &Grid, level: impl Fn(usize) -> f64) -> Self {
        let depth = grid
            .heights()
            .iter()
            .enumerate()
            .map(|(i, z)| (level(i) - z).max(0.0))
            .collect();
        let n = grid.heights().len();
        Self {
            depth,
            qx: vec![0.0; n],
            qz: vec![0.0; n],
        }
    }
}

/// A cell whose level the solve holds: a body's water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fixed {
    pub(crate) level: f64,
    /// A receiving body lets the current leave as it arrives; a source body is at rest.
    pub(crate) receiver: bool,
}

/// What a step did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepStats {
    /// Cells updated.
    pub updates: u64,
    /// The largest level rate over the updated cells, metres a second.
    pub level_rate: f64,
}

/// One face's fluxes: mass, the normal momentum each side sees with its bed's pull folded in,
/// and the tangential momentum carried by the mass.
type Flux = [f64; 4];

/// One row's depth, discharges and level rates.
type RowSlices<'a> = (
    ((&'a mut [f64], &'a mut [f64]), &'a mut [f64]),
    &'a mut [f64],
);

/// Marches a state in pseudo-time over a grid with walls at its edges.
pub struct Stepper {
    nx: usize,
    nz: usize,
    dx: f64,
    courant: f64,
    bed: Vec<f64>,
    manning: Vec<f64>,
    fixed: Vec<Option<Fixed>>,
    source: Vec<f64>,
    seed: Vec<bool>,
    spans: Option<Vec<[usize; 2]>>,
    used: Vec<[usize; 2]>,
    xf: Vec<Flux>,
    zf: Vec<Flux>,
    speed: Vec<f64>,
    rate: Vec<f64>,
    pool: Arc<rayon::ThreadPool>,
}

impl Stepper {
    /// A stepper over `grid` with walls at its edges, no sources and no held cells.
    /// # Errors
    /// Refuses an invalid roughness, Courant number or thread pool.
    pub fn new(
        grid: &Grid,
        manning: &Manning,
        courant: f64,
        threads: usize,
    ) -> Result<Self, RiverError> {
        let n = grid.heights().len();
        let rough: Vec<f64> = (0..n)
            .map(|i| match manning {
                Manning::Grid(v) if v.len() != n => f64::NAN,
                m => m.at(i),
            })
            .collect();
        if rough.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(RiverError::invalid("manning", "invalid roughness"));
        }
        if !(courant > 0.0 && courant <= 0.5) {
            return Err(RiverError::invalid(
                "profile",
                "Courant number outside (0, 0.5]",
            ));
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|_| RiverError::invalid("profile", "thread pool"))?;
        let [nx, nz] = grid.size();
        Ok(Self {
            nx,
            nz,
            dx: grid.spacing(),
            courant,
            bed: grid.heights().to_vec(),
            manning: rough,
            fixed: vec![None; n],
            source: vec![0.0; n],
            seed: vec![false; n],
            spans: None,
            used: Vec::new(),
            xf: vec![[0.0; 4]; (nx + 1) * nz],
            zf: vec![[0.0; 4]; nx * (nz + 1)],
            speed: vec![0.0; n],
            rate: vec![0.0; n],
            pool: Arc::new(pool),
        })
    }

    /// Water rises in `cell` at `discharge` cubic metres a second.
    pub fn add_source(&mut self, cell: usize, discharge: f64) {
        self.source[cell] += discharge / (self.dx * self.dx);
        self.seed[cell] = true;
        self.spans = None;
    }

    pub(crate) fn hold(&mut self, cell: usize, fixed: Fixed) {
        self.fixed[cell] = Some(fixed);
        self.spans = None;
    }

    pub(crate) fn finish_holds(&mut self) {
        // The cells beside a source body's water are always worked on.
        for i in 0..self.bed.len() {
            if self.fixed[i].is_some() {
                continue;
            }
            if self
                .neighbours(i)
                .any(|j| self.fixed[j].is_some_and(|f| !f.receiver && f.level > self.bed[j]))
            {
                self.seed[i] = true;
            }
        }
        self.spans = None;
    }

    pub(crate) fn neighbours(&self, i: usize) -> impl Iterator<Item = usize> + use<> {
        let (nx, nz) = (self.nx, self.nz);
        let (x, z) = (i % nx, i / nx);
        [
            (x > 0).then(|| i - 1),
            (x + 1 < nx).then_some(i + 1),
            (z > 0).then(|| i - nx),
            (z + 1 < nz).then_some(i + nx),
        ]
        .into_iter()
        .flatten()
    }

    pub(crate) fn source_rate(&self, i: usize) -> f64 {
        self.source[i]
    }
    /// Mass flux through the x-face left of cell (x, z), m²/s; `x` may be the column count.
    pub(crate) fn x_face(&self, x: usize, z: usize) -> f64 {
        self.xf[z * (self.nx + 1) + x][0]
    }
    /// Mass flux through the z-face under cell (x, z), m²/s; `z` may be the row count.
    pub(crate) fn z_face(&self, x: usize, z: usize) -> f64 {
        self.zf[z * self.nx + x][0]
    }
    /// The level rate of each cell at the last step, metres a second.
    pub(crate) fn rates(&self) -> &[f64] {
        &self.rate
    }
    /// The spans the last step worked on.
    pub(crate) fn spans(&self) -> &[[usize; 2]] {
        &self.used
    }

    fn full_spans(&self, state: &State) -> Vec<[usize; 2]> {
        let live: Vec<_> = (0..self.nz)
            .map(|z| {
                let row = z * self.nx..(z + 1) * self.nx;
                let mut lo = usize::MAX;
                let mut hi = 0;
                for i in row {
                    if self.live(state, i) {
                        lo = lo.min(i % self.nx);
                        hi = hi.max(i % self.nx + 1);
                    }
                }
                [lo, hi]
            })
            .collect();
        self.dilate(&live)
    }

    fn live(&self, state: &State, i: usize) -> bool {
        self.seed[i] || (self.fixed[i].is_none() && state.depth[i] > DRY)
    }

    fn dilate(&self, live: &[[usize; 2]]) -> Vec<[usize; 2]> {
        (0..self.nz)
            .map(|z| {
                let mut lo = usize::MAX;
                let mut hi = 0;
                for row in &live[z.saturating_sub(1)..=(z + 1).min(self.nz - 1)] {
                    if row[0] < row[1] {
                        lo = lo.min(row[0]);
                        hi = hi.max(row[1]);
                    }
                }
                if lo < hi {
                    [lo.saturating_sub(1), (hi + 1).min(self.nx)]
                } else {
                    [0, 0]
                }
            })
            .collect()
    }

    /// One pseudo-time step of every active cell, each at its own step.
    pub fn step(&mut self, state: &mut State) -> StepStats {
        let spans = match self.spans.take() {
            Some(s) => s,
            None => self.full_spans(state),
        };
        // Small work runs on the calling thread; handing it to workers would cost more.
        let active: usize = spans.iter().map(|s| s[1].saturating_sub(s[0])).sum();
        let (stats, live) = if active >= PARALLEL_CELLS && self.pool.current_num_threads() > 1 {
            let pool = Arc::clone(&self.pool);
            pool.install(|| self.step_spans(state, &spans, true))
        } else {
            self.step_spans(state, &spans, false)
        };
        self.spans = Some(self.dilate(&live));
        self.used = spans;
        stats
    }

    fn step_spans(
        &mut self,
        state: &mut State,
        spans: &[[usize; 2]],
        parallel: bool,
    ) -> (StepStats, Vec<[usize; 2]>) {
        let nx = self.nx;
        let nz = self.nz;
        // Rows to a task: enough cells that a task outweighs handing it to a thread.
        let rows = (512 / nx).max(1);
        let mut xf = std::mem::take(&mut self.xf);
        let mut speed = std::mem::take(&mut self.speed);
        let mut zf = std::mem::take(&mut self.zf);
        {
            let this = &*self;
            let st = &*state;
            let x_row = |(z, (faces, speed)): (usize, (&mut [Flux], &mut [f64]))| {
                let [a, b] = spans[z];
                if a >= b {
                    return;
                }
                for (x, s) in speed.iter_mut().enumerate().take(b).skip(a) {
                    *s = this.signal(st, z * nx + x);
                }
                for (x, f) in faces.iter_mut().enumerate().take(b + 1).skip(a) {
                    *f = this.face_x(st, x, z);
                }
            };
            let z_row = |(z, faces): (usize, &mut [Flux])| {
                let below = if z > 0 { spans[z - 1] } else { [0, 0] };
                let above = if z < nz { spans[z] } else { [0, 0] };
                let (a, b) = match (below[0] < below[1], above[0] < above[1]) {
                    (true, true) => (below[0].min(above[0]), below[1].max(above[1])),
                    (true, false) => (below[0], below[1]),
                    (false, true) => (above[0], above[1]),
                    (false, false) => return,
                };
                for (x, face) in faces.iter_mut().enumerate().take(b).skip(a) {
                    *face = this.face_z(st, x, z);
                }
            };
            if parallel {
                xf.par_chunks_mut(nx + 1)
                    .zip(speed.par_chunks_mut(nx))
                    .enumerate()
                    .with_min_len(rows)
                    .for_each(x_row);
                zf.par_chunks_mut(nx)
                    .enumerate()
                    .with_min_len(rows)
                    .for_each(z_row);
            } else {
                xf.chunks_mut(nx + 1)
                    .zip(speed.chunks_mut(nx))
                    .enumerate()
                    .for_each(x_row);
                zf.chunks_mut(nx).enumerate().for_each(z_row);
            }
        }
        self.xf = xf;
        self.speed = speed;
        self.zf = zf;
        let mut rate = std::mem::take(&mut self.rate);
        let this = &*self;
        let update = |(z, (((h, qx), qz), rate)): (usize, RowSlices<'_>)| {
            this.update_row(z, spans[z], h, qx, qz, rate)
        };
        let results: Vec<(f64, [usize; 2])> = if parallel {
            state
                .depth
                .par_chunks_mut(nx)
                .zip(state.qx.par_chunks_mut(nx))
                .zip(state.qz.par_chunks_mut(nx))
                .zip(rate.par_chunks_mut(nx))
                .enumerate()
                .with_min_len(rows)
                .map(update)
                .collect()
        } else {
            state
                .depth
                .chunks_mut(nx)
                .zip(state.qx.chunks_mut(nx))
                .zip(state.qz.chunks_mut(nx))
                .zip(rate.chunks_mut(nx))
                .enumerate()
                .map(update)
                .collect()
        };
        self.rate = rate;
        let mut stats = StepStats::default();
        let mut live = Vec::with_capacity(nz);
        for (z, (r, l)) in results.into_iter().enumerate() {
            stats.updates += (spans[z][1].saturating_sub(spans[z][0])) as u64;
            stats.level_rate = stats.level_rate.max(r);
            live.push(l);
        }
        (stats, live)
    }

    fn velocity(h: f64, q: f64) -> f64 {
        if h <= DRY {
            return 0.0;
        }
        if h >= THIN {
            return q / h;
        }
        let h4 = h * h * h * h;
        std::f64::consts::SQRT_2 * h * q / (h4 + THIN.powi(4)).sqrt()
    }

    fn signal(&self, st: &State, i: usize) -> f64 {
        let h = st.depth[i];
        if h <= DRY || self.fixed[i].is_some() {
            return 0.0;
        }
        Self::velocity(h, st.qx[i]).hypot(Self::velocity(h, st.qz[i])) + (G * h).sqrt()
    }

    /// Depth and the current normal and tangential to a face, for cell `i` seen across a face
    /// whose other cell is `other`; `normal` picks qx (true) or qz.
    fn side(&self, st: &State, i: usize, other: usize, x_normal: bool) -> (f64, f64, f64) {
        let pick = |c: usize| {
            let h = st.depth[c];
            let (qn, qt) = if x_normal {
                (st.qx[c], st.qz[c])
            } else {
                (st.qz[c], st.qx[c])
            };
            (Self::velocity(h, qn), Self::velocity(h, qt))
        };
        match self.fixed[i] {
            None => {
                let (u, v) = pick(i);
                (st.depth[i], u, v)
            }
            Some(f) => {
                let h = (f.level - self.bed[i]).max(0.0);
                if f.receiver && self.fixed[other].is_none() {
                    let (u, v) = pick(other);
                    (h, u, v)
                } else {
                    (h, 0.0, 0.0)
                }
            }
        }
    }

    fn face_x(&self, st: &State, x: usize, z: usize) -> Flux {
        let nx = self.nx;
        let row = z * nx;
        if x == 0 || x == nx {
            let i = if x == 0 { row } else { row + nx - 1 };
            return self.wall(st, i, true, x == 0);
        }
        self.face_between(st, row + x - 1, row + x, true)
    }

    fn face_z(&self, st: &State, x: usize, z: usize) -> Flux {
        let nx = self.nx;
        if z == 0 || z == self.nz {
            let i = if z == 0 { x } else { (self.nz - 1) * nx + x };
            return self.wall(st, i, false, z == 0);
        }
        self.face_between(st, (z - 1) * nx + x, z * nx + x, false)
    }

    fn wall(&self, st: &State, i: usize, x_normal: bool, ghost_left: bool) -> Flux {
        if self.fixed[i].is_some() || st.depth[i] <= DRY {
            return [0.0; 4];
        }
        let (h, u, _) = self.side(st, i, i, x_normal);
        let z = self.bed[i];
        if ghost_left {
            let (_, _, r) = face(h, -u, z, h, u, z);
            [0.0, 0.0, r, 0.0]
        } else {
            let (_, l, _) = face(h, u, z, h, -u, z);
            [0.0, l, 0.0, 0.0]
        }
    }

    fn face_between(&self, st: &State, a: usize, b: usize, x_normal: bool) -> Flux {
        if self.fixed[a].is_some() && self.fixed[b].is_some() {
            return [0.0; 4];
        }
        let (hl, ul, vl) = self.side(st, a, b, x_normal);
        let (hr, ur, vr) = self.side(st, b, a, x_normal);
        if hl <= DRY && hr <= DRY {
            return [0.0; 4];
        }
        let (m, l, r) = face(hl, ul, self.bed[a], hr, ur, self.bed[b]);
        [m, l, r, m * if m > 0.0 { vl } else { vr }]
    }

    fn update_row(
        &self,
        z: usize,
        span: [usize; 2],
        h: &mut [f64],
        qx: &mut [f64],
        qz: &mut [f64],
        rate: &mut [f64],
    ) -> (f64, [usize; 2]) {
        let nx = self.nx;
        let dx = self.dx;
        let mut worst = 0.0_f64;
        let mut lo = usize::MAX;
        let mut hi = 0;
        for x in span[0]..span[1] {
            let i = z * nx + x;
            if self.fixed[i].is_some() {
                rate[x] = 0.0;
                continue;
            }
            let w = self.xf[z * (nx + 1) + x];
            let e = self.xf[z * (nx + 1) + x + 1];
            let s = self.zf[z * nx + x];
            let n = self.zf[(z + 1) * nx + x];
            let r = self.source[i] - (e[0] - w[0] + n[0] - s[0]) / dx;
            rate[x] = r;
            if h[x] <= DRY && r == 0.0 && w[0] == 0.0 && e[0] == 0.0 && s[0] == 0.0 && n[0] == 0.0 {
                qx[x] = 0.0;
                qz[x] = 0.0;
                if self.seed[i] {
                    lo = lo.min(x);
                    hi = hi.max(x + 1);
                }
                continue;
            }
            worst = worst.max(r.abs());
            let mut sp = self.speed[i];
            if x > 0 {
                sp = sp.max(self.speed[i - 1]);
            }
            if x + 1 < nx {
                sp = sp.max(self.speed[i + 1]);
            }
            if z > 0 {
                sp = sp.max(self.speed[i - nx]);
            }
            if z + 1 < self.nz {
                sp = sp.max(self.speed[i + nx]);
            }
            let dt = self.courant * dx / (2.0 * sp.max(MIN_SPEED));
            let hn = h[x] + dt * r;
            let qxs = qx[x] - dt / dx * (e[1] - w[2] + n[3] - s[3]);
            let qzs = qz[x] - dt / dx * (n[1] - s[2] + e[3] - w[3]);
            if hn <= DRY {
                h[x] = hn.max(0.0);
                qx[x] = 0.0;
                qz[x] = 0.0;
            } else {
                let q = qx[x].hypot(qz[x]);
                let k = 1.0 + dt * G * self.manning[i].powi(2) * q / (hn * hn * hn.cbrt());
                h[x] = hn;
                qx[x] = qxs / k;
                qz[x] = qzs / k;
            }
            if h[x] > DRY || self.seed[i] {
                lo = lo.min(x);
                hi = hi.max(x + 1);
            }
        }
        (worst, if lo < hi { [lo, hi] } else { [usize::MAX, 0] })
    }
}

/// HLL flux of mass and momentum between two reconstructed states.
fn hll(hl: f64, ul: f64, hr: f64, ur: f64) -> (f64, f64) {
    if hl <= DRY && hr <= DRY {
        return (0.0, 0.0);
    }
    let cl = (G * hl).sqrt();
    let cr = (G * hr).sqrt();
    let (sl, sr) = if hl <= DRY {
        (ur - 2.0 * cr, ur + cr)
    } else if hr <= DRY {
        (ul - cl, ul + 2.0 * cl)
    } else {
        let us = f64::midpoint(ul, ur) + cl - cr;
        let cs = f64::midpoint(cl, cr) + 0.25 * (ul - ur);
        ((ul - cl).min(us - cs), (ur + cr).max(us + cs))
    };
    let (fl0, fl1) = (hl * ul, hl * ul * ul + 0.5 * G * hl * hl);
    let (fr0, fr1) = (hr * ur, hr * ur * ur + 0.5 * G * hr * hr);
    if sl >= 0.0 {
        return (fl0, fl1);
    }
    if sr <= 0.0 {
        return (fr0, fr1);
    }
    let d = sr - sl;
    (
        (sr * fl0 - sl * fr0 + sl * sr * (hr - hl)) / d,
        (sr * fl1 - sl * fr1 + sl * sr * (hr * ur - hl * ul)) / d,
    )
}

/// Flux through a face with the bed's pull folded in, by Chen and Noelle's reconstruction:
/// (mass, momentum seen by the left cell, momentum seen by the right cell).
pub(crate) fn face(hl: f64, ul: f64, zl: f64, hr: f64, ur: f64, zr: f64) -> (f64, f64, f64) {
    let zf = zl.max(zr).min((hl + zl).min(hr + zr));
    let sl = (hl + zl - zf).min(hl).max(0.0);
    let sr = (hr + zr - zf).min(hr).max(0.0);
    let (f0, f1) = hll(sl, ul, sr, ur);
    (
        f0,
        f1 + 0.5 * G * (hl + sl) * (zf - zl),
        f1 + 0.5 * G * (hr + sr) * (zf - zr),
    )
}
