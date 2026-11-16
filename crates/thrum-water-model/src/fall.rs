//! A fall's record, the f64 reference for its sheet, and its plunge.
//!
//! A flow solve writes a fall record where the water leaves a brink: along the lip, one segment
//! for each face of the brink, with the water's path from the lip and the time it takes, its
//! discharge for each metre of lip and its launch. The sheet down that path follows from those
//! alone: its speed from energy, `v = sqrt(v0^2 + 2 g z)` at a drop `z`; its thickness, the
//! discharge over the speed; its white share, from 0 at the lip to 1 at the breakup length
//! `L = 6 q^0.32` m of path; and its presence, its thickness over the tear thickness, at most 1.
//!
//! Where a path meets the water below, the plunge gives births, at the fall's power `rho g q z`
//! for each metre of lip, split between foam and submerged air, and an impulse a tick: the wave
//! share of the momentum flux into the surface, times the sheet's own pattern arriving there with
//! its mean over the pattern's loop removed and its variance made 1. A path that meets no water
//! gives neither.
use crate::WaterError;
use thrum_sea_model::LOOP_SECONDS;

/// Gravity, m/s².
pub const GRAVITY: f64 = 9.81;
/// Density of water, kg/m³.
pub const WATER_DENSITY: f64 = 1000.0;

/// Where a fall's path ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Below {
    /// In a declared body's water, at the low end of its stage's range.
    Body(String),
    /// On ground gentler than the steepest chart slope, where the named reach takes the water.
    Reach(String),
}

/// A point of a fall's path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    /// World x/z, metres.
    pub place: [f64; 2],
    /// Height, metres.
    pub height: f64,
    /// Time since the water left the lip, seconds.
    pub time: f64,
    /// Length of path from the lip, metres.
    pub distance: f64,
}

/// One face of a fall's lip and the water's path from it.
#[derive(Clone, Debug, PartialEq)]
pub struct FallSegment {
    /// Stable ID, the same for every flow sample and every solve of the same bed.
    pub id: u64,
    /// The face's two ends along the lip, world x/z.
    pub lip: [[f64; 2]; 2],
    /// Discharge for each metre of lip, m²/s.
    pub discharge: f64,
    /// Depth of the water as it leaves the lip, metres.
    pub launch_depth: f64,
    /// Speed of the water as it leaves the lip, m/s: the discharge over the launch depth.
    pub launch_speed: f64,
    /// Where the path ends.
    pub below: Below,
    /// The path from the lip, at least two points, in time order.
    pub path: Vec<PathPoint>,
}

/// One fall: a run of brink faces that water leaves.
#[derive(Clone, Debug, PartialEq)]
pub struct Fall {
    /// The fall's name, for diagnostics.
    pub name: String,
    /// The name of the reach the water leaves.
    pub above: String,
    /// Its segments along the lip.
    pub segments: Vec<FallSegment>,
}

/// The sheet's physical laws. Never a quality setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetProfile {
    /// Breakup length's coefficient: `L = breakup q^exponent` metres of path.
    pub breakup: f64,
    /// Breakup length's exponent on the discharge for each metre of lip.
    pub breakup_exponent: f64,
    /// Thickness under which a sheet is strands with gaps, metres.
    pub tear: f64,
}

impl Default for SheetProfile {
    /// The first profile: Horeni's breakup length and a 5 mm tear.
    fn default() -> Self {
        Self {
            breakup: 6.0,
            breakup_exponent: 0.32,
            tear: 0.005,
        }
    }
}

/// The sheet at a point of its path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetSample {
    /// Speed, m/s.
    pub speed: f64,
    /// Thickness, metres.
    pub thickness: f64,
    /// White share, 0 at the lip to 1 at the breakup length.
    pub white: f64,
    /// Presence, the thickness over the tear thickness, at most 1.
    pub presence: f64,
}

impl SheetProfile {
    /// Breakup length for a discharge of `q` m²/s for each metre of lip, metres of path.
    pub fn breakup_length(&self, q: f64) -> f64 {
        self.breakup * q.max(0.0).powf(self.breakup_exponent)
    }

    /// The sheet of `q` m²/s for each metre of lip, launched at `v0` m/s, at a drop of `z`
    /// metres below the lip and `s` metres of path from it.
    pub fn sample(&self, q: f64, v0: f64, z: f64, s: f64) -> SheetSample {
        let speed = (v0 * v0 + 2.0 * GRAVITY * z.max(0.0)).sqrt();
        let thickness = if speed > 0.0 { q / speed } else { 0.0 };
        let length = self.breakup_length(q);
        let white = if length > 0.0 {
            smooth(s / length)
        } else {
            1.0
        };
        SheetSample {
            speed,
            thickness,
            white,
            presence: (thickness / self.tear).min(1.0),
        }
    }
}

/// The plunge's laws and the tick it is stepped at. Never a quality setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlungeProfile {
    /// Share of a plunge's births that is submerged air; the rest is foam.
    pub air_share: f64,
    /// Share of the momentum flux's unsteady part that makes waves (`alpha`).
    pub wave_share: f64,
    /// The interaction step's tick, seconds.
    pub tick: f64,
    /// The foam's cell, metres: a birth's footprint is never narrower.
    pub foam_cell: f64,
}

