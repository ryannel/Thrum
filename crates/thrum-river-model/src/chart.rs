//! What a solve writes for the water map: each reach as a chart, with its footprint, its level
//! under the bed wherever the solve left a cell dry, its current, loss and ends.
use crate::brink::Side;
use crate::solve::{Held, Solution};
use crate::{Brink, RiverError};
use std::collections::VecDeque;
use thrum_water_model::{Bed, Chart, EndKind, RiverEnd, RiverFile, RiverReach};

impl Solution {
    /// The river's one reach, as the chart the river file stores, on the solve's rectangle.
    /// # Errors
    /// As [`Solution::reaches`], and a river that its falls cut into several reaches.
    pub fn reach(&self) -> Result<RiverReach, RiverError> {
        let mut all = self.reaches()?;
        if all.len() != 1 {
            return Err(RiverError::invalid(
                &self.declaration.name,
                "the river has several reaches",
            ));
        }
        Ok(all.remove(0))
    }

    /// Each reach as the chart the river file stores, on the solve's rectangle, in the order of
    /// [`Solution::reach_names`].
    ///
    /// Its footprint is the wet cells, the cells of each body that the river runs into or out of,
    /// and up to the profile's margin of rings of dry cells whose beds stand over the wet level
    /// beside them. A wet cell keeps the solver's level and current. A dry cell's level is the
    /// nearest wet level where that lies under its bed, and otherwise is held the dry margin under
    /// its bed; a footprint cell's level is also held under the bed of any lower cell outside the
    /// footprint beside it, so the chart's banks contain its water.
    ///
    /// A reach ends at each run of its brinks in a brink end, through the centres of its lip
    /// cells along one row or column. No footprint crosses a brink. The receiving reach can take
    /// the rock at its bank below the brink, and no two footprints share a cell. A body the reach's water meets is joined over
    /// the profile's join length; a body it borders only across a brink or dry ground, as a pool
    /// at a cliff's foot, is joined within half a cell, a seam.
    /// # Errors
    /// Refuses an end whose cells do not lie along one row or column.
    pub fn reaches(&self) -> Result<Vec<RiverReach>, RiverError> {
        let n = self.wet.len();
        let mut claimed = self.wet.clone();
        let mut out = Vec::with_capacity(self.reach_names.len());
        for r in 0..self.reach_names.len() {
            let wet: Vec<bool> = (0..n).map(|i| self.reach_of[i] == Some(r)).collect();
            let banks = self.claim_faces_for(r, &claimed, &self.wet);
            let reach = self.one_reach(r, &wet, &banks, None)?;
            for (c, &f) in claimed.iter_mut().zip(&reach.footprint) {
                *c |= f;
            }
            out.push(reach);
        }
        Ok(out)
    }

    pub(crate) fn claim_faces_for(
        &self,
        reach: usize,
        claimed: &[bool],
        wet: &[bool],
    ) -> Vec<bool> {
        let mut banks = claimed.to_vec();
        for brink in self.brinks.iter().filter(|b| wet[b.cell]) {
            if self.reach_of[brink.foot] != Some(reach) {
                for cell in brink.face_cells(&self.grid) {
                    banks[cell] = true;
                }
            }
        }
        banks
    }

    /// Whether the face between two neighbouring cells is a brink.
    fn brink_between(&self, i: usize, j: usize) -> bool {
        let [nx, _] = self.grid.size();
        let (a, b) = (i.min(j), i.max(j));
        let (x, z) = (b % nx, b / nx);
        if b == a + 1 {
            self.x_brink[z * (nx + 1) + x]
        } else {
            self.z_brink[z * nx + x]
        }
    }

