use super::{
    BODY_CAPACITY, BodyBounds, GRID_HEIGHT, GRID_WIDTH, Inputs, MirrorRoughness, Optics, SHALLOW,
    SOURCE,
};
use crate::map_gpu::BODY_LIMIT;
use bytemuck::Zeroable;
use glam::{Mat4, UVec2, Vec2, Vec3, Vec4};
use thrum_render::contribution::ContributionFrame;
use thrum_water_model::{BodyKind, Map};

const GRID: UVec2 = UVec2::new(32, 18);
const TILE_CELLS: usize = 8;

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl From<BodyBounds> for Bounds {
    fn from(body: BodyBounds) -> Self {
        Self {
            min: Vec4::from_array(body.min).truncate(),
            max: Vec4::from_array(body.max).truncate(),
        }
    }
}

#[derive(Clone, Copy)]
struct Rectangle {
    low: UVec2,
    high: UVec2,
}

struct View {
    matrix: Mat4,
    inverse: Mat4,
    extent: UVec2,
    grid: UVec2,
}

impl View {
    fn project(&self, bounds: Bounds) -> Option<Rectangle> {
        let corners: [Vec4; 8] = core::array::from_fn(|i| {
            let coordinate = |axis| {
                if i & (1_usize << axis) == 0 {
                    bounds.min[axis]
                } else {
                    bounds.max[axis]
                }
            };
            self.matrix * Vec4::new(coordinate(0), coordinate(1), coordinate(2), 1.0)
        });
        let mut crosses = false;
        for plane in FRUSTUM {
            let outside = corners
                .iter()
                .filter(|point| plane.dot(**point) < 0.0)
                .count();
            if outside == corners.len() {
                return None;
            }
            crosses |= outside != 0;
        }

        let mut low = Vec2::splat(f32::INFINITY);
        let mut high = Vec2::splat(f32::NEG_INFINITY);
        let mut include = |clip: Vec4| {
            if clip.w > 0.0 {
                let uv = clip.truncate().truncate() / clip.w * Vec2::new(0.5, -0.5) + 0.5;
                low = low.min(uv);
                high = high.max(uv);
            }
        };
        if crosses {
            for face in BOX_FACES {
                clip_face(face.map(|index| corners[index]), &mut include);
            }
            // Box faces find every intersection vertex except a frustum corner
            // inside the box, including when the box encloses the entire view.
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [0.0, 1.0] {
                        let clip = Vec4::new(x, y, z, 1.0);
                        let world = self.inverse * clip;
                        if world.w > 0.0 {
                            let point = world.truncate() / world.w;
                            if point.cmpge(bounds.min).all() && point.cmple(bounds.max).all() {
                                include(clip);
                            }
                        }
                    }
                }
            }
        } else {
            corners.into_iter().for_each(include);
        }
        if !low.is_finite() {
            return None;
        }
        // Include the nearest-neighbour depth and colour filter's footprint.
        let pad = Vec2::splat(1.0) / self.extent.as_vec2();
        low -= pad;
        high += pad;
        if low.cmpge(Vec2::ONE).any() || high.cmple(Vec2::ZERO).any() {
            return None;
        }
        let extent = self.extent.as_vec2();
        Some(Rectangle {
            low: (low.max(Vec2::ZERO) * extent).floor().as_uvec2(),
            high: (high.min(Vec2::ONE) * extent).ceil().as_uvec2(),
        })
    }

    fn coarse(&self, rectangle: Rectangle) -> Rectangle {
        Rectangle {
            low: rectangle.low * self.grid / self.extent,
            high: (rectangle.high - UVec2::ONE) * self.grid / self.extent + UVec2::ONE,
        }
    }
}

