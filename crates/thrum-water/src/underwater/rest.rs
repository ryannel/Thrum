//! Rest-lattice bounds shared by admission and the surface mesh.

#[derive(Clone, Copy)]
struct Grid {
    origin: [f64; 2],
    first: [i32; 2],
    last: [i32; 2],
    spacing: f64,
    inner: i32,
    camera_relative: bool,
    vertices: Vertices,
}

#[derive(Clone, Copy)]
pub(crate) enum Vertices {
    Indexed { first: u32, surf_diagonal: bool },
    Expanded { first: u32 },
    Hollows { first: u32 },
}

#[derive(Default)]
pub(crate) struct RestMesh(Vec<Grid>);

impl RestMesh {
    pub(crate) fn cell_capacity(&self) -> u64 {
        self.0
            .iter()
            .map(|grid| {
                let [width, height] = std::array::from_fn(|i| {
                    u64::try_from(grid.last[i] - grid.first[i]).expect("ordered water grid")
                });
                let hole = 2 * u64::try_from(grid.inner).expect("nonnegative ring hole");
                width * height - hole * hole
            })
            .sum()
    }

    pub(crate) fn grid(
        &mut self,
        first: [i32; 2],
        last: [i32; 2],
        spacing: f32,
        inner: i32,
        camera_relative: bool,
        vertices: Vertices,
    ) {
        if (0..2).any(|axis| first[axis] >= last[axis]) {
            return;
        }
        self.0.push(Grid {
            origin: [0.0; 2],
            first,
            last,
            spacing: f64::from(spacing),
            inner,
            camera_relative,
            vertices,
        });
    }

    pub(crate) fn hollows(&mut self, origin: [f64; 2], spacing: f64, size: [usize; 2], first: u32) {
        self.0.push(Grid {
            origin,
            first: [0; 2],
            last: size.map(|size| i32::try_from(size).expect("bounded hollows")),
            spacing,
            inner: 0,
            camera_relative: false,
            vertices: Vertices::Hollows { first },
        });
    }

    pub(super) fn bounds(
        &self,
        [low, high]: [[f64; 2]; 2],
        anchor: [f64; 2],
    ) -> Option<[[f64; 2]; 2]> {
        let mut bounds: Option<[[f64; 2]; 2]> = None;
        for grid in &self.0 {
            let Some(cells) = grid.cell_bounds([low, high], anchor) else {
                continue;
            };
            let origin = grid.world_origin(anchor);
            for [first, last] in grid.sections(cells) {
                let lo =
                    std::array::from_fn(|axis| first[axis].mul_add(grid.spacing, origin[axis]));
                let hi = std::array::from_fn(|axis| last[axis].mul_add(grid.spacing, origin[axis]));
                bounds = Some(bounds.map_or([lo, hi], |[prior_lo, prior_hi]| {
                    [
                        std::array::from_fn(|axis| prior_lo[axis].min(lo[axis])),
                        std::array::from_fn(|axis| prior_hi[axis].max(hi[axis])),
                    ]
                }));
            }
        }
        bounds
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "grid sections contain integral i32 cell coordinates"
    )]
    pub(super) fn cells<'a>(
        &'a self,
        footprint: [[f64; 2]; 2],
        anchor: [f64; 2],
        hollows: Option<&'a crate::prepare::hollow_tiles::HollowTiles>,
    ) -> impl Iterator<Item = [u32; 4]> + 'a {
        self.0.iter().flat_map(move |grid| {
            grid.cell_bounds(footprint, anchor)
                .into_iter()
                .flat_map(|bounds| grid.sections(bounds))
                .flat_map(move |[lo, hi]| {
                    let [x0, z0] = lo.map(|v| v as i32);
                    let [x1, z1] = hi.map(|v| v as i32);
                    (z0..z1)
                        .flat_map(move |z| (x0..x1).map(move |x| grid.corners(x, z)))
                        .filter(move |corners| {
                            !matches!(grid.vertices, Vertices::Hollows { .. })
                                || hollows.is_some_and(|tiles| tiles.draws_cell_at(corners[0]))
                        })
                })
        })
    }
}

