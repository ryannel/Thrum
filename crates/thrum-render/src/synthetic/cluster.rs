//! Compact, deterministic 3D light lists for a checked synthetic view.
//! The source registry is external; these are prepared per-view GPU products.

use super::RenderError;
use super::view::{Projection, SyntheticView};
use glam::Vec3;
use std::cmp::Ordering;

pub(super) const X: usize = 32;
pub(super) const Y: usize = 18;
pub(super) const Z: usize = 16;
const COUNT: usize = X * Y * Z;
const CAPACITY: usize = 32;
pub(super) const GPU_MAX_SOURCES: usize = 4;

/// One finite-radius local source in the synthetic fixture. Position is in
/// the fixture's world coordinates and intensity is linear RGB candela.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticPointLight {
    /// World-space position.
    pub position: [f32; 3],
    /// Positive finite influence radius in world units.
    pub radius: f32,
    /// Finite nonnegative RGB intensity.
    pub candela: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Candidate {
    index: u32,
    score: f32,
}

#[derive(Clone, Copy)]
struct Slot {
    len: u8,
    candidates: [Candidate; CAPACITY],
}

enum ScoreGeometry {
    Orthographic,
    Perspective {
        view: glam::Mat4,
        fx: f32,
        fy: f32,
        depths: [f32; Z],
    },
}

impl ScoreGeometry {
    fn new(view: &SyntheticView) -> Self {
        match view.projection() {
            Projection::OrthographicFixture => Self::Orthographic,
            Projection::Perspective {
                view,
                fx,
                fy,
                near,
                far,
            } => Self::Perspective {
                view,
                fx,
                fy,
                depths: std::array::from_fn(|z| {
                    near * (far / near).powf((z as f32 + 0.5) / Z as f32)
                }),
            },
        }
    }

    fn light_position(&self, world: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Orthographic => world,
            Self::Perspective { view, .. } => {
                view.transform_point3(Vec3::from_array(world)).to_array()
            }
        }
    }

    fn center(&self, x: usize, y: usize, z: usize) -> [f32; 3] {
        let nx = -1.0 + 2.0 * (x as f32 + 0.5) / X as f32;
        let ny = -1.0 + 2.0 * (y as f32 + 0.5) / Y as f32;
        match self {
            Self::Orthographic => [nx, ny, (z as f32 + 0.5) / Z as f32],
            Self::Perspective { fx, fy, depths, .. } => {
                let depth = depths[z];
                [nx * depth / fx, ny * depth / fy, -depth]
            }
        }
    }
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            len: 0,
            candidates: [Candidate::default(); CAPACITY],
        }
    }
}

/// GPU bytes and diagnosed quality degradation. Empty arrays carry one dummy
/// record so all four bind-group entries remain valid on baseline WebGPU.
pub(super) struct ClusterTable {
    pub(super) lights: Vec<u8>,
    pub(super) heads: Vec<u8>,
    pub(super) indices: Vec<u8>,
    pub(super) bounds: Vec<u8>,
    pub(super) dropped_references: usize,
    pub(super) dropped_sources: usize,
    pub(super) active: bool,
    slots: Vec<Slot>,
    max_lights: usize,
}

impl ClusterTable {
    fn empty() -> Self {
        Self {
            lights: vec![0; 32],
            heads: vec![0; 8],
            indices: vec![0; 4],
            bounds: Vec::new(),
            dropped_references: 0,
            dropped_sources: 0,
            active: false,
            slots: Vec::new(),
            max_lights: 0,
        }
    }

    pub(super) fn build(
        lights: &[SyntheticPointLight],
        view: &SyntheticView,
    ) -> Result<Self, RenderError> {
        if lights.len() > 4096 {
            return Err(RenderError::SceneCapacity);
        }
        let mut table = Self::empty();
        table.max_lights = lights.len();
        if table.max_lights != 0 {
            table.slots.resize(COUNT, Slot::default());
            table.lights.reserve(table.max_lights * 32);
            table.heads.reserve(COUNT * 8);
            table
                .indices
                .reserve(COUNT * table.max_lights.min(CAPACITY) * 4);
        }
        table.rebuild(lights, view)?;
        Ok(table)
    }

