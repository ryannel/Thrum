//! A hollows body: a rectangle of ground whose cells each belong to one hollow, with one
//! level for each hollow (PUDDLES ruling 1). A point's water is its cell's hollow's level
//! where that stands above the cell's bed. The game moves the levels with the tick; the
//! rectangle, the bed and the labels are content and never change while it runs.
use crate::{BodyKind, Map, MapSample, Row, WaterError};
use std::sync::Arc;

/// Label of a cell that belongs to no hollow: its water leaves over the rectangle's edge.
pub const NO_HOLLOW: u32 = u32::MAX;

/// Additional optical attenuation and backscatter for a hollow's water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HollowOptics {
    /// Sediment extinction in inverse metres.
    pub extinction: f32,
    /// RGB backscatter divided by sediment extinction, in the unit interval.
    pub scatter: [f32; 3],
    /// Linear colour multiplier for the submerged bed, in the unit interval.
    pub bed_tint: [f32; 3],
}

/// Limits a profile sets on hollows bodies, counted at preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HollowsProfile {
    /// Most cells one hollows body may hold.
    pub max_cells: usize,
    /// Most hollows one hollows body may hold.
    pub max_hollows: usize,
}

impl Default for HollowsProfile {
    fn default() -> Self {
        Self {
            max_cells: 1 << 20,
            max_hollows: 4096,
        }
    }
}

/// The declaration of a hollows body. Cells are squares; cell `(x, z)` covers
/// `origin + [x, z] * spacing` to `origin + [x + 1, z + 1] * spacing`, row-major x-fastest.
#[derive(Clone, Debug)]
pub struct Hollows {
    pub(crate) origin: [f64; 2],
    pub(crate) spacing: f64,
    pub(crate) size: [usize; 2],
    pub(crate) bed: Arc<[f64]>,
    pub(crate) labels: Arc<[u32]>,
    pub(crate) levels: Vec<f64>,
    optical_types: Vec<Option<HollowOptics>>,
}

impl Hollows {
    /// Validate a rectangle, its bed, a label for each cell and a level for each hollow.
    /// # Errors
    /// Refuses a bad grid, nonfinite heights or levels, and a label naming no hollow.
    pub fn new(
        origin: [f64; 2],
        spacing: f64,
        size: [usize; 2],
        bed: Vec<f64>,
        labels: Vec<u32>,
        levels: Vec<f64>,
    ) -> Result<Self, WaterError> {
        let count = size[0].checked_mul(size[1]);
        if origin.iter().any(|v| !v.is_finite())
            || !spacing.is_finite()
            || spacing <= 0.0
            || size.contains(&0)
            || count != Some(bed.len())
            || count != Some(labels.len())
            || bed.iter().chain(&levels).any(|v| !v.is_finite())
            || labels
                .iter()
                .any(|&l| l != NO_HOLLOW && l as usize >= levels.len())
        {
            return Err(WaterError::new("hollows", "invalid hollows grid"));
        }
        Ok(Self {
            origin,
            spacing,
            size,
            bed: bed.into(),
            labels: labels.into(),
            levels,
            optical_types: Vec::new(),
        })
    }

    /// Author the additional optical type of each hollow. An empty table uses the water
    /// contribution's optical type throughout. A missing entry names that same base type.
    /// # Errors
    /// Refuses a wrong table length or invalid attenuation or backscatter.
    pub fn with_optical_types(
        mut self,
        table: Vec<Option<HollowOptics>>,
    ) -> Result<Self, WaterError> {
        if (!table.is_empty() && table.len() != self.hollow_count())
            || table.iter().flatten().any(|optics| {
                !optics.extinction.is_finite()
                    || optics.extinction < 0.0
                    || optics
                        .scatter
                        .iter()
                        .chain(&optics.bed_tint)
                        .any(|x| !(0.0..=1.0).contains(x))
            })
        {
            return Err(WaterError::new("hollows", "invalid optical types"));
        }
        self.optical_types = table;
        Ok(self)
    }

    /// Authored optical overrides in hollow-label order, or an empty table.
    #[must_use]
    pub fn optical_types(&self) -> &[Option<HollowOptics>] {
        &self.optical_types
    }

    /// Number of hollows.
    pub fn hollow_count(&self) -> usize {
        self.levels.len()
    }

    /// Number of cells.
    pub fn cell_count(&self) -> usize {
        self.bed.len()
    }

