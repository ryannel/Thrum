use glam::{Mat4, Vec2, Vec4};
use thrum_water_model::{Sheet, SheetPoint};

pub(super) const EXTENT: [u32; 2] = [96, 64];
pub(super) const CENTER: [f32; 2] = [-28.705, 3.2795];
const SIDE: usize = 41;

#[derive(Clone, Copy)]
pub(super) struct ProjectedNode {
    now: Vec4,
    before: Vec4,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "analytic world positions projected into the submitted f32 camera"
)]
pub(super) fn projected_nodes(
    now: &impl Sheet,
    old: &impl Sheet,
    matrix: Mat4,
    level: [f64; 2],
) -> Vec<ProjectedNode> {
    let project = |p: SheetPoint, stage: f64| {
        matrix
            * Vec4::new(
                p.world[0] as f32,
                (p.height + stage) as f32,
                p.world[1] as f32,
                1.0,
            )
    };
    let origin = CENTER.map(|v| f64::from(v.floor()) - 5.0);
    (0..SIDE)
        .flat_map(|z| {
            (0..SIDE).map(move |x| [origin[0] + x as f64 * 0.25, origin[1] + z as f64 * 0.25])
        })
        .map(|rest| ProjectedNode {
            now: project(now.forward(rest), level[0]),
            before: project(old.forward(rest), level[1]),
        })
        .collect()
}

pub(super) fn cut_ray(nodes: &[ProjectedNode], pixel: Vec2) -> Vec<(f32, Vec2)> {
    let mut hits = Vec::new();
    for row in 0..SIDE - 1 {
        for column in 0..SIDE - 1 {
            let a = row * SIDE + column;
            let d = a + SIDE + 1;
            for ids in [[a, a + 1, d], [a, d, a + SIDE]] {
                let triangle = ids.map(|i| nodes[i]);
                let ndc = triangle.map(|n| n.now.truncate() / n.now.w);
                let edges = [
                    ndc[1].truncate() - ndc[0].truncate(),
                    ndc[2].truncate() - ndc[0].truncate(),
                ];
                let from_a = pixel - ndc[0].truncate();
                let determinant = edges[0].perp_dot(edges[1]);
                if determinant.abs() < 1e-10 {
                    continue;
                }
                let u = from_a.perp_dot(edges[1]) / determinant;
                let v = edges[0].perp_dot(from_a) / determinant;
                let weights = [1.0 - u - v, u, v];
                if weights.iter().any(|w| *w < 0.0001) {
                    continue;
                }
                let mut previous = Vec4::ZERO;
                let mut depth = 0.0;
                for i in 0..3 {
                    depth += weights[i] * ndc[i].z;
                    previous += (weights[i] / triangle[i].now.w) * triangle[i].before;
                }
                let motion = (previous.truncate().truncate() / previous.w - pixel)
                    * Vec2::new(EXTENT[0] as f32 * 0.5, -(EXTENT[1] as f32) * 0.5);
                hits.push((depth, motion));
            }
        }
    }
    hits.sort_by(|a, b| b.0.total_cmp(&a.0));
    hits
}