// WebGPU's homogeneous clip volume. Depth is reverse-Z, so w − z is the near plane.
const FRUSTUM: [Vec4; 6] = [
    Vec4::new(0.0, 0.0, -1.0, 1.0),
    Vec4::Z,
    Vec4::new(1.0, 0.0, 0.0, 1.0),
    Vec4::new(-1.0, 0.0, 0.0, 1.0),
    Vec4::new(0.0, 1.0, 0.0, 1.0),
    Vec4::new(0.0, -1.0, 0.0, 1.0),
];
const BOX_FACES: [[usize; 4]; 6] = [
    [0, 2, 6, 4],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 1, 3, 2],
    [4, 5, 7, 6],
];

fn clip_face(face: [Vec4; 4], include: &mut impl FnMut(Vec4)) {
    // A plane adds at most one vertex to a convex face.
    let mut input = [Vec4::ZERO; 10];
    let mut output = [Vec4::ZERO; 10];
    input[..4].copy_from_slice(&face);
    let mut count = 4;
    for plane in FRUSTUM {
        let mut next = 0;
        let mut previous = input[count - 1];
        let mut previous_side = plane.dot(previous);
        for &point in &input[..count] {
            let side = plane.dot(point);
            if (side < 0.0 && previous_side > 0.0) || (side > 0.0 && previous_side < 0.0) {
                output[next] = previous.lerp(point, previous_side / (previous_side - side));
                next += 1;
            }
            if side >= 0.0 {
                output[next] = point;
                next += 1;
            }
            previous = point;
            previous_side = side;
        }
        if next == 0 {
            return;
        }
        std::mem::swap(&mut input, &mut output);
        count = next;
    }
    input[..count].iter().copied().for_each(include);
}

fn mark(mask: &mut [[[u32; 4]; GRID_WIDTH / 4]; GRID_HEIGHT], rect: Rectangle, bits: u32) {
    for row in &mut mask[rect.low.y as usize..rect.high.y as usize] {
        for x in rect.low.x as usize..rect.high.x as usize {
            row[x / 4][x % 4] |= bits;
        }
    }
}

fn receiving(body: Bounds, water_level: f32, a: f32, theta: f32, reach: f32) -> [Bounds; 2] {
    let top = (body.max.y - water_level)
        .abs()
        .max((body.min.y - water_level).abs());
    let horizontal = 2.0 * (top + a + reach * theta.sin()) * theta.sin();
    let reflection = Bounds {
        min: Vec3::new(
            body.min.x - horizontal,
            water_level - top - 2.0 * (a + reach * theta.sin()),
            body.min.z - horizontal,
        ),
        max: Vec3::new(
            body.max.x + horizontal,
            water_level + a,
            body.max.z + horizontal,
        ),
    };
    let depth = (water_level + a - body.min.y).max(0.0);
    let horizontal = depth * (48.8_f32.to_radians() + theta).tan();
    let refraction = Bounds {
        min: Vec3::new(
            body.min.x - horizontal,
            water_level - a,
            body.min.z - horizontal,
        ),
        max: Vec3::new(
            body.max.x + horizontal,
            water_level + a,
            body.max.z + horizontal,
        ),
    };
    [reflection, refraction]
}

#[derive(Default)]
pub(super) struct Shallows {
    revision: Option<(u64, [u32; 4], MirrorRoughness)>,
    shallow_tiles: Vec<(Bounds, bool)>,
    surface_mirrors: Vec<Bounds>,
}

