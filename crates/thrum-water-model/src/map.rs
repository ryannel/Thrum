use crate::{Bed, Gauge, Hollows, HollowsProfile, WaterError};
use std::collections::VecDeque;
use std::sync::Arc;

/// Bounded additive stage in metres.
#[derive(Clone, Copy, Debug)]
pub struct Stage {
    /// Lowest permitted offset.
    pub min: f64,
    /// Highest permitted offset.
    pub max: f64,
    /// Initial offset.
    pub value: f64,
}

/// Water's spatial preparation parameters.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Maximum shore rise reached monotonically from still water, metres.
    pub margin_height: f64,
    /// Swash depth as a fraction of the row's significant wave height.
    pub swash_ratio: f64,
}

/// Continuous quantities shared by every reader of a body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Row {
    /// Water level in metres.
    pub level: f64,
    /// Current in x/z metres per second.
    pub current: [f64; 2],
    /// Amplitude gains for long waves and three cascades.
    pub gains: [f64; 4],
}

impl Row {
    fn mix(self, other: Self, w: f64) -> Self {
        Self {
            level: self.level * (1.0 - w) + other.level * w,
            current: std::array::from_fn(|i| self.current[i] * (1.0 - w) + other.current[i] * w),
            gains: std::array::from_fn(|i| self.gains[i] * (1.0 - w) + other.gains[i] * w),
        }
    }
}

/// The `body` of a brink end: a stretch of a chart footprint's edge where the ground drops away
/// and the water leaves as a fall. A brink end joins no body. Preparation does not refuse a
/// footprint cell on it for lower ground outside the footprint beside it, the chart's row there
/// is its own with no mix. Receiving water and its shore margin can reach the rock below.
/// Its `length` is not used and its
/// `mouth` flag is ignored; a chart may have several, beside one source end and one mouth.
pub const BRINK: usize = usize::MAX;

/// A segment on a chart footprint's boundary joining an earlier declaration, or a brink.
#[derive(Clone, Debug)]
pub struct End {
    /// Joined body's index. Earlier indices make cyclic joins impossible. [`BRINK`] declares a
    /// brink end.
    pub body: usize,
    /// End segment endpoints in world x/z.
    pub segment: [[f64; 2]; 2],
    /// Smooth transition distance inside the chart, metres.
    pub length: f64,
    /// Whether receiving-body reach also raises the mix (a mouth).
    pub mouth: bool,
}

impl End {
    /// Whether this end is a brink, where the water leaves as a fall.
    pub fn is_brink(&self) -> bool {
        self.body == BRINK
    }

    pub(crate) fn distance(&self, p: [f64; 2]) -> f64 {
        let [a, b] = self.segment;
        let d = [b[0] - a[0], b[1] - a[1]];
        let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
            .clamp(0.0, 1.0);
        (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
    }
}

/// A flow solver's prepared level/current grid and footprint, on the bed's lattice.
#[derive(Clone, Debug)]
pub struct Chart {
    /// Level at each bed centre, row-major.
    pub levels: Vec<f64>,
    /// Current at each bed centre.
    pub currents: Vec<[f64; 2]>,
    /// Centres belonging to the footprint.
    pub footprint: Vec<bool>,
    /// Source and receiving ends; a spring may omit its source.
    pub ends: Vec<End>,
}

/// Declaration of one kind of water.
#[derive(Clone, Debug)]
pub enum BodyKind {
    /// An unbounded level sea.
    Sea,
    /// Connected fill from a world x/z seed.
    Basin {
        /// World seed inside the bed.
        seed: [f64; 2],
    },
    /// Authored steady flow.
    Chart(Chart),
    /// Puddles in the hollows of a rectangle of ground with its own bed; the body's
    /// `level`, `stage` and `gains` are unused, and its levels are set with `set_levels`.
    Hollows(Hollows),
}

/// Named declaration. Engine code contains no game-specific water presets.
#[derive(Clone, Debug)]
pub struct Body {
    /// Unique diagnostic name.
    pub name: String,
    /// Kind and authored spatial data.
    pub kind: BodyKind,
    /// Base level (charts use their grid instead).
    pub level: f64,
    /// Declared stage range and initial value.
    pub stage: Stage,
    /// Exposed wind fetch in metres. Used by `Map::set_forcing` to update shelter.
    pub fetch: f64,
    /// Fetch-selected wave gains, recomputed by the caller when forcing changes.
    pub gains: [f64; 4],
}

/// An answer on still water or on a shore margin; depth may be negative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapSample {
    /// Index of the owning declaration; a chart keeps ownership through a join.
    pub body: usize,
    /// Mixed row at this point.
    pub row: Row,
    /// Ground height; none beyond the bed extent.
    pub bed: Option<f64>,
    /// Still depth, infinite beyond the bed.
    pub depth: f64,
}

