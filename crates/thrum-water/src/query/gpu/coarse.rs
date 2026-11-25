//! Lay out shared coarse rectangles for queries of the same forward sheet.
use super::{GROUP_WIDTH, Point};
use std::collections::HashMap;
use thrum_water_model::{BodyKind, CutConfig, Map, ThrowReach};

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
    pub(super) masks: Vec<[u32; 4]>,
    pub(super) records: u64,
    pub(super) coarse_nodes: u32,
    pub(super) fine_groups: u32,
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
        map: &Map,
        reach: &mut ThrowReach<'_>,
        cut: CutConfig,
        max_records: u64,
        max_groups: u32,
        bed_ranges: &mut HashMap<[i32; 2], [f64; 2]>,
    ) -> Self {
        let mut sheets: HashMap<SheetKey, Vec<usize>> = HashMap::new();
        let mut grids: Vec<Grid> = Vec::new();
        let step = i32::try_from(cut.coarse_cells).expect("prepared cut controls");
        for (index, point) in points.iter_mut().enumerate() {
            if point.body == u32::MAX || point.flags == 0 {
                continue;
            }
            let (distance, throws) = reach.at(point.position.map(f64::from));
            let Some(fine) = fine_bounds(point.position, distance, cut) else {
                continue;
            };
            let bounds = Rect {
                lo: fine.lo.map(|v| v.div_euclid(step)),
                hi: fine.hi.map(|v| -(-v).div_euclid(step)),
            };
            if one_sea_owner(map, point.body, bounds, cut) {
                point.flags |= super::QUERY_ONE_SHEET;
            }
            if !throws {
                continue;
            }
            point.fine_bounds = [fine.lo[0], fine.lo[1], fine.hi[0], fine.hi[1]];
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
            masks: Vec::new(),
            records: 0,
            coarse_nodes: 0,
            fine_groups: 0,
        };
        let mut fine = Vec::new();
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
            let unshared_owner = one_sea_owner(map, points[grid.members[0]].body, grid.bounds, cut);
            let begin = plan.masks.len();
            plan.masks.resize(begin + count as usize, [0, 0, 0, 3]);
            let masks = &mut plan.masks[begin..];
            if unshared_owner {
                let stations = grid
                    .members
                    .iter()
                    .map(|&i| points[i].position.map(f64::from));
                mask_cells(
                    masks,
                    grid.bounds,
                    cut,
                    &stations,
                    f64::from(points[grid.members[0]].level_current_bed[0]),
                    map,
                    reach,
                    bed_ranges,
                );
            }
            let evaluated = u32::try_from(masks.iter().filter(|record| record[3] & 2 != 0).count())
                .expect("coarse node count");
            for start in (0..count).step_by(GROUP_WIDTH as usize) {
                plan.jobs.push([first, start, offset, count]);
            }
            for &index in &grid.members {
                let point = &mut points[index];
                point.coarse_grid = [
                    grid.bounds.lo[0],
                    grid.bounds.lo[1],
                    i32::try_from(width).expect("coarse grid width"),
                    i32::try_from(offset).expect("coarse storage fits a storage binding"),
                ];
                if !unshared_owner {
                    point.flags &= !super::QUERY_ONE_SHEET;
                }
                point.surf_stage[1] = f32::from_bits(first);
                point.surf_stage[2] = f32::from_bits(evaluated);
            }
            plan.records += nodes + groups;
            if cut.coarse_cells <= 8 && grid.members.len() >= 2 {
                fine.push((
                    grid.members,
                    nodes,
                    plan.jobs.len() as u64 * u64::from(GROUP_WIDTH),
                ));
            }
        }
        plan.coarse_nodes = u32::try_from(plan.records - plan.jobs.len() as u64)
            .expect("bounded coarse node count");
        let fine_records = 2 * u64::from(plan.coarse_nodes);
        if plan.records + fine_records > max_records {
            return plan;
        }
        let mut fine_offset = 0_u64;
        let per_cell = u64::from((cut.coarse_cells + 1).pow(2));
        // Reserve at most 16 MiB of dense fine storage per ring slot.
        // Small station groups and grids beyond that limit cut locally.
        let fine_limit = (16 * 1024 * 1024) / 16;
        for (members, nodes, groups) in fine {
            let storage = nodes * per_cell;
            if plan.records + fine_records + storage > max_records
                || fine_offset + storage > fine_limit
            {
                continue;
            }
            for index in members {
                points[index].surf_stage[3] =
                    f32::from_bits(u32::try_from(fine_offset + 1).expect("fine storage offset"));
            }
            plan.fine_groups = u32::try_from(groups / u64::from(GROUP_WIDTH))
                .expect("bounded fine selection group count");
            plan.records += storage;
            fine_offset += storage;
        }
        if plan.fine_groups != 0 {
            plan.records += fine_records;
        }
        plan
    }
}

