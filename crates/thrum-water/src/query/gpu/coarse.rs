//! Lay out shared coarse rectangles for queries of the same forward sheet.
use super::{GROUP_WIDTH, Point};
use std::collections::HashMap;
use thrum_water_model::{CutConfig, ThrowReach};

#[derive(Clone, Copy)]
struct Rect {
    lo: [i32; 2],
    hi: [i32; 2],
}

impl Rect {
    fn union(self, other: Self) -> Self {
        Self {
            lo: std::array::from_fn(|a| self.lo[a].min(other.lo[a])),
            hi: std::array::from_fn(|a| self.hi[a].max(other.hi[a])),
        }
    }

    fn width(self) -> u32 {
        self.hi[0].abs_diff(self.lo[0]) + 1
    }

    fn nodes(self) -> u64 {
        u64::from(self.width()) * u64::from(self.hi[1].abs_diff(self.lo[1]) + 1)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SheetKey {
    body: u32,
    stage: u32,
    level: u32,
    gains: [u32; 4],
    time: u32,
}

struct Grid {
    bounds: Rect,
    members: Vec<usize>,
}

pub(super) struct CoarsePlan {
    pub(super) jobs: Vec<[u32; 4]>,
    pub(super) records: u64,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "world lattice coordinates are checked before upload"
)]
fn fine_bounds(point: [f32; 2], reach: f64, cut: CutConfig) -> Option<Rect> {
    let lo = point.map(|p| ((f64::from(p) - reach) / cut.cell).floor());
    let hi = point.map(|p| ((f64::from(p) + reach) / cut.cell).ceil() + 1.0);
    if lo.into_iter().chain(hi).any(|v| !(-1e9..=1e9).contains(&v)) {
        return None;
    }
    Some(Rect {
        lo: lo.map(|v| v as i32),
        hi: hi.map(|v| v as i32),
    })
}

impl CoarsePlan {
    pub(super) fn new(
        points: &mut [Point],
        reach: &ThrowReach<'_>,
        cut: CutConfig,
        max_records: u64,
        max_groups: u32,
    ) -> Self {
        let mut sheets: HashMap<SheetKey, Vec<usize>> = HashMap::new();
        let mut grids: Vec<Grid> = Vec::new();
        let step = i32::try_from(cut.coarse_cells).expect("prepared cut controls");
        for (index, point) in points.iter_mut().enumerate() {
            if point.body == u32::MAX || point.has_bed == 0 {
                continue;
            }
            let (distance, throws) = reach.at(point.position.map(f64::from));
            if !throws {
                continue;
            }
            let Some(fine) = fine_bounds(point.position, distance, cut) else {
                continue;
            };
            point.fine_bounds = [fine.lo[0], fine.lo[1], fine.hi[0], fine.hi[1]];
            let bounds = Rect {
                lo: fine.lo.map(|v| v.div_euclid(step)),
                hi: fine.hi.map(|v| -(-v).div_euclid(step)),
            };
            let owned = sheets
                .entry(SheetKey {
                    body: point.body,
                    stage: point.surf_stage[0].to_bits(),
                    level: point.level_current_bed[0].to_bits(),
                    gains: point.gains.map(f32::to_bits),
                    time: point.dt.to_bits(),
                })
                .or_default();
            // A nearby rectangle of the same sheet can share its nodes even
            // across two game bodies. Avoid growing a rectangle across empty
            // water when separate reaches would evaluate fewer nodes.
            if let Some((grid, union)) = owned.iter().rev().find_map(|&i| {
                let existing = grids[i].bounds;
                let union = existing.union(bounds);
                (union.nodes() <= existing.nodes() + bounds.nodes()).then_some((i, union))
            }) {
                grids[grid].bounds = union;
                grids[grid].members.push(index);
            } else {
                owned.push(grids.len());
                grids.push(Grid {
                    bounds,
                    members: vec![index],
                });
            }
        }

        let mut plan = Self {
            jobs: Vec::new(),
            records: 0,
        };
        for grid in grids {
            let nodes = grid.bounds.nodes();
            let groups = nodes.div_ceil(u64::from(GROUP_WIDTH));
            if plan.records + nodes + groups > max_records
                || plan.jobs.len() as u64 + groups > u64::from(max_groups)
            {
                // Large disjoint reaches use the same two passes, evaluating
                // coarse corners locally without retaining a shared rectangle.
                continue;
            }
            let offset = u32::try_from(plan.records - plan.jobs.len() as u64)
                .expect("coarse storage fits a storage binding");
            let first = u32::try_from(grid.members[0]).expect("point index");
            let width = grid.bounds.width();
            let count = u32::try_from(nodes).expect("coarse storage fits a storage binding");
            for start in (0..count).step_by(GROUP_WIDTH as usize) {
                plan.jobs.push([first, start, offset, count]);
            }
            for index in grid.members {
                let point = &mut points[index];
                point.coarse_grid = [
                    grid.bounds.lo[0],
                    grid.bounds.lo[1],
                    i32::try_from(width).expect("coarse grid width"),
                    i32::try_from(offset).expect("coarse storage fits a storage binding"),
                ];
                point.surf_stage[1] = f32::from_bits(first);
                point.surf_stage[2] = f32::from_bits(count);
            }
            plan.records += nodes + groups;
        }
        plan
    }
}