    /// Present level of each hollow.
    pub fn levels(&self) -> &[f64] {
        &self.levels
    }

    /// Outer corner, spacing and cell counts of the cell-centred bed.
    #[must_use]
    pub fn lattice(&self) -> ([f64; 2], f64, [usize; 2]) {
        (self.origin, self.spacing, self.size)
    }

    /// Bed heights in metres, in x-fastest cell order.
    #[must_use]
    pub fn heights(&self) -> &[f64] {
        &self.bed
    }

    /// Cell-to-hollow labels, in the same order as the heights.
    #[must_use]
    pub fn labels(&self) -> &[u32] {
        &self.labels
    }

    /// The cell holding a world x/z point, or none outside the rectangle.
    pub(crate) fn cell(&self, point: [f64; 2]) -> Option<usize> {
        let x = (point[0] - self.origin[0]) / self.spacing;
        let z = (point[1] - self.origin[1]) / self.spacing;
        if !(x >= 0.0 && z >= 0.0 && x < self.size[0] as f64 && z < self.size[1] as f64) {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "coordinates checked within the rectangle"
        )]
        let (ix, iz) = (x as usize, z as usize);
        Some(iz * self.size[0] + ix)
    }

    /// Centre of a cell in world x/z.
    pub(crate) fn centre(&self, i: usize) -> [f64; 2] {
        [
            self.origin[0] + ((i % self.size[0]) as f64 + 0.5) * self.spacing,
            self.origin[1] + ((i / self.size[0]) as f64 + 0.5) * self.spacing,
        ]
    }

    /// The point test: the level standing above a cell, or none where it is dry.
    pub(crate) fn water(&self, i: usize) -> Option<f64> {
        let label = self.labels[i];
        if label == NO_HOLLOW {
            return None;
        }
        let level = self.levels[label as usize];
        (level > self.bed[i]).then_some(level)
    }

    /// The level at a cell: its own water, or on a dry cell the highest level among the
    /// wet cells that touch it (its shore margin), or none.
    pub(crate) fn level_at(&self, i: usize) -> Option<(f64, bool)> {
        if let Some(level) = self.water(i) {
            return Some((level, true));
        }
        self.touching(i)
            .filter_map(|j| self.water(j))
            .reduce(f64::max)
            .map(|level| (level, false))
    }

    fn touching(&self, i: usize) -> impl Iterator<Item = usize> {
        let [w, h] = self.size;
        let (x, z) = (i % w, i / w);
        let xs = x.saturating_sub(1)..=(x + 1).min(w - 1);
        (z.saturating_sub(1)..=(z + 1).min(h - 1))
            .flat_map(move |nz| xs.clone().map(move |nx| nz * w + nx))
            .filter(move |&j| j != i)
    }

    /// Whether a table of levels stands two different levels in wet cells that share an
    /// edge. Such a table cannot come from water that is level in every puddle.
    pub(crate) fn splits_level(&self, levels: &[f64]) -> bool {
        let [w, h] = self.size;
        let wet = |i: usize| {
            let l = self.labels[i];
            (l != NO_HOLLOW && levels[l as usize] > self.bed[i]).then(|| levels[l as usize])
        };
        (0..self.bed.len()).any(|i| {
            let Some(a) = wet(i) else { return false };
            let right = (i % w + 1 < w).then(|| i + 1);
            let down = (i / w + 1 < h).then(|| i + w);
            [right, down]
                .into_iter()
                .flatten()
                .any(|j| wet(j).is_some_and(|b| b.to_bits() != a.to_bits()))
        })
    }
}

impl Map {
    /// Connectivity and hollow-level revisions for a body-query snapshot.
    pub fn query_revision(&self) -> [u64; 2] {
        [self.spatial_revision(), self.hollow_revision]
    }

    /// Local bed inside a hollows rectangle, including cells with no water.
    pub fn hollow_bed_at(&self, point: [f64; 2]) -> Option<f64> {
        self.bodies.iter().find_map(|body| {
            let BodyKind::Hollows(hollows) = &body.kind else {
                return None;
            };
            hollows.cell(point).map(|cell| hollows.bed[cell])
        })
    }

