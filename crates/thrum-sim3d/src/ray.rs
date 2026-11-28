use thrum_core::profile::Number;

use crate::{
    QueryError, Shape, Vector3,
    hull::{Hull, box_axes, closest_on_segment},
    pair::pair,
    predicates::{conditioned, cross, determinant},
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
    let length = motion.length();
    let radius = shape.radius();
    let contact = match hull {
        Hull::Point(center) => sphere(center, radius, motion),
        Hull::Segment(ends) => capsule(ends, radius, motion, direction),
        Hull::Box { center, half_edges } => cuboid(center, half_edges, direction),
        Hull::Triangle(_) => unreachable!("triangles use the plane predicate"),
    };
    Ok(contact
        .filter(|(time, _)| *time <= length)
        .map(|(time, normal)| {
            let fraction = time / length;
            RayHit {
                fraction,
                point: start + motion * fraction,
                normal,
                depth: Number::ZERO,
            }
        }))
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
        return sphere(a, radius, motion);
    };
    let mut hit = [a, b]
        .into_iter()
        .filter_map(|end| sphere(end, radius, motion))
        .min_by(|first, second| first.0.partial_cmp(&second.0).expect("finite times"));
    let line = conditioned(motion);
    let edge_direction = conditioned(edge);
    let transverse = cross(line, edge_direction);
    if transverse != Vector3::ZERO {
        let edge_length = edge_direction.length();
        let speed = transverse.length() / (line.length() * edge_length);
        let center = cross(a, edge_direction) / edge_length;
        if let Some((when, _)) = sphere(center, radius, transverse) {
            let time = when / speed;
            let point = direction * time;
            let along = (point - a).dot(axis);
            if along >= Number::ZERO
                && along <= edge.length()
                && hit.is_none_or(|(previous, _)| time < previous)
            {
                let nearest = closest_on_segment(ends, point);
                hit = positive_hit(time, (point - nearest).unit().unwrap_or(-direction));
            }
        }
    }
    hit
}

fn cuboid(
    center: Vector3,
    half_edges: [Vector3; 3],
    direction: Vector3,
) -> Option<(Number, Vector3)> {
    let mut lower = Number::ZERO;
    let mut upper = None;
    let mut normal = -direction;
    for axis in box_axes(half_edges) {
        let position = (-center).dot(axis.direction);
        let speed = direction.dot(axis.direction);
        if speed == Number::ZERO {
            if position.abs() > axis.extent {
                return None;
            }
            continue;
        }
        let a = (-axis.extent - position) / speed;
        let b = (axis.extent - position) / speed;
        let near = a.min(b);
        let far = a.max(b);
        if near > lower {
            lower = near;
            normal = if speed > Number::ZERO {
                -axis.direction
            } else {
                axis.direction
            };
        }
        upper = Some(upper.map_or(far, |upper: Number| upper.min(far)));
        if lower > upper? {
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
