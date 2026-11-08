//! The dense flythrough field: a ground slab, walls and 100k props over an area
//! far larger than the view, so camera culling has work to save. One layout
//! seeds the simulation and sizes the prepared instances.

use std::f32::consts::{PI, TAU};

use thrum_render::abi::MaterialRecord;

/// Slot 0 is the ground; walls follow; props fill the rest.
pub(crate) const WALL_COUNT: usize = 512;
/// Props on a lattice; the last four are the controlled peers.
pub(crate) const PROP_COUNT: usize = 100_000;
pub(crate) const ENTITY_COUNT: usize = 1 + WALL_COUNT + PROP_COUNT;

const COLUMNS: usize = 316;
const SPACING_M: f32 = 2.5;
const PROP_HALF_M: [f32; 2] = [0.2, 0.6];
const WALL_HALF_M: [f32; 3] = [10.0, 3.0, 0.5];
const GROUND_HALF_M: f32 = 420.0;
/// Every twentieth prop drifts, so history and interpolation are exercised.
const MOVING_EVERY: usize = 20;
const DRIFT_M_PER_S: f64 = 0.8;

const GROUND: u64 = 0;
const WALL: u64 = 1;
const PROPS: [u64; 3] = [2, 3, 4];

/// One seeded entity: authoritative position, velocity and material, plus the
/// cold oriented box the renderer is prepared with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FieldEntity {
    pub(crate) position: [f64; 3],
    pub(crate) velocity: [f64; 3],
    pub(crate) material: u64,
    /// Box half-extents along its local axes, in metres.
    pub(crate) half_extent: [f32; 3],
    /// Rotation about +Y, in radians.
    pub(crate) yaw: f32,
}

fn mix(index: usize, salt: u64) -> u64 {
    // SplitMix64 finaliser: a deterministic per-index hash, not a random stream.
    let mut z = (index as u64 ^ salt).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A hash-chosen index below `count`.
pub(crate) fn pick(index: usize, salt: u64, count: usize) -> usize {
    let count = u64::try_from(count).expect("small count");
    usize::try_from(mix(index, salt) % count).expect("below a usize count")
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a 24-bit hash fraction is exact in f32"
)]
fn unit(index: usize, salt: u64) -> f32 {
    (mix(index, salt) >> 40) as f32 / (1u64 << 24) as f32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "lattice coordinates are small integers"
)]
fn lattice(column: usize, row: usize) -> [f32; 2] {
    let rows = PROP_COUNT.div_ceil(COLUMNS);
    [
        (column as f32 - (COLUMNS - 1) as f32 * 0.5) * SPACING_M,
        (row as f32 - (rows - 1) as f32 * 0.5) * SPACING_M,
    ]
}

/// The deterministic layout of one slot.
///
/// # Panics
/// For an index at or beyond [`ENTITY_COUNT`].
pub(crate) fn entity(index: usize) -> FieldEntity {
    assert!(index < ENTITY_COUNT, "field slot {index} out of range");
    if index == 0 {
        return FieldEntity {
            position: [0.0, -0.5, 0.0],
            velocity: [0.0; 3],
            material: GROUND,
            half_extent: [GROUND_HALF_M, 0.5, GROUND_HALF_M],
            yaw: 0.0,
        };
    }
    if index <= WALL_COUNT {
        // Walls sit between prop rows, so static props never intersect them.
        let rows = PROP_COUNT.div_ceil(COLUMNS);
        let column = pick(index, 1, COLUMNS);
        let row = pick(index, 2, rows - 1);
        let [x, z] = lattice(column, row);
        let across = mix(index, 3) & 1 == 1;
        let (x, z, yaw) = if across {
            (x + SPACING_M * 0.5, z, PI * 0.5)
        } else {
            (x, z + SPACING_M * 0.5, 0.0)
        };
        return FieldEntity {
            position: [f64::from(x), f64::from(WALL_HALF_M[1]), f64::from(z)],
            velocity: [0.0; 3],
            material: WALL,
            half_extent: WALL_HALF_M,
            yaw,
        };
    }
    let prop = index - 1 - WALL_COUNT;
    let [x, z] = lattice(prop % COLUMNS, prop / COLUMNS);
    let half = PROP_HALF_M[0] + (PROP_HALF_M[1] - PROP_HALF_M[0]) * unit(index, 4);
    let height = half * (0.6 + 1.4 * unit(index, 5));
    FieldEntity {
        position: [f64::from(x), f64::from(height), f64::from(z)],
        velocity: if prop.is_multiple_of(MOVING_EVERY) {
            [DRIFT_M_PER_S, 0.0, 0.0]
        } else {
            [0.0; 3]
        },
        material: PROPS[pick(index, 6, PROPS.len())],
        half_extent: [half, height, half],
        yaw: unit(index, 7) * TAU,
    }
}