    /// One reach's chart from this solve's wet cells of it. With `union`, the footprint (before
    /// the bodies' rows) is its first part, the cells every sample of a river solves, and a cell
    /// this solve leaves out of its own margin takes the wet level carried to it where that lies
    /// under its bed, and otherwise the dry margin under its bed; its second part, the cells any
    /// sample wets, decides whether the reach's water meets a body.
    #[expect(
        clippy::too_many_lines,
        reason = "one footprint: its margin, the union's cells, the bodies' rows and the brink ends"
    )]
    pub(crate) fn one_reach(
        &self,
        r: usize,
        wet: &[bool],
        claimed: &[bool],
        union: Option<(&[bool], &[bool])>,
    ) -> Result<RiverReach, RiverError> {
        let grid = &self.grid;
        let [nx, nz] = grid.size();
        let z = grid.heights();
        let n = z.len();
        let p = &self.profile;
        let mut footprint = wet.to_vec();
        let mut levels: Vec<f64> = (0..n).map(|i| z[i] - p.dry_margin).collect();
        let mut carried = vec![f64::NAN; n];
        let mut queue = VecDeque::new();
        for i in 0..n {
            if wet[i] {
                levels[i] = self.level(i);
                carried[i] = levels[i];
                queue.push_back((i, 0));
            }
        }
        let neighbours = |i: usize| {
            let (x, y) = (i % nx, i / nx);
            [
                (x > 0).then(|| i - 1),
                (x + 1 < nx).then_some(i + 1),
                (y > 0).then(|| i - nx),
                (y + 1 < nz).then_some(i + nx),
            ]
            .into_iter()
            .flatten()
        };
        while let Some((i, ring)) = queue.pop_front() {
            if ring >= p.margin_cells {
                continue;
            }
            for j in neighbours(i) {
                if footprint[j]
                    || claimed[j]
                    || self.held[j].is_some()
                    || z[j] < carried[i]
                    || self.brink_between(i, j)
                {
                    continue;
                }
                footprint[j] = true;
                carried[j] = carried[i];
                levels[j] = if carried[i] < z[j] {
                    carried[i]
                } else {
                    z[j] - p.dry_margin
                };
                queue.push_back((j, ring + 1));
            }
        }
        let meets = union.map_or(wet, |(_, any)| any);
        if let Some((union, _)) = union {
            // Carry levels across the river's other cells, outward from this sample's own.
            let mut queue: VecDeque<usize> = (0..n).filter(|&i| footprint[i]).collect();
            while let Some(i) = queue.pop_front() {
                for j in neighbours(i) {
                    if footprint[j] || !union[j] || self.brink_between(i, j) {
                        continue;
                    }
                    footprint[j] = true;
                    carried[j] = carried[i];
                    levels[j] = if carried[i] < z[j] {
                        carried[i]
                    } else {
                        z[j] - p.dry_margin
                    };
                    queue.push_back(j);
                }
            }
            footprint.copy_from_slice(union);
        }
        let solved = footprint.clone();
        let mut ends = Vec::new();
        let mut currents = vec![[0.0; 2]; n];
        for (i, c) in currents.iter_mut().enumerate() {
            if wet[i] {
                *c = self.current(i);
            }
        }
        let bodies = self
            .declaration
            .receivers
            .iter()
            .enumerate()
            .map(|(k, r)| (Held::Receiver(k), r.name.as_str(), r.level, true))
            .chain(
                self.declaration
                    .sources
                    .iter()
                    .enumerate()
                    .map(|(k, s)| (Held::Source(k), s.name.as_str(), s.level, false)),
            );
        for (held, name, level, mouth) in bodies {
            let row: Vec<usize> = (0..n)
                .filter(|&i| {
                    self.held[i] == Some(held)
                        && neighbours(i).any(|j| solved[j] && !self.brink_between(i, j))
                })
                .collect();
            if row.is_empty() {
                continue;
            }
            for &i in &row {
                footprint[i] = true;
                levels[i] = level;
                let depth = level - z[i];
                let (x, y) = (i % nx, i / nx);
                // Discharge into the cell through each face from the solve, over its depth and width.
                let open = |j: usize| solved[j] && !self.brink_between(i, j);
                let into = [
                    (x > 0 && open(i - 1)).then(|| [self.x_flux[y * (nx + 1) + x], 0.0]),
                    (x + 1 < nx && open(i + 1)).then(|| [self.x_flux[y * (nx + 1) + x + 1], 0.0]),
                    (y > 0 && open(i - nx)).then(|| [0.0, self.z_flux[y * nx + x]]),
                    (y + 1 < nz && open(i + nx)).then(|| [0.0, self.z_flux[(y + 1) * nx + x]]),
                ];
                for q in into.into_iter().flatten() {
                    currents[i][0] += q[0] / (depth * grid.spacing());
                    currents[i][1] += q[1] / (depth * grid.spacing());
                }
            }
            // The reach's water meets the body where a row cell lies beside a wet cell, not
            // across a brink. Where it never does, as where the reach leaves over a cliff into
            // the body and only dry ground on the cliff's top lies beside it, the end is a seam:
            // a join within half a cell, so the chart's water is not drawn down the cliff.
            let joined = row.iter().any(|&i| {
                neighbours(i).any(|j| solved[j] && meets[j] && !self.brink_between(i, j))
            });
            ends.push(RiverEnd {
                body: name.into(),
                segment: segment(self, &row, &solved, name)?,
                length: if joined {
                    p.join_length
                } else {
                    0.5 * grid.spacing()
                },
                kind: if mouth {
                    EndKind::Mouth
                } else {
                    EndKind::Source
                },
            });
        }
        // Brink ends: each straight run of this reach's lips that face the same way.
        let lips: Vec<&Brink> = self
            .brinks
            .iter()
            .filter(|b| {
                self.reach_of[b.cell] == Some(r) && self.brink_between(b.cell, b.below(grid))
            })
            .collect();
        for side in Side::ALL {
            let mut cells: Vec<usize> = lips
                .iter()
                .filter(|b| b.side == side)
                .map(|b| b.cell)
                .collect();
            // Along the lip: by column for an x-face, by row for a z-face; then across it.
            let key = |&c: &usize| {
                if side.along_x() {
                    (c % nx, c / nx)
                } else {
                    (c / nx, c % nx)
                }
            };
            cells.sort_by_key(key);
            let mut start = 0;
            for k in 1..=cells.len() {
                let split = k == cells.len() || {
                    let (a, b) = (key(&cells[k - 1]), key(&cells[k]));
                    a.0 != b.0 || a.1 + 1 != b.1
                };
                if split {
                    let run = &cells[start..k];
                    let a = grid.point(run[0]);
                    let b = grid.point(run[run.len() - 1]);
                    let half = 0.5 * grid.spacing();
                    let segment = if run.len() > 1 {
                        [a, b]
                    } else if side.along_x() {
                        [[a[0], a[1] - half], [a[0], a[1] + half]]
                    } else {
                        [[a[0] - half, a[1]], [a[0] + half, a[1]]]
                    };
                    ends.push(RiverEnd {
                        body: String::new(),
                        segment,
                        length: p.join_length,
                        kind: EndKind::Brink,
                    });
                    start = k;
                }
            }
        }
        // Containment: no footprint cell's level stands at or over a lower outside bed beside it.
        for i in 0..n {
            if !footprint[i] || wet[i] || self.held[i].is_some() {
                continue;
            }
            for j in neighbours(i) {
                if !footprint[j] && z[j] - p.dry_margin < levels[i] {
                    levels[i] = z[j] - p.dry_margin;
                }
            }
        }
        let loss = (0..n)
            .map(|i| if wet[i] { self.loss[i] } else { 0.0 })
            .collect();
        let gains = (0..n)
            .map(|i| if wet[i] { self.gains[i] } else { [0.0; 2] })
            .collect();
        Ok(RiverReach {
            name: self.reach_names[r].clone(),
            origin: grid.origin(),
            spacing: grid.spacing(),
            size: grid.size(),
            dry_margin: p.dry_margin,
            levels,
            currents,
            loss,
            gains,
            footprint,
            upper: Vec::new(),
            ends,
        })
    }

    /// The solve as a river file under `key`: every reach and every fall.
    /// # Errors
    /// As [`Solution::reaches`].
    pub fn file(&self, key: [u8; 32]) -> Result<RiverFile, RiverError> {
        Ok(RiverFile {
            key,
            gauge_level: self.declaration.sources.first().map_or(0.0, |s| s.level),
            reaches: self.reaches()?,
            falls: self.falls.clone(),
            upper: Vec::new(),
        })
    }

    /// The reach as a chart on the water map's bed, its ends naming bodies by `body`.
    /// # Errors
    /// As [`Solution::reach`], and a rectangle off the bed's lattice or an unknown body.
    pub fn chart(
        &self,
        bed: &Bed,
        body: impl Fn(&str) -> Option<usize>,
    ) -> Result<Chart, RiverError> {
        self.reach()?
            .chart(bed, body)
            .map_err(|e| RiverError::Invalid {
                name: e.name,
                reason: e.reason,
            })
    }
}

/// The end's segment through the centres of a body's cells that the river runs into or out of.
fn segment(
    s: &Solution,
    row: &[usize],
    solved: &[bool],
    name: &str,
) -> Result<[[f64; 2]; 2], RiverError> {
    let [nx, _] = s.grid.size();
    let column = row.iter().all(|&i| i % nx == row[0] % nx);
    let line = row.iter().all(|&i| i / nx == row[0] / nx);
    let a = s.grid.point(row[0]);
    let b = s.grid.point(*row.last().expect("non-empty row"));
    let half = 0.5 * s.grid.spacing();
    match (column, line) {
        // One cell: the segment lies across the direction the solve meets it from.
        (true, true) => {
            let i = row[0];
            let along_x =
                (!i.is_multiple_of(nx) && solved[i - 1]) || (i % nx + 1 < nx && solved[i + 1]);
            Ok(if along_x {
                [[a[0], a[1] - half], [a[0], a[1] + half]]
            } else {
                [[a[0] - half, a[1]], [a[0] + half, a[1]]]
            })
        }
        (true, false) | (false, true) => Ok([a, b]),
        (false, false) => Err(RiverError::EndNotStraight { name: name.into() }),
    }
}
