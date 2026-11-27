//! The river file's reader. A river file holds what one steady flow solve wrote, so a game can
//! prepare solved rivers without linking the solver. `thrum-river-model` writes the same layout.
//!
//! A river is solved once for each flow sample. The file's reaches, falls and gauge level are its
//! lowest sample's; each sample above it adds its gauge level, a row of each reach and its falls.
//!
//! Layout, little-endian throughout:
//! `b"THRUMRIV"`, format `u32`, key `[u8; 32]`, gauge level `f64`, upper sample count `u32` and
//! each upper sample's gauge level `f64`, reach count `u32`, then for each reach: name (`u32`
//! length and UTF-8), origin `2 × f64`, spacing `f64`, size `2 × u32`, dry margin `f64`, then for
//! each of its cells, row-major x-fastest: level `f64`, current `2 × f64`, loss `f64`, flow gains
//! `2 × f64` and footprint `u8`; then for each upper sample and each cell: level, current, loss
//! and flow gains as before and its wetting gauge (`u8` 0 for none, or 1 and `f64`); then an end
//! count `u32` and for each end: body name (empty for a brink), segment `4 × f64`, join length
//! `f64` and kind `u8` (0 source, 1 mouth, 2 brink). Then the lowest sample's falls: a fall count
//! `u32` and for each fall: name, the reach above's name and a segment count `u32`; for each
//! segment: ID `u64`, lip `4 × f64`, discharge, launch depth and launch speed `f64`, where it ends
//! (`u8`, 0 a body and 1 a reach, then its name) and a point count `u32`; for each point: place
//! `2 × f64`, height, time and distance `f64`. Then for each upper sample its falls as before and
//! a count `u32` of lip wetting gauges, each a segment ID `u64` and a gauge `f64`. Nothing follows.
use crate::fall::{Below, Fall, FallSegment, PathPoint};
use crate::samples::bracket;
use crate::{BRINK, Bed, Chart, ChartSamples, End, FlowSample, Gauge, River, WaterError, mix_fall};

/// The magic bytes every river file starts with.
pub const RIVER_MAGIC: [u8; 8] = *b"THRUMRIV";
/// The layout version this reader accepts.
pub const RIVER_FORMAT: u32 = 3;

/// A solve's stored output and the key of the inputs it was solved from.
#[derive(Clone, Debug, PartialEq)]
pub struct RiverFile {
    /// Hash of the bed inside the rectangle, the declaration, the solve's profile and the solver's version.
    pub key: [u8; 32],
    /// The level at the river's source end for the lowest flow sample, metres.
    pub gauge_level: f64,
    /// One chart for each reach.
    pub reaches: Vec<RiverReach>,
    /// One record for each fall, in the lowest sample.
    pub falls: Vec<Fall>,
    /// The samples above the lowest, rising.
    pub upper: Vec<UpperSample>,
}

/// A flow sample above a river's lowest: its gauge level and its falls. Its rows are in each
/// reach's [`RiverReach::upper`].
#[derive(Clone, Debug, PartialEq)]
pub struct UpperSample {
    /// The level at the river's source end for this sample, metres.
    pub gauge_level: f64,
    /// The same falls as the lowest sample's, by name and order, each with the segments this
    /// sample runs.
    pub falls: Vec<Fall>,
    /// For each segment this sample runs and the one below does not, the gauge at which its lip
    /// cell first holds water on the way to this sample.
    pub lip_wetting: Vec<(u64, f64)>,
}

/// The fall records of a solved river, independent of the reach grids.
#[derive(Clone, Debug)]
pub struct FallSamples {
    lowest: Vec<Fall>,
    upper: Vec<UpperSample>,
    gauges: Vec<f64>,
}

impl FallSamples {
    /// The largest number of segments that may run at once across a gauge interval.
    pub fn segment_capacity(&self) -> usize {
        let count = |falls: &[Fall]| falls.iter().map(|fall| fall.segments.len()).sum();
        let counts = std::iter::once(count(&self.lowest))
            .chain(self.upper.iter().map(|sample| count(&sample.falls)))
            .collect::<Vec<usize>>();
        counts
            .windows(2)
            .map(|pair| pair[0] + pair[1])
            .max()
            .unwrap_or(counts[0])
    }

    /// Gauge levels from the same solve as the recorded paths.
    pub fn gauges(&self) -> &[f64] {
        &self.gauges
    }

    /// Mix the two samples that bracket the live river gauge.
    pub fn at(&self, gauge: f64) -> Vec<Fall> {
        falls_at(&self.lowest, &self.upper, &self.gauges, gauge)
    }

