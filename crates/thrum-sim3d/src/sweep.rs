use thrum_core::profile::Number;

use crate::{
    QueryError, Shape, Vector3, n,
    pair::{Pair, pair},
    shape::validate_motion_end,
};

/// Maximum length of either motion or their difference for a half-gap sweep.
pub const MAX_HALF_GAP_MOTION: i32 = 4_096;

/// The denominator of the contact tolerance, expressed as its exact reciprocal.
pub const CONTACT_TOLERANCE_DENOMINATOR: i32 = 128;

/// Gap to stop within in a collision sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepGap {
    /// The contact tolerance.
    Tolerance,
    /// Half the contact tolerance, used by the kinematic move.
    HalfTolerance,
}

impl SweepGap {
    /// The requested gap in world units.
    pub fn units(self) -> Number {
        let denominator = match self {
            Self::Tolerance => CONTACT_TOLERANCE_DENOMINATOR,
            Self::HalfTolerance => CONTACT_TOLERANCE_DENOMINATOR * 2,
        };
        Number::ONE / n(denominator)
    }
}

/// Where the mover first stands within the requested gap of the struck shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepHit {
    /// Fraction of the supplied motions from zero through one.
    pub fraction: Number,
    /// World-space point on the struck shape at the reported fraction.
    pub point: Vector3,
    /// Unit direction from the struck shape toward the mover.
    pub normal: Vector3,
    /// Initial depth when it exceeds the contact tolerance, or zero.
    pub depth: Number,
}

/// Sweep the mover along a motion against a stationary shape.
///
/// # Errors
/// Returns the input fault for invalid shapes or motion.
pub fn sweep(
    mover: Shape,
    motion: Vector3,
    struck: Shape,
    gap: SweepGap,
) -> Result<Option<SweepHit>, QueryError> {
    sweep_two(mover, motion, struck, Vector3::ZERO, gap)
}

/// Sweep the mover and the struck shape along their respective motions.
///
/// Both shapes are placed using their own motions, and the returned point is
/// on the struck shape at that fraction. Where a patch is equally near the
/// mover, the first witness in stored vertex order, then edge order, wins.
///
/// # Errors
/// Returns the input fault for invalid shapes or either motion.
pub fn sweep_two(
    mover: Shape,
    motion: Vector3,
    struck: Shape,
    struck_motion: Vector3,
    gap: SweepGap,
) -> Result<Option<SweepHit>, QueryError> {
    mover.validate()?;
    struck.validate()?;
    if matches!(mover, Shape::Triangle { .. }) {
        return Err(QueryError::TriangleMover);
    }
    if !motion.is_finite() || !struck_motion.is_finite() {
        return Err(QueryError::NonFinite);
    }
    validate_motion_end(mover.translated(motion).validate())?;
    validate_motion_end(struck.translated(struck_motion).validate())?;
    let relative_motion = motion - struck_motion;
    if gap == SweepGap::HalfTolerance
        && [motion, struck_motion, relative_motion]
            .into_iter()
            .any(|motion| motion.length() > n(MAX_HALF_GAP_MOTION))
    {
        return Err(QueryError::HalfGapMotionTooLong);
    }

    let search = Search {
        mover,
        motion,
        struck,
        struck_motion,
        relative_motion,
        gap: gap.units(),
    };
    Ok(search.find())
}

struct Search {
    mover: Shape,
    motion: Vector3,
    struck: Shape,
    struck_motion: Vector3,
    relative_motion: Vector3,
    gap: Number,
}

struct Sample {
    fraction: Number,
    first: Shape,
    second: Shape,
    contact: Pair,
}

impl Sample {
    fn hit(self) -> SweepHit {
        let depth = if self.contact.depth > SweepGap::Tolerance.units() {
            self.contact.depth
        } else {
            Number::ZERO
        };
        SweepHit {
            fraction: self.fraction,
            point: self.contact.surface(self.first, self.second),
            normal: self.contact.normal,
            depth,
        }
    }
}

impl Search {
    fn sample(&self, fraction: Number) -> Sample {
        // Place both shapes exactly as a caller does before testing the gap.
        let first = self.mover.translated(self.motion * fraction);
        let second = self.struck.translated(self.struck_motion * fraction);
        let contact = pair(first, second);
        Sample {
            fraction,
            first,
            second,
            contact,
        }
    }

    fn closing(&self, sample: &Sample) -> Number {
        -sample.contact.normal.dot(self.relative_motion)
    }

    fn find(&self) -> Option<SweepHit> {
        let mut clear = self.sample(Number::ZERO);
        if clear.contact.gap <= self.gap {
            return Some(clear.hit());
        }

        for _ in 0..12 {
            let speed = self.closing(&clear);
            if speed <= Number::ZERO {
                return None;
            }
            let next = (clear.fraction + clear.contact.gap / speed).min(Number::ONE);
            if next == clear.fraction {
                break;
            }
            let sample = self.sample(next);
            if sample.contact.gap == Number::ZERO {
                return self.refine(clear.fraction, sample.fraction, None);
            }
            if self.closing(&sample) >= Number::ZERO && sample.contact.gap <= self.gap {
                return Some(sample.hit());
            }
            if self.closing(&sample) <= Number::ZERO {
                return self.find_minimum(clear.fraction, next);
            }
            if next == Number::ONE {
                return None;
            }
            clear = sample;
        }
        self.find_minimum(clear.fraction, Number::ONE)
    }

    fn find_minimum(&self, mut low: Number, mut high: Number) -> Option<SweepHit> {
        // Distance to the convex hull difference along a line decreases to a
        // minimum, then increases. The normal tells which side we are on.
        let end = self.sample(high);
        if end.contact.gap == Number::ZERO {
            return self.refine(low, high, None);
        }
        let mut near = (end.contact.gap <= self.gap).then_some(end);
        for _ in 0..32 {
            let middle = midpoint(low, high);
            if middle == low || middle == high {
                break;
            }
            let sample = self.sample(middle);
            if sample.contact.gap == Number::ZERO {
                return self.refine(low, middle, None);
            }
            if self.closing(&sample) > Number::ZERO && sample.contact.gap > self.gap {
                low = middle;
            } else {
                high = middle;
                if sample.contact.gap <= self.gap {
                    near = Some(sample);
                }
            }
        }
        near.map(Sample::hit)
    }

    fn refine(
        &self,
        mut low: Number,
        mut high: Number,
        mut near: Option<Sample>,
    ) -> Option<SweepHit> {
        // A reported hit is always a tested clear sample. The upper endpoint
        // is already overlapping and rules out any witness on the far side.
        for _ in 0..32 {
            let middle = midpoint(low, high);
            if middle == low || middle == high {
                break;
            }
            let sample = self.sample(middle);
            if sample.contact.gap > self.gap {
                low = middle;
            } else {
                high = middle;
                if sample.contact.gap > Number::ZERO {
                    near = Some(sample);
                }
            }
        }
        near.map(Sample::hit)
    }
}

fn midpoint(low: Number, high: Number) -> Number {
    low + (high - low) / n(2)
}
