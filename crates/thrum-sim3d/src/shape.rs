use std::fmt;

use thrum_core::profile::{Number, StateEncode};

use crate::{Vector3, n, predicates::sum};

/// Inclusive bound of the query domain on each axis, in world units.
pub const DOMAIN_BOUND: i32 = 32_768;

/// Shapes accepted by the pair tests. A triangle is a two-sided surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A centre and a nonnegative radius.
    Sphere {
        /// Centre in world coordinates.
        center: Vector3,
        /// Radius in world units.
        radius: Number,
    },
    /// A segment thickened by a nonnegative radius.
    Capsule {
        /// Ends of the segment in world coordinates.
        ends: [Vector3; 2],
        /// Radius in world units.
        radius: Number,
    },
    /// An oriented box with mutually perpendicular nonzero half-edges.
    Cuboid {
        /// Centre in world coordinates.
        center: Vector3,
        /// The three half-edges. A zero edge collapses one dimension.
        half_edges: [Vector3; 3],
    },
    /// A two-sided triangular surface.
    Triangle {
        /// Corners in world coordinates.
        corners: [Vector3; 3],
    },
}

/// Invalid input to a collision query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    /// A coordinate or radius is not finite.
    NonFinite,
    /// A shape or query origin lies outside the domain.
    OutsideDomain,
    /// A radius is negative.
    NegativeRadius,
    /// A motion places an endpoint outside the domain.
    MotionOutsideDomain,
    /// A half-gap sweep exceeds the allowed motion length.
    HalfGapMotionTooLong,
    /// A triangle was supplied as the moving shape.
    TriangleMover,
    /// The nonzero half-edges of a cuboid are not perpendicular.
    SkewedCuboid,
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "collision input is not finite",
            Self::OutsideDomain => "collision input is outside the query domain",
            Self::NegativeRadius => "collision radius is negative",
            Self::MotionOutsideDomain => "collision motion ends outside the query domain",
            Self::HalfGapMotionTooLong => "half-gap sweep exceeds 4,096 units",
            Self::TriangleMover => "a triangle cannot be the mover",
            Self::SkewedCuboid => "cuboid half-edges must be perpendicular",
        })
    }
}

impl std::error::Error for QueryError {}

impl Shape {
    /// Translate the shape using the same coordinate operations as a sweep.
    #[must_use]
    pub fn translated(self, displacement: Vector3) -> Self {
        match self {
            Self::Sphere { center, radius } => Self::Sphere {
                center: center + displacement,
                radius,
            },
            Self::Capsule { ends, radius } => Self::Capsule {
                ends: ends.map(|end| end + displacement),
                radius,
            },
            Self::Cuboid { center, half_edges } => Self::Cuboid {
                center: center + displacement,
                half_edges,
            },
            Self::Triangle { corners } => Self::Triangle {
                corners: corners.map(|corner| corner + displacement),
            },
        }
    }

    pub(crate) fn radius(self) -> Number {
        match self {
            Self::Sphere { radius, .. } | Self::Capsule { radius, .. } => radius,
            _ => Number::ZERO,
        }
    }

    pub(crate) fn reference(self) -> Vector3 {
        match self {
            Self::Sphere { center, .. } | Self::Cuboid { center, .. } => center,
            Self::Capsule { ends, .. } => ends[0],
            Self::Triangle { corners } => corners[0],
        }
    }

    pub(crate) fn validate(self) -> Result<(), QueryError> {
        let radius = self.radius();
        if !radius.is_finite() {
            return Err(QueryError::NonFinite);
        }
        if radius < Number::ZERO {
            return Err(QueryError::NegativeRadius);
        }

        if let Self::Cuboid { half_edges, .. } = self {
            for edge in half_edges {
                validate_point(edge)?;
            }
            let directions = half_edges.map(Vector3::unit);
            for (index, first) in directions.iter().enumerate() {
                for second in directions.iter().skip(index + 1) {
                    if let (Some(first), Some(second)) = (first, second)
                        && first.dot(*second).abs() > Number::ONE / n(1 << 20)
                    {
                        return Err(QueryError::SkewedCuboid);
                    }
                }
            }
        }
        self.validate_bounds(Vector3::ZERO)
    }

    /// Also test the exact endpoint so rounding the translation onto a domain
    /// face cannot hide a coordinate outside it.
    pub(crate) fn validate_motion_end(self, motion: Vector3) -> Result<(), QueryError> {
        if motion == Vector3::ZERO {
            return Ok(());
        }
        if motion.max_component() > n(2 * DOMAIN_BOUND) {
            return Err(QueryError::MotionOutsideDomain);
        }
        self.validate_bounds(motion)
            .and_then(|()| self.translated(motion).validate_bounds(Vector3::ZERO))
            .map_err(|_| QueryError::MotionOutsideDomain)
    }

    fn validate_bounds(self, motion: Vector3) -> Result<(), QueryError> {
        let check = |point, extents| validate_extent(point, motion, extents);
        match self {
            Self::Sphere { center, radius } => {
                check(center, [splat(radius), Vector3::ZERO, Vector3::ZERO])
            }
            Self::Capsule { ends, radius } => {
                for end in ends {
                    check(end, [splat(radius), Vector3::ZERO, Vector3::ZERO])?;
                }
                Ok(())
            }
            Self::Triangle { corners } => {
                for corner in corners {
                    check(corner, [Vector3::ZERO; 3])?;
                }
                Ok(())
            }
            Self::Cuboid { center, half_edges } => check(center, half_edges),
        }
    }
}

fn splat(value: Number) -> Vector3 {
    Vector3::new(value, value, value)
}

fn validate_extent(
    center: Vector3,
    motion: Vector3,
    extents: [Vector3; 3],
) -> Result<(), QueryError> {
    validate_point(center)?;
    let [a, b, c] = extents;
    let bound = n(DOMAIN_BOUND);
    for (coordinate, displacement, edges) in [
        (center.x, motion.x, [a.x, b.x, c.x]),
        (center.y, motion.y, [a.y, b.y, c.y]),
        (center.z, motion.z, [a.z, b.z, c.z]),
    ] {
        let [a, b, c] = edges.map(Number::abs);
        if [a, b, c].into_iter().any(|extent| extent > bound) {
            return Err(QueryError::OutsideDomain);
        }
        for sign in [Number::ONE, -Number::ONE] {
            let at = sign * coordinate;
            let delta = sign * displacement;
            let rounded = (at + delta) + ((a + b) + c);
            // Away from a face the rounded sum already decides the result.
            // Near it, preserve the remainder in the sum of the input words.
            let margin = Number::ONE / n(32);
            if rounded > bound + margin
                || ((rounded - bound).abs() <= margin
                    && sum([at, delta, a, b, c, -bound]) > Number::ZERO)
            {
                return Err(QueryError::OutsideDomain);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_point(point: Vector3) -> Result<(), QueryError> {
    if !point.is_finite() {
        return Err(QueryError::NonFinite);
    }
    if point.max_component() > n(DOMAIN_BOUND) {
        return Err(QueryError::OutsideDomain);
    }
    Ok(())
}