/// Prepared water and its current stage connectivity. No graphics device is used.
#[derive(Clone)]
pub struct Map {
    pub(crate) bed: Arc<Bed>,
    pub(crate) bodies: Vec<Body>,
    pub(crate) profile: Profile,
    pub(crate) surf: Option<Arc<crate::surf::Surf>>,
    // Each margin centre remembers the water centre whose row it carries.
    pub(crate) support: Vec<Arc<[Option<usize>]>>,
    // Upper brink centres that bound each receiving chart on its uphill side.
    brink_banks: Vec<Arc<[usize]>>,
    brink_lips: Arc<[usize]>,
    /// Number of stage requests held at a range endpoint.
    pub clamped_stages: u64,
    pub(crate) hollows_profile: HollowsProfile,
    pub(crate) generations: Vec<u64>,
    /// Number of hollows level tables held at the last table because they stood two
    /// levels in wet cells that touch.
    pub held_level_tables: u64,
    pub(crate) rain: Option<crate::RainBands>,
    pub(crate) flow: Option<crate::FlowBands>,
    pub(crate) rivers: Vec<crate::River>,
    // Each body's river and chart index, for a river's chart.
    pub(crate) sampled: Vec<Option<(usize, usize)>>,
    spatial_revision: u64,
    pub(crate) hollow_revision: u64,
    pub(crate) surface_bounds: Vec<Option<[[f64; 2]; 2]>>,
}

impl Map {
    /// Bed used to prepare the map's connectivity.
    pub fn bed(&self) -> &Bed {
        &self.bed
    }

    /// The map's shore margin and swash settings.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Changes when stage-dependent support or authored gains change.
    pub fn spatial_revision(&self) -> u64 {
        self.spatial_revision
    }

    /// Prepared support at each bed centre, as the index of its water origin.
    pub fn support_origins(&self, body: usize) -> &[Option<usize>] {
        &self.support[body]
    }

    /// Upper brink centres next to this receiving chart, in bed index order.
    pub fn brink_banks(&self, body: usize) -> &[usize] {
        &self.brink_banks[body]
    }

    /// Bounds of a finite surface over its declared stage range, including its margin.
    pub fn surface_bounds(&self, body: usize) -> Option<[[f64; 2]; 2]> {
        self.surface_bounds.get(body).copied().flatten()
    }

    /// Read a body's authored declaration and current stage without duplicating its authority.
    pub fn body(&self, index: usize) -> Option<&Body> {
        self.bodies.get(index)
    }
    /// Validate declarations over their stage ranges and prepare initial connectivity.
    /// # Errors
    /// Names invalid inputs, overlapping kinds, escaping basins and uncontained charts.
    pub fn prepare(bed: Bed, bodies: Vec<Body>, profile: Profile) -> Result<Self, WaterError> {
        Self::prepare_with(bed, bodies, profile, HollowsProfile::default())
    }

    /// Prepare with explicit limits on hollows bodies.
    /// # Errors
    /// As [`Map::prepare`], and names a hollows body over the limits or one with a cell in
    /// another body's water or shore margin anywhere in that body's stage range.
    pub fn prepare_with(
        bed: Bed,
        bodies: Vec<Body>,
        profile: Profile,
        hollows_profile: HollowsProfile,
    ) -> Result<Self, WaterError> {
        if !profile.margin_height.is_finite()
            || profile.margin_height < 0.0
            || !profile.swash_ratio.is_finite()
            || profile.swash_ratio < 0.0
        {
            return Err(WaterError::new("profile", "invalid margin or swash ratio"));
        }
        let mut map = Self {
            bed: Arc::new(bed),
            bodies,
            profile,
            surf: None,
            support: Vec::new(),
            brink_banks: Vec::new(),
            brink_lips: Arc::new([]),
            clamped_stages: 0,
            generations: Vec::new(),
            hollows_profile,
            held_level_tables: 0,
            rain: None,
            flow: None,
            rivers: Vec::new(),
            sampled: Vec::new(),
            spatial_revision: 0,
            hollow_revision: 0,
            surface_bounds: Vec::new(),
        };
        map.generations = vec![0; map.bodies.len()];
        map.sampled = vec![None; map.bodies.len()];
        map.validate_inputs()?;
        map.prepare_brink_banks();
        let stages: Vec<_> = map.bodies.iter().map(|b| b.stage.value).collect();
        for b in &mut map.bodies {
            b.stage.value = b.stage.min;
        }
        let minimum_fills: Vec<_> = (0..map.bodies.len()).map(|id| map.flood(id)).collect();
        // At maximum stages all possible basin connectivity is included. Chart containment
        // is also tested at minima and maxima of each joined row, independently below.
        for b in &mut map.bodies {
            b.stage.value = b.stage.max;
        }
        map.rebuild();
        map.validate_topology()?;
        map.validate_banks(&minimum_fills)?;
        map.validate_hollows()?;
        map.surface_bounds = map
            .support
            .iter()
            .map(|support| {
                map.surface_bounds_for(
                    support
                        .iter()
                        .enumerate()
                        .filter_map(|(i, origin)| origin.is_some().then_some(i)),
                )
            })
            .collect();
        for (b, s) in map.bodies.iter_mut().zip(stages) {
            b.stage.value = s;
        }
        map.rebuild();
        Ok(map)
    }

