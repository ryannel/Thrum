//! Compact, deterministic 3D light lists for the fixed orthographic fixture.
//! The source registry is external; these are prepared per-view GPU products.

use super::RenderError;
use std::cmp::Ordering;

pub(super) const X: usize = 32;
pub(super) const Y: usize = 18;
pub(super) const Z: usize = 16;
const COUNT: usize = X * Y * Z;
const CAPACITY: usize = 32;

/// One finite-radius local source in the synthetic fixture. Position is in
/// the fixture's world coordinates and intensity is linear RGB candela.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticPointLight {
    /// Position relative to the fixed orthographic camera.
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
    pub(super) dropped_references: usize,
    pub(super) active: bool,
}

impl ClusterTable {
    fn empty() -> Self {
        Self {
            lights: vec![0; 32],
            heads: vec![0; 8],
            indices: vec![0; 4],
            dropped_references: 0,
            active: false,
        }
    }

    pub(super) fn build(lights: &[SyntheticPointLight]) -> Result<Self, RenderError> {
        if lights.len() > 4096 {
            return Err(RenderError::SceneCapacity);
        }
        for light in lights.iter().copied() {
            validate(light)?;
        }
        if !lights
            .iter()
            .any(|light| light.candela.iter().any(|value| *value > 0.0) && intersects_view(*light))
        {
            // The shader's uniform off-state returns before reading clusters.
            // Keep valid minimal bindings without preparing a dead 3D grid.
            return Ok(Self::empty());
        }
        let mut slots = vec![Slot::default(); COUNT];
        let mut light_bytes = Vec::with_capacity(lights.len().max(1) * 32);
        let mut dropped = 0;
        for (index, light) in lights.iter().copied().enumerate() {
            pack_light(light, &mut light_bytes);
            if light.candela.iter().all(|value| *value == 0.0) {
                continue;
            }
            let Some((lo, hi)) = bounds(light) else {
                continue;
            };
            let index = u32::try_from(index).map_err(|_| RenderError::SceneCapacity)?;
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        if !intersects(light, x, y, z) {
                            continue;
                        }
                        let slot = &mut slots[(z * Y + y) * X + x];
                        let candidate = Candidate {
                            index,
                            score: score(light, x, y, z),
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
        if light_bytes.is_empty() {
            light_bytes.resize(32, 0);
        }
        let mut heads = Vec::with_capacity(COUNT * 8);
        let mut indices = Vec::new();
        for slot in &mut slots {
            let len = usize::from(slot.len);
            slot.candidates[..len].sort_unstable_by_key(|candidate| candidate.index);
            let offset =
                u32::try_from(indices.len() / 4).map_err(|_| RenderError::SceneCapacity)?;
            heads.extend_from_slice(&offset.to_le_bytes());
            heads.extend_from_slice(&u32::from(slot.len).to_le_bytes());
            for candidate in &slot.candidates[..len] {
                indices.extend_from_slice(&candidate.index.to_le_bytes());
            }
        }
        if indices.is_empty() {
            return Ok(Self::empty());
        }
        Ok(Self {
            lights: light_bytes,
            heads,
            indices,
            dropped_references: dropped,
            active: true,
        })
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

fn bounds(light: SyntheticPointLight) -> Option<([usize; 3], [usize; 3])> {
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

fn score(light: SyntheticPointLight, x: usize, y: usize, z: usize) -> f32 {
    let center = [
        -1.0 + 2.0 * (x as f32 + 0.5) / X as f32,
        -1.0 + 2.0 * (y as f32 + 0.5) / Y as f32,
        (z as f32 + 0.5) / Z as f32,
    ];
    let squared = (0..3)
        .map(|axis| (center[axis] - light.position[axis]).powi(2))
        .sum::<f32>();
    light.candela.iter().copied().fold(0.0, f32::max) / squared.max(0.0001)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_has_no_references() {
        let table = ClusterTable::build(&[]).expect("empty table");
        assert_eq!(table.heads.len(), 8);
        assert_eq!(table.indices.len(), 4);
        assert!(table.heads.iter().all(|byte| *byte == 0));
        let disabled = ClusterTable::build(&[SyntheticPointLight {
            position: [0.0, 0.0, 0.5],
            radius: 1.0,
            candela: [0.0; 3],
        }])
        .expect("disabled source");
        assert_eq!(disabled.heads.len(), 8);
        assert_eq!(disabled.indices.len(), 4);
        let offscreen = ClusterTable::build(&[SyntheticPointLight {
            position: [4.0, 4.0, 4.0],
            radius: 0.5,
            candela: [10.0; 3],
        }])
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
        let table = ClusterTable::build(&lights).expect("overlap");
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
    fn malformed_light_refuses_before_gpu_preparation() {
        let light = SyntheticPointLight {
            position: [f32::NAN, 0.0, 0.5],
            radius: 1.0,
            candela: [1.0; 3],
        };
        assert!(matches!(
            ClusterTable::build(&[light]),
            Err(RenderError::InvalidLighting)
        ));
    }
}
