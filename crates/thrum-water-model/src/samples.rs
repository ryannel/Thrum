//! A river's flow samples and its gauge (`RIVERS` ruling 2). A river is solved once for each flow
//! sample; each sample records its gauge level. Every chart row of the river (level, current,
//! loss and flow gains) is the mix, linear in the gauge, of the two samples whose gauge levels
//! bracket it, in every cell both samples wet. Between two samples a cell the upper wets and the
//! lower leaves dry holds water only from its prepared wetting gauge: below it the cell has no
//! water, wave support or shore margin and its level lies under its bed; from it the cell's level
//! runs linearly from its bed to the upper sample's, and its current, loss and gains from nothing
//! to the upper sample's. The gauge is the source body's level, its stage included, where the
//! river has one, and the river's own stage where it has none; every chart of a river takes the
//! same gauge. A source body is a sea, a basin or another river's chart: a chart's level is its
//! own at the place the river leaves it, so a river below follows the river above through the
//! one map. A river with one sample answers as a chart always did.
use crate::fall::{Fall, FallSegment, PathPoint};
use crate::{BodyKind, Map, Stage, WaterError};

/// One flow sample of a chart, on the map bed's lattice, row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowSample {
    /// Level at each bed centre; under the bed wherever the sample leaves the cell dry.
    pub levels: Vec<f64>,
    /// Current at each bed centre; none in a dry cell.
    pub currents: Vec<[f64; 2]>,
    /// The power that breaks the surface, watts for each square metre.
    pub loss: Vec<f64>,
    /// A gain for each flow band.
    pub gains: Vec<[f64; 2]>,
}

/// One chart body's flow samples, in the river's gauge order.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSamples {
    /// The chart's body. Its declared grids are the first sample's, and its stage is zero.
    pub body: usize,
    /// One for each of the river's gauge levels.
    pub samples: Vec<FlowSample>,
    /// For each pair of neighbouring samples, the gauge at which each cell first holds water on
    /// the way from the lower to the upper; none where the lower sample wets the cell or the
    /// upper leaves it dry.
    pub wetting: Vec<Vec<Option<f64>>>,
}

/// What a river's gauge reads.
#[derive(Clone, Copy, Debug)]
pub enum Gauge {
    /// A source body's level, its stage included: a sea or a basin declared before the charts.
    Body(usize),
    /// A source body that is a chart: its own level, before its joins, at the place this river
    /// leaves it. The chart is another river's, already in the map, or a chart with no river,
    /// whose stage then adds to its level; it cannot be one of this river's own.
    Chart {
        /// The chart's body.
        body: usize,
        /// World x/z of a place in the chart's footprint, at this river's source end.
        place: [f64; 2],
    },
    /// The river's own stage, for a river fed by springs.
    Stage(Stage),
}

/// A river's charts and the gauge levels of its samples.
#[derive(Clone, Debug)]
pub struct River {
    /// Diagnostic name.
    pub name: String,
    /// Each sample's gauge level, rising.
    pub gauges: Vec<f64>,
    /// What the gauge reads.
    pub gauge: Gauge,
    /// Every chart of the river.
    pub charts: Vec<ChartSamples>,
}

/// One cell's mixed row.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Mixed {
    pub(crate) level: f64,
    pub(crate) current: [f64; 2],
    pub(crate) loss: f64,
    pub(crate) gains: [f64; 2],
}

/// Where a gauge lies among a river's samples: the lower sample and the weight of the upper.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bracket {
    pub(crate) lower: usize,
    pub(crate) weight: f64,
    pub(crate) gauge: f64,
}

pub(crate) fn bracket(gauges: &[f64], g: f64) -> Bracket {
    let last = gauges.len() - 1;
    let g = g.clamp(gauges[0], gauges[last]);
    if last == 0 {
        return Bracket {
            lower: 0,
            weight: 0.0,
            gauge: g,
        };
    }
    let lower = gauges[..last]
        .iter()
        .rposition(|&a| a <= g)
        .unwrap_or(0)
        .min(last - 1);
    let (a, b) = (gauges[lower], gauges[lower + 1]);
    Bracket {
        lower,
        weight: ((g - a) / (b - a)).clamp(0.0, 1.0),
        gauge: g,
    }
}