    pub(crate) fn surface_bounds_for(
        &self,
        cells: impl Iterator<Item = usize>,
    ) -> Option<[[f64; 2]; 2]> {
        let mut points = cells.map(|i| self.bed.point(i));
        let first = points.next()?;
        let [min, max] = points.fold([first, first], |[min, max], point| {
            [
                std::array::from_fn(|axis| min[axis].min(point[axis])),
                std::array::from_fn(|axis| max[axis].max(point[axis])),
            ]
        });
        Some([
            min.map(|v| v - self.bed.spacing),
            max.map(|v| v + self.bed.spacing),
        ])
    }

    /// Set a stage, counting range clamps and rebuilding stage-dependent support.
    /// # Errors
    /// Refuses an unknown body or nonfinite stage without changing state.
    pub fn set_stage(&mut self, body: usize, stage: f64) -> Result<(), WaterError> {
        let b = self
            .bodies
            .get_mut(body)
            .ok_or_else(|| WaterError::new("stage", "unknown body"))?;
        if matches!(b.kind, BodyKind::Hollows(_)) {
            return Err(WaterError::new(
                &b.name,
                "hollows body has levels, not a stage",
            ));
        }
        if !stage.is_finite() {
            return Err(WaterError::new(&b.name, "nonfinite stage"));
        }
        let value = stage.clamp(b.stage.min, b.stage.max);
        if stage < b.stage.min || stage > b.stage.max {
            self.clamped_stages += 1;
        }
        if b.stage.value.to_bits() != value.to_bits() {
            b.stage.value = value;
            self.rebuild();
        }
        Ok(())
    }

    /// Sample bounded stages for presentation, sharing unchanged connectivity
    /// and leaving the source map's tick state untouched. The stage slice
    /// supplies one value per declaration in body order.
    /// # Errors
    /// Refuses a wrong stage count or a nonfinite stage.
    pub fn sample_stages(&self, stages: &[f64]) -> Result<Self, WaterError> {
        self.sample_stages_and_gauges(stages, &[])
    }

    /// Sample the body's stage endpoints and any spring-fed river's own gauge for display.
    /// A `None` entry keeps the river's authored gauge. Gauges that read a source body
    /// follow that body's sampled stage or chart without a separate value.
    /// # Errors
    /// Refuses a wrong stage count, nonfinite stage or gauge, and gauge values for
    /// nonexistent rivers or rivers whose gauge follows a body.
    pub fn sample_stages_and_gauges(
        &self,
        stages: &[f64],
        gauges: &[Option<f64>],
    ) -> Result<Self, WaterError> {
        if stages.len() != self.bodies.len() || stages.iter().any(|value| !value.is_finite()) {
            return Err(WaterError::new("stage sample", "invalid stage values"));
        }
        let mut sample = self.clone();
        let mut changed = false;
        for (body, &stage) in sample.bodies.iter_mut().zip(stages) {
            if !matches!(body.kind, BodyKind::Hollows(_)) {
                let stage = stage.clamp(body.stage.min, body.stage.max);
                changed |= stage.to_bits() != body.stage.value.to_bits();
                body.stage.value = stage;
            }
        }
        for (id, value) in gauges.iter().copied().enumerate() {
            let Some(value) = value else { continue };
            let Some(river) = sample.rivers.get_mut(id).filter(|_| value.is_finite()) else {
                return Err(WaterError::new("stage sample", "invalid river gauge"));
            };
            let Gauge::Stage(gauge) = &mut river.gauge else {
                return Err(WaterError::new(
                    &river.name,
                    "gauge follows its source body",
                ));
            };
            let bounded = value.clamp(gauge.min, gauge.max);
            changed |= bounded.to_bits() != gauge.value.to_bits();
            gauge.value = bounded;
        }
        if changed {
            sample.spatial_revision = sample.spatial_revision.wrapping_add(1);
            for (id, body) in sample.bodies.iter().enumerate() {
                if body.stage.value.to_bits() != self.bodies[id].stage.value.to_bits()
                    || matches!(body.kind, BodyKind::Chart(_))
                {
                    sample.support[id] = sample.build_support(id);
                }
            }
        }
        Ok(sample)
    }

    /// Recompute every body's own gains from its declared fetch and the world's forcing.
    /// The exposed sea takes the complete field; basins and charts take wind energy
    /// only, including when the field also carries swell. Joins then mix those rows.
    /// The caller supplies the same forcing used to build the world's SEA field.
    /// # Errors
    /// Refuses invalid forcing without changing any body's gains.
    pub fn set_forcing(&mut self, forcing: &thrum_sea_model::Forcing) -> Result<(), WaterError> {
        // Validate even when every declaration is an exposed sea.
        forcing
            .band_variances()
            .map_err(|_| WaterError::new("waves", "invalid forcing"))?;
        let gains: Result<Vec<_>, _> = self
            .bodies
            .iter()
            .map(|body| {
                if matches!(body.kind, BodyKind::Hollows(_)) {
                    Ok([0.0; 4])
                } else if matches!(body.kind, BodyKind::Sea) && body.fetch >= forcing.fetch {
                    Ok([1.0; 4])
                } else {
                    crate::shelter(forcing, body.fetch)
                }
            })
            .collect();
        let gains = gains?;
        if self
            .bodies
            .iter()
            .zip(&gains)
            .any(|(body, gains)| body.gains.map(f64::to_bits) != gains.map(f64::to_bits))
        {
            for (body, gains) in self.bodies.iter_mut().zip(gains) {
                body.gains = gains;
            }
            self.spatial_revision = self.spatial_revision.wrapping_add(1);
        }
        Ok(())
    }