    pub(super) fn build_gpu(
        lights: &[SyntheticPointLight],
        view: &SyntheticView,
    ) -> Result<Self, RenderError> {
        if lights.is_empty() || lights.len() > GPU_MAX_SOURCES {
            return Err(RenderError::SceneCapacity);
        }
        let mut table = Self::empty();
        table.max_lights = lights.len();
        table.lights.reserve(table.max_lights * 32);
        table.bounds.reserve(table.max_lights * 32);
        table.repack_gpu(lights, view)?;
        Ok(table)
    }

    /// Pack only moving source records and conservative projected cell bounds;
    /// GPU work writes the full head/index grid before the forward pass.
    pub(super) fn repack_gpu(
        &mut self,
        lights: &[SyntheticPointLight],
        view: &SyntheticView,
    ) -> Result<(), RenderError> {
        let retained = &lights[..lights.len().min(self.max_lights)];
        for light in retained.iter().copied() {
            validate(light)?;
        }
        self.lights.clear();
        self.bounds.clear();
        for light in retained.iter().copied() {
            pack_light(light, &mut self.lights);
            let cell_bounds = if light.candela.iter().any(|value| *value > 0.0) {
                bounds(light, view)
            } else {
                None
            };
            pack_bounds(cell_bounds, &mut self.bounds);
        }
        for _ in retained.len()..self.max_lights {
            self.lights.extend_from_slice(&[0; 32]);
            pack_bounds(None, &mut self.bounds);
        }
        self.active = retained
            .iter()
            .any(|light| light.candela.iter().any(|value| *value > 0.0));
        self.dropped_sources = lights.len().saturating_sub(self.max_lights);
        self.dropped_references = 0;
        Ok(())
    }

    pub(super) fn capacities(&self) -> [u64; 3] {
        if self.max_lights == 0 {
            return [32, 8, 4];
        }
        [
            (self.max_lights * 32) as u64,
            (COUNT * 8) as u64,
            (COUNT * self.max_lights.min(CAPACITY) * 4) as u64,
        ]
    }

    /// Reuse the bounded candidate and packed-byte capacity across live views.
    pub(super) fn rebuild(
        &mut self,
        lights: &[SyntheticPointLight],
        view: &SyntheticView,
    ) -> Result<(), RenderError> {
        let retained = &lights[..lights.len().min(self.max_lights)];
        for light in retained.iter().copied() {
            validate(light)?;
        }
        self.dropped_sources = lights.len().saturating_sub(self.max_lights);
        let lights = retained;
        if !lights.iter().any(|light| {
            light.candela.iter().any(|value| *value > 0.0) && bounds(*light, view).is_some()
        }) {
            // The shader's uniform off-state returns before reading clusters.
            // Keep valid minimal bindings without preparing a dead 3D grid.
            self.lights.clear();
            self.lights.resize(32, 0);
            self.heads.clear();
            self.heads.resize(8, 0);
            self.indices.clear();
            self.indices.resize(4, 0);
            self.dropped_references = 0;
            self.active = false;
            return Ok(());
        }
        if self.slots.len() == COUNT {
            for slot in &mut self.slots {
                slot.len = 0;
            }
        } else {
            self.slots.resize(COUNT, Slot::default());
        }
        let score_geometry = ScoreGeometry::new(view);
        self.lights.clear();
        self.lights.reserve(self.max_lights * 32);
        self.heads.clear();
        self.heads.reserve(COUNT * 8);
        self.indices.clear();
        self.indices
            .reserve(COUNT * self.max_lights.min(CAPACITY) * 4);
        let mut dropped = 0;
        for (index, light) in lights.iter().copied().enumerate() {
            pack_light(light, &mut self.lights);
            if light.candela.iter().all(|value| *value == 0.0) {
                continue;
            }
            let Some((lo, hi)) = bounds(light, view) else {
                continue;
            };
            let index = u32::try_from(index).map_err(|_| RenderError::SceneCapacity)?;
            let score_position = score_geometry.light_position(light.position);
            let intensity = light.candela.into_iter().fold(0.0, f32::max);
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        if matches!(view.projection(), Projection::OrthographicFixture)
                            && !intersects(light, x, y, z)
                        {
                            continue;
                        }
                        let slot = &mut self.slots[(z * Y + y) * X + x];
                        let candidate = Candidate {
                            index,
                            score: score(score_position, intensity, x, y, z, &score_geometry),
                        };
                        let len = usize::from(slot.len);
                        if len < CAPACITY {
                            slot.candidates[len] = candidate;
                            slot.len += 1;
                        } else {
                            dropped += 1;
                            let worst = slot.candidates.iter().enumerate().min_by(|a, b| {
                                a.1.score
                                    .total_cmp(&b.1.score)
                                    .then_with(|| b.1.index.cmp(&a.1.index))
                            });
                            if let Some((worst_index, weakest)) = worst
                                && (candidate.score.total_cmp(&weakest.score) == Ordering::Greater
                                    || (candidate.score.total_cmp(&weakest.score)
                                        == Ordering::Equal
                                        && candidate.index < weakest.index))
                            {
                                slot.candidates[worst_index] = candidate;
                            }
                        }
                    }
                }
            }
        }
        if self.lights.is_empty() {
            self.lights.resize(32, 0);
        }
        for slot in &mut self.slots {
            let len = usize::from(slot.len);
            slot.candidates[..len].sort_unstable_by_key(|candidate| candidate.index);
            let offset =
                u32::try_from(self.indices.len() / 4).map_err(|_| RenderError::SceneCapacity)?;
            self.heads.extend_from_slice(&offset.to_le_bytes());
            self.heads
                .extend_from_slice(&u32::from(slot.len).to_le_bytes());
            for candidate in &slot.candidates[..len] {
                self.indices
                    .extend_from_slice(&candidate.index.to_le_bytes());
            }
        }
        if self.indices.is_empty() {
            self.indices.resize(4, 0);
            self.active = false;
        } else {
            self.active = true;
        }
        self.dropped_references = dropped;
        Ok(())
    }
}