impl ChartSamples {
    /// The mixed row at cell `i` whose bed is `bed`.
    pub(crate) fn cell(&self, gauges: &[f64], at: Bracket, i: usize, bed: f64) -> Mixed {
        let a = &self.samples[at.lower];
        if self.samples.len() == 1 {
            return Mixed {
                level: a.levels[i],
                current: a.currents[i],
                loss: a.loss[i],
                gains: a.gains[i],
            };
        }
        let b = &self.samples[at.lower + 1];
        let lerp = |x: f64, y: f64, w: f64| x * (1.0 - w) + y * w;
        let (wet_a, wet_b) = (a.levels[i] > bed, b.levels[i] > bed);
        if wet_a || !wet_b {
            let w = at.weight;
            return Mixed {
                level: lerp(a.levels[i], b.levels[i], w),
                current: std::array::from_fn(|k| lerp(a.currents[i][k], b.currents[i][k], w)),
                loss: lerp(a.loss[i], b.loss[i], w),
                gains: std::array::from_fn(|k| lerp(a.gains[i][k], b.gains[i][k], w)),
            };
        }
        let (ga, gb) = (gauges[at.lower], gauges[at.lower + 1]);
        let wg = self.wetting[at.lower][i].unwrap_or(gb).clamp(ga, gb);
        let g = at.gauge;
        if g <= wg && g < gb {
            // Dry: the level runs from the lower sample's, under the bed, to the bed itself.
            let f = if wg > ga { (g - ga) / (wg - ga) } else { 1.0 };
            return Mixed {
                level: lerp(a.levels[i], bed, f),
                ..Mixed::default()
            };
        }
        let f = if gb > wg { (g - wg) / (gb - wg) } else { 1.0 };
        Mixed {
            level: lerp(bed, b.levels[i], f),
            current: b.currents[i].map(|c| c * f),
            loss: b.loss[i] * f,
            gains: b.gains[i].map(|c| c * f),
        }
    }

    /// Whether cell `i` waits for its wetting gauge, and so has no water and no shore margin.
    pub(crate) fn waits(&self, gauges: &[f64], at: Bracket, i: usize, bed: f64) -> bool {
        if self.samples.len() == 1 {
            return false;
        }
        let (a, b) = (&self.samples[at.lower], &self.samples[at.lower + 1]);
        if a.levels[i] > bed || b.levels[i] <= bed {
            return false;
        }
        let gb = gauges[at.lower + 1];
        let wg = self.wetting[at.lower][i].unwrap_or(gb);
        at.gauge <= wg && at.gauge < gb
    }
}

