use thrum_core::profile::Number;

use crate::{
    QueryError, Shape, Vector3,
    hull::{Hull, box_axes, closest_on_segment},
    pair::pair,
    predicates::{conditioned, cross, cylinder_gap, determinant, dot_sum, squared_gap},
    shape::validate_point,
};

/// First contact of a ray with a shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Fraction of the supplied motion from zero through one.
    pub fraction: Number,
    /// Point on the ray, in world coordinates.
    pub point: Vector3,
    /// Unit direction out of the struck shape.
    pub normal: Vector3,
    /// Initial depth, or zero for an exterior hit.
    pub depth: Number,
}

/// Cast the finite ray from `start` to `start + motion` against a shape.
///
/// # Errors
/// Returns the input fault for an invalid shape, start or motion.
pub fn ray(start: Vector3, motion: Vector3, shape: Shape) -> Result<Option<RayHit>, QueryError> {
    shape.validate()?;
    validate_point(start)?;
    if !motion.is_finite() {
        return Err(QueryError::NonFinite);
    }
    let point = Shape::Sphere {
        center: start,
        radius: Number::ZERO,
    };
    point.validate_motion_end(motion)?;
    if let Shape::Triangle { corners } = shape {
        return Ok(triangle(start, motion, corners));
    }

    let hull = Hull::new(shape, start);
    let contact = pair(point, shape);
    if contact.gap == Number::ZERO {
        return Ok(Some(RayHit {
            fraction: Number::ZERO,
            point: start,
            normal: contact.normal,
            depth: contact.depth,
        }));
    }
    let Some(direction) = motion.unit() else {
        return Ok(None);
    };
    let radius = shape.radius();
    let contact = match hull {
        Hull::Point(center) => {
            finite_sphere(center, radius, motion, motion.length(), [motion, center])
        }
        Hull::Segment(ends) => capsule(ends, radius, motion, direction),
        Hull::Box { center, half_edges } => cuboid(center, half_edges, motion, direction),
        Hull::Triangle(_) => unreachable!("triangles use the plane predicate"),
    };
    Ok(contact.map(|(fraction, normal)| RayHit {
        fraction,
        point: start + motion * fraction,
        normal,
        depth: Number::ZERO,
    }))
}

fn finite_sphere(
    center: Vector3,
    radius: Number,
    direction: Vector3,
    length: Number,
    [end, end_center]: [Vector3; 2],
) -> Option<(Number, Vector3)> {
    // Ahead of the closest approach the endpoint must already be inside the
    // sphere. Compare in the endpoint's local frame before rounding a time.
    let toward = dot_sum(conditioned(direction), [end_center, -end]);
    if toward >= Number::ZERO {
        let side = squared_gap(end, end_center, radius);
        if side > Number::ZERO {
            return None;
        }
        if side == Number::ZERO {
            let normal = (end - end_center).unit().or_else(|| (-direction).unit())?;
            return Some((Number::ONE, normal));
        }
    }
    let (time, normal) = sphere(center, radius, direction)?;
    Some(((time / length).min(Number::ONE), normal))
}

fn sphere(center: Vector3, radius: Number, motion: Vector3) -> Option<(Number, Vector3)> {
    let direction = motion.unit()?;
    let projected = center.dot(direction);
    // Keep the supplied direction for the parallel test. Rounding it to a
    // unit vector first can make a ray miss an exactly collinear point.
    let motion = conditioned(motion);
    let perpendicular = cross(motion, center) / motion.length();
    let across = perpendicular.length();
    if projected < Number::ZERO || across > radius {
        return None;
    }
    let offset = ((radius - across) * (radius + across)).sqrt();
    let time = projected - offset;
    positive_hit(
        time,
        (-direction * offset + direction.cross(perpendicular))
            .unit()
            .unwrap_or(-direction),
    )
}