fn validate(light: SyntheticPointLight) -> Result<(), RenderError> {
    if !light.position.iter().all(|v| v.is_finite())
        || !light.radius.is_finite()
        || !(0.0..=128.0).contains(&light.radius)
        || light.radius == 0.0
        || !light
            .candela
            .iter()
            .all(|v| v.is_finite() && (0.0..=10_000.0).contains(v))
    {
        return Err(RenderError::InvalidLighting);
    }
    Ok(())
}

fn pack_light(light: SyntheticPointLight, out: &mut Vec<u8>) {
    for value in light.position.into_iter().chain([light.radius]) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for value in light.candela.into_iter().chain([0.0]) {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn pack_bounds(bounds: Option<([usize; 3], [usize; 3])>, out: &mut Vec<u8>) {
    let (lo, hi) = bounds.unwrap_or(([usize::MAX; 3], [0; 3]));
    for value in lo
        .into_iter()
        .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
        .chain([0])
        .chain(
            hi.into_iter()
                .map(|value| u32::try_from(value).unwrap_or(u32::MAX)),
        )
        .chain([0])
    {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

// All float-to-index casts are preceded by finite source validation and
// clamping to the exact cluster range. The result is never an array bound.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "validated finite coordinates are clamped into a small exact integer range"
)]
fn grid_index(value: f32, axis: usize) -> usize {
    let extent = if axis == 2 { 1.0 } else { 2.0 };
    let shifted = if axis == 2 { value } else { value + 1.0 };
    let divisions = [X, Y, Z][axis];
    ((shifted / extent * divisions as f32)
        .floor()
        .clamp(0.0, (divisions - 1) as f32)) as usize
}

fn bounds(light: SyntheticPointLight, view: &SyntheticView) -> Option<([usize; 3], [usize; 3])> {
    match view.projection() {
        Projection::OrthographicFixture => bounds_orthographic(light),
        Projection::Perspective {
            view,
            fx,
            fy,
            near,
            far,
        } => bounds_perspective(light, view, fx, fy, near, far),
    }
}

fn bounds_orthographic(light: SyntheticPointLight) -> Option<([usize; 3], [usize; 3])> {
    if !intersects_view(light) {
        return None;
    }
    if light.position[0] + light.radius < -1.0
        || light.position[0] - light.radius > 1.0
        || light.position[1] + light.radius < -1.0
        || light.position[1] - light.radius > 1.0
        || light.position[2] + light.radius < 0.0
        || light.position[2] - light.radius > 1.0
    {
        return None;
    }
    let mut lo = [0; 3];
    let mut hi = [0; 3];
    for axis in 0..3 {
        lo[axis] = grid_index(light.position[axis] - light.radius, axis);
        hi[axis] = grid_index(light.position[axis] + light.radius, axis);
    }
    Some((lo, hi))
}

// Conservative projected sphere bounds. If it crosses the near plane, all XY
// columns are included. False-positive cells cost shading; a false negative
// would make a local light disappear, so this first tier favors coverage.
fn bounds_perspective(
    light: SyntheticPointLight,
    view: glam::Mat4,
    fx: f32,
    fy: f32,
    near: f32,
    far: f32,
) -> Option<([usize; 3], [usize; 3])> {
    let p = view.transform_point3(Vec3::from_array(light.position));
    let depth = -p.z;
    let near_depth = depth - light.radius;
    let far_depth = depth + light.radius;
    if far_depth < near || near_depth > far {
        return None;
    }
    let z_lo = depth_index(near_depth.max(near), near, far);
    let z_hi = depth_index(far_depth.min(far), near, far);
    if near_depth <= near {
        return Some(([0, 0, z_lo], [X - 1, Y - 1, z_hi]));
    }
    let (x_lo, x_hi) = projected_axis(p.x, light.radius, near_depth, far_depth, fx);
    let (y_lo, y_hi) = projected_axis(p.y, light.radius, near_depth, far_depth, fy);
    if x_hi < -1.0 || x_lo > 1.0 || y_hi < -1.0 || y_lo > 1.0 {
        return None;
    }
    Some((
        [ndc_index(x_lo, X), ndc_index(y_lo, Y), z_lo],
        [ndc_index(x_hi, X), ndc_index(y_hi, Y), z_hi],
    ))
}

fn projected_axis(
    center: f32,
    radius: f32,
    near_depth: f32,
    far_depth: f32,
    focal: f32,
) -> (f32, f32) {
    let a = (center - radius) * focal;
    let b = (center + radius) * focal;
    let values = [a / near_depth, a / far_depth, b / near_depth, b / far_depth];
    (
        values.into_iter().fold(f32::INFINITY, f32::min),
        values.into_iter().fold(f32::NEG_INFINITY, f32::max),
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite projected coordinate is clamped to a small cluster axis"
)]
fn ndc_index(value: f32, cells: usize) -> usize {
    ((value.midpoint(1.0) * cells as f32)
        .floor()
        .clamp(0.0, (cells - 1) as f32)) as usize
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite positive depth is clamped to a small cluster axis"
)]
fn depth_index(depth: f32, near: f32, far: f32) -> usize {
    ((depth / near).ln() / (far / near).ln() * Z as f32)
        .floor()
        .clamp(0.0, (Z - 1) as f32) as usize
}

fn axis_distance(center: f32, low: f32, high: f32) -> f32 {
    (low - center).max(0.0).max(center - high)
}

fn intersects_view(light: SyntheticPointLight) -> bool {
    let squared = (0..3)
        .map(|axis| {
            let low = if axis == 2 { 0.0 } else { -1.0 };
            axis_distance(light.position[axis], low, 1.0).powi(2)
        })
        .sum::<f32>();
    squared <= light.radius * light.radius
}

fn intersects(light: SyntheticPointLight, x: usize, y: usize, z: usize) -> bool {
    let low = [
        -1.0 + 2.0 * x as f32 / X as f32,
        -1.0 + 2.0 * y as f32 / Y as f32,
        z as f32 / Z as f32,
    ];
    let high = [
        -1.0 + 2.0 * (x + 1) as f32 / X as f32,
        -1.0 + 2.0 * (y + 1) as f32 / Y as f32,
        (z + 1) as f32 / Z as f32,
    ];
    let squared = (0..3)
        .map(|axis| axis_distance(light.position[axis], low[axis], high[axis]).powi(2))
        .sum::<f32>();
    squared <= light.radius * light.radius
}

fn score(
    position: [f32; 3],
    intensity: f32,
    x: usize,
    y: usize,
    z: usize,
    geometry: &ScoreGeometry,
) -> f32 {
    let center = geometry.center(x, y, z);
    let squared = (0..3)
        .map(|axis| (center[axis] - position[axis]).powi(2))
        .sum::<f32>();
    intensity / squared.max(0.0001)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_has_no_references() {
        let view = SyntheticView::default();
        let table = ClusterTable::build(&[], &view).expect("empty table");
        assert_eq!(table.heads.len(), 8);
        assert_eq!(table.indices.len(), 4);
        assert!(table.heads.iter().all(|byte| *byte == 0));
        let disabled = ClusterTable::build(
            &[SyntheticPointLight {
                position: [0.0, 0.0, 0.5],
                radius: 1.0,
                candela: [0.0; 3],
            }],
            &view,
        )
        .expect("disabled source");
        assert_eq!(disabled.heads.len(), 8);
        assert_eq!(disabled.indices.len(), 4);
        let offscreen = ClusterTable::build(
            &[SyntheticPointLight {
                position: [4.0, 4.0, 4.0],
                radius: 0.5,
                candela: [10.0; 3],
            }],
            &view,
        )
        .expect("offscreen source");
        assert!(!offscreen.active);
        assert_eq!(offscreen.heads.len(), 8);
    }

    #[test]
    fn overlapping_sources_are_bounded_and_diagnosed() {
        let mut lights = vec![
            SyntheticPointLight {
                position: [0.0, 0.0, 0.5],
                radius: 0.5,
                candela: [1.0; 3],
            };
            CAPACITY + 1
        ];
        lights[CAPACITY].candela = [100.0; 3];
        let table = ClusterTable::build(&lights, &SyntheticView::default()).expect("overlap");
        assert!(table.dropped_references > 0);
        let mut strong_retained = false;
        for head in table.heads.as_chunks::<8>().0 {
            let offset = u32::from_le_bytes(head[..4].try_into().expect("offset"));
            let count = u32::from_le_bytes(head[4..8].try_into().expect("count"));
            assert!(count <= u32::try_from(CAPACITY).expect("small capacity"));
            let offset = usize::try_from(offset).expect("small offset") * 4;
            let count = usize::try_from(count).expect("small count");
            strong_retained |= table.indices[offset..offset + count * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .any(|item| u32::from_le_bytes(*item) == u32::try_from(CAPACITY).unwrap());
        }
        assert!(strong_retained, "strongest source must survive overflow");
    }

    #[test]
    fn rebuilt_overflow_matches_fresh_view_without_old_references() {
        let mut lights = vec![
            SyntheticPointLight {
                position: [0.0, 0.0, 0.5],
                radius: 0.5,
                candela: [1.0; 3],
            };
            CAPACITY + 1
        ];
        let view = SyntheticView::default();
        let mut reused = ClusterTable::build(&lights, &view).expect("first view");
        lights[0].position = [4.0; 3];
        lights[CAPACITY].candela = [100.0; 3];
        reused.rebuild(&lights, &view).expect("changed view");
        let fresh = ClusterTable::build(&lights, &view).expect("fresh reference");
        assert_eq!(reused.lights, fresh.lights);
        assert_eq!(reused.heads, fresh.heads);
        assert_eq!(reused.indices, fresh.indices);
        assert_eq!(reused.dropped_references, fresh.dropped_references);
    }

    #[test]
    fn initially_dark_light_activates_without_growing_cpu_buffers() {
        let mut light = SyntheticPointLight {
            position: [0.0, 0.0, 0.5],
            radius: 0.5,
            candela: [0.0; 3],
        };
        let view = SyntheticView::default();
        let mut table = ClusterTable::build(&[light], &view).expect("dark prepare");
        let capacities = [
            table.slots.capacity(),
            table.lights.capacity(),
            table.heads.capacity(),
            table.indices.capacity(),
        ];
        light.candela = [10.0; 3];
        table.rebuild(&[light], &view).expect("activate light");
        assert!(table.active);
        assert_eq!(
            capacities,
            [
                table.slots.capacity(),
                table.lights.capacity(),
                table.heads.capacity(),
                table.indices.capacity(),
            ]
        );
    }

    #[test]
    fn gpu_bounds_repack_has_fixed_capacity_and_counts_excess_sources() {
        let view = SyntheticView::default();
        let mut light = SyntheticPointLight {
            position: [0.0, 0.0, 0.5],
            radius: 0.5,
            candela: [0.0; 3],
        };
        let mut table = ClusterTable::build_gpu(&[light], &view).expect("GPU source");
        assert!(table.slots.is_empty());
        assert_eq!(table.bounds.len(), 32);
        assert_eq!(table.heads.len(), 8);
        light.candela = [2.0; 3];
        table
            .repack_gpu(&[light, light], &view)
            .expect("bounded frame");
        assert_eq!(table.dropped_sources, 1);
        assert!(table.active);
        assert_eq!(table.lights.len(), 32);
        assert_eq!(table.bounds.len(), 32);
        assert_ne!(table.bounds[..4], u32::MAX.to_le_bytes());
    }

    #[test]
    fn malformed_light_refuses_before_gpu_preparation() {
        let light = SyntheticPointLight {
            position: [f32::NAN, 0.0, 0.5],
            radius: 1.0,
            candela: [1.0; 3],
        };
        assert!(matches!(
            ClusterTable::build(&[light], &SyntheticView::default()),
            Err(RenderError::InvalidLighting)
        ));
    }

    #[test]
    fn perspective_translated_camera_keeps_strong_nearby_light_on_overflow() {
        let view = SyntheticView::perspective(
            [100.0, 0.0, 2.0],
            [100.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("translated view");
        let mut lights = vec![
            SyntheticPointLight {
                position: [0.0, 0.0, 1.0],
                radius: 128.0,
                candela: [1.0; 3],
            };
            CAPACITY
        ];
        lights.push(SyntheticPointLight {
            position: [100.0, 0.0, 1.0],
            radius: 1.0,
            candela: [100.0; 3],
        });
        let table = ClusterTable::build(&lights, &view).expect("valid clustered view");
        assert!(table.dropped_references > 0);
        let slot = (depth_index(1.0, 0.1, 20.0) * Y + Y / 2) * X + X / 2;
        let head = &table.heads[slot * 8..slot * 8 + 8];
        let offset = u32::from_le_bytes(head[..4].try_into().expect("offset")) as usize;
        let count = u32::from_le_bytes(head[4..].try_into().expect("count")) as usize;
        assert_eq!(count, CAPACITY);
        assert!(
            table.indices[offset * 4..(offset + count) * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .any(|item| {
                    u32::from_le_bytes(*item) == u32::try_from(CAPACITY).expect("small capacity")
                })
        );
    }

    #[test]
    fn perspective_bounds_cover_translated_rotated_and_near_plane_samples() {
        let cases = [
            ([5.0, 1.0, 5.0], [0.0, 0.0, 0.0], [0.1, 0.0, 0.2], 0.4),
            ([0.0, 0.0, 2.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.85], 0.2),
        ];
        for (eye, target, position, radius) in cases {
            let camera =
                SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], 1.0, 1.0, 0.1, 20.0)
                    .expect("camera");
            let light = SyntheticPointLight {
                position,
                radius,
                candela: [1.0; 3],
            };
            let (lo, hi) = bounds(light, &camera).expect("visible sphere");
            let Projection::Perspective {
                view,
                fx,
                fy,
                near,
                far,
            } = camera.projection()
            else {
                panic!("perspective fixture");
            };
            let p = view.transform_point3(Vec3::from_array(position));
            let d = -p.z;
            let cell = [
                ndc_index(p.x * fx / d, X),
                ndc_index(p.y * fy / d, Y),
                depth_index(d, near, far),
            ];
            assert!((0..3).all(|axis| (lo[axis]..=hi[axis]).contains(&cell[axis])));
        }
    }
}