    pub(crate) fn records(&self) -> impl Iterator<Item = &Fall> {
        self.lowest
            .iter()
            .chain(self.upper.iter().flat_map(|sample| &sample.falls))
    }
}

/// A reach's row in a sample above the lowest, on its rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct ReachSample {
    /// Level at each cell; under the bed in every cell the sample leaves dry.
    pub levels: Vec<f64>,
    /// Current in x/z metres per second; none in a dry cell.
    pub currents: Vec<[f64; 2]>,
    /// The power that breaks the surface, watts for each square metre.
    pub loss: Vec<f64>,
    /// A gain for each flow band.
    pub gains: Vec<[f64; 2]>,
    /// The gauge at which each cell first holds water on the way from the sample below to this
    /// one; none where the sample below wets it or this one leaves it dry.
    pub wetting: Vec<Option<f64>>,
}

/// One reach's chart on its solve rectangle, which lies on the map bed's lattice.
#[derive(Clone, Debug, PartialEq)]
pub struct RiverReach {
    /// The reach's name, for diagnostics.
    pub name: String,
    /// World x/z of the rectangle's first cell centre.
    pub origin: [f64; 2],
    /// Cell centre spacing in metres.
    pub spacing: f64,
    /// Cell counts in x and z.
    pub size: [usize; 2],
    /// How far under the bed a cell outside the footprint is held, metres.
    pub dry_margin: f64,
    /// Level at each cell; under the bed in every cell the solve left dry.
    pub levels: Vec<f64>,
    /// Current in x/z metres per second; none in a dry cell.
    pub currents: Vec<[f64; 2]>,
    /// The power that breaks the surface, watts for each square metre.
    pub loss: Vec<f64>,
    /// A gain for each flow band: the flow's chop over each band's own rms height.
    pub gains: Vec<[f64; 2]>,
    /// Cells that belong to the chart's footprint: those any sample wets and their margin.
    pub footprint: Vec<bool>,
    /// The reach's row in each sample above the lowest.
    pub upper: Vec<ReachSample>,
    /// Source ends, mouths and brinks, by the name of the body each joins.
    pub ends: Vec<RiverEnd>,
}

/// What a chart end is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndKind {
    /// Water enters from the joined body.
    Source,
    /// Water leaves into the joined body.
    Mouth,
    /// Water leaves as a fall, where the ground drops away; it joins no body.
    Brink,
}

/// A chart end that joins a declared body, or a brink.
#[derive(Clone, Debug, PartialEq)]
pub struct RiverEnd {
    /// The joined body's declared name; empty for a brink.
    pub body: String,
    /// End segment endpoints in world x/z.
    pub segment: [[f64; 2]; 2],
    /// Join length inside the chart, metres.
    pub length: f64,
    /// Source end, mouth or brink.
    pub kind: EndKind,
}

impl RiverFile {
    /// Decode a river file and check that it was solved from the inputs whose key is `expected`.
    /// # Errors
    /// Refuses, naming the river file, a stale key, a foreign or newer layout, truncated or
    /// trailing bytes, and nonfinite or inconsistent values.
    pub fn read(bytes: &[u8], expected: &[u8; 32]) -> Result<Self, WaterError> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(8)? != RIVER_MAGIC {
            return Err(fail("not a river file"));
        }
        if r.u32()? != RIVER_FORMAT {
            return Err(fail("unsupported river file format"));
        }
        let key: [u8; 32] = r.take(32)?.try_into().map_err(|_| fail("truncated"))?;
        if &key != expected {
            return Err(fail("river file key does not match its inputs"));
        }
        let gauge_level = r.f64()?;
        let count = r.u32()? as usize;
        if count > (bytes.len() - r.at) / 8 {
            return Err(fail("truncated"));
        }
        let mut gauges = vec![gauge_level];
        for _ in 0..count {
            gauges.push(r.f64()?);
        }
        if gauges.windows(2).any(|w| w[1] <= w[0]) {
            return Err(fail("gauge levels do not rise"));
        }
        let count = r.u32()? as usize;
        let mut reaches = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            reaches.push(r.reach(&gauges)?);
        }
        let falls = r.falls()?;
        let mut upper = Vec::with_capacity(gauges.len() - 1);
        for &gauge_level in &gauges[1..] {
            let falls = r.falls()?;
            let count = r.u32()? as usize;
            if count > (bytes.len() - r.at) / 16 {
                return Err(fail("truncated"));
            }
            let mut lip_wetting = Vec::with_capacity(count);
            for _ in 0..count {
                lip_wetting.push((r.u64()?, r.f64()?));
            }
            upper.push(UpperSample {
                gauge_level,
                falls,
                lip_wetting,
            });
        }
        if r.at != bytes.len() {
            return Err(fail("trailing bytes"));
        }
        Ok(Self {
            key,
            gauge_level,
            reaches,
            falls,
            upper,
        })
    }
}