    /// Set every level of a hollows body for this tick. The body's identity and generation
    /// do not change, whatever its puddles do. A table that stands two different levels in
    /// wet cells that share an edge is held at the last table and counted in
    /// `held_level_tables`; the result is then `false`.
    /// # Errors
    /// Refuses an unknown body, a body of another kind, a wrong count or a nonfinite level,
    /// without changing state.
    pub fn set_levels(&mut self, body: usize, levels: &[f64]) -> Result<bool, WaterError> {
        let b = self
            .bodies
            .get_mut(body)
            .ok_or_else(|| WaterError::new("levels", "unknown body"))?;
        let BodyKind::Hollows(h) = &mut b.kind else {
            return Err(WaterError::new(
                &b.name,
                "levels set on a body that is not hollows",
            ));
        };
        if levels.len() != h.levels.len() || levels.iter().any(|v| !v.is_finite()) {
            return Err(WaterError::new(&b.name, "invalid level table"));
        }
        if h.levels == levels {
            return Ok(true);
        }
        if h.splits_level(levels) {
            self.held_level_tables += 1;
            return Ok(false);
        }
        h.levels.copy_from_slice(levels);
        self.hollow_revision = self.hollow_revision.wrapping_add(1);
        Ok(true)
    }

    /// The body's generation: it changes only when the body's content is prepared again,
    /// never when a stage or a hollows level moves.
    pub fn generation(&self, body: usize) -> Option<u64> {
        self.generations.get(body).copied()
    }

    /// A point inside a hollows rectangle is answered by that body alone: its cell's water,
    /// its shore margin, or dry.
    pub(crate) fn sample_hollows(&self, point: [f64; 2]) -> Option<HollowsAnswer> {
        self.bodies.iter().enumerate().find_map(|(id, b)| {
            let BodyKind::Hollows(h) = &b.kind else {
                return None;
            };
            let cell = h.cell(point)?;
            Some(HollowsAnswer(h.level_at(cell).map(|(level, _)| {
                MapSample {
                    body: id,
                    row: Row {
                        level,
                        ..Row::default()
                    },
                    bed: Some(h.bed[cell]),
                    depth: level - h.bed[cell],
                }
            })))
        })
    }

    /// Hollows bodies within their limits, apart from each other and from every other
    /// body's water and shore margin at the stages the map was left at by preparation's
    /// highest-stage pass, which holds every stage in range.
    pub(crate) fn validate_hollows(&self) -> Result<(), WaterError> {
        for (id, b) in self.bodies.iter().enumerate() {
            let BodyKind::Hollows(h) = &b.kind else {
                continue;
            };
            let fail = |reason| WaterError::new(&b.name, reason);
            if h.cell_count() > self.hollows_profile.max_cells {
                return Err(fail("hollows body over the profile's count of cells"));
            }
            if h.hollow_count() > self.hollows_profile.max_hollows {
                return Err(fail("hollows body over the profile's count of hollows"));
            }
            for (other, o) in self.bodies.iter().enumerate() {
                if other == id {
                    continue;
                }
                if let BodyKind::Hollows(g) = &o.kind
                    && overlaps(h, g)
                {
                    return Err(fail("same-kind bodies overlap"));
                }
                for i in 0..h.cell_count() {
                    if self.reaches_cell(other, h.centre(i), h.spacing) {
                        return Err(fail("hollows cell in another body's water or margin"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether a body's water or margin, at its highest stage, touches a square cell.
    fn reaches_cell(&self, body: usize, centre: [f64; 2], spacing: f64) -> bool {
        let half = 0.5 * spacing;
        let corners = [
            [-half, -half],
            [half, -half],
            [-half, half],
            [half, half],
            [0.0, 0.0],
        ];
        corners.iter().any(|d| {
            let p = [centre[0] + d[0], centre[1] + d[1]];
            match self.bed.weights(p) {
                Some(w) => w.iter().any(|&(i, _)| self.support[body][i].is_some()),
                None => matches!(self.bodies[body].kind, BodyKind::Sea),
            }
        })
    }
}

/// What a hollows body answers at a point inside its rectangle: water, margin or dry.
pub(crate) struct HollowsAnswer(pub(crate) Option<MapSample>);

fn overlaps(a: &Hollows, b: &Hollows) -> bool {
    (0..2).all(|axis| {
        let (a0, a1) = (
            a.origin[axis],
            a.origin[axis] + a.size[axis] as f64 * a.spacing,
        );
        let (b0, b1) = (
            b.origin[axis],
            b.origin[axis] + b.size[axis] as f64 * b.spacing,
        );
        a0 < b1 && b0 < a1
    })
}