fn capsule(
    ends: [Vector3; 2],
    radius: Number,
    motion: Vector3,
    direction: Vector3,
) -> Option<(Number, Vector3)> {
    let [a, b] = ends;
    let edge = b - a;
    let Some(axis) = edge.unit() else {
        return finite_sphere(a, radius, motion, motion.length(), [motion, a]);
    };
    let mut hit = [a, b]
        .into_iter()
        .filter_map(|end| finite_sphere(end, radius, motion, motion.length(), [motion, end]))
        .min_by(|first, second| first.0.partial_cmp(&second.0).expect("finite times"));
    let line = conditioned(motion);
    let edge_direction = conditioned(edge);
    let transverse = cross(line, edge_direction);
    if transverse != Vector3::ZERO {
        let edge_length = edge_direction.length();
        let center = cross(a, edge_direction) / edge_length;
        let speed = transverse.length() / (line.length() * edge_length);
        let endpoint = a - motion;
        let toward = dot_sum(transverse, [cross(endpoint, edge_direction)]);
        let fraction = if toward >= Number::ZERO {
            let gap = cylinder_gap(endpoint, edge_direction, radius);
            if gap > Number::ZERO {
                return hit;
            }
            (gap == Number::ZERO).then_some(Number::ONE)
        } else {
            None
        };
        let fraction = fraction.or_else(|| {
            let (when, _) = sphere(center, radius, transverse)?;
            Some((when / (speed * motion.length())).min(Number::ONE))
        });
        if let Some(fraction) = fraction {
            let point = motion * fraction;
            let along = (point - a).dot(axis);
            if along >= Number::ZERO
                && along <= edge.length()
                && hit.is_none_or(|(previous, _)| fraction < previous)
            {
                let nearest = closest_on_segment(ends, point);
                hit = Some((fraction, (point - nearest).unit().unwrap_or(-direction)));
            }
        }
    }
    hit
}

fn cuboid(
    center: Vector3,
    half_edges: [Vector3; 3],
    motion: Vector3,
    direction: Vector3,
) -> Option<(Number, Vector3)> {
    let mut lower = Number::ZERO;
    let mut upper = Number::ONE;
    let mut normal = -direction;
    for axis in box_axes(half_edges) {
        let along = axis.scaled_direction();
        let speed = dot_sum(along, [motion]);
        let to_near = dot_sum(along, [center, -axis.edge]);
        let to_far = dot_sum(along, [center, axis.edge]);
        if speed == Number::ZERO {
            if to_near > Number::ZERO || to_far < Number::ZERO {
                return None;
            }
            continue;
        }
        let (entry, exit, entry_edge, outward) = if speed > Number::ZERO {
            (to_near, to_far, -axis.edge, -axis.direction)
        } else {
            (to_far, to_near, axis.edge, axis.direction)
        };
        let remaining = dot_sum(along, [center, entry_edge, -motion]);
        if (remaining > Number::ZERO && speed > Number::ZERO)
            || (remaining < Number::ZERO && speed < Number::ZERO)
        {
            return None;
        }
        let near = (entry / speed).min(Number::ONE);
        let far = exit / speed;
        if near > lower {
            lower = near;
            normal = outward;
        }
        upper = upper.min(far);
        if lower > upper {
            return None;
        }
    }
    positive_hit(lower, normal)
}

fn triangle(start: Vector3, motion: Vector3, [a, b, c]: [Vector3; 3]) -> Option<RayHit> {
    let first = conditioned(b - a);
    let second = conditioned(c - a);
    let [a, b, c] = [a - start, b - start, c - start];
    let speed = determinant(first, second, motion);
    if speed == Number::ZERO {
        return None;
    }
    let toward_plane = determinant(first, second, a);
    let reaches_plane = if speed > Number::ZERO {
        toward_plane >= Number::ZERO && toward_plane <= speed
    } else {
        toward_plane <= Number::ZERO && toward_plane >= speed
    };
    if !reaches_plane {
        return None;
    }
    // The determinant of each edge and the ray gives the sign of the
    // opposite barycentric coordinate without rounding a point onto the plane.
    let forward = speed > Number::ZERO;
    for (from, to) in [(a, b), (b, c), (c, a)] {
        let side = determinant(conditioned(from), conditioned(to), conditioned(motion));
        if side != Number::ZERO && (side > Number::ZERO) != forward {
            return None;
        }
    }
    let fraction = toward_plane / speed;
    let normal = cross(first, second).unit()?;
    Some(RayHit {
        fraction,
        point: start + motion * fraction,
        normal: if forward { -normal } else { normal },
        depth: Number::ZERO,
    })
}

fn positive_hit(time: Number, normal: Vector3) -> Option<(Number, Vector3)> {
    (time >= Number::ZERO).then_some((time, normal))
}
