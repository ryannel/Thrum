//! What a solve writes for the water map: each reach as a chart, with its footprint, its level
//! under the bed wherever the solve left a cell dry, its current, loss and ends.
use crate::RiverError;
use crate::solve::{Held, Solution};
use std::collections::VecDeque;
use thrum_water_model::{Bed, Chart, RiverEnd, RiverFile, RiverReach};

impl Solution {
    /// The reach as the chart the river file stores, on the solve's rectangle.
    ///
    /// Its footprint is the wet cells, the cells of each body that the river runs into or out of,
    /// and up to the profile's margin of rings of dry cells whose beds stand over the wet level
    /// beside them. A wet cell keeps the solver's level and current. A dry cell's level is the
    /// nearest wet level where that lies under its bed, and otherwise is held the dry margin under
    /// its bed; a footprint cell's level is also held under the bed of any lower cell outside the
    /// footprint beside it, so the chart's banks contain its water.
    /// # Errors
    /// Refuses an end whose cells do not lie along one row or column.
    pub fn reach(&self) -> Result<RiverReach, RiverError> {
        let grid = &self.grid;
        let [nx, nz] = grid.size();
        let z = grid.heights();
        let n = z.len();
        let p = &self.profile;
        let mut footprint = self.wet.clone();
        let mut levels: Vec<f64> = (0..n).map(|i| z[i] - p.dry_margin).collect();
        let mut carried = vec![f64::NAN; n];
        let mut queue = VecDeque::new();
        for i in 0..n {
            if self.wet[i] {
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
                if footprint[j] || self.held[j].is_some() || z[j] < carried[i] {
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
        let solved = footprint.clone();
        let mut ends = Vec::new();
        let mut currents = vec![[0.0; 2]; n];
        for (i, c) in currents.iter_mut().enumerate() {
            if self.wet[i] {
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
                .filter(|&i| self.held[i] == Some(held) && neighbours(i).any(|j| solved[j]))
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
                let into = [
                    (x > 0 && solved[i - 1]).then(|| [self.x_flux[y * (nx + 1) + x], 0.0]),
                    (x + 1 < nx && solved[i + 1]).then(|| [self.x_flux[y * (nx + 1) + x + 1], 0.0]),
                    (y > 0 && solved[i - nx]).then(|| [0.0, self.z_flux[y * nx + x]]),
                    (y + 1 < nz && solved[i + nx]).then(|| [0.0, self.z_flux[(y + 1) * nx + x]]),
                ];
                for q in into.into_iter().flatten() {
                    currents[i][0] += q[0] / (depth * grid.spacing());
                    currents[i][1] += q[1] / (depth * grid.spacing());
                }
            }
            ends.push(RiverEnd {
                body: name.into(),
                segment: segment(self, &row, &solved, name)?,
                length: p.join_length,
                mouth,
            });
        }
        // Containment: no footprint cell's level stands at or over a lower outside bed beside it.
        for i in 0..n {
            if !footprint[i] || self.wet[i] || self.held[i].is_some() {
                continue;
            }
            for j in neighbours(i) {
                if !footprint[j] && z[j] - p.dry_margin < levels[i] {
                    levels[i] = z[j] - p.dry_margin;
                }
            }
        }
        let loss = (0..n)
            .map(|i| if self.wet[i] { self.loss[i] } else { 0.0 })
            .collect();
        Ok(RiverReach {
            name: self.declaration.name.clone(),
            origin: grid.origin(),
            spacing: grid.spacing(),
            size: grid.size(),
            dry_margin: p.dry_margin,
            levels,
            currents,
            loss,
            footprint,
            ends,
        })
    }

    /// The solve as a river file under `key`.
    /// # Errors
    /// As [`Solution::reach`].
    pub fn file(&self, key: [u8; 32]) -> Result<RiverFile, RiverError> {
        Ok(RiverFile {
            key,
            gauge_level: self.declaration.sources.first().map_or(0.0, |s| s.level),
            reaches: vec![self.reach()?],
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