impl Shallows {
    fn refresh(
        &mut self,
        map: &Map,
        profile: Optics,
        displacement: [f32; 2],
        roughness: MirrorRoughness,
    ) {
        let [horizontal, amplitude] = displacement.map(f64::from);
        let sight = profile.sight_depth();
        let revision = (
            map.spatial_revision(),
            [
                displacement[0],
                displacement[1],
                sight,
                profile.mirror_limit,
            ]
            .map(f32::to_bits),
            roughness,
        );
        if self.revision == Some(revision) {
            return;
        }
        self.revision = Some(revision);
        self.shallow_tiles.clear();
        self.surface_mirrors.clear();
        let bed = map.bed();
        let (_, spacing, [width, length]) = bed.lattice();
        let mut gain_floors = [0.0_f64; BODY_LIMIT];
        let mut body_index = 0;
        while let Some(body) = map.body(body_index) {
            let support = map.support_origins(body_index);
            let surface_mirror = matches!(body.kind, BodyKind::Basin { .. } | BodyKind::Chart(_));
            let mut gain_floor = body.gains[3];
            let mut joined = false;
            if let BodyKind::Chart(chart) = &body.kind {
                // Every join and bank continuation is a convex mix. Ends
                // reference earlier bodies, so their floors include their own joins.
                for end in chart.ends.iter().filter(|end| !end.is_brink()) {
                    gain_floor = gain_floor.min(gain_floors[end.body]);
                    joined = true;
                }
            }
            gain_floors[body_index] = gain_floor;
            for z in (0..length - 1).step_by(TILE_CELLS) {
                for x in (0..width - 1).step_by(TILE_CELLS) {
                    let last_x = (x + TILE_CELLS).min(width - 1);
                    let last_z = (z + TILE_CELLS).min(length - 1);
                    let mut level_min = f64::INFINITY;
                    let mut level_max = f64::NEG_INFINITY;
                    let mut bed_min = f64::INFINITY;
                    let mut bed_max = f64::NEG_INFINITY;
                    for j in z..=last_z {
                        for i in x..=last_x {
                            let index = j * width + i;
                            let height = bed.heights()[index];
                            bed_min = bed_min.min(height);
                            bed_max = bed_max.max(height);
                            if support.get(index).is_some_and(Option::is_some) {
                                let level = map.row(body_index, bed.point(index)).level;
                                level_min = level_min.min(level);
                                level_max = level_max.max(level);
                            }
                        }
                    }
                    if !level_min.is_finite() {
                        continue;
                    }
                    // The shader limits the shortest-band gain at the rest
                    // point. Bilinear bed interpolation cannot exceed bed_max.
                    let depth = level_min - bed_max;
                    let shallow = depth - amplitude < f64::from(sight);
                    if !shallow && !surface_mirror {
                        continue;
                    }
                    // A depth minimum sampled at lattice centres need not
                    // bound the varying join mix between them.
                    let mirror_depth = if joined { 0.0 } else { depth };
                    let mirror = matches!(body.kind, BodyKind::Hollows(_))
                        || roughness.can_mirror(gain_floor, mirror_depth, profile.mirror_limit);
                    if !shallow && !mirror {
                        continue;
                    }
                    let mut lo = bed.point(z * width + x);
                    let mut hi = bed.point(last_z * width + last_x);
                    for axis in 0..2 {
                        lo[axis] -= spacing;
                        hi[axis] += spacing;
                    }
                    if shallow {
                        self.shallow_tiles.push((
                            bounds(
                                lo,
                                hi,
                                bed_min.max(level_min - amplitude - f64::from(sight)),
                                level_max + amplitude,
                            ),
                            !surface_mirror && mirror,
                        ));
                    }
                    // Inland reflection visibility is independent of bed
                    // visibility. Project the displaced surface alone.
                    if surface_mirror && mirror {
                        self.surface_mirrors.push(bounds(
                            lo.map(|v| v - horizontal),
                            hi.map(|v| v + horizontal),
                            level_min - amplitude,
                            level_max + amplitude,
                        ));
                    }
                }
            }
            body_index += 1;
        }
    }