impl RiverReach {
    /// The reach as a chart on the map's bed lattice, its ends naming bodies by `body`.
    /// Cells outside the rectangle are outside the footprint, held the dry margin under the bed.
    /// # Errors
    /// Refuses a rectangle off the bed's lattice or outside it, and an end naming an unknown body.
    pub fn chart(
        &self,
        bed: &Bed,
        body: impl Fn(&str) -> Option<usize>,
    ) -> Result<Chart, WaterError> {
        let (ox, oz) = self.offset(bed)?;
        let n = bed.heights.len();
        let mut chart = Chart {
            levels: bed.heights.iter().map(|h| h - self.dry_margin).collect(),
            currents: vec![[0.0; 2]; n],
            footprint: vec![false; n],
            ends: Vec::with_capacity(self.ends.len()),
        };
        for z in 0..self.size[1] {
            for x in 0..self.size[0] {
                let i = z * self.size[0] + x;
                let j = (z + oz) * bed.size[0] + x + ox;
                chart.levels[j] = self.levels[i];
                chart.currents[j] = self.currents[i];
                chart.footprint[j] = self.footprint[i];
            }
        }
        for end in &self.ends {
            let body = if end.kind == EndKind::Brink {
                BRINK
            } else {
                body(&end.body)
                    .ok_or_else(|| WaterError::new(&self.name, "end names an unknown body"))?
            };
            chart.ends.push(End {
                body,
                segment: end.segment,
                length: end.length,
                mouth: end.kind != EndKind::Source,
            });
        }
        Ok(chart)
    }
}

impl RiverReach {
    /// Where the rectangle's first cell lies on the bed's lattice.
    fn offset(&self, bed: &Bed) -> Result<(usize, usize), WaterError> {
        let off = |axis: usize| {
            let k = (self.origin[axis] - bed.origin[axis]) / bed.spacing;
            let r = k.round();
            ((r - k).abs() < 1e-6 && r >= 0.0).then_some(r)
        };
        let (Some(ox), Some(oz)) = (off(0), off(1)) else {
            return Err(WaterError::new(
                &self.name,
                "reach is off the bed's lattice",
            ));
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked whole and non-negative"
        )]
        let (ox, oz) = (ox as usize, oz as usize);
        if (self.spacing - bed.spacing).abs() > 1e-9 * bed.spacing
            || ox + self.size[0] > bed.size[0]
            || oz + self.size[1] > bed.size[1]
        {
            return Err(WaterError::new(&self.name, "reach is outside the bed"));
        }
        Ok((ox, oz))
    }

    /// The reach's samples on the bed's lattice, as the map's river takes them. Cells outside
    /// the rectangle are held the dry margin under the bed, with no current, loss, gain or
    /// wetting gauge.
    /// # Errors
    /// Refuses a rectangle off the bed's lattice or outside it.
    pub fn samples(&self, bed: &Bed, body: usize) -> Result<ChartSamples, WaterError> {
        let (ox, oz) = self.offset(bed)?;
        let n = bed.heights.len();
        let place = |levels: &[f64], currents: &[[f64; 2]], loss: &[f64], gains: &[[f64; 2]]| {
            let mut s = FlowSample {
                levels: bed.heights.iter().map(|h| h - self.dry_margin).collect(),
                currents: vec![[0.0; 2]; n],
                loss: vec![0.0; n],
                gains: vec![[0.0; 2]; n],
            };
            for z in 0..self.size[1] {
                for x in 0..self.size[0] {
                    let i = z * self.size[0] + x;
                    let j = (z + oz) * bed.size[0] + x + ox;
                    s.levels[j] = levels[i];
                    s.currents[j] = currents[i];
                    s.loss[j] = loss[i];
                    s.gains[j] = gains[i];
                }
            }
            s
        };
        let mut samples = vec![place(&self.levels, &self.currents, &self.loss, &self.gains)];
        let mut wetting = Vec::with_capacity(self.upper.len());
        for u in &self.upper {
            samples.push(place(&u.levels, &u.currents, &u.loss, &u.gains));
            let mut w = vec![None; n];
            for z in 0..self.size[1] {
                for x in 0..self.size[0] {
                    w[(z + oz) * bed.size[0] + x + ox] = u.wetting[z * self.size[0] + x];
                }
            }
            wetting.push(w);
        }
        Ok(ChartSamples {
            body,
            samples,
            wetting,
        })
    }
}

