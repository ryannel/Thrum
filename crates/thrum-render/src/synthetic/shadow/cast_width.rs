//! Conservative inscribed diameters for the Sun caster size rule.

use glam::DVec3;
use std::collections::BTreeMap;

use super::super::mesh_arena::MeshVertex;

/// A sampled interior point gives a lower bound: no ball centred there can
/// cross the nearest triangle. Search at registration, independently of the Sun.
#[expect(
    clippy::cast_possible_truncation,
    reason = "The mesh and GPU width use f32"
)]
pub(in crate::synthetic) fn measure(
    vertices: &[MeshVertex],
    indices: &[u32],
    [minimum, maximum]: [[f32; 3]; 2],
) -> f32 {
    let minimum = DVec3::from_array(minimum.map(f64::from));
    let maximum = DVec3::from_array(maximum.map(f64::from));
    let span = maximum - minimum;
    if span.max_element() == 0.0 {
        return 0.0;
    }
    let points: Vec<_> = vertices
        .iter()
        .map(|vertex| DVec3::from_array(vertex.position.map(f64::from)))
        .collect();
    let mut edges = BTreeMap::new();
    let mut triangles = Vec::with_capacity(indices.len() / 3);
    for &[a, b, c] in indices.as_chunks::<3>().0 {
        let triangle = [a, b, c].map(|index| points[index as usize]);
        let [a, b, c] = triangle;
        let normal = (b - a).cross(c - a);
        if normal.length_squared() == 0.0 {
            continue;
        }
        triangles.push((triangle, normal.normalize()));
        for (a, b) in [(a, b), (b, c), (c, a)] {
            let key = |p: DVec3| p.to_array().map(|v| if v == 0.0 { 0 } else { v.to_bits() });
            let mut edge = [key(a), key(b)];
            edge.sort_unstable();
            edges.entry(edge).or_insert(([a, b], 0)).1 += 1;
        }
    }
    let Some(&(first, normal)) = triangles.first() else {
        return 0.0;
    };
    let closed = edges.values().all(|&(_, count)| count % 2 == 0);
    let boundary: Vec<_> = if closed {
        Vec::new()
    } else {
        // An open surface only has a cast width when it contains a planar disc.
        if points
            .iter()
            .any(|&p| normal.dot(p - first[0]).abs() > 1e-10 * span.max_element())
        {
            return 0.0;
        }
        edges
            .values()
            .filter_map(|&(edge, count)| (count % 2 != 0).then_some(edge))
            .collect()
    };

    let centre = (minimum + maximum) * 0.5;
    let axis = DVec3::new(1.0, 0.357_406_744, 0.709_811_225).normalize();
    let mut best = 0.0_f64;
    for [x, y, z] in (0..5).flat_map(|x| (0..5).flat_map(move |y| (0..5).map(move |z| [x, y, z]))) {
        let offset = DVec3::new(f64::from(x - 2), f64::from(y - 2), f64::from(z - 2));
        let sample = centre + span * offset * 0.18;
        let point = if closed {
            sample
        } else {
            sample - normal * normal.dot(sample - first[0])
        };
        if closed {
            let mut intersections = 0;
            let mut radius_squared = f64::INFINITY;
            for &([a, b, c], normal) in &triangles {
                radius_squared =
                    radius_squared.min(triangle_distance_squared(point, [a, b, c], normal));
                if radius_squared <= best * best {
                    break;
                }
                let divisor = normal.dot(axis);
                if divisor.abs() > 1e-10 {
                    let ray = normal.dot(a - point) / divisor;
                    if ray > 0.0 && inside_triangle(point + ray * axis, [a, b, c], normal) {
                        intersections += 1;
                    }
                }
            }
            if intersections % 2 != 0 {
                best = best.max(radius_squared.sqrt());
            }
        } else if triangles
            .iter()
            .any(|&(triangle, normal)| inside_triangle(point, triangle, normal))
        {
            let radius_squared = boundary
                .iter()
                .map(|&edge| segment_distance_squared(point, edge))
                .fold(f64::INFINITY, f64::min);
            if radius_squared.is_finite() {
                best = best.max(radius_squared.sqrt());
            }
        }
    }
    // Round inward when recording the width in the GPU's precision.
    (2.0 * (1.0 - f64::from(f32::EPSILON)) * best) as f32
}

fn inside_triangle(p: DVec3, [a, b, c]: [DVec3; 3], n: DVec3) -> bool {
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .all(|(a, b)| (b - a).cross(p - a).dot(n) >= 0.0)
}

fn segment_distance_squared(p: DVec3, [a, b]: [DVec3; 2]) -> f64 {
    let d = b - a;
    let fraction = ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0);
    p.distance_squared(a + d * fraction)
}

fn triangle_distance_squared(p: DVec3, [a, b, c]: [DVec3; 3], n: DVec3) -> f64 {
    let depth = n.dot(p - a);
    if inside_triangle(p - depth * n, [a, b, c], n) {
        depth * depth
    } else {
        [[a, b], [b, c], [c, a]]
            .into_iter()
            .map(|edge| segment_distance_squared(p, edge))
            .fold(f64::INFINITY, f64::min)
    }
}
