use thrum_core::profile::Number;

use crate::{
    Shape, Vector3,
    predicates::{conditioned, cross, determinant},
};

/// Hull coordinates translated into the same local frame.
#[derive(Clone, Copy)]
pub(crate) enum Hull {
    Point(Vector3),
    Segment([Vector3; 2]),
    Box {
        center: Vector3,
        half_edges: [Vector3; 3],
    },
    Triangle([Vector3; 3]),
}

impl Hull {
    pub(crate) fn new(shape: Shape, origin: Vector3) -> Self {
        match shape {
            Shape::Sphere { center, .. } => Self::Point(center - origin),
            Shape::Capsule { ends, .. } => Self::Segment(ends.map(|end| end - origin)),
            Shape::Cuboid { center, half_edges } => Self::Box {
                center: center - origin,
                half_edges,
            },
            Shape::Triangle { corners } => Self::Triangle(corners.map(|corner| corner - origin)),
        }
    }

    pub(crate) fn translated(self, displacement: Vector3) -> Self {
        match self {
            Self::Point(point) => Self::Point(point + displacement),
            Self::Segment(ends) => Self::Segment(ends.map(|end| end + displacement)),
            Self::Box { center, half_edges } => Self::Box {
                center: center + displacement,
                half_edges,
            },
            Self::Triangle(corners) => Self::Triangle(corners.map(|corner| corner + displacement)),
        }
    }

    pub(crate) fn vertices(self) -> ([Vector3; 8], usize) {
        let mut points = [Vector3::ZERO; 8];
        let count = match self {
            Self::Point(point) => {
                points[0] = point;
                1
            }
            Self::Segment(ends) => {
                points[..2].copy_from_slice(&ends);
                2
            }
            Self::Box { center, half_edges } => {
                for (index, point) in points.iter_mut().enumerate() {
                    *point = center;
                    for (axis, half_edge) in half_edges.into_iter().enumerate() {
                        *point = if index & (1 << axis) == 0 {
                            *point - half_edge
                        } else {
                            *point + half_edge
                        };
                    }
                }
                8
            }
            Self::Triangle(corners) => {
                points[..3].copy_from_slice(&corners);
                3
            }
        };
        (points, count)
    }

    fn edges(self) -> &'static [(usize, usize)] {
        match self {
            Self::Point(_) => &[],
            Self::Segment(_) => &[(0, 1)],
            Self::Box { .. } => &[
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
                (0, 2),
                (1, 3),
                (4, 6),
                (5, 7),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ],
            Self::Triangle(_) => &[(0, 1), (1, 2), (2, 0)],
        }
    }

    pub(crate) fn edge_directions(self) -> [Vector3; 3] {
        match self {
            Self::Point(_) => [Vector3::ZERO; 3],
            Self::Segment([a, b]) => [b - a, Vector3::ZERO, Vector3::ZERO],
            Self::Box { half_edges, .. } => half_edges,
            Self::Triangle([a, b, c]) => [b - a, c - b, a - c],
        }
    }

    fn span(self) -> [Vector3; 3] {
        match self {
            // The two incident edges span the triangle. Subtracting its third
            // edge independently could give it an artificial third dimension.
            Self::Triangle([a, b, c]) => [b - a, c - a, Vector3::ZERO],
            _ => self.edge_directions(),
        }
    }

    fn reference(self) -> Vector3 {
        match self {
            Self::Point(point) | Self::Box { center: point, .. } => point,
            Self::Segment([first, _]) | Self::Triangle([first, _, _]) => first,
        }
    }

    pub(crate) fn project(self, axis: Vector3) -> (Number, Number) {
        match self {
            Self::Point(point) => {
                let at = point.dot(axis);
                (at, at)
            }
            Self::Box { center, half_edges } => {
                let at = center.dot(axis);
                let extent = half_edges[0].dot(axis).abs()
                    + half_edges[1].dot(axis).abs()
                    + half_edges[2].dot(axis).abs();
                (at - extent, at + extent)
            }
            Self::Segment([a, b]) => {
                let a = a.dot(axis);
                let b = b.dot(axis);
                (a.min(b), a.max(b))
            }
            Self::Triangle([a, b, c]) => {
                let a = a.dot(axis);
                let b = b.dot(axis);
                let c = c.dot(axis);
                (a.min(b).min(c), a.max(b).max(c))
            }
        }
    }

    fn closest_to_point(self, point: Vector3) -> Vector3 {
        match self {
            Self::Point(position) => position,
            Self::Segment(ends) => closest_on_segment(ends, point),
            Self::Box { center, half_edges } => {
                let offset = point - center;
                let mut nearest = offset;
                for axis in box_axes(half_edges) {
                    let position = offset.dot(axis.direction);
                    let clamped = position.clamp(-axis.extent, axis.extent);
                    nearest = nearest + axis.direction * (clamped - position);
                }
                center + nearest
            }
            Self::Triangle(corners) => closest_on_triangle(corners, point),
        }
    }
}