impl Map {
    /// Give the map a river's flow samples. Its charts must already be declared, each with its
    /// first sample's grids and a stage of zero; the gauge then moves every chart of the river.
    /// Returns the river's index.
    /// # Errors
    /// Refuses, naming the river, gauge levels that are not finite and rising, a gauge body that
    /// is not a sea or a basin or whose stage reaches outside the samples, a gauge chart that is
    /// not a chart, is one of the river's own, or whose level at a place outside its footprint
    /// or outside the samples is asked for, a stage outside them, a chart that is not a chart,
    /// already in a river, the gauge of an earlier river, staged, or whose declared grids are not
    /// its first sample's, grids of the wrong size or with nonfinite or negative values, wetting
    /// gauges outside their samples, and any sample whose banks do not hold its water.
    /// A refused river leaves the map and every answer it gives unchanged.
    pub fn add_river(&mut self, river: River) -> Result<usize, WaterError> {
        let fail = |reason| Err(WaterError::new(&river.name, reason));
        let g = &river.gauges;
        if river.name.is_empty()
            || g.is_empty()
            || g.iter().any(|v| !v.is_finite())
            || g.windows(2).any(|w| w[1] <= w[0])
        {
            return fail("gauge levels are not finite and rising");
        }
        let (lo, hi) = (g[0] - 1e-9, g[g.len() - 1] + 1e-9);
        match river.gauge {
            Gauge::Body(id) => {
                let Some(b) = self.bodies.get(id) else {
                    return fail("gauge body is unknown");
                };
                if matches!(b.kind, BodyKind::Chart(_)) {
                    return fail("gauge body is a chart, whose level needs a place");
                }
                if !matches!(b.kind, BodyKind::Sea | BodyKind::Basin { .. }) {
                    return fail("gauge body is not a sea or a basin");
                }
                if b.level + b.stage.min < lo || b.level + b.stage.max > hi {
                    return fail("gauge body's stage reaches outside the samples");
                }
            }
            Gauge::Chart { body, place } => {
                let Some(b) = self.bodies.get(body) else {
                    return fail("gauge chart is unknown");
                };
                let BodyKind::Chart(chart) = &b.kind else {
                    return fail("gauge chart is not a chart");
                };
                if river.charts.iter().any(|c| c.body == body) {
                    return fail("gauge chart is one of the river's own");
                }
                let inside = place.iter().all(|v| v.is_finite())
                    && self
                        .bed
                        .weights(place)
                        .is_some_and(|w| w.iter().all(|&(i, a)| a <= 1e-12 || chart.footprint[i]));
                if !inside {
                    return fail("gauge place is not in the gauge chart's footprint");
                }
                let [min, max] = self.chart_level_range(body, place);
                if min < lo || max > hi {
                    return fail("gauge chart's level reaches outside the samples");
                }
            }
            Gauge::Stage(s) => {
                if ![s.min, s.max, s.value].iter().all(|v| v.is_finite())
                    || s.min > s.max
                    || !(s.min..=s.max).contains(&s.value)
                    || s.min < lo
                    || s.max > hi
                {
                    return fail("river stage is invalid or outside the samples");
                }
            }
        }
        let n = self.bed.heights.len();
        if river.charts.is_empty() {
            return fail("river has no chart");
        }
        for (k, c) in river.charts.iter().enumerate() {
            let Some(body) = self.bodies.get(c.body) else {
                return fail("chart body is unknown");
            };
            let BodyKind::Chart(chart) = &body.kind else {
                return fail("chart body is not a chart");
            };
            if self.sampled[c.body].is_some() || river.charts[..k].iter().any(|o| o.body == c.body)
            {
                return fail("chart is already in a river");
            }
            // A gauge reads only what was in the map before its river, so no gauge reads itself.
            if self
                .rivers
                .iter()
                .any(|r| matches!(r.gauge, Gauge::Chart { body, .. } if body == c.body))
            {
                return fail("chart is the gauge of an earlier river");
            }
            if body.stage.min != 0.0 || body.stage.max != 0.0 {
                return fail("a river's chart has a stage");
            }
            if c.samples.len() != g.len() || c.wetting.len() != g.len() - 1 {
                return fail("chart's samples do not match the gauge levels");
            }
            for s in &c.samples {
                if s.levels.len() != n
                    || s.currents.len() != n
                    || s.loss.len() != n
                    || s.gains.len() != n
                    || s.levels
                        .iter()
                        .chain(s.currents.iter().flatten())
                        .any(|v| !v.is_finite())
                    || s.loss
                        .iter()
                        .chain(s.gains.iter().flatten())
                        .any(|v| !v.is_finite() || *v < 0.0)
                {
                    return fail("invalid sample grid");
                }
            }
            if chart.levels != c.samples[0].levels || chart.currents != c.samples[0].currents {
                return fail("chart's grids are not its first sample's");
            }
            for (k, w) in c.wetting.iter().enumerate() {
                if w.len() != n
                    || w.iter()
                        .flatten()
                        .any(|v| !v.is_finite() || *v < g[k] || *v > g[k + 1])
                {
                    return fail("invalid wetting gauges");
                }
            }
        }
        // Every sample's banks hold its water, by the rule the map prepared the first with.
        for k in 1..g.len() {
            self.put_sample(&river, k);
            let held = self.check_banks();
            self.put_sample(&river, 0);
            if let Err(e) = held {
                // The check left support built from the refused sample: a refused river
                // leaves the map answering exactly as before.
                self.rebuild();
                return Err(WaterError::new(&river.name, e.reason));
            }
        }
        let id = self.rivers.len();
        for (k, c) in river.charts.iter().enumerate() {
            if g.len() > 1
                && let BodyKind::Chart(chart) = &self.bodies[c.body].kind
            {
                // The surface mesh is prepared once, even for reaches that
                // have no support at the first sample. Wetting gates still
                // select the pixels within this coverage at display time.
                let footprint = chart
                    .footprint
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &inside)| inside.then_some(i));
                if let Some([min, max]) = self.surface_bounds_for(footprint) {
                    let [lo, hi] = self.surface_bounds[c.body].unwrap_or([min, max]);
                    self.surface_bounds[c.body] = Some([
                        std::array::from_fn(|axis| lo[axis].min(min[axis])),
                        std::array::from_fn(|axis| hi[axis].max(max[axis])),
                    ]);
                }
            }
            self.sampled[c.body] = Some((id, k));
        }
        self.rivers.push(river);
        self.rebuild();
        Ok(id)
    }

    /// Set the declared levels of a river's charts to sample `k`'s.
    fn put_sample(&mut self, river: &River, k: usize) {
        for c in &river.charts {
            if let BodyKind::Chart(chart) = &mut self.bodies[c.body].kind {
                chart.levels.clone_from(&c.samples[k].levels);
            }
        }
    }

    /// Set a river's own stage, for a river fed by springs, counting range clamps.
    /// # Errors
    /// Refuses an unknown river, a river whose gauge is a body's level, and a nonfinite stage.
    pub fn set_gauge(&mut self, river: usize, value: f64) -> Result<(), WaterError> {
        let r = self
            .rivers
            .get_mut(river)
            .ok_or_else(|| WaterError::new("gauge", "unknown river"))?;
        let Gauge::Stage(s) = &mut r.gauge else {
            return Err(WaterError::new(
                &r.name,
                "the gauge follows its source body's level",
            ));
        };
        if !value.is_finite() {
            return Err(WaterError::new(&r.name, "nonfinite stage"));
        }
        s.value = value.clamp(s.min, s.max);
        if value < s.min || value > s.max {
            self.clamped_stages += 1;
        }
        self.rebuild();
        Ok(())
    }

    /// A river's gauge now.
    /// # Panics
    /// If `river` is not a river of the map.
    pub fn gauge(&self, river: usize) -> f64 {
        let r = &self.rivers[river];
        match r.gauge {
            Gauge::Body(id) => self.bodies[id].level + self.bodies[id].stage.value,
            Gauge::Chart { body, place } => self.own_level(body, place),
            Gauge::Stage(s) => s.value,
        }
    }

    /// The lowest and highest values a river's gauge may take.
    fn gauge_range(&self, river: usize) -> [f64; 2] {
        match self.rivers[river].gauge {
            Gauge::Body(id) => {
                let b = &self.bodies[id];
                [b.level + b.stage.min, b.level + b.stage.max]
            }
            Gauge::Chart { body, place } => self.chart_level_range(body, place),
            Gauge::Stage(s) => [s.min, s.max],
        }
    }

    /// The lowest and highest own levels a chart may take at a place on the bed: over its
    /// river's gauge range for a river's chart, else over its stage's range. A river chart's
    /// level is linear in the gauge between its samples' gauges and its cells' wetting gauges,
    /// so its extremes lie at those and at the range's ends.
    pub(crate) fn chart_level_range(&self, body: usize, place: [f64; 2]) -> [f64; 2] {
        let b = &self.bodies[body];
        let (BodyKind::Chart(chart), Some(w)) = (&b.kind, self.bed.weights(place)) else {
            return [b.level + b.stage.min, b.level + b.stage.max];
        };
        let Some((r, k)) = self.sampled[body] else {
            let grid: f64 = w.iter().map(|&(i, a)| chart.levels[i] * a).sum();
            return [grid + b.stage.min, grid + b.stage.max];
        };
        let river = &self.rivers[r];
        let c = &river.charts[k];
        let g = &river.gauges;
        let [lo, hi] = self.gauge_range(r);
        let (lo, hi) = (lo.max(g[0]), hi.min(g[g.len() - 1]).max(lo.max(g[0])));
        let knots = g.iter().copied().chain(
            c.wetting
                .iter()
                .flat_map(|cells| w.iter().filter_map(|&(i, _)| cells[i])),
        );
        let mut range = [f64::INFINITY, f64::NEG_INFINITY];
        for gauge in [lo, hi]
            .into_iter()
            .chain(knots.filter(|v| (lo..=hi).contains(v)))
        {
            let at = bracket(g, gauge);
            let level: f64 = w
                .iter()
                .map(|&(i, a)| c.cell(g, at, i, self.bed.heights[i]).level * a)
                .sum();
            range = [range[0].min(level), range[1].max(level)];
        }
        range
    }

    /// A river's declaration.
    pub fn river(&self, river: usize) -> Option<&River> {
        self.rivers.get(river)
    }

    /// The river and the chart a body is, if it is a river's chart.
    pub fn river_of(&self, body: usize) -> Option<(usize, usize)> {
        self.sampled.get(body).copied().flatten()
    }

    /// Where a body's river's gauge lies among its samples, with the body's chart samples.
    pub(crate) fn bracket_of(&self, body: usize) -> Option<(&ChartSamples, &[f64], Bracket)> {
        let (r, k) = self.sampled.get(body).copied().flatten()?;
        let river = &self.rivers[r];
        let at = bracket(&river.gauges, self.gauge(r));
        Some((&river.charts[k], &river.gauges, at))
    }

    /// A river chart's own mixed row at a point, before any join, by its four bed centres.
    pub(crate) fn own_mixed(&self, body: usize, point: [f64; 2]) -> Option<Mixed> {
        let (c, gauges, at) = self.bracket_of(body)?;
        let w = self.bed.weights(point)?;
        let mut out = Mixed::default();
        for (i, a) in w {
            let m = c.cell(gauges, at, i, self.bed.heights[i]);
            out.level += m.level * a;
            out.loss += m.loss * a;
            for k in 0..2 {
                out.current[k] += m.current[k] * a;
                out.gains[k] += m.gains[k] * a;
            }
        }
        Some(out)
    }

    /// Whether a bed centre of a river chart waits for its wetting gauge.
    pub(crate) fn waits(&self, body: usize, i: usize) -> bool {
        self.bracket_of(body)
            .is_some_and(|(c, gauges, at)| c.waits(gauges, at, i, self.bed.heights[i]))
    }

    /// A body's flow-band gains and loss at a point, joins included: a river chart's own mix
    /// toward each joined body's by the join's weight. Every body that is not a river's chart,
    /// and joins only such bodies, has none.
    pub fn flow(&self, body: usize, point: [f64; 2]) -> ([f64; 2], f64) {
        let BodyKind::Chart(c) = &self.bodies[body].kind else {
            return ([0.0; 2], 0.0);
        };
        let own = self.own_mixed(body, point).unwrap_or_default();
        let mut gains = own.gains;
        let mut loss = own.loss;
        let own_level = self.own_level(body, point);
        let mut total = 1.0;
        for end in c.ends.iter().filter(|e| !e.is_brink()) {
            let other_level = self.row(end.body, point).level;
            let weight = self.join_weight(end, point, own_level, other_level);
            let (g, l) = self.flow(end.body, point);
            if weight >= 1.0 {
                return (g, l);
            }
            let relative = weight / (1.0 - weight);
            total += relative;
            let w = relative / total;
            gains = std::array::from_fn(|k| gains[k] * (1.0 - w) + g[k] * w);
            loss = loss * (1.0 - w) + l * w;
        }
        (gains, loss)
    }

    /// A body's flow-band gains at a point.
    pub fn flow_gains(&self, body: usize, point: [f64; 2]) -> [f64; 2] {
        self.flow(body, point).0
    }

    /// A chart's own level at a point before its joins: the mix for a river's chart, else its
    /// grid plus its stage.
    pub(crate) fn own_level(&self, body: usize, point: [f64; 2]) -> f64 {
        let b = &self.bodies[body];
        if let Some(m) = self.own_mixed(body, point) {
            return m.level;
        }
        match (&b.kind, self.bed.weights(point)) {
            (BodyKind::Chart(c), Some(w)) => {
                b.stage.value + w.iter().map(|&(i, a)| c.levels[i] * a).sum::<f64>()
            }
            _ => b.level + b.stage.value,
        }
    }
}

