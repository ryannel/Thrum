//! Mip-zero reference for the GPU's two repeat-sampled, unorm8 volumes.
use crate::Field;
use std::cell::Cell;

/// Period in metres of the shape volume.
pub const SHAPE_PERIOD: f64 = 6400.0;
/// Period in metres of the detail volume.
pub const DETAIL_PERIOD: f64 = 800.0;
/// Shape volume extent on each axis.
pub const SHAPE_SIZE: u32 = 128;
/// Detail volume extent on each axis.
pub const DETAIL_SIZE: u32 = 64;

const SHAPE_CACHE_SIZE: usize = 1024;
thread_local! {
    // Rays repeatedly interpolate the same neighbouring texels. Cache only
    // immutable hash results, keyed by both seed and wrapped texel index.
    static SHAPE_TEXELS: [Cell<(u64, f64)>; SHAPE_CACHE_SIZE] =
        const { [const { Cell::new((u64::MAX, 0.0)) }; SHAPE_CACHE_SIZE] };
}

impl Field {
    pub(crate) fn shape_noise(&self, point: [f64; 3]) -> f64 {
        SHAPE_TEXELS.with(|cache| {
            volume(point, SHAPE_PERIOD, SHAPE_SIZE, |texel, uv| {
                let width = u64::from(SHAPE_SIZE);
                let [x, y, z] = texel.map(u64::from);
                let index = (z * width + y) * width + x;
                let key = (u64::from(self.seed) << 32) | index;
                let hash = (key ^ (key >> 17)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
                let at = usize::try_from(hash >> (u64::BITS - SHAPE_CACHE_SIZE.ilog2()))
                    .expect("bounded cache slot");
                let (previous, value) = cache[at].get();
                if previous == key {
                    return value;
                }
                let sum = 0.55 * self.billow(uv, 8, 50)
                    + 0.30 * self.billow(uv, 20, 53)
                    + 0.15 * self.billow(uv, 50, 56);
                let value = ((sum - 0.22) / 0.56).clamp(0.0, 1.0);
                cache[at].set((key, value));
                value
            })
        })
    }

    pub(crate) fn detail_noise(&self, point: [f64; 3]) -> f64 {
        volume(point, DETAIL_PERIOD, DETAIL_SIZE, |_, uv| {
            let (site, f) = lattice(uv.map(|x| x * 32.0));
            interpolate(f.map(crate::smooth), |offset| {
                let [x, y, z] = std::array::from_fn(|i| (site[i] + offset[i]).rem_euclid(32));
                self.random(x, y, z, 41)
            })
        })
    }

    fn billow(&self, uv: [f64; 3], period: i64, stream: u32) -> f64 {
        let (site, local) = lattice(uv.map(|x| x * period as f64));
        let mut nearest: f64 = 9.0;
        for z in -1..=1 {
            for y in -1..=1 {
                for x in -1..=1 {
                    let offset = [x, y, z];
                    let [i, j, k] =
                        std::array::from_fn(|axis| (site[axis] + offset[axis]).rem_euclid(period));
                    let mut distance = 0.0;
                    for axis in 0..3 {
                        let centre = self.random(i, j, k, stream + [0, 1, 2][axis]);
                        let delta = offset[axis] as f64 + centre - local[axis];
                        distance += delta * delta;
                    }
                    nearest = nearest.min(distance);
                }
            }
        }
        (1.0 - nearest.sqrt()).max(0.0)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "wrapped texel coordinates are below the u32 texture size"
)]
fn volume(
    point: [f64; 3],
    period: f64,
    size: u32,
    texel: impl Fn([u32; 3], [f64; 3]) -> f64,
) -> f64 {
    let size_f = f64::from(size);
    let (site, local) = lattice(point.map(|x| (x.rem_euclid(period) / period) * size_f - 0.5));
    interpolate(local, |offset| {
        let wrapped =
            std::array::from_fn(|i| (site[i] + offset[i]).rem_euclid(i64::from(size)) as u32);
        let uv = wrapped.map(|x| (f64::from(x) + 0.5) / size_f);
        (texel(wrapped, uv) * 255.0).round() / 255.0
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "coordinates are bounded lattice positions"
)]
fn lattice(position: [f64; 3]) -> ([i64; 3], [f64; 3]) {
    (
        position.map(|x| x.floor() as i64),
        position.map(|x| x - x.floor()),
    )
}

fn interpolate(f: [f64; 3], texel: impl Fn([i64; 3]) -> f64) -> f64 {
    let mut value = 0.0;
    for z in 0..=1 {
        for y in 0..=1 {
            for x in 0..=1 {
                let offset = [x, y, z];
                let weight = (0..3)
                    .map(|i| if offset[i] == 0 { 1.0 - f[i] } else { f[i] })
                    .product::<f64>();
                value += weight * texel(offset);
            }
        }
    }
    value
}