fn mask_cells(
    masks: &mut [[u32; 4]],
    bounds: Rect,
    cut: CutConfig,
    stations: &(impl Iterator<Item = [f64; 2]> + Clone),
    level: f64,
    map: &Map,
    reach: &ThrowReach<'_>,
    bed_ranges: &mut HashMap<[i32; 2], [f64; 2]>,
) {
    let cell = cut.cell * f64::from(cut.coarse_cells);
    let width = bounds.width() as usize;
    let pyramid = map.surf_bed_pyramid().expect("prepared surf");
    let (origin, spacing, size) = map.bed().lattice();
    let end: [f64; 2] =
        std::array::from_fn(|axis| origin[axis] + (size[axis] - 1) as f64 * spacing);
    // Bound cache reuse is spatial. Moving fleets can accumulate different
    // coarse rectangles, so keep only a bounded working set.
    if bed_ranges.len() > 65_536 {
        bed_ranges.clear();
    }
    masks.fill([0; 4]);
    for (row, z) in (bounds.lo[1]..bounds.hi[1]).enumerate() {
        for (column, x) in (bounds.lo[0]..bounds.hi[0]).enumerate() {
            let lo = [f64::from(x) * cell, f64::from(z) * cell];
            let hi = lo.map(|metres| metres + cell);
            let centre = lo.map(|metres| metres + 0.5 * cell);
            let [low_bed, high_bed] = *bed_ranges.entry([x, z]).or_insert_with(|| {
                // Keep neighbouring bed samples at a floating-point boundary.
                let [low, high] = pyramid
                    .range(centre, 0.5 * cell + 0.001)
                    .unwrap_or([f64::NEG_INFINITY; 2]);
                if (0..2).any(|axis| lo[axis] < origin[axis] || hi[axis] > end[axis]) {
                    [f64::NEG_INFINITY, high]
                } else {
                    [low, high]
                }
            });
            let bound = reach.cell_bound([level - high_bed, level - low_bed]) + 0.01;
            let admits = stations.clone().any(|point| {
                (0..2)
                    .all(|axis| lo[axis] - bound <= point[axis] && point[axis] <= hi[axis] + bound)
            });
            if admits {
                let index = row * width + column;
                // Bit one marks a possible cell, bit two marks each of its four nodes.
                masks[index][3] |= 3;
                for at in [index + 1, index + width, index + width + 1] {
                    masks[at][3] |= 2;
                }
            }
        }
    }
}

fn one_sea_owner(map: &Map, sea: u32, bounds: Rect, cut: CutConfig) -> bool {
    let Some(body) = map.body(sea as usize) else {
        return false;
    };
    if !matches!(body.kind, BodyKind::Sea) {
        return false;
    }
    let cell = cut.cell * f64::from(cut.coarse_cells);
    // Include the derivative probe and GPU rounding outside the last node.
    let padding = cut.cell + 0.01;
    let world = [
        bounds.lo.map(|x| f64::from(x) * cell - padding),
        bounds.hi.map(|x| f64::from(x) * cell + padding),
    ];
    (0..)
        .map_while(|id| map.body(id).map(|body| (id, body)))
        .filter(|&(id, _)| id != sea as usize)
        .all(|(id, body)| {
            !matches!(body.kind, BodyKind::Chart(_))
                && map.surface_bounds(id).is_none_or(|other| {
                    (0..2).any(|a| world[0][a] > other[1][a] || world[1][a] < other[0][a])
                })
        })
}