/// A fall's segments at a gauge, from its records in the two samples that bracket it.
/// `lower` and `upper` are the same fall in each, `wetting` the gauge at which each segment's
/// lip cell first holds water on the way to the upper sample (for a segment only the upper runs).
/// Where both run a segment, its discharge, launch depth and speed and its path are the mix.
/// A segment only the upper runs gives nothing up to its lip's wetting gauge, and from there its
/// discharge and launch depth rise from nothing along the upper's path at the upper's speed. A
/// segment only the lower runs dries toward the upper. A segment with no discharge is left out.
pub fn mix_fall(
    lower: Option<&Fall>,
    upper: Option<&Fall>,
    gauges: [f64; 2],
    gauge: f64,
    wetting: impl Fn(u64) -> Option<f64>,
) -> Option<Fall> {
    let base = upper.or(lower)?;
    let [ga, gb] = gauges;
    let g = gauge.clamp(ga, gb);
    let w = if gb > ga { (g - ga) / (gb - ga) } else { 1.0 };
    let find = |f: Option<&Fall>, id: u64| {
        f.and_then(|f| f.segments.iter().find(|s| s.id == id))
            .cloned()
    };
    let mut ids: Vec<u64> = lower
        .iter()
        .chain(upper.iter())
        .flat_map(|f| f.segments.iter().map(|s| s.id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let mut segments = Vec::with_capacity(ids.len());
    for id in ids {
        let seg = match (find(lower, id), find(upper, id)) {
            (Some(a), Some(b)) => {
                let lerp = |x: f64, y: f64| x * (1.0 - w) + y * w;
                FallSegment {
                    discharge: lerp(a.discharge, b.discharge),
                    launch_depth: lerp(a.launch_depth, b.launch_depth),
                    launch_speed: lerp(a.launch_speed, b.launch_speed),
                    path: mix_path(&a.path, &b.path, w),
                    below: if w < 0.5 { a.below.clone() } else { b.below },
                    ..a
                }
            }
            (None, Some(b)) => {
                let wg = wetting(id).unwrap_or(gb).clamp(ga, gb);
                if g <= wg {
                    continue;
                }
                let f = if gb > wg { (g - wg) / (gb - wg) } else { 1.0 };
                FallSegment {
                    discharge: b.discharge * f,
                    launch_depth: b.launch_depth * f,
                    ..b
                }
            }
            (Some(a), None) => FallSegment {
                discharge: a.discharge * (1.0 - w),
                launch_depth: a.launch_depth * (1.0 - w),
                ..a
            },
            (None, None) => continue,
        };
        if seg.discharge > 0.0 {
            segments.push(seg);
        }
    }
    (!segments.is_empty()).then(|| Fall {
        name: base.name.clone(),
        above: base.above.clone(),
        segments,
    })
}

/// Two paths mixed at matching shares of their length: each is resampled to the larger point
/// count by distance along it, and the points are mixed.
fn mix_path(a: &[PathPoint], b: &[PathPoint], w: f64) -> Vec<PathPoint> {
    let n = a.len().max(b.len()).max(2);
    let at = |p: &[PathPoint], u: f64| -> PathPoint {
        let total = p[p.len() - 1].distance;
        let d = u * total;
        let k = p
            .windows(2)
            .position(|s| d <= s[1].distance)
            .unwrap_or(p.len().saturating_sub(2));
        let (s, e) = (p[k], p[(k + 1).min(p.len() - 1)]);
        let span = e.distance - s.distance;
        let f = if span > 0.0 {
            ((d - s.distance) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let l = |x: f64, y: f64| x + (y - x) * f;
        PathPoint {
            place: [l(s.place[0], e.place[0]), l(s.place[1], e.place[1])],
            height: l(s.height, e.height),
            time: l(s.time, e.time),
            distance: d,
        }
    };
    (0..n)
        .map(|j| {
            #[expect(clippy::cast_precision_loss, reason = "path point counts are small")]
            let u = j as f64 / (n - 1) as f64;
            let (p, q) = (at(a, u), at(b, u));
            let l = |x: f64, y: f64| x * (1.0 - w) + y * w;
            PathPoint {
                place: [l(p.place[0], q.place[0]), l(p.place[1], q.place[1])],
                height: l(p.height, q.height),
                time: l(p.time, q.time),
                distance: l(p.distance, q.distance),
            }
        })
        .collect()
}
