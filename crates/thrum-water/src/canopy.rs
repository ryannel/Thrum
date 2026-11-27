//! Compact static lookup for canopy columns scattered across a large world.
use thrum_water_model::CanopyGrid;

pub(super) const COLUMNS_WGSL: &str = include_str!("canopy.wgsl");

pub(super) struct SparseColumns {
    pub(super) code: i32,
    pub(super) texels: Vec<[f32; 4]>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "canopy thresholds are projected to the GPU coordinate lattice"
)]
pub(super) fn ceiling(height: f64) -> f32 {
    let nearest = height as f32;
    if f64::from(nearest) < height {
        nearest.next_up()
    } else {
        nearest
    }
}

#[expect(clippy::cast_possible_truncation, reason = "GPU transmission fraction")]
pub(super) fn layers([[a, b], [c, d]]: [[f64; 2]; 2]) -> [f32; 4] {
    [ceiling(a), b as f32, ceiling(c), d as f32]
}

#[expect(clippy::cast_possible_truncation, reason = "canopy GPU coordinates")]
pub(super) fn origin_parts(grid: &CanopyGrid) -> ([f32; 2], [f32; 2]) {
    if grid.sparse_cells.is_empty() {
        return (grid.origin.map(|value| value as f32), [0.0; 2]);
    }
    let cells = grid.origin.map(|value| (value / grid.cell).floor());
    let fraction = std::array::from_fn(|axis| (grid.origin[axis] - cells[axis] * grid.cell) as f32);
    (fraction, cells.map(|value| value as f32))
}

impl SparseColumns {
    pub(super) fn pack(grid: &CanopyGrid) -> Option<Self> {
        if grid.sparse_cells.len() != grid.columns.len() {
            return None;
        }
        let length = grid
            .columns
            .len()
            .checked_mul(2)?
            .checked_next_power_of_two()?;
        let mask = u32::try_from(length - 1).ok()?;
        // Header values and cell indices are stored in exactly representable texels.
        if length >= 1 << 24 {
            return None;
        }
        // A single opaque canopy can use the key and the only active layer
        // together. The two-layer path retains a full second texel.
        let single = grid.columns.iter().all(|column| column[1][1] == 1.0);
        let stride = if single { 1 } else { 2 };
        let mut texels = vec![[0.0; 4]; length.checked_mul(stride)?];
        for (&[x, z], &column) in grid.sparse_cells.iter().zip(&grid.columns) {
            if x as usize >= grid.size[0]
                || z as usize >= grid.size[1]
                || x >= (1 << 24) - 1
                || z >= 1 << 24
            {
                return None;
            }
            let mut slot = (x.wrapping_mul(0x8da6_b343) ^ z.wrapping_mul(0xd816_3841)) & mask;
            loop {
                let index = slot as usize * stride;
                let entry = texels[index];
                let key = [(x + 1) as f32, z as f32];
                if entry[0] == 0.0 {
                    let packed = layers(column);
                    texels[index] = [key[0], key[1], packed[0], packed[1]];
                    if !single {
                        texels[index + 1] = packed;
                    }
                    break;
                }
                if entry[..2] == key {
                    return None;
                }
                slot = (slot + 1) & mask;
            }
        }
        let mask = i32::try_from(mask).ok()?;
        Some(Self {
            code: if single { -mask } else { mask },
            texels,
        })
    }
}
