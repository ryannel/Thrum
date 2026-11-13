//! The mixed field's prop meshes: a rock, a column and a spruce, each a few
//! hundred to a few thousand triangles inside the unit cube, so a prop's box
//! half-extents still bound it. Every fifth prop is mirrored. Slots keep their
//! mesh for the whole run; the cube field stays the default content.

use std::f32::consts::TAU;
use std::sync::Arc;

use thrum_render::synthetic::{
    SyntheticMesh, SyntheticMeshChoice, SyntheticMeshes, SyntheticVertex,
};

use crate::field;

/// Props per mirrored prop.
pub(crate) const MIRRORED_EVERY: usize = 5;

/// Whether a prop slot draws mirrored: its basis x column is negated.
pub(crate) fn mirrored(index: usize) -> bool {
    index > field::WALL_COUNT && index.is_multiple_of(MIRRORED_EVERY)
}

/// The authored meshes and each slot's choice: the ground and walls keep the
/// cube; props cycle through the three meshes by a hash of their slot.
pub(crate) fn selection() -> Arc<SyntheticMeshes> {
    selection_for(field::ENTITY_COUNT, true)
}

/// Reuse the bounded prop arena for the valley's entity count as well.
pub(crate) fn selection_for(count: usize, keep_walls: bool) -> Arc<SyntheticMeshes> {
    let slots = (0..count)
        .map(|index| {
            if keep_walls && index <= field::WALL_COUNT {
                SyntheticMeshChoice::Cube
            } else {
                SyntheticMeshChoice::Mesh(
                    u32::try_from(field::pick(index, 8, 3)).expect("three meshes"),
                )
            }
        })
        .collect();
    Arc::new(SyntheticMeshes {
        meshes: vec![rock(), column(), spruce()],
        slots,
    })
}

/// Flat-shaded triangles, wound counter-clockwise from outside.
struct Builder {
    vertices: Vec<SyntheticVertex>,
}

impl Builder {
    fn triangle(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        let u = sub(b, a);
        let v = sub(c, a);
        let n = cross(u, v);
        let length = dot(n, n).sqrt();
        if length <= 1e-12 {
            return;
        }
        let normal = n.map(|value| value / length);
        let t = u.map(|value| value / dot(u, u).sqrt());
        for position in [a, b, c] {
            self.vertices.push(SyntheticVertex {
                position,
                normal,
                uv: [position[0] * 0.5 + 0.5, position[1] * 0.5 + 0.5],
                tangent: [t[0], t[1], t[2], 1.0],
            });
        }
    }

