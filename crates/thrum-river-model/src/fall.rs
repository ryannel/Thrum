//! A fall's path: the water's free path from the lip, launched at the lip's speed along the
//! current there, wherever that clears the rock; where the rock stands in its way the path
//! follows the rock's face at the speed energy gives. It ends in a declared body's water at the
//! low end of its stage's range, or on ground gentler than the steepest chart slope.
use crate::brink::Brink;
use crate::scheme::G;
use crate::solve::Held;
use crate::{Grid, Profile, RiverError};
use thrum_water_model::{Bed, PathPoint};

/// The share of the critical depth at which a free overfall's water leaves its lip where the
/// flow above is slow (Rouse's brink depth).
pub const LAUNCH_SHARE: f64 = 0.715;

/// Internal step of a path, seconds.
const STEP: f64 = 1e-3;
/// A path that has not landed after this long is refused, seconds.
const LONGEST: f64 = 30.0;

/// The lip's launch: depth, speed and direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Launch {
    pub(crate) depth: f64,
    pub(crate) speed: f64,
    pub(crate) direction: [f64; 2],
    /// Discharge for each metre of lip, m²/s.
    pub(crate) discharge: f64,
}

/// The launch from a lip cell of depth `h` and discharge `[qx, qz]` over a face whose
/// discharge is `q` for each metre: 0.715 of the critical depth where the flow normal to the
/// face is slow, the flow's own depth where it is fast, at the speed that carries `q`, along the
/// lip's current where that leaves over the face and along the face's normal otherwise.
pub(crate) fn launch(brink: &Brink, h: f64, flow: [f64; 2], q: f64) -> Launch {
    let n = brink.side.normal();
    let q = q.max(0.0);
    let un = if h > 0.0 {
        (flow[0] * n[0] + flow[1] * n[1]) / h
    } else {
        0.0
    };
    let fast = h > 0.0 && un >= (G * h).sqrt();
    let depth = if fast {
        h
    } else {
        LAUNCH_SHARE * (q * q / G).cbrt()
    };
    let speed = if depth > 0.0 { q / depth } else { 0.0 };
    let along = flow[0].hypot(flow[1]);
    let direction = if along > 0.0 && flow[0] * n[0] + flow[1] * n[1] > 0.0 {
        [flow[0] / along, flow[1] / along]
    } else {
        n
    };
    Launch {
        depth,
        speed,
        direction,
        discharge: q,
    }
}

/// Where a path ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ends {
    /// In receiving body `k`'s water.
    Body(usize),
    /// On the ground at a cell, landing at a horizontal velocity.
    Ground(usize, [f64; 2]),
}

/// A traced path.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Path {
    pub(crate) points: Vec<PathPoint>,
    pub(crate) ends: Ends,
}

/// What a path is traced over.
pub(crate) struct Ground<'a> {
    pub(crate) grid: &'a Grid,
    pub(crate) bed: Bed,
    pub(crate) held: &'a [Option<Held>],
    pub(crate) levels: Vec<f64>,
    /// Each receiving body's box, world x/z corners; a path over it beyond the rectangle is
    /// over its water.
    pub(crate) boxes: Vec<[[f64; 2]; 2]>,
    pub(crate) profile: &'a Profile,
    pub(crate) name: &'a str,
}