    pub(super) fn inputs(
        &mut self,
        profile: Optics,
        frame: &ContributionFrame,
        map: Option<&Map>,
        boxes: &[BodyBounds],
        level: f64,
        displacement: [f32; 2],
        roughness: MirrorRoughness,
    ) -> Inputs {
        let extent = UVec2::from_array(frame.render_extent);
        let matrix = Mat4::from_cols_array_2d(&frame.view_proj);
        let view = View {
            matrix,
            inverse: matrix.inverse(),
            extent,
            grid: extent.min(GRID),
        };
        let mut packed = Inputs::zeroed();
        let sight = profile.sight_depth();
        let [r, g, b] = profile.extinction;
        packed.extinction_sight = [r, g, b, sight];
        let [r, g, b] = profile.downwelling;
        packed.downwelling_mirror = [r, g, b, profile.mirror_limit];
        let theta = profile.slope_allowance.to_radians();
        packed.reach_slope_scene = [profile.reflection_reach, theta.cos(), 0.0, 0.0];
        for (index, &record) in boxes.iter().take(BODY_CAPACITY).enumerate() {
            let body = Bounds::from(record);
            let Some(source) = view.project(body) else {
                continue;
            };
            let center = 0.5 * (body.min + body.max);
            if (center - Vec3::from_array(frame.camera)).length() > 4000.0 {
                continue;
            }
            let body_level = map
                .and_then(|map| map.sample([f64::from(center.x), f64::from(center.z)]))
                .map_or_else(
                    || sampled_level(level),
                    |sample| sampled_level(sample.row.level),
                );
            packed.bodies[index] = record;
            packed.sources[index] = [source.low.x, source.low.y, source.high.x, source.high.y];
            packed.reach_slope_scene[3] =
                f32::from(u8::try_from(index + 1).expect("bounded body count"));
            mark(&mut packed.mask, view.coarse(source), SOURCE);
            for area in receiving(
                body,
                body_level,
                displacement[1],
                theta,
                profile.reflection_reach,
            ) {
                if let Some(rect) = view.project(area) {
                    mark(&mut packed.mask, view.coarse(rect), 1u32 << index);
                }
            }
        }
        if let Some(map) = map {
            self.refresh(map, profile, displacement, roughness);
            let mut mirror_visible = false;
            for &(tile, mirror) in &self.shallow_tiles {
                if let Some(rect) = view.project(tile) {
                    mirror_visible |= mirror;
                    mark(&mut packed.mask, view.coarse(rect), SOURCE | SHALLOW);
                }
            }
            // An unbounded sea with an empty bed never requests a scene mirror.
            let mirror = mirror_visible
                || self
                    .surface_mirrors
                    .iter()
                    .any(|&area| view.project(area).is_some());
            if mirror {
                packed.reach_slope_scene[2] = 1.0;
                mark(
                    &mut packed.mask,
                    Rectangle {
                        low: UVec2::ZERO,
                        high: view.grid,
                    },
                    SOURCE,
                );
            }
        }
        packed.dispatch = dispatch(&packed, &view);
        packed
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "map GPU projection uses f32 metres"
)]
fn bounds(lo: [f64; 2], hi: [f64; 2], low_y: f64, high_y: f64) -> Bounds {
    Bounds {
        min: Vec3::new(lo[0] as f32, low_y as f32, lo[1] as f32),
        max: Vec3::new(hi[0] as f32, high_y as f32, hi[1] as f32),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "map levels and amplitude gains have GPU f32 counterparts"
)]
fn sampled_level(level: f64) -> f32 {
    level as f32
}

fn dispatch(inputs: &Inputs, view: &View) -> [u32; 4] {
    let mut low = view.grid;
    let mut high = UVec2::ZERO;
    for (y, row) in (0_u32..).zip(inputs.mask.iter()) {
        for (x, &bits) in (0_u32..).zip(row.iter().flatten()) {
            if bits & SOURCE != 0 {
                low = low.min(UVec2::new(x, y));
                high = high.max(UVec2::new(x + 1, y + 1));
            }
        }
    }
    if high == UVec2::ZERO {
        return [0; 4];
    }
    let edge = |tile: UVec2| {
        let edge = tile * view.extent;
        UVec2::new(edge.x.div_ceil(view.grid.x), edge.y.div_ceil(view.grid.y))
    };
    let base = edge(low);
    let size = edge(high) - base;
    [base.x, base.y, size.x, size.y]
}

#[cfg(test)]
mod tests;