    /// Set one body's four amplitude gains against the prepared ambient wave field.
    /// # Errors
    /// Refuses unknown bodies or gains outside the finite unit interval.
    pub fn set_wave_gains(&mut self, body: usize, gains: [f64; 4]) -> Result<(), WaterError> {
        let body = self
            .bodies
            .get_mut(body)
            .ok_or_else(|| WaterError::new("waves", "unknown body"))?;
        if gains.iter().any(|gain| !(0.0..=1.0).contains(gain)) {
            return Err(WaterError::new(
                &body.name,
                "amplitude gains must be between zero and one",
            ));
        }
        if body.gains.map(f64::to_bits) != gains.map(f64::to_bits) {
            body.gains = gains;
            self.spatial_revision = self.spatial_revision.wrapping_add(1);
        }
        Ok(())
    }

    /// Query the continuous row and bed at a world point.
    pub fn sample(&self, point: [f64; 2]) -> Option<MapSample> {
        if point.iter().any(|v| !v.is_finite()) {
            return None;
        }
        if let Some(answer) = self.sample_hollows(point) {
            return answer.0;
        }
        let bed = self.bed.height(point);
        let weights = self.bed.weights(point);
        let mut best: Option<(usize, Row, u8)> = None;
        for (id, b) in self.bodies.iter().enumerate() {
            let rank = match b.kind {
                BodyKind::Sea => 0,
                BodyKind::Basin { .. } => 1,
                BodyKind::Chart(_) => 2,
                BodyKind::Hollows(_) => continue,
            };
            let row = self.supported_row(id, point);
            let supported = weights.is_some_and(|w| {
                w.iter().any(|&(i, a)| {
                    a > 0.0
                        && self.support[id][i].is_some_and(|origin| {
                            let level = self.row(id, self.bed.point(origin)).level;
                            if matches!(b.kind, BodyKind::Basin { .. })
                                && bed.is_some_and(|height| height < row.level)
                            {
                                self.bed.wet_reaches(i, w, level)
                            } else {
                                self.bed.reaches(i, w, level)
                            }
                        })
                })
            });
            // A chart declares its water pointwise; it has no seeded component.
            let inside_chart = matches!(&b.kind, BodyKind::Chart(c) if self.in_chart(c, point));
            let shared_bank = !inside_chart
                && supported
                && matches!(&b.kind, BodyKind::Chart(c)
                if weights.is_some_and(|w| self.below_brink(id, c, w)));
            let chart_water =
                (inside_chart || shared_bank) && bed.is_some_and(|height| height < row.level);
            if !(supported || chart_water || bed.is_none() && matches!(b.kind, BodyKind::Sea)) {
                continue;
            }
            if rank == 2 && !chart_water && bed.is_some_and(|h| h < row.level) {
                continue;
            }
            if bed.is_some_and(|h| h > row.level + self.profile.margin_height) {
                continue;
            }
            if best.is_none_or(|(_, old, r)| {
                if bed.is_some_and(|h| h >= row.level && h >= old.level) {
                    row.level > old.level
                } else {
                    rank > r || (rank == r && row.level > old.level)
                }
            }) {
                best = Some((id, row, rank));
            }
        }
        best.map(|(body, row, _)| MapSample {
            body,
            row,
            bed,
            depth: bed.map_or(f64::INFINITY, |h| row.level - h),
        })
    }

    /// A body's joined row, continued from its prepared wet support at the bank.
    /// This reads a named body without resolving ownership at the point.
    /// # Panics
    /// Panics if the body index is outside this map.
    pub fn supported_row(&self, id: usize, point: [f64; 2]) -> Row {
        let mut row = self.row(id, point);
        if let BodyKind::Chart(c) = &self.bodies[id].kind
            && !self.in_chart(c, point)
            && let Some(weights) = self.bed.weights(point)
        {
            // Continue the same water-origin rows onto the bank for both height
            // and its slope, without nearest-centre steps.
            let mut total = 0.0;
            for (i, w) in weights {
                if let Some(origin) = self.support[id][i]
                    && w > 0.0
                {
                    total += w;
                    row = row.mix(self.row(id, self.bed.point(origin)), w / total);
                }
            }
        }
        row
    }

    /// Spatial derivative of a sampled body's joined or continued still level.
    /// # Panics
    /// Panics if the body index is outside this map.
    pub fn level_slope(&self, body: usize, point: [f64; 2]) -> [f64; 2] {
        let BodyKind::Chart(chart) = &self.bodies[body].kind else {
            return [0.0; 2];
        };
        for end in chart.ends.iter().filter(|e| !e.is_brink()) {
            if end.distance(point) <= 1e-8 {
                return self.level_slope(end.body, point);
            }
        }
        // The CPU oracle differentiates the complete joined/continued level.
        // At a bilinear cell crease this gives the symmetric derivative. Gains'
        // spatial derivatives remain omitted as declared by the design.
        let step = self.bed.spacing * 1e-4;
        std::array::from_fn(|axis| {
            let mut lo = point;
            let mut hi = point;
            lo[axis] -= step;
            hi[axis] += step;
            (self.supported_row(body, hi).level - self.supported_row(body, lo).level) / (2.0 * step)
        })
    }