impl RiverFile {
    /// Each sample's gauge level, rising.
    pub fn gauges(&self) -> Vec<f64> {
        std::iter::once(self.gauge_level)
            .chain(self.upper.iter().map(|u| u.gauge_level))
            .collect()
    }

    /// The range of the river's gauge: its lowest and highest sample's gauge levels.
    pub fn gauge_range(&self) -> [f64; 2] {
        let g = self.gauges();
        [g[0], g[g.len() - 1]]
    }

    /// The river as the map takes it, `bodies[k]` being the chart body declared from reach `k`.
    /// # Errors
    /// Refuses a body count that is not the reach count, and a reach off the bed.
    pub fn river(
        &self,
        bed: &Bed,
        name: &str,
        bodies: &[usize],
        gauge: Gauge,
    ) -> Result<River, WaterError> {
        if bodies.len() != self.reaches.len() {
            return Err(WaterError::new(name, "one body is needed for each reach"));
        }
        Ok(River {
            name: name.into(),
            gauges: self.gauges(),
            gauge,
            charts: self
                .reaches
                .iter()
                .zip(bodies)
                .map(|(r, &b)| r.samples(bed, b))
                .collect::<Result<_, _>>()?,
        })
    }

    /// The river's falls at a gauge: each the mix of its records in the two samples that
    /// bracket the gauge, by [`mix_fall`]. A fall that runs in neither is left out.
    pub fn falls_at(&self, gauge: f64) -> Vec<Fall> {
        falls_at(&self.falls, &self.upper, &self.gauges(), gauge)
    }

    /// Copy the fall records without the reach grids for runtime preparation.
    pub fn fall_samples(&self) -> FallSamples {
        FallSamples {
            lowest: self.falls.clone(),
            upper: self.upper.clone(),
            gauges: self.gauges(),
        }
    }
}

fn falls_at(lowest: &[Fall], upper: &[UpperSample], gauges: &[f64], gauge: f64) -> Vec<Fall> {
    let at = bracket(gauges, gauge);
    if gauges.len() == 1 {
        return lowest.to_vec();
    }
    let sample = |k: usize| {
        if k == 0 { lowest } else { &upper[k - 1].falls }
    };
    let wetting = &upper[at.lower].lip_wetting;
    let (lower, upper) = (sample(at.lower), sample(at.lower + 1));
    let mut names: Vec<&str> = lower.iter().chain(upper).map(|f| f.name.as_str()).collect();
    names.dedup();
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for name in names {
        if seen.contains(&name) {
            continue;
        }
        seen.push(name);
        let a = lower.iter().find(|f| f.name == name);
        let b = upper.iter().find(|f| f.name == name);
        if let Some(f) = mix_fall(
            a,
            b,
            [gauges[at.lower], gauges[at.lower + 1]],
            at.gauge,
            |id| wetting.iter().find(|w| w.0 == id).map(|w| w.1),
        ) {
            out.push(f);
        }
    }
    out
}