impl Default for PlungeProfile {
    /// The first profile: half the births as air, a wave share of 0.1, a 60 Hz tick and
    /// `WATERS`' 0.25 m foam cells.
    fn default() -> Self {
        Self {
            air_share: 0.5,
            wave_share: 0.1,
            tick: 1.0 / 60.0,
            foam_cell: 0.25,
        }
    }
}

/// Where a segment's path meets the water below.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plunge {
    /// The segment's stable ID.
    pub segment: u64,
    /// World x/z, metres.
    pub place: [f64; 2],
    /// The water's level there, metres.
    pub level: f64,
    /// Time from the lip, seconds.
    pub time: f64,
    /// Drop from the lip, metres.
    pub drop: f64,
    /// Length of path from the lip, metres.
    pub distance: f64,
    /// The sheet arriving.
    pub sheet: SheetSample,
    /// Speed into the surface, m/s.
    pub normal_speed: f64,
}

/// What a plunge gives the foam each tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Births {
    /// The segment's stable ID.
    pub segment: u64,
    /// World x/z of the footprint's centre.
    pub place: [f64; 2],
    /// Footprint across the lip, metres: the segment's width.
    pub width: f64,
    /// Footprint along the path: the sheet's thickness, and never under the foam's cell, metres.
    pub length: f64,
    /// Power into foam, watts.
    pub foam: f64,
    /// Power into submerged air, watts.
    pub air: f64,
}

/// An impulse record, as `WATERS`' interaction step takes one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impulse {
    /// The segment's stable ID.
    pub id: u64,
    /// World x/z, metres.
    pub place: [f64; 2],
    /// Vertical impulse over the tick, N·s, downward.
    pub vertical: f64,
    /// Radius of its footprint, metres.
    pub radius: f64,
    /// The tick it is for.
    pub tick: u64,
}

/// The sheet's pattern: `WATERS`' foam pattern, which the game supplies, read at a place along
/// the lip and a distance down the sheet.
pub trait SheetPattern {
    /// The pattern's value at `along` metres along the lip and `down` metres down the sheet.
    fn value(&self, along: f64, down: f64) -> f64;
}

impl<F: Fn(f64, f64) -> f64> SheetPattern for F {
    fn value(&self, along: f64, down: f64) -> f64 {
        self(along, down)
    }
}

/// A segment's pattern over the cascades' loop: its mean and standard deviation on the ticks
/// of one loop, worked out once, so a tick's impulse costs one read of the pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternLoop {
    /// Mean over the loop.
    pub mean: f64,
    /// Standard deviation over the loop.
    pub deviation: f64,
}

impl FallSegment {
    /// The segment's width along the lip, metres.
    pub fn width(&self) -> f64 {
        let [a, b] = self.lip;
        (b[0] - a[0]).hypot(b[1] - a[1])
    }