    /// A body's continued row, including joins; useful on the two sides of a mesh seam.
    pub fn row(&self, body: usize, point: [f64; 2]) -> Row {
        let b = &self.bodies[body];
        if let BodyKind::Hollows(h) = &b.kind {
            return Row {
                level: h.cell(point).and_then(|i| h.level_at(i)).map_or_else(
                    || h.cell(point).map_or(f64::NEG_INFINITY, |i| h.bed[i]),
                    |(level, _)| level,
                ),
                ..Row::default()
            };
        }
        let mut row = Row {
            level: b.level + b.stage.value,
            current: [0.0; 2],
            gains: b.gains,
        };
        if let BodyKind::Chart(c) = &b.kind {
            if let Some(m) = self.own_mixed(body, point) {
                // A river's chart: the mix of its samples at the river's gauge.
                row.level = m.level;
                row.current = m.current;
            } else if let Some(w) = self.bed.weights(point) {
                row.level = b.stage.value + w.iter().map(|&(i, a)| c.levels[i] * a).sum::<f64>();
                row.current = std::array::from_fn(|axis| {
                    w.iter().map(|&(i, a)| c.currents[i][axis] * a).sum()
                });
            }
            let own_level = row.level;
            let current = self.river_of(body).and_then(|(river, _)| {
                (self.river(river)?.gauges.len() > 1
                    && c.ends.iter().any(|end| self.variable_mouth(end)))
                .then(|| self.river_join_current(c, point, row.current))
            });
            let mut total = 1.0;
            // A brink end joins no body: the row up to it is the chart's own.
            for end in c.ends.iter().filter(|e| !e.is_brink()) {
                let other = self.row(end.body, point);
                let weight = self.join_weight(end, point, own_level, other.level);
                if weight >= 1.0 {
                    return Row {
                        current: current.unwrap_or(other.current),
                        ..other
                    };
                }
                // Relative weights preserve each end's exact boundary even when
                // transition regions overlap. A single end still mixes by weight.
                let relative = weight / (1.0 - weight);
                total += relative;
                row = row.mix(other, relative / total);
            }
            if let Some(current) = current {
                row.current = current;
            }
        }
        row
    }

    fn river_join_current(
        &self,
        chart: &Chart,
        point: [f64; 2],
        mut current: [f64; 2],
    ) -> [f64; 2] {
        let mut total = 1.0;
        for end in chart.ends.iter().filter(|end| !end.is_brink()) {
            // Backwater can match the receiving level well before the mouth.
            // Keep the solved discharge there, spreading the jet into the
            // receiving current only across the last map cell.
            let length = if self.variable_mouth(end) {
                end.length.min(self.bed.lattice().1)
            } else {
                end.length
            };
            let fade = smooth(end.distance(point) / length);
            if fade >= 1.0 {
                continue;
            }
            let other = self.row(end.body, point).current;
            if fade <= 0.0 {
                return other;
            }
            let relative = (1.0 - fade) / fade;
            total += relative;
            for (component, target) in current.iter_mut().zip(other) {
                *component += (target - *component) * (relative / total);
            }
        }
        current
    }

    fn variable_mouth(&self, end: &End) -> bool {
        end.mouth && !end.is_brink() && {
            let stage = self.bodies[end.body].stage;
            stage.min < stage.max
        }
    }

    pub(crate) fn join_weight(
        &self,
        end: &End,
        point: [f64; 2],
        own_level: f64,
        other_level: f64,
    ) -> f64 {
        let weight = 1.0 - smooth(end.distance(point) / end.length);
        if end.mouth {
            weight.max(
                1.0 - smooth((own_level - other_level - 0.175 * self.profile.margin_height) / 0.25),
            )
        } else {
            weight
        }
    }

    fn in_chart(&self, c: &Chart, p: [f64; 2]) -> bool {
        self.bed
            .weights(p)
            .is_some_and(|w| w.iter().all(|&(i, a)| a <= 1e-12 || c.footprint[i]))
    }

    fn brink_above(&self, chart: &Chart, upper: usize, lower: usize) -> bool {
        chart.footprint[upper]
            && self.bed.heights[lower] < self.bed.heights[upper]
            && chart
                .ends
                .iter()
                .any(|end| end.is_brink() && end.distance(self.bed.point(upper)) <= 1e-8)
    }