fn fail(reason: &'static str) -> WaterError {
    WaterError::new("river file", reason)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], WaterError> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len());
        let end = end.ok_or_else(|| fail("truncated"))?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, WaterError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| fail("truncated"))?,
        ))
    }
    fn u64(&mut self) -> Result<u64, WaterError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| fail("truncated"))?,
        ))
    }
    fn u8(&mut self) -> Result<u8, WaterError> {
        Ok(self.take(1)?[0])
    }
    fn f64(&mut self) -> Result<f64, WaterError> {
        let v = f64::from_le_bytes(self.take(8)?.try_into().map_err(|_| fail("truncated"))?);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(fail("nonfinite value"))
        }
    }
    fn flag(&mut self) -> Result<bool, WaterError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(fail("invalid flag")),
        }
    }
    fn name(&mut self) -> Result<String, WaterError> {
        let n = self.u32()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| fail("invalid name"))
    }
    fn falls(&mut self) -> Result<Vec<Fall>, WaterError> {
        let count = self.u32()? as usize;
        let mut falls = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            falls.push(self.fall()?);
        }
        Ok(falls)
    }
    fn cell(&mut self) -> Result<(f64, [f64; 2], f64, [f64; 2]), WaterError> {
        let level = self.f64()?;
        let current = [self.f64()?, self.f64()?];
        let loss = self.f64()?;
        let gains = [self.f64()?, self.f64()?];
        if loss < 0.0 || gains.iter().any(|&g| g < 0.0) {
            return Err(fail("negative loss or gain"));
        }
        Ok((level, current, loss, gains))
    }
    fn reach(&mut self, gauges: &[f64]) -> Result<RiverReach, WaterError> {
        let name = self.name()?;
        let origin = [self.f64()?, self.f64()?];
        let spacing = self.f64()?;
        let size = [self.u32()? as usize, self.u32()? as usize];
        let dry_margin = self.f64()?;
        let cells = size[0]
            .checked_mul(size[1])
            .filter(|&c| c > 0 && c <= (self.bytes.len() - self.at) / 49)
            .ok_or_else(|| fail("invalid reach size"))?;
        if spacing <= 0.0 || dry_margin < 0.0 {
            return Err(fail("invalid reach spacing or margin"));
        }
        let mut reach = RiverReach {
            name,
            origin,
            spacing,
            size,
            dry_margin,
            levels: Vec::with_capacity(cells),
            currents: Vec::with_capacity(cells),
            loss: Vec::with_capacity(cells),
            gains: Vec::with_capacity(cells),
            footprint: Vec::with_capacity(cells),
            upper: Vec::with_capacity(gauges.len() - 1),
            ends: Vec::new(),
        };
        for _ in 0..cells {
            let (level, current, loss, gains) = self.cell()?;
            reach.levels.push(level);
            reach.currents.push(current);
            reach.loss.push(loss);
            reach.gains.push(gains);
            reach.footprint.push(self.flag()?);
        }
        for k in 1..gauges.len() {
            if cells > (self.bytes.len() - self.at) / 49 {
                return Err(fail("truncated"));
            }
            let mut sample = ReachSample {
                levels: Vec::with_capacity(cells),
                currents: Vec::with_capacity(cells),
                loss: Vec::with_capacity(cells),
                gains: Vec::with_capacity(cells),
                wetting: Vec::with_capacity(cells),
            };
            for _ in 0..cells {
                let (level, current, loss, gains) = self.cell()?;
                sample.levels.push(level);
                sample.currents.push(current);
                sample.loss.push(loss);
                sample.gains.push(gains);
                let wetting = if self.flag()? {
                    let g = self.f64()?;
                    if g < gauges[k - 1] || g > gauges[k] {
                        return Err(fail("wetting gauge outside its samples"));
                    }
                    Some(g)
                } else {
                    None
                };
                sample.wetting.push(wetting);
            }
            reach.upper.push(sample);
        }
        let ends = self.u32()? as usize;
        for _ in 0..ends {
            reach.ends.push(RiverEnd {
                body: self.name()?,
                segment: [[self.f64()?, self.f64()?], [self.f64()?, self.f64()?]],
                length: self.f64()?,
                kind: match self.u8()? {
                    0 => EndKind::Source,
                    1 => EndKind::Mouth,
                    2 => EndKind::Brink,
                    _ => return Err(fail("invalid end kind")),
                },
            });
        }
        Ok(reach)
    }
    fn fall(&mut self) -> Result<Fall, WaterError> {
        let name = self.name()?;
        let above = self.name()?;
        let count = self.u32()? as usize;
        let mut segments = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let id = self.u64()?;
            let lip = [[self.f64()?, self.f64()?], [self.f64()?, self.f64()?]];
            let (discharge, launch_depth, launch_speed) = (self.f64()?, self.f64()?, self.f64()?);
            let below = match self.u8()? {
                0 => Below::Body(self.name()?),
                1 => Below::Reach(self.name()?),
                _ => return Err(fail("invalid fall end")),
            };
            let points = self.u32()? as usize;
            if points > (self.bytes.len() - self.at) / 40 {
                return Err(fail("truncated"));
            }
            let mut path = Vec::with_capacity(points);
            for _ in 0..points {
                path.push(PathPoint {
                    place: [self.f64()?, self.f64()?],
                    height: self.f64()?,
                    time: self.f64()?,
                    distance: self.f64()?,
                });
            }
            let segment = FallSegment {
                id,
                lip,
                discharge,
                launch_depth,
                launch_speed,
                below,
                path,
            };
            segment.validate()?;
            segments.push(segment);
        }
        Ok(Fall {
            name,
            above,
            segments,
        })
    }
}