/// Material rows indexed by [`FieldEntity::material`].
pub(crate) fn materials() -> Vec<MaterialRecord> {
    [
        ([0.20, 0.22, 0.17, 1.0], [1.0, 0.9, 0.0, 1.0]),
        ([0.42, 0.40, 0.37, 1.0], [1.0, 0.8, 0.0, 1.0]),
        ([0.55, 0.18, 0.10, 1.0], [1.0, 0.5, 0.0, 1.0]),
        ([0.12, 0.30, 0.55, 1.0], [1.0, 0.35, 0.0, 1.0]),
        ([0.65, 0.58, 0.30, 1.0], [1.0, 0.6, 0.0, 1.0]),
    ]
    .into_iter()
    .map(|(base_rgba, surface)| MaterialRecord {
        base_rgba,
        surface,
        ..Default::default()
    })
    .collect()
}

/// Column-major object basis (three columns) and its inverse-transpose normal
/// basis for an oriented box.
pub(crate) fn basis(entity: &FieldEntity) -> ([[f32; 3]; 3], [[f32; 3]; 3]) {
    let (sin, cos) = entity.yaw.sin_cos();
    let axes = [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]];
    let mut current = [[0.0; 3]; 3];
    let mut normal = [[0.0; 3]; 3];
    for (axis, column) in axes.into_iter().enumerate() {
        for (row, value) in column.into_iter().enumerate() {
            current[axis][row] = value * entity.half_extent[axis];
            normal[axis][row] = value / entity.half_extent[axis];
        }
    }
    (current, normal)
}

/// Ticks in one loop of the flythrough (60 s at 60 Hz).
pub(crate) const PATH_TICKS: u64 = 3600;
const PATH_RADIUS_M: f32 = 250.0;
/// Within each 600-tick block the camera snaps to look backwards for 120 ticks.
const TURN_BLOCK: u64 = 600;
const TURN_START: u64 = 300;
const TURN_TICKS: u64 = 120;
/// Every 1800 ticks the camera cuts a quarter of the way round the loop.
const CUT_BLOCK: u64 = 1800;

/// One flythrough camera sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FieldCamera {
    pub(crate) eye: [f32; 3],
    /// Heading in the XZ plane, forward = (cos, 0, sin), before the player's yaw.
    pub(crate) heading: f32,
    /// Camera continuity changes on a cut or snap turn and remains visible
    /// to the renderer even when the first sample in a segment is dropped.
    pub(crate) history_epoch: u64,
}

/// The scripted flythrough at `tick` and presentation `alpha`.
#[expect(
    clippy::cast_precision_loss,
    reason = "tick phases are reduced modulo small loop lengths first"
)]
pub(crate) fn camera(tick: u64, alpha: f32) -> FieldCamera {
    let step = tick.saturating_sub(1);
    let cuts = step / CUT_BLOCK;
    let phase = ((step % PATH_TICKS) as f32 + alpha) / PATH_TICKS as f32 * TAU
        + (cuts % 4) as f32 * (TAU * 0.25);
    let (sin, cos) = phase.sin_cos();
    let height = 3.5 + 2.5 * (phase * 3.0).sin();
    let eye = [PATH_RADIUS_M * cos, height, PATH_RADIUS_M * sin];
    // Counter-clockwise travel: forward (cos h, 0, sin h) is the tangent.
    let mut heading = phase + PI * 0.5;
    let in_block = step % TURN_BLOCK;
    if (TURN_START..TURN_START + TURN_TICKS).contains(&in_block) {
        heading += PI;
    }
    let turns = 2 * (step / TURN_BLOCK)
        + u64::from(in_block >= TURN_START)
        + u64::from(in_block >= TURN_START + TURN_TICKS);
    FieldCamera {
        eye,
        heading,
        history_epoch: cuts + turns,
    }
}

#[cfg(test)]
mod tests;