    fn prepare_brink_banks(&mut self) {
        let mut banks = vec![Vec::new(); self.bodies.len()];
        let mut lips = Vec::new();
        for b in &self.bodies {
            let BodyKind::Chart(upper) = &b.kind else {
                continue;
            };
            if !upper.ends.iter().any(End::is_brink) {
                continue;
            }
            for (i, &inside) in upper.footprint.iter().enumerate() {
                if !inside {
                    continue;
                }
                for j in self.bed.neighbours(i) {
                    if upper.footprint[j] || !self.brink_above(upper, i, j) {
                        continue;
                    }
                    lips.push(i);
                    for (id, b) in self.bodies.iter().enumerate() {
                        if matches!(&b.kind, BodyKind::Chart(lower) if lower.footprint[j]) {
                            banks[id].push(i);
                        }
                    }
                }
            }
        }
        self.brink_banks = banks
            .into_iter()
            .map(|mut indices| {
                indices.sort_unstable();
                indices.dedup();
                indices.into()
            })
            .collect();
        lips.sort_unstable();
        lips.dedup();
        self.brink_lips = lips.into();
    }

    fn below_brink(&self, id: usize, chart: &Chart, weights: [(usize, f64); 4]) -> bool {
        // A cell can straddle the end of the shared brink edge. The
        // connected wet corner and the bed still decide the lateral bank.
        !self.brink_banks[id].is_empty()
            && [(0, 1), (0, 2), (1, 3), (2, 3)].into_iter().any(|(a, b)| {
                let ((i, u), (j, v)) = (weights[a], weights[b]);
                u > 1e-12
                    && v > 1e-12
                    && ((chart.footprint[i] && self.brink_banks[id].binary_search(&j).is_ok())
                        || (chart.footprint[j] && self.brink_banks[id].binary_search(&i).is_ok()))
            })
    }

    fn flood(&self, id: usize) -> Vec<bool> {
        let b = &self.bodies[id];
        if let BodyKind::Chart(c) = &b.kind {
            return c
                .footprint
                .iter()
                .enumerate()
                .map(|(i, &inside)| {
                    inside && self.bed.heights[i] < self.row(id, self.bed.point(i)).level
                })
                .collect();
        }
        if matches!(b.kind, BodyKind::Hollows(_)) {
            return vec![false; self.bed.heights.len()];
        }
        self.still_fill(id, b.stage.value)
    }

    pub(crate) fn still_fill(&self, id: usize, stage: f64) -> Vec<bool> {
        let n = self.bed.heights.len();
        let b = &self.bodies[id];
        if matches!(b.kind, BodyKind::Hollows(_)) {
            return vec![false; n];
        }
        let level = b.level + stage;
        let blocked = |i| {
            self.bodies
                .iter()
                .any(|b| matches!(&b.kind,BodyKind::Chart(c) if c.footprint[i]))
        };
        if matches!(b.kind, BodyKind::Sea) {
            return self.bed.heights.iter().map(|&h| h < level).collect();
        }
        let mut wet = vec![false; n];
        let mut q = VecDeque::new();
        if let BodyKind::Basin { seed } = b.kind
            && let Some(w) = self.bed.weights(seed)
            && self.bed.height(seed).is_some_and(|h| h < level)
        {
            for (i, _) in w {
                if !blocked(i) && self.bed.wet_reaches(i, w, level) {
                    wet[i] = true;
                    q.push_back(i);
                }
            }
        }
        while let Some(i) = q.pop_front() {
            for (j, weights) in self.bed.connections(i) {
                if !wet[j] && !blocked(j) && self.bed.wet_reaches(i, weights, level) {
                    wet[j] = true;
                    q.push_back(j);
                }
            }
        }
        wet
    }

    pub(crate) fn rebuild(&mut self) {
        self.spatial_revision = self.spatial_revision.wrapping_add(1);
        self.support = (0..self.bodies.len())
            .map(|id| self.build_support(id))
            .collect();
    }

    fn build_support(&self, id: usize) -> Arc<[Option<usize>]> {
        let wet = self.flood(id);
        let mut origins: Vec<_> = wet
            .iter()
            .enumerate()
            .map(|(i, &w)| w.then_some(i))
            .collect();
        let mut q: VecDeque<_> = wet
            .iter()
            .enumerate()
            .filter_map(|(i, &w)| w.then_some(i))
            .collect();
        while let Some(i) = q.pop_front() {
            for (j, weights) in self.bed.connections(i) {
                let h = self.bed.heights[j];
                let row = self.row(
                    id,
                    self.bed
                        .point(origins[i].expect("queued centre has origin")),
                );
                if origins[j].is_none()
                    && !self.waits(id, j)
                    && (self.brink_lips.binary_search(&j).is_err()
                        || matches!(&self.bodies[id].kind, BodyKind::Chart(c) if c.footprint[j]))
                    && h >= row.level
                    && self.bed.reaches(i, weights, row.level)
                    && h <= row.level + self.profile.margin_height
                {
                    origins[j] = origins[i];
                    q.push_back(j);
                }
            }
        }
        origins.into()
    }

    /// Every chart's banks hold its water over the stage ranges, as preparation checks them.
    pub(crate) fn check_banks(&mut self) -> Result<(), WaterError> {
        let stages: Vec<_> = self.bodies.iter().map(|b| b.stage.value).collect();
        for b in &mut self.bodies {
            b.stage.value = b.stage.min;
        }
        let minimum_fills: Vec<_> = (0..self.bodies.len()).map(|id| self.flood(id)).collect();
        for b in &mut self.bodies {
            b.stage.value = b.stage.max;
        }
        self.rebuild();
        let held = self.validate_banks(&minimum_fills);
        for (b, s) in self.bodies.iter_mut().zip(stages) {
            b.stage.value = s;
        }
        self.rebuild();
        held
    }