    /// The middle of the segment's face, world x/z.
    pub fn centre(&self) -> [f64; 2] {
        let [a, b] = self.lip;
        [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])]
    }

    /// The place along the whole lip at which this segment reads the pattern: the distance of
    /// its centre from the world origin along the lip's direction.
    fn along(&self) -> f64 {
        let [a, b] = self.lip;
        let w = self.width();
        let c = self.centre();
        if w > 0.0 {
            (c[0] * (b[0] - a[0]) + c[1] * (b[1] - a[1])) / w
        } else {
            0.0
        }
    }

    /// The sheet at path point `k`.
    pub fn sheet(&self, profile: &SheetProfile, k: usize) -> SheetSample {
        let p = self.path[k];
        profile.sample(
            self.discharge,
            self.launch_speed,
            self.path[0].height - p.height,
            p.distance,
        )
    }

    /// Where the path first meets the water below, if it does. `level` gives the water's level
    /// at a world point, or none where there is no water; the lip's own water is not asked.
    pub fn plunge(
        &self,
        profile: &SheetProfile,
        level: impl Fn([f64; 2]) -> Option<f64>,
    ) -> Option<Plunge> {
        let above = |p: &PathPoint| level(p.place).map(|l| p.height - l);
        let mut last: Option<(usize, f64)> = None;
        for k in 1..self.path.len() {
            let Some(gap) = above(&self.path[k]) else {
                last = None;
                continue;
            };
            if gap <= 0.0 {
                // Between the last point over water and this one, on the chord, where the
                // height meets the level.
                let (a, b) = (self.path[k - 1], self.path[k]);
                let f = match last {
                    Some((j, g)) if j == k - 1 && g > 0.0 => g / (g - gap),
                    _ => 1.0,
                };
                let lerp = |x: f64, y: f64| x + (y - x) * f;
                let place = [lerp(a.place[0], b.place[0]), lerp(a.place[1], b.place[1])];
                let height = lerp(a.height, b.height);
                let dt = b.time - a.time;
                let horizontal = (b.place[0] - a.place[0]).hypot(b.place[1] - a.place[1]);
                let drop = self.path[0].height - height;
                let distance = lerp(a.distance, b.distance);
                let sheet = profile.sample(self.discharge, self.launch_speed, drop, distance);
                let along = if dt > 0.0 { horizontal / dt } else { 0.0 };
                return Some(Plunge {
                    segment: self.id,
                    place,
                    level: height,
                    time: lerp(a.time, b.time),
                    drop,
                    distance,
                    sheet,
                    normal_speed: (sheet.speed * sheet.speed - along * along).max(0.0).sqrt(),
                });
            }
            last = Some((k, gap));
        }
        None
    }

    /// The segment's births each tick at a plunge: the fall's power, `rho g q z` for each metre
    /// of lip over its width, split between foam and submerged air.
    pub fn births(&self, profile: &PlungeProfile, plunge: &Plunge) -> Births {
        let power = WATER_DENSITY * GRAVITY * self.discharge * plunge.drop.max(0.0) * self.width();
        Births {
            segment: self.id,
            place: plunge.place,
            width: self.width(),
            length: plunge.sheet.thickness.max(profile.foam_cell),
            foam: power * (1.0 - profile.air_share),
            air: power * profile.air_share,
        }
    }

    /// The pattern's mean and deviation for this segment over the ticks of one loop.
    pub fn pattern_loop(
        &self,
        profile: &PlungeProfile,
        pattern: &impl SheetPattern,
    ) -> PatternLoop {
        let ticks = loop_ticks(profile.tick);
        let along = self.along();
        let mut sum = 0.0;
        let mut squares = 0.0;
        for n in 0..ticks {
            let v = pattern.value(along, self.down(profile, n, 0.0));
            sum += v;
            squares += v * v;
        }
        let count = ticks as f64;
        let mean = sum / count;
        PatternLoop {
            mean,
            deviation: (squares / count - mean * mean).max(0.0).sqrt(),
        }
    }

    /// Distance down the sheet of the pattern arriving `time` seconds from the lip at tick `n`:
    /// it left the lip at `n tick - time`, on the cascades' loop, and moves at the launch speed.
    fn down(&self, profile: &PlungeProfile, n: u64, time: f64) -> f64 {
        let age = (n as f64 * profile.tick - time).rem_euclid(LOOP_SECONDS);
        age * self.launch_speed
    }

    /// The pattern arriving at the plunge at tick `n`, with the loop's mean removed and its
    /// variance made 1; zero where the pattern is flat.
    pub fn arriving(
        &self,
        profile: &PlungeProfile,
        pattern: &impl SheetPattern,
        stats: &PatternLoop,
        plunge: &Plunge,
        n: u64,
    ) -> f64 {
        if stats.deviation <= 0.0 {
            return 0.0;
        }
        (pattern.value(self.along(), self.down(profile, n, plunge.time)) - stats.mean)
            / stats.deviation
    }

    /// The segment's impulse at tick `n`: `alpha rho q v_n w dt zeta`, over a footprint as wide
    /// as the segment.
    pub fn impulse(
        &self,
        profile: &PlungeProfile,
        pattern: &impl SheetPattern,
        stats: &PatternLoop,
        plunge: &Plunge,
        n: u64,
    ) -> Impulse {
        let zeta = self.arriving(profile, pattern, stats, plunge, n);
        Impulse {
            id: self.id,
            place: plunge.place,
            vertical: self.impulse_rms(profile, plunge) * zeta,
            radius: 0.5 * self.width().max(profile.foam_cell),
            tick: n,
        }
    }

    /// The impulse's root mean square over the loop, as the design gives it:
    /// `alpha rho q v_n w dt`.
    pub fn impulse_rms(&self, profile: &PlungeProfile, plunge: &Plunge) -> f64 {
        profile.wave_share
            * WATER_DENSITY
            * self.discharge
            * plunge.normal_speed
            * self.width()
            * profile.tick
    }

    pub(crate) fn validate(&self) -> Result<(), WaterError> {
        let bad = |v: f64| !v.is_finite();
        let fail = |reason| WaterError::new("fall record", reason);
        if self.lip.iter().flatten().any(|&v| bad(v))
            || self.width() <= 0.0
            || bad(self.discharge)
            || self.discharge < 0.0
            || bad(self.launch_depth)
            || self.launch_depth < 0.0
            || bad(self.launch_speed)
            || self.launch_speed < 0.0
        {
            return Err(fail("invalid lip or launch"));
        }
        if self.path.len() < 2 {
            return Err(fail("path has under two points"));
        }
        for w in self.path.windows(2) {
            if w[1].time < w[0].time || w[1].distance < w[0].distance {
                return Err(fail("path is not in time order"));
            }
        }
        Ok(())
    }
}

/// Ticks in one of the cascades' loops.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a positive tick gives a bounded whole count of ticks in the loop"
)]
pub fn loop_ticks(tick: f64) -> u64 {
    ((LOOP_SECONDS / tick).round() as u64).max(1)
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
