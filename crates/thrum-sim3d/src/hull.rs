use thrum_core::profile::Number;

use crate::{
    Shape, Vector3, n,
    predicates::{conditioned, cross, determinant, dot_sum, sum, two_sum},
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

    fn vertices(self) -> ([Vertex; 8], usize) {
        let mut points = [Vertex::new(Vector3::ZERO); 8];
        let count = match self {
            Self::Point(point) => {
                points[0] = Vertex::new(point);
                1
            }
            Self::Segment(ends) => {
                points[..2].copy_from_slice(&ends.map(Vertex::new));
                2
            }
            Self::Box { center, half_edges } => {
                for (index, point) in points.iter_mut().enumerate() {
                    *point = Vertex::cuboid(center, half_edges, index);
                }
                8
            }
            Self::Triangle(corners) => {
                points[..3].copy_from_slice(&corners.map(Vertex::new));
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

    fn edge(self, index: usize, vertices: &[Vertex; 8]) -> (Vertex, Vector3) {
        let (start, end) = self.edges()[index];
        let direction = match self {
            // The four edges of each orientation use the supplied half-edge.
            // Subtracting rounded vertices can make parallel edges diverge.
            Self::Box { half_edges, .. } => half_edges[index / 4] * n(2),
            _ => vertices[end].point - vertices[start].point,
        };
        (vertices[start], direction)
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

    fn project(self, axis: Vector3) -> (Number, Number) {
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

    /// Terms of the support point, before adding the edges to the centre.
    fn support(self, axis: Vector3) -> [Vector3; 4] {
        let mut terms = [Vector3::ZERO; 4];
        terms[0] = match self {
            Self::Point(point) => point,
            Self::Segment(ends) => extreme_point(ends, axis),
            Self::Triangle(corners) => extreme_point(corners, axis),
            Self::Box { center, half_edges } => {
                for (term, edge) in terms[1..].iter_mut().zip(half_edges) {
                    *term = if dot_sum(axis, [edge]) >= Number::ZERO {
                        edge
                    } else {
                        -edge
                    };
                }
                center
            }
        };
        terms
    }

    fn projection_scale(self) -> Number {
        match self {
            Self::Point(point) => point.max_component(),
            Self::Segment(ends) => ends[0].max_component().max(ends[1].max_component()),
            Self::Triangle(corners) => corners
                .into_iter()
                .map(Vector3::max_component)
                .fold(Number::ZERO, Number::max),
            Self::Box { center, half_edges } => half_edges
                .into_iter()
                .map(Vector3::max_component)
                .fold(center.max_component(), Number::max),
        }
    }

    fn closest_to_point(self, vertex: Vertex) -> (Vector3, Vector3) {
        let Vertex { point, residual } = vertex;
        let nearest = match self {
            Self::Point(position) => position,
            Self::Segment(ends) => closest_on_segment(ends, point),
            Self::Box { center, half_edges } => {
                // Keep the displacement in the box frame. Subtracting the
                // two rounded world-frame points can erase it or turn it.
                let mut delta = Vector3::ZERO;
                for axis in box_axes(half_edges) {
                    let projected = (point - center).dot(axis.direction);
                    let face = if projected >= Number::ZERO {
                        axis.edge
                    } else {
                        -axis.edge
                    };
                    let direction = axis.scaled_direction();
                    let excess =
                        dot_sum(direction, [point, residual, -center, -face]) / direction.length();
                    let outside = if projected >= Number::ZERO {
                        excess.max(Number::ZERO)
                    } else {
                        excess.min(Number::ZERO)
                    };
                    delta = delta + axis.direction * outside;
                }
                return (point + (residual - delta), delta);
            }
            Self::Triangle(corners) => closest_on_triangle(corners, point),
        };
        (nearest, (point - nearest) + residual)
    }
}

#[derive(Clone, Copy)]
struct Vertex {
    point: Vector3,
    residual: Vector3,
}

impl Vertex {
    fn new(point: Vector3) -> Self {
        Self {
            point,
            residual: Vector3::ZERO,
        }
    }

    fn cuboid(center: Vector3, edges: [Vector3; 3], index: usize) -> Self {
        let [a, b, c] = std::array::from_fn(|axis| {
            if index & (1 << axis) == 0 {
                -edges[axis]
            } else {
                edges[axis]
            }
        });
        let coordinate = |center, a, b, c| {
            let (head, first) = two_sum(center, a);
            let (head, second) = two_sum(head, b);
            let (head, third) = two_sum(head, c);
            (head, sum([first, second, third]))
        };
        let (x, x_tail) = coordinate(center.x, a.x, b.x, c.x);
        let (y, y_tail) = coordinate(center.y, a.y, b.y, c.y);
        let (z, z_tail) = coordinate(center.z, a.z, b.z, c.z);
        Self {
            point: Vector3::new(x, y, z),
            residual: Vector3::new(x_tail, y_tail, z_tail),
        }
    }
}

fn extreme_point<const N: usize>(points: [Vector3; N], axis: Vector3) -> Vector3 {
    let mut extreme = points[0];
    for point in points.into_iter().skip(1) {
        if dot_sum(axis, [point, -extreme]) > Number::ZERO {
            extreme = point;
        }
    }
    extreme
}

pub(crate) struct BoxAxis {
    pub(crate) direction: Vector3,
    pub(crate) edge: Vector3,
}

impl BoxAxis {
    pub(crate) fn scaled_direction(&self) -> Vector3 {
        if self.edge == Vector3::ZERO {
            self.direction
        } else {
            conditioned(self.edge)
        }
    }
}

/// Nonzero half-edges give the frame directly. Complete it for a flat box,
/// a line or a point so slab and projection tests still constrain every axis.
pub(crate) fn box_axes(half_edges: [Vector3; 3]) -> [BoxAxis; 3] {
    let mut axes = std::array::from_fn(|_| BoxAxis {
        direction: Vector3::ZERO,
        edge: Vector3::ZERO,
    });
    let mut count = 0;
    for edge in half_edges {
        if let Some(direction) = edge.unit() {
            axes[count] = BoxAxis { direction, edge };
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
    fn new(second: Vector3, delta: Vector3) -> Self {
        Self {
            delta,
            point_on_second: second,
            distance: delta.length(),
        }
    }

    fn consider(&mut self, second: Vector3, delta: Vector3) {
        let candidate = Self::new(second, delta);
        if candidate.distance < self.distance {
            *self = candidate;
        }
    }
}

/// The minimum for disjoint hulls is attained by a vertex against a hull or
/// between two edges. Edge endpoints are already covered by vertex projection.
/// Keep the first witness on a tie: first-hull vertices, second-hull vertices,
/// then edge pairs, all in stored order.
pub(crate) fn closest(first: Hull, second: Hull, separation: Option<(Vector3, Number)>) -> Witness {
    let (first_points, first_count) = first.vertices();
    let (second_points, second_count) = second.vertices();
    let (point, delta) = second.closest_to_point(first_points[0]);
    let mut fallback = Witness::new(point, delta);
    let mut nearest = None;
    let mut consider = |point, delta| {
        if separation.is_none_or(|(axis, gap)| axis.dot(delta) >= gap - gap / n(1 << 16)) {
            let candidate = Witness::new(point, delta);
            if nearest.is_none_or(|best: Witness| candidate.distance < best.distance) {
                nearest = Some(candidate);
            }
        }
    };
    consider(point, delta);
    for &point in &first_points[1..first_count] {
        let (nearest_point, delta) = second.closest_to_point(point);
        fallback.consider(nearest_point, delta);
        consider(nearest_point, delta);
    }
    if !matches!(first, Hull::Point(_)) {
        for &point in &second_points[..second_count] {
            let (_, delta) = first.closest_to_point(point);
            let point = point.point + point.residual;
            fallback.consider(point, -delta);
            consider(point, -delta);
        }
        for first_edge in 0..first.edges().len() {
            let (a, u) = first.edge(first_edge, &first_points);
            for second_edge in 0..second.edges().len() {
                let (c, v) = second.edge(second_edge, &second_points);
                if let Some((point, delta)) = closest_inside_edges(a, u, c, v) {
                    consider(point, delta);
                }
            }
        }
    }
    nearest.unwrap_or_else(|| {
        let (axis, gap) = separation.expect("a separating plane excluded the rounded witnesses");
        let correction = gap - axis.dot(fallback.delta);
        Witness::new(fallback.point_on_second, fallback.delta + axis * correction)
    })
}

fn closest_inside_edges(
    a: Vertex,
    first_edge: Vector3,
    c: Vertex,
    second_edge: Vector3,
) -> Option<(Vector3, Vector3)> {
    let u = first_edge.unit()?;
    let v = second_edge.unit()?;
    let cross_axis = cross(conditioned(first_edge), conditioned(second_edge));
    let normal = cross_axis.unit()?;
    let denominator = cross(u, v).dot(normal);
    if denominator <= Number::ZERO {
        return None;
    }
    let between = (c.point - a.point) + (c.residual - a.residual);
    let first = between.cross(v).dot(normal) / denominator;
    let second = between.cross(u).dot(normal) / denominator;
    if first > Number::ZERO
        && first < first_edge.length()
        && second > Number::ZERO
        && second < second_edge.length()
    {
        // An interior edge pair differs only along its common normal.
        // Project the edge origins before adding the closest-point offsets.
        let offset = dot_sum(cross_axis, [a.point, a.residual, -c.point, -c.residual]);
        Some((
            c.point + (c.residual + v * second),
            normal * (offset / cross_axis.length()),
        ))
    } else {
        None
    }
}

/// A positive projection overlap measures the shallowest hull penetration.
/// A negative one bounds the closest witnesses on the separating side.
/// The facet normals are the nonzero cross products of the edge directions.
pub(crate) fn signed_overlap(first: Hull, second: Hull) -> Option<(Vector3, Number)> {
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
        let separation = overlap_on_axis(first, second, axis);
        if separation.1 < Number::ZERO {
            return Some(separation);
        }
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
        return edges
            .into_iter()
            .filter_map(|edge| cross(edge, plane).unit())
            .map(|axis| overlap_on_axis(first, second, axis))
            .fold(Some((plane, Number::ZERO)), keep_shallower);
    }

    let mut best = None;
    for (index, &edge) in edges.iter().enumerate() {
        for &next in &edges[index + 1..] {
            if let Some(axis) = cross(edge, next).unit() {
                best = keep_shallower(best, overlap_on_axis(first, second, axis));
            }
        }
    }
    best
}

fn keep_shallower(
    best: Option<(Vector3, Number)>,
    candidate: (Vector3, Number),
) -> Option<(Vector3, Number)> {
    if best.is_none_or(|(_, depth)| candidate.1 < depth) {
        Some(candidate)
    } else {
        best
    }
}

fn overlap_on_axis(first: Hull, second: Hull, axis: Vector3) -> (Vector3, Number) {
    let (first_min, first_max) = first.project(axis);
    let (second_min, second_max) = second.project(axis);
    let positive = projection_gap(second, first, axis, second_max, first_min);
    let negative = projection_gap(first, second, axis, first_max, second_min);
    if positive <= negative {
        (axis, positive)
    } else {
        (-axis, negative)
    }
}

fn projection_gap(
    high_hull: Hull,
    low_hull: Hull,
    axis: Vector3,
    high: Number,
    low: Number,
) -> Number {
    let rounded = high - low;
    let scale = high_hull
        .projection_scale()
        .max(low_hull.projection_scale());
    let margin = scale / n(1 << 19);
    if rounded.abs() > margin {
        return rounded;
    }
    projection_gap_exact(high_hull, low_hull, axis)
}

pub(crate) fn projection_gap_exact(high: Hull, low: Hull, axis: Vector3) -> Number {
    let [a, b, c, d] = high.support(axis);
    let [e, f, g, h] = low.support(-axis);
    dot_sum(axis, [a, b, c, d, -e, -f, -g, -h])
}