pub(crate) struct BoxAxis {
    pub(crate) direction: Vector3,
    pub(crate) extent: Number,
}

/// Nonzero half-edges give the frame directly. Complete it for a flat box,
/// a line or a point so slab and projection tests still constrain every axis.
pub(crate) fn box_axes(half_edges: [Vector3; 3]) -> [BoxAxis; 3] {
    let mut axes = std::array::from_fn(|_| BoxAxis {
        direction: Vector3::ZERO,
        extent: Number::ZERO,
    });
    let mut count = 0;
    for edge in half_edges {
        if let Some(direction) = edge.unit() {
            axes[count] = BoxAxis {
                direction,
                extent: edge.length(),
            };
            count += 1;
        }
    }
    if count == 0 {
        for (axis, direction) in axes.iter_mut().zip(Vector3::AXES) {
            axis.direction = direction;
        }
        return axes;
    }
    if count == 1 {
        axes[1].direction = perpendicular(axes[0].direction);
    }
    if count < 3 {
        axes[2].direction = axes[0]
            .direction
            .cross(axes[1].direction)
            .unit()
            .expect("box axes are independent");
    }
    axes
}

pub(crate) fn perpendicular(line: Vector3) -> Vector3 {
    // Prefer the world axis with the smallest absolute component along the line.
    let [x, y, z] = Vector3::AXES;
    let axis = if line.x.abs() <= line.y.abs() && line.x.abs() <= line.z.abs() {
        x
    } else if line.y.abs() <= line.z.abs() {
        y
    } else {
        z
    };
    line.cross(axis)
        .unit()
        .expect("the world axis is independent of the line")
}

pub(crate) fn closest_on_segment([a, b]: [Vector3; 2], point: Vector3) -> Vector3 {
    let edge = b - a;
    let scale = edge.max_component();
    if scale == Number::ZERO {
        return a;
    }
    let direction = edge / scale;
    let along = ((point - a).dot(direction) / direction.dot(direction)).clamp(Number::ZERO, scale);
    a + direction * along
}

/// Test the three inward half-planes in the triangle's plane.
pub(crate) fn triangle_contains([a, b, c]: [Vector3; 3], point: Vector3, normal: Vector3) -> bool {
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .all(|(start, end)| normal.cross(end - start).dot(point - start) >= Number::ZERO)
}

pub(crate) fn triangle_normal([a, b, c]: [Vector3; 3]) -> Option<Vector3> {
    cross(conditioned(b - a), conditioned(c - a)).unit()
}

fn closest_on_triangle(corners: [Vector3; 3], point: Vector3) -> Vector3 {
    let [a, b, c] = corners;
    if let Some(normal) = triangle_normal(corners) {
        let projected = point - normal * (point - a).dot(normal);
        if triangle_contains(corners, projected, normal) {
            return projected;
        }
    }
    let mut nearest = a;
    let mut best = (point - a).length();
    for edge in [[a, b], [b, c], [c, a]] {
        let candidate = closest_on_segment(edge, point);
        let distance = (point - candidate).length();
        if distance < best {
            nearest = candidate;
            best = distance;
        }
    }
    nearest
}

#[derive(Clone, Copy)]
pub(crate) struct Witness {
    pub(crate) delta: Vector3,
    pub(crate) point_on_second: Vector3,
    pub(crate) distance: Number,
}

impl Witness {
    fn new(first: Vector3, second: Vector3) -> Self {
        let delta = first - second;
        Self {
            delta,
            point_on_second: second,
            distance: delta.length(),
        }
    }

    fn consider(&mut self, first: Vector3, second: Vector3) {
        let candidate = Self::new(first, second);
        if candidate.distance < self.distance {
            *self = candidate;
        }
    }
}