impl Grid {
    #[expect(
        clippy::cast_sign_loss,
        reason = "the cell comes from this grid's checked section"
    )]
    fn corners(self, x: i32, z: i32) -> [u32; 4] {
        let width = (self.last[0] - self.first[0]) as u32;
        let column = (x - self.first[0]) as u32;
        let row = (z - self.first[1]) as u32;
        match self.vertices {
            Vertices::Indexed {
                first,
                surf_diagonal,
            } => {
                let stride = width + 1;
                let a = first + row * stride + column;
                if surf_diagonal {
                    return [a + 1, a, a + stride + 1, a + stride];
                }
                [a, a + 1, a + stride, a + stride + 1]
            }
            Vertices::Expanded { first } => {
                let inner = self.inner;
                let prior_inner_rows = (z + inner).clamp(0, 2 * inner) as u32;
                let skipped_in_row = if (-inner..inner).contains(&z) && x >= inner {
                    (2 * inner) as u32
                } else {
                    0
                };
                let a = first
                    + 6 * (row * width + column
                        - prior_inner_rows * (2 * inner) as u32
                        - skipped_in_row);
                [a, a + 2, a + 1, a + 5]
            }
            Vertices::Hollows { first } => {
                let a = first + 4 * (row * width + column);
                [a, a + 1, a + 2, a + 3]
            }
        }
    }
    fn world_origin(self, anchor: [f64; 2]) -> [f64; 2] {
        if self.camera_relative {
            anchor
        } else {
            self.origin
        }
    }

    fn cell_bounds(self, [low, high]: [[f64; 2]; 2], anchor: [f64; 2]) -> Option<[[f64; 2]; 2]> {
        let origin = self.world_origin(anchor);
        let mut first = [0.0; 2];
        let mut last = [0.0; 2];
        for axis in 0..2 {
            // Inclusive contact keeps both cells when a lens point falls
            // exactly on a grid edge. Bound f32 vertex placement as well.
            let rounding = 8.0
                * f64::from(f32::EPSILON)
                * origin[axis]
                    .abs()
                    .max(low[axis].abs())
                    .max(high[axis].abs())
                    .max(1.0);
            let lo = (low[axis] - rounding - origin[axis]) / self.spacing;
            let hi = (high[axis] + rounding - origin[axis]) / self.spacing;
            let limit_lo = f64::from(self.first[axis]);
            let limit_hi = f64::from(self.last[axis]);
            if lo > limit_hi || hi < limit_lo {
                return None;
            }
            first[axis] = lo.floor().clamp(limit_lo, limit_hi - 1.0);
            last[axis] = (hi.floor() + 1.0).clamp(limit_lo + 1.0, limit_hi);
        }
        Some([first, last])
    }

    fn sections(
        self,
        [[left, bottom], [right, top]]: [[f64; 2]; 2],
    ) -> impl Iterator<Item = [[f64; 2]; 2]> {
        let inner = f64::from(self.inner);
        // A sea ring is four nonoverlapping strips. Keep the shared edge
        // vertices when joining the inner and outer ring's candidates.
        let sections = if self.inner == 0 {
            [
                [[left, bottom], [right, top]],
                [[0.0; 2]; 2],
                [[0.0; 2]; 2],
                [[0.0; 2]; 2],
            ]
        } else {
            let mid_left = left.max(-inner);
            let mid_right = right.min(inner);
            [
                [[left, bottom], [right.min(-inner), top]],
                [[left.max(inner), bottom], [right, top]],
                [[mid_left, bottom], [mid_right, top.min(-inner)]],
                [[mid_left, bottom.max(inner)], [mid_right, top]],
            ]
        };
        sections
            .into_iter()
            .filter(|[lo, hi]| (0..2).all(|axis| lo[axis] < hi[axis]))
    }
}

#[cfg(test)]
mod tests {
    use super::{RestMesh, Vertices};

    #[test]
    fn bounds_use_the_reachable_cells_and_skip_the_larger_rings_hole() {
        let mut mesh = RestMesh::default();
        let vertices = Vertices::Indexed {
            first: 0,
            surf_diagonal: false,
        };
        mesh.grid([-32; 2], [32; 2], 0.25, 0, true, vertices);
        mesh.grid([-32; 2], [32; 2], 0.5, 16, true, vertices);
        mesh.grid([0; 2], [16; 2], 4.0, 0, false, vertices);
        assert_eq!(
            mesh.bounds([[101.14, -44.0], [101.20, -43.94]], [101.0, -44.0]),
            Some([[101.0, -44.25], [101.25, -43.75]])
        );
        assert_eq!(
            mesh.bounds([[108.9, -44.1], [109.1, -43.9]], [101.0, -44.0]),
            Some([[108.75, -44.5], [109.5, -43.5]])
        );
    }
}
