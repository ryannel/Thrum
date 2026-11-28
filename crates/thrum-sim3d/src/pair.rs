use thrum_core::profile::Number;

use crate::{
    QueryError, Shape, Vector3,
    hull::{Hull, closest, intersection},
};

/// The shallowest translation that parts a pair of overlapping shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Penetration {
    /// Depth in world units.
    pub depth: Number,
    /// Unit direction out of the struck shape, or zero for a disjoint pair.
    pub normal: Vector3,
}

/// Distance between the closest points, or zero for an overlapping pair.
///
/// # Errors
/// Returns the input fault if either shape is invalid.
pub fn distance(first: Shape, second: Shape) -> Result<Number, QueryError> {
    first.validate()?;
    second.validate()?;
    Ok(pair(first, second).gap)
}

/// Whether two shapes touch exactly or cross.
///
/// # Errors
/// Returns the input fault if either shape is invalid.
pub fn overlap(first: Shape, second: Shape) -> Result<bool, QueryError> {
    Ok(distance(first, second)? == Number::ZERO)
}

/// Depth and normal of the shallowest way out, or zeros for a disjoint pair.
///
/// Equal face or edge normals use their first occurrence in the two shapes'
/// edge order, positive cross product before negative. On a line the normal
/// is the cross product with the world axis least aligned to the line (x, y,
/// z on a tie). Coincident points use positive x.
///
/// # Errors
/// Returns the input fault if either shape is invalid.
pub fn depth(first: Shape, second: Shape) -> Result<Penetration, QueryError> {
    first.validate()?;
    second.validate()?;
    let contact = pair(first, second);
    if contact.gap > Number::ZERO {
        return Ok(Penetration {
            depth: Number::ZERO,
            normal: Vector3::ZERO,
        });
    }
    Ok(Penetration {
        depth: contact.depth,
        normal: contact.normal,
    })
}

pub(crate) struct Pair {
    pub(crate) gap: Number,
    pub(crate) depth: Number,
    pub(crate) normal: Vector3,
    pub(crate) origin: Vector3,
    pub(crate) point_on_hull: Vector3,
    pub(crate) intersecting_hulls: bool,
}

pub(crate) fn pair(first: Shape, second: Shape) -> Pair {
    let origin = first.reference();
    let first_radius = first.radius();
    let second_radius = second.radius();
    let radius = first_radius + second_radius;
    let first = Hull::new(first, origin);
    let second = Hull::new(second, origin);
    if let Some((normal, depth)) = intersection(first, second) {
        return Pair {
            gap: Number::ZERO,
            depth: depth + radius,
            normal,
            origin,
            point_on_hull: Vector3::ZERO,
            intersecting_hulls: true,
        };
    }
    let witness = closest(first, second);
    let signed_gap = witness.distance - radius;
    Pair {
        gap: signed_gap.max(Number::ZERO),
        depth: (-signed_gap).max(Number::ZERO),
        normal: witness.delta.unit().unwrap_or(Vector3::AXES[0]),
        origin,
        point_on_hull: witness.point_on_second,
        intersecting_hulls: false,
    }
}

impl Pair {
    pub(crate) fn surface(&self, first: Shape, second: Shape) -> Vector3 {
        let point = if self.intersecting_hulls {
            // At the shallowest separating translation, closest features give
            // a deterministic witness on the struck hull's surface.
            let moved = Hull::new(first, self.origin).translated(self.normal * self.depth);
            closest(moved, Hull::new(second, self.origin)).point_on_second
        } else {
            self.point_on_hull
        };
        self.origin + (point + self.normal * second.radius())
    }
}
