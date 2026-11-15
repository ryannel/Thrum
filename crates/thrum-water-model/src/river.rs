//! The river file's reader. A river file holds what one steady flow solve wrote, so a game can
//! prepare solved rivers without linking the solver. `thrum-river-model` writes the same layout.
//!
//! Layout, little-endian throughout:
//! `b"THRUMRIV"`, format `u32`, key `[u8; 32]`, gauge level `f64`, reach count `u32`, then for
//! each reach: name (`u32` length and UTF-8), origin `2 × f64`, spacing `f64`, size `2 × u32`,
//! dry margin `f64`, then for each of its cells, row-major x-fastest: level `f64`, current
//! `2 × f64`, loss `f64` and footprint `u8`; then an end count `u32` and for each end: body name,
//! segment `4 × f64`, join length `f64` and mouth `u8`. Nothing follows the last reach.
use crate::{Bed, Chart, End, WaterError};

/// The magic bytes every river file starts with.
pub const RIVER_MAGIC: [u8; 8] = *b"THRUMRIV";
/// The layout version this reader accepts.
pub const RIVER_FORMAT: u32 = 1;

/// A solve's stored output and the key of the inputs it was solved from.
#[derive(Clone, Debug, PartialEq)]
pub struct RiverFile {
    /// Hash of the bed inside the rectangle, the declaration, the solve's profile and the solver's version.
    pub key: [u8; 32],
    /// The level at the river's source end for this flow sample, metres.
    pub gauge_level: f64,
    /// One chart for each reach.
    pub reaches: Vec<RiverReach>,
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
    /// Cells that belong to the chart's footprint.
    pub footprint: Vec<bool>,
    /// Source ends and mouths, by the name of the body each joins.
    pub ends: Vec<RiverEnd>,
}

/// A chart end that joins a declared body.
#[derive(Clone, Debug, PartialEq)]
pub struct RiverEnd {
    /// The joined body's declared name.
    pub body: String,
    /// End segment endpoints in world x/z.
    pub segment: [[f64; 2]; 2],
    /// Join length inside the chart, metres.
    pub length: f64,
    /// Whether the end is a mouth (water leaves) rather than a source end.
    pub mouth: bool,
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
        let mut reaches = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            reaches.push(r.reach()?);
        }
        if r.at != bytes.len() {
            return Err(fail("trailing bytes"));
        }
        Ok(Self {
            key,
            gauge_level,
            reaches,
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
            chart.ends.push(End {
                body: body(&end.body)
                    .ok_or_else(|| WaterError::new(&self.name, "end names an unknown body"))?,
                segment: end.segment,
                length: end.length,
                mouth: end.mouth,
            });
        }
        Ok(chart)
    }
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
    fn reach(&mut self) -> Result<RiverReach, WaterError> {
        let name = self.name()?;
        let origin = [self.f64()?, self.f64()?];
        let spacing = self.f64()?;
        let size = [self.u32()? as usize, self.u32()? as usize];
        let dry_margin = self.f64()?;
        let cells = size[0]
            .checked_mul(size[1])
            .filter(|&c| c > 0 && c <= (self.bytes.len() - self.at) / 33)
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
            footprint: Vec::with_capacity(cells),
            ends: Vec::new(),
        };
        for _ in 0..cells {
            reach.levels.push(self.f64()?);
            reach.currents.push([self.f64()?, self.f64()?]);
            reach.loss.push(self.f64()?);
            reach.footprint.push(self.flag()?);
        }
        let ends = self.u32()? as usize;
        for _ in 0..ends {
            reach.ends.push(RiverEnd {
                body: self.name()?,
                segment: [[self.f64()?, self.f64()?], [self.f64()?, self.f64()?]],
                length: self.f64()?,
                mouth: self.flag()?,
            });
        }
        Ok(reach)
    }
}