/// The minimum for disjoint hulls is attained by a vertex against a hull or
/// between two edges. Edge endpoints are already covered by vertex projection.
/// Keep the first witness on a tie: first-hull vertices, second-hull vertices,
/// then edge pairs, all in stored order.
pub(crate) fn closest(first: Hull, second: Hull) -> Witness {
    let (first_points, first_count) = first.vertices();
    let (second_points, second_count) = second.vertices();
    let mut nearest = Witness::new(first_points[0], second.closest_to_point(first_points[0]));
    for &point in &first_points[1..first_count] {
        nearest.consider(point, second.closest_to_point(point));
    }
    if matches!(first, Hull::Point(_)) {
        return nearest;
    }
    for &point in &second_points[..second_count] {
        nearest.consider(first.closest_to_point(point), point);
    }
    for &(a, b) in first.edges() {
        for &(c, d) in second.edges() {
            if let Some((first, second)) = closest_inside_edges(
                first_points[a],
                first_points[b],
                second_points[c],
                second_points[d],
            ) {
                nearest.consider(first, second);
            }
        }
    }
    nearest
}

fn closest_inside_edges(
    a: Vector3,
    b: Vector3,
    c: Vector3,
    d: Vector3,
) -> Option<(Vector3, Vector3)> {
    let first_edge = b - a;
    let second_edge = d - c;
    let u = first_edge.unit()?;
    let v = second_edge.unit()?;
    let normal = cross(conditioned(first_edge), conditioned(second_edge)).unit()?;
    let denominator = cross(u, v).dot(normal);
    if denominator <= Number::ZERO {
        return None;
    }
    let between = c - a;
    let first = between.cross(v).dot(normal) / denominator;
    let second = between.cross(u).dot(normal) / denominator;
    if first > Number::ZERO
        && first < first_edge.length()
        && second > Number::ZERO
        && second < second_edge.length()
    {
        Some((a + u * first, c + v * second))
    } else {
        None
    }
}

/// The facet normals of the hull difference are the nonzero cross products of
/// the edge directions. Keep the first normal on a tie, in shape/edge order,
/// testing the cross product's positive sense before its negative sense.
pub(crate) fn intersection(first: Hull, second: Hull) -> Option<(Vector3, Number)> {
    let [a, b, c] = first.edge_directions().map(conditioned);
    let [d, e, f] = second.edge_directions().map(conditioned);
    let edges = [a, b, c, d, e, f];
    let [a, b, c] = first.span().map(conditioned);
    let [d, e, f] = second.span().map(conditioned);
    let span = [a, b, c, d, e, f];
    let offset = conditioned(second.reference() - first.reference());
    let Some(line) = span.into_iter().find(|edge| *edge != Vector3::ZERO) else {
        return (offset == Vector3::ZERO).then_some((Vector3::AXES[0], Number::ZERO));
    };
    let plane = span
        .into_iter()
        .find_map(|edge| cross(line, edge).unit().map(|normal| (edge, normal)));
    let Some((second_edge, plane)) = plane else {
        if cross(line, offset) != Vector3::ZERO {
            return None;
        }
        // A world axis through the largest line component orders the points
        // along the line without a rounded projection onto a unit vector.
        let largest = line.max_component();
        let axis = if line.x.abs() == largest {
            Vector3::AXES[0]
        } else if line.y.abs() == largest {
            Vector3::AXES[1]
        } else {
            Vector3::AXES[2]
        };
        consider_axis(first, second, axis, &mut None)?;
        return Some((perpendicular(line), Number::ZERO));
    };

    if span
        .iter()
        .all(|edge| determinant(line, second_edge, *edge) == Number::ZERO)
    {
        if determinant(line, second_edge, offset) != Number::ZERO {
            return None;
        }
        // A shared plane has zero depth along either normal. Its in-plane
        // separating axes still distinguish disjoint coplanar hulls.
        for edge in edges {
            if let Some(axis) = cross(edge, plane).unit() {
                consider_axis(first, second, axis, &mut None)?;
            }
        }
        return Some((plane, Number::ZERO));
    }

    let mut best = None;
    for (index, &edge) in edges.iter().enumerate() {
        for &next in &edges[index + 1..] {
            if let Some(axis) = cross(edge, next).unit() {
                consider_axis(first, second, axis, &mut best)?;
            }
        }
    }
    best
}

fn consider_axis(
    first: Hull,
    second: Hull,
    axis: Vector3,
    best: &mut Option<(Vector3, Number)>,
) -> Option<()> {
    let (first_min, first_max) = first.project(axis);
    let (second_min, second_max) = second.project(axis);
    let positive = second_max - first_min;
    let negative = first_max - second_min;
    if positive < Number::ZERO || negative < Number::ZERO {
        return None;
    }
    let candidate = if positive <= negative {
        (axis, positive)
    } else {
        (-axis, negative)
    };
    if best.is_none_or(|(_, depth)| candidate.1 < depth) {
        *best = Some(candidate);
    }
    Some(())
}