    fn finish(self) -> SyntheticMesh {
        let count = u32::try_from(self.vertices.len()).expect("small mesh");
        SyntheticMesh {
            vertices: self.vertices,
            indices: (0..count).collect(),
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let length = dot(a, a).sqrt();
    a.map(|value| value / length)
}

/// A boulder: an icosahedron subdivided three times (1,280 triangles), its
/// radius perturbed by a few fixed low-frequency lobes and flattened below.
fn rock() -> SyntheticMesh {
    let t = f32::midpoint(1.0, 5.0_f32.sqrt());
    let mut points: Vec<[f32; 3]> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .map(normalize)
    .to_vec();
    let mut faces: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..3 {
        let mut midpoints = std::collections::HashMap::new();
        let mut midpoint = |a: usize, b: usize, points: &mut Vec<[f32; 3]>| {
            *midpoints.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let (p, q) = (points[a], points[b]);
                points.push(normalize(std::array::from_fn(|axis| {
                    f32::midpoint(p[axis], q[axis])
                })));
                points.len() - 1
            })
        };
        faces = faces
            .iter()
            .flat_map(|&[a, b, c]| {
                let ab = midpoint(a, b, &mut points);
                let bc = midpoint(b, c, &mut points);
                let ca = midpoint(c, a, &mut points);
                [[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
            })
            .collect();
    }
    let lobes = [
        ([0.6, 0.3, 0.74], 0.12),
        ([-0.5, 0.2, 0.84], 0.09),
        ([0.1, -0.9, 0.42], 0.07),
        ([-0.7, 0.6, -0.39], 0.1),
    ];
    let shaped: Vec<[f32; 3]> = points
        .iter()
        .map(|&p| {
            let bump: f32 = lobes
                .iter()
                .map(|&(axis, amount)| amount * dot(p, normalize(axis)).powi(3))
                .sum();
            let radius = 0.82 + bump;
            let y = (p[1] * radius).max(-0.7);
            [p[0] * radius, y, p[2] * radius]
        })
        .collect();
    let mut builder = Builder {
        vertices: Vec::new(),
    };
    for [a, b, c] in faces {
        // The icosahedron's faces wind counter-clockwise from outside.
        builder.triangle(shaped[a], shaped[b], shaped[c]);
    }
    builder.finish()
}

/// A surface of revolution about +Y through `profile` (radius, height) points
/// listed bottom to top, capped at both ends.
fn lathe(profile: &[(f32, f32)], segments: usize) -> SyntheticMesh {
    let mut builder = Builder {
        vertices: Vec::new(),
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "segment counts are small integers"
    )]
    let ring = |radius: f32, y: f32, segment: usize| {
        let angle = TAU * (segment % segments) as f32 / segments as f32;
        [radius * angle.cos(), y, -radius * angle.sin()]
    };
    for pair in profile.windows(2) {
        let [(r0, y0), (r1, y1)] = [pair[0], pair[1]];
        for segment in 0..segments {
            let (a, b) = (ring(r0, y0, segment), ring(r0, y0, segment + 1));
            let (c, d) = (ring(r1, y1, segment + 1), ring(r1, y1, segment));
            builder.triangle(a, b, c);
            builder.triangle(a, c, d);
        }
    }
    let (bottom, top) = (profile[0], profile[profile.len() - 1]);
    for segment in 0..segments {
        builder.triangle(
            [0.0, bottom.1, 0.0],
            ring(bottom.0, bottom.1, segment + 1),
            ring(bottom.0, bottom.1, segment),
        );
        builder.triangle(
            [0.0, top.1, 0.0],
            ring(top.0, top.1, segment),
            ring(top.0, top.1, segment + 1),
        );
    }
    builder.finish()
}

/// A fluted column with a base and capital: 40 segments over eight bands
/// (720 triangles).
fn column() -> SyntheticMesh {
    lathe(
        &[
            (0.95, -1.0),
            (0.95, -0.85),
            (0.7, -0.75),
            (0.6, -0.6),
            (0.52, 0.6),
            (0.62, 0.72),
            (0.9, 0.82),
            (0.9, 0.92),
            (0.6, 1.0),
        ],
        40,
    )
}

/// A spruce: a trunk under three stacked cones, 32 segments (640 triangles).
fn spruce() -> SyntheticMesh {
    lathe(
        &[
            (0.12, -1.0),
            (0.12, -0.6),
            (0.95, -0.55),
            (0.25, 0.0),
            (0.75, -0.05),
            (0.2, 0.45),
            (0.55, 0.4),
            (0.0, 1.0),
        ],
        32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meshes_have_realistic_counts_and_stay_in_the_unit_box() {
        for (name, mesh, low, high) in [
            ("rock", rock(), 1000, 1400),
            ("column", column(), 600, 800),
            ("spruce", spruce(), 400, 700),
        ] {
            let triangles = mesh.indices.len() / 3;
            assert!(
                (low..=high).contains(&triangles),
                "{name}: {triangles} triangles"
            );
            for vertex in &mesh.vertices {
                assert!(
                    vertex.position.iter().all(|value| value.abs() <= 1.0),
                    "{name} leaves the unit box: {:?}",
                    vertex.position
                );
            }
        }
    }

    #[test]
    fn closed_meshes_wind_outward() {
        for (name, mesh) in [("rock", rock()), ("column", column()), ("spruce", spruce())] {
            // A closed mesh wound counter-clockwise from outside encloses a
            // positive signed volume; its flat normals agree with the winding.
            let volume: f32 = mesh
                .vertices
                .chunks(3)
                .map(|t| dot(t[0].position, cross(t[1].position, t[2].position)) / 6.0)
                .sum();
            assert!(volume > 0.5, "{name}: signed volume {volume}");
            for t in mesh.vertices.chunks(3) {
                let n = cross(
                    sub(t[1].position, t[0].position),
                    sub(t[2].position, t[0].position),
                );
                assert!(dot(n, t[0].normal) > 0.0, "{name}: normal against winding");
            }
        }
    }
}
