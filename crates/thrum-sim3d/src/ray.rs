use thrum_core::profile::Number;

use crate::{
    QueryError, Shape, Vector3,
    hull::{Hull, box_axes, closest_on_segment, triangle_contains, triangle_normal},
    pair::pair,
    shape::{validate_motion_end, validate_point},
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
    validate_motion_end(validate_point(start + motion))?;

    let hull = Hull::new(shape, start);
    if !matches!(hull, Hull::Triangle(_)) {
        let point = Shape::Sphere {
            center: start,
            radius: Number::ZERO,
        };
        let contact = pair(point, shape);
        if contact.gap == Number::ZERO {
            return Ok(Some(RayHit {
                fraction: Number::ZERO,
                point: start,
                normal: contact.normal,
                depth: contact.depth,
            }));
        }
    }
    let Some(direction) = motion.unit() else {
        return Ok(None);
    };
    let length = motion.length();
    let radius = shape.radius();
    let contact = match hull {
        Hull::Point(center) => sphere(center, radius, direction),
        Hull::Segment(ends) => capsule(ends, radius, direction),
        Hull::Box { center, half_edges } => cuboid(center, half_edges, direction),
        Hull::Triangle(corners) => triangle(corners, direction),
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

fn sphere(center: Vector3, radius: Number, direction: Vector3) -> Option<(Number, Vector3)> {
    let projected = center.dot(direction);
    let perpendicular = direction.cross(center);
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

fn capsule(ends: [Vector3; 2], radius: Number, direction: Vector3) -> Option<(Number, Vector3)> {
    let [a, b] = ends;
    let Some(axis) = (b - a).unit() else {
        return sphere(a, radius, direction);
    };
    let mut hit = [a, b]
        .into_iter()
        .filter_map(|end| sphere(end, radius, direction))
        .min_by(|first, second| first.0.partial_cmp(&second.0).expect("finite times"));
    let transverse = direction.cross(axis);
    let speed = transverse.length();
    if let Some(unit) = transverse.unit() {
        let center = a.cross(axis);
        if let Some((when, _)) = sphere(center, radius, unit) {
            let time = when / speed;
            let point = direction * time;
            let length = (b - a).length();
            let along = (point - a).dot(axis);
            if along >= Number::ZERO
                && along <= length
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

fn triangle(corners: [Vector3; 3], direction: Vector3) -> Option<(Number, Vector3)> {
    let normal = triangle_normal(corners)?;
    let speed = normal.dot(direction);
    if speed == Number::ZERO {
        return None;
    }
    let time = normal.dot(corners[0]) / speed;
    if !triangle_contains(corners, direction * time, normal) {
        return None;
    }
    positive_hit(
        time,
        if speed > Number::ZERO {
            -normal
        } else {
            normal
        },
    )
}

fn positive_hit(time: Number, normal: Vector3) -> Option<(Number, Vector3)> {
    (time >= Number::ZERO).then_some((time, normal))
}