    fn validate_inputs(&self) -> Result<(), WaterError> {
        for (id, b) in self.bodies.iter().enumerate() {
            let fail = |reason| WaterError::new(&b.name, reason);
            if b.name.is_empty() || self.bodies[..id].iter().any(|a| a.name == b.name) {
                return Err(fail("duplicate or empty name"));
            }
            if [b.level, b.stage.min, b.stage.max, b.stage.value]
                .iter()
                .any(|v| !v.is_finite())
                || !b.fetch.is_finite()
                || b.fetch <= 0.0
                || b.stage.min > b.stage.max
                || !(b.stage.min..=b.stage.max).contains(&b.stage.value)
                || b.gains
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err(fail("invalid level, stage or gains"));
            }
            match &b.kind {
                BodyKind::Basin { seed } if self.bed.weights(*seed).is_none() => {
                    return Err(fail("seed outside bed"));
                }
                BodyKind::Chart(c) => {
                    let joins = || c.ends.iter().filter(|e| !e.is_brink());
                    if joins().filter(|e| e.mouth).count() > 1
                        || joins().filter(|e| !e.mouth).count() > 1
                    {
                        return Err(fail("chart has multiple sources or mouths"));
                    }
                    let n = self.bed.heights.len();
                    if c.levels.len() != n
                        || c.currents.len() != n
                        || c.footprint.len() != n
                        || !c.footprint.iter().any(|&v| v)
                        || c.levels
                            .iter()
                            .chain(c.currents.iter().flatten())
                            .any(|v| !v.is_finite())
                    {
                        return Err(fail("invalid chart grid"));
                    }
                    for e in &c.ends {
                        if e.body < id && matches!(self.bodies[e.body].kind, BodyKind::Hollows(_)) {
                            return Err(fail("chart joins a hollows body"));
                        }
                        if (e.body >= id && !e.is_brink())
                            || !e.length.is_finite()
                            || e.length <= 0.0
                            || e.segment.iter().flatten().any(|v| !v.is_finite())
                            || (e.segment[0][0] - e.segment[1][0])
                                .hypot(e.segment[0][1] - e.segment[1][1])
                                < 1e-8
                        {
                            return Err(fail("invalid end or cyclic join"));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_topology(&mut self) -> Result<(), WaterError> {
        let wets: Vec<_> = (0..self.bodies.len()).map(|id| self.flood(id)).collect();
        let sea_reach: Vec<_> = self
            .bodies
            .iter()
            .enumerate()
            .map(|(id, b)| {
                if matches!(b.kind, BodyKind::Sea) {
                    self.exterior_water(&wets[id], b.level + b.stage.value)
                } else {
                    vec![false; self.bed.heights.len()]
                }
            })
            .collect();
        for (id, b) in self.bodies.iter().enumerate() {
            let fail = |reason| WaterError::new(&b.name, reason);
            for j in 0..id {
                let overlap = match (&b.kind, &self.bodies[j].kind) {
                    (BodyKind::Chart(a), BodyKind::Chart(b)) => {
                        a.footprint.iter().zip(&b.footprint).any(|(&a, &b)| a && b)
                    }
                    _ => wets[id].iter().zip(&wets[j]).any(|(&a, &b)| a && b),
                };
                if std::mem::discriminant(&b.kind) == std::mem::discriminant(&self.bodies[j].kind)
                    && (matches!(b.kind, BodyKind::Sea) || overlap)
                {
                    return Err(fail("same-kind bodies overlap"));
                }
            }
            if matches!(b.kind, BodyKind::Basin { .. }) {
                if wets[id]
                    .iter()
                    .enumerate()
                    .any(|(i, &w)| w && self.bed.edge(i))
                {
                    return Err(fail("basin reaches bed edge"));
                }
                for (i, &wet) in wets[id].iter().enumerate() {
                    if !wet {
                        continue;
                    }
                    // A connected path to sea below its maximum tide is a spill.
                    if self
                        .bodies
                        .iter()
                        .enumerate()
                        .any(|(j, s)| matches!(s.kind, BodyKind::Sea) && sea_reach[j][i])
                    {
                        return Err(fail("basin reaches sea"));
                    }
                    for j in self.bed.neighbours(i) {
                        for other in &self.bodies {
                            if let BodyKind::Chart(c) = &other.kind
                                && c.footprint[j]
                                && !self.brink_above(c, j, i)
                                && !c.ends.iter().any(|e| {
                                    e.body == id
                                        && e.distance(self.bed.point(j)) <= self.bed.spacing * 1.01
                                })
                            {
                                return Err(fail("basin reaches chart away from end"));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn exterior_water(&self, wet: &[bool], level: f64) -> Vec<bool> {
        // A sealed basin may have its floor below sea level. Precedence owns that
        // column; a spill means connection to exterior sea, not merely low ground.
        let blocked = |i| {
            self.bodies
                .iter()
                .any(|b| matches!(&b.kind,BodyKind::Chart(c) if c.footprint[i]))
        };
        let mut reached = vec![false; wet.len()];
        let mut q = VecDeque::new();
        for (i, &w) in wet.iter().enumerate() {
            if w && self.bed.edge(i) && !blocked(i) {
                reached[i] = true;
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            for (j, weights) in self.bed.connections(i) {
                if wet[j] && !reached[j] && !blocked(j) && self.bed.wet_reaches(i, weights, level) {
                    reached[j] = true;
                    q.push_back(j);
                }
            }
        }
        reached
    }

    fn validate_banks(&self, minimum_fills: &[Vec<bool>]) -> Result<(), WaterError> {
        for id in 0..self.bodies.len() {
            let BodyKind::Chart(c) = &self.bodies[id].kind else {
                continue;
            };
            for end in &c.ends {
                if end.is_brink() {
                    // A brink lies on the footprint's edge, where the ground drops away.
                    let on_edge = |i: usize| {
                        c.footprint[i]
                            && end.distance(self.bed.point(i)) <= 1e-8
                            && self.bed.neighbours(i).any(|j| {
                                !c.footprint[j] && self.bed.heights[j] < self.bed.heights[i]
                            })
                    };
                    if !(0..c.footprint.len()).any(on_edge) {
                        return Err(WaterError::new(
                            &self.bodies[id].name,
                            "brink misses footprint edge",
                        ));
                    }
                    continue;
                }
                let joined = &self.bodies[end.body];
                let mut edge_cells = 0;
                for (i, &inside) in c.footprint.iter().enumerate() {
                    if !inside || end.distance(self.bed.point(i)) > 1e-8 {
                        continue;
                    }
                    if !self.bed.neighbours(i).any(|j| !c.footprint[j]) {
                        return Err(WaterError::new(
                            &self.bodies[id].name,
                            "join is not on footprint edge",
                        ));
                    }
                    edge_cells += 1;
                    let level = match &joined.kind {
                        BodyKind::Chart(other) => other.levels[i] + joined.stage.min,
                        _ => joined.level + joined.stage.min,
                    };
                    if self.bed.heights[i] >= level {
                        return Err(WaterError::new(
                            &self.bodies[id].name,
                            "join sill stands above minimum stage",
                        ));
                    }
                    // One row cannot equal two different receivers simultaneously.
                    // Test the chart's lowest own stage against the receiving reach
                    // at its highest stage, not just today's mix.
                    let p = self.bed.point(i);
                    let own_min = c.levels[i] + self.bodies[id].stage.min;
                    if c.ends.iter().filter(|e| !e.is_brink()).any(|other| {
                        other.body != end.body
                            && (other.distance(p) <= 1e-8
                                || (other.mouth
                                    && own_min
                                        <= self.row(other.body, p).level
                                            + 0.175 * self.profile.margin_height))
                    }) {
                        return Err(WaterError::new(
                            &self.bodies[id].name,
                            "conflicting fully weighted joins",
                        ));
                    }
                }
                if edge_cells == 0 {
                    return Err(WaterError::new(
                        &self.bodies[id].name,
                        "join misses footprint edge",
                    ));
                }
            }
            for (i, &inside) in c.footprint.iter().enumerate() {
                if !inside {
                    continue;
                }
                if self.bed.edge(i) {
                    return Err(WaterError::new(
                        &self.bodies[id].name,
                        "chart reaches bed edge",
                    ));
                }
                for j in self.bed.neighbours(i).filter(|&j| !c.footprint[j]) {
                    let p = self.bed.point(i);
                    let own = c.levels[i] + self.bodies[id].stage.max;
                    // A raised source can lift a bank inside its blend even when
                    // the chart's own prepared level would remain contained.
                    let highest = own.max(self.row(id, p).level);
                    // The water leaves over a declared brink: the ground below drops away
                    // and is in no body.
                    let brink = c.ends.iter().any(|e| {
                        e.is_brink()
                            && e.distance(p) <= 1e-8
                            && self.bed.heights[j] < self.bed.heights[i]
                    });
                    let joined = brink
                        || self.brink_banks[id].binary_search(&j).is_ok()
                        || c.ends.iter().filter(|e| !e.is_brink()).any(|e| {
                            let b = &self.bodies[e.body];
                            let min = b.level + b.stage.min;
                            let wet = match &b.kind {
                                BodyKind::Sea => self.bed.heights[j] < min,
                                BodyKind::Basin { .. } => {
                                    // Max-stage connectivity must also have a wet sill at minimum stage.
                                    minimum_fills[e.body][j] && self.bed.heights[j] < min
                                }
                                BodyKind::Chart(other) => {
                                    other.footprint[j]
                                        && self.bed.heights[j] < other.levels[j] + b.stage.min
                                }
                                // Refused in validate_inputs: no water joins a hollows body.
                                BodyKind::Hollows(_) => false,
                            };
                            e.distance(p) <= 1e-8 && wet
                        });
                    if self.bed.heights[j] <= highest && !joined {
                        return Err(WaterError::new(
                            &self.bodies[id].name,
                            "chart bank does not contain water",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