impl Ground<'_> {
    fn height(&self, p: [f64; 2]) -> Result<f64, RiverError> {
        self.bed.height(p).ok_or_else(|| RiverError::ReachesEdge {
            name: self.name.into(),
            place: p,
        })
    }

    /// The bed's fall along a horizontal unit direction at a point, by central differences a
    /// hundredth of a cell apart: positive downhill.
    fn fall_along(&self, p: [f64; 2], dir: [f64; 2]) -> Result<f64, RiverError> {
        let e = 0.01 * self.grid.spacing();
        Ok((self.height([p[0] - dir[0] * e, p[1] - dir[1] * e])?
            - self.height([p[0] + dir[0] * e, p[1] + dir[1] * e])?)
            / (2.0 * e))
    }

    /// Whether the ground along the path's way is gentler than the steepest chart slope.
    fn gentle(&self, p: [f64; 2], dir: [f64; 2]) -> Result<bool, RiverError> {
        Ok(self.fall_along(p, dir)?.abs() <= self.profile.steepest_slope)
    }

    /// The receiving body whose water lies at `p`, and its level: a cell it holds, or beyond
    /// the rectangle, a point in its box.
    fn body(&self, p: [f64; 2]) -> Option<(usize, f64)> {
        if self.bed.height(p).is_none() {
            let inside = |b: &[[f64; 2]; 2]| {
                (b[0][0]..=b[1][0]).contains(&p[0]) && (b[0][1]..=b[1][1]).contains(&p[1])
            };
            return self
                .boxes
                .iter()
                .position(inside)
                .map(|k| (k, self.levels[k]));
        }
        match self.held[self.grid.cell(p)?] {
            Some(Held::Receiver(k)) => Some((k, self.levels[k])),
            _ => None,
        }
    }

    /// Trace the path from `start`, at height `top`, with a launch.
    pub(crate) fn trace(
        &self,
        start: [f64; 2],
        top: f64,
        launch: &Launch,
    ) -> Result<Path, RiverError> {
        let d = launch.direction;
        let v0 = launch.speed;
        let mut p = start;
        let mut y = top;
        let mut vh = [v0 * d[0], v0 * d[1]];
        let mut vy = 0.0;
        let mut t = 0.0;
        let mut s = 0.0;
        let mut on_face = false;
        let mut points = vec![PathPoint {
            place: p,
            height: y,
            time: 0.0,
            distance: 0.0,
        }];
        let mut stored = 0.0;
        let speed = |y: f64| (v0 * v0 + 2.0 * G * (top - y).max(0.0)).sqrt();
        let mut record = |p: [f64; 2], y: f64, t: f64, s: f64, last: bool, stored: &mut f64| {
            if last || s - *stored >= self.profile.path_spacing {
                points.push(PathPoint {
                    place: p,
                    height: y,
                    time: t,
                    distance: s,
                });
                *stored = s;
            }
        };
        loop {
            if t > LONGEST {
                return Err(RiverError::invalid(
                    self.name,
                    "a fall's path does not land",
                ));
            }
            if !on_face {
                let at = |tau: f64| {
                    (
                        [p[0] + vh[0] * tau, p[1] + vh[1] * tau],
                        y + vy * tau - 0.5 * G * tau * tau,
                    )
                };
                let (p1, y1) = at(STEP);
                // Into a body's water: where the free path meets its level.
                if let Some((k, level)) = self.body(p1)
                    && y1 <= level
                {
                    let tau = (vy + (vy * vy + 2.0 * G * (y - level)).max(0.0).sqrt()) / G;
                    let (pe, _) = at(tau);
                    s += chord(p, y, pe, level);
                    record(pe, level, t + tau, s, true, &mut stored);
                    return Ok(Path {
                        points,
                        ends: Ends::Body(k),
                    });
                }
                // Beyond the rectangle, over a body's box, the path meets no rock. A path that
                // leaves both the rectangle and the box over a body's water has gone into it.
                let off = self.bed.height(p1).is_none();
                if off
                    && self.body(p1).is_none()
                    && let Some((k, _)) = self.body(p)
                {
                    record(p, y, t, s, true, &mut stored);
                    return Ok(Path {
                        points,
                        ends: Ends::Body(k),
                    });
                }
                let clear = off && self.body(p1).is_some();
                if clear || y1 > self.height(p1)? {
                    s += chord(p, y, p1, y1);
                    (p, y, vy, t) = (p1, y1, vy - G * STEP, t + STEP);
                    record(p, y, t, s, false, &mut stored);
                    continue;
                }
                // The rock: where the free path meets it.
                let (mut a, mut b) = (0.0, STEP);
                for _ in 0..60 {
                    let m = f64::midpoint(a, b);
                    let (pm, ym) = at(m);
                    if ym > self.height(pm)? {
                        a = m;
                    } else {
                        b = m;
                    }
                }
                let (pc, _) = at(b);
                let yc = self.height(pc)?;
                s += chord(p, y, pc, yc);
                (p, y, vy, t) = (pc, yc, vy - G * b, t + b);
                if self.gentle(p, unit(vh).unwrap_or(d))? {
                    record(p, y, t, s, true, &mut stored);
                    return Ok(Path {
                        points,
                        ends: Ends::Ground(self.cell(p)?, vh),
                    });
                }
                on_face = true;
                record(p, y, t, s, false, &mut stored);
                continue;
            }
            // On the rock's face: along the launch's direction at the speed energy gives.
            let dir = unit(vh).unwrap_or(d);
            let slope = self.fall_along(p, dir)?;
            let v = speed(y);
            let along = v / slope.mul_add(slope, 1.0).sqrt();
            // The water leaves the face where the free path from here clears it.
            let pf = [p[0] + dir[0] * along * STEP, p[1] + dir[1] * along * STEP];
            let yf = y - slope * along * STEP - 0.5 * G * STEP * STEP;
            if yf > self.height(pf)? + 1e-9 {
                vh = [dir[0] * along, dir[1] * along];
                vy = -slope * along;
                on_face = false;
                continue;
            }
            let y1 = self.height(pf)?;
            if let Some((k, level)) = self.body(pf)
                && y1 <= level
            {
                let f = if y > y1 { (y - level) / (y - y1) } else { 1.0 };
                let pe = [p[0] + (pf[0] - p[0]) * f, p[1] + (pf[1] - p[1]) * f];
                s += chord(p, y, pe, level);
                record(pe, level, t + f * STEP, s, true, &mut stored);
                return Ok(Path {
                    points,
                    ends: Ends::Body(k),
                });
            }
            s += chord(p, y, pf, y1);
            (p, y, t) = (pf, y1, t + STEP);
            vh = [dir[0] * along, dir[1] * along];
            if self.gentle(p, dir)? {
                record(p, y, t, s, true, &mut stored);
                return Ok(Path {
                    points,
                    ends: Ends::Ground(self.cell(p)?, vh),
                });
            }
            record(p, y, t, s, false, &mut stored);
        }
    }

    fn cell(&self, p: [f64; 2]) -> Result<usize, RiverError> {
        self.grid.cell(p).ok_or_else(|| RiverError::ReachesEdge {
            name: self.name.into(),
            place: p,
        })
    }
}

fn chord(a: [f64; 2], ya: f64, b: [f64; 2], yb: f64) -> f64 {
    let h = (b[0] - a[0]).hypot(b[1] - a[1]);
    h.hypot(yb - ya)
}

fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let l = v[0].hypot(v[1]);
    (l > 0.0).then(|| [v[0] / l, v[1] / l])
}
