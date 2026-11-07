//! Frozen SplitMix64 integer mapping, from Vigna's public-domain numerical
//! algorithm (prng.di.unimi.it/splitmix.c). Independent Rust implementation.
//! Seed contract: separate batch/path labels and separate path/shadow streams;
//! open uniforms use the upper 52 bits plus 0.5, scaled by 2^-52.

use super::{Budget, Unresolved, geometry::Vec3};
use std::f64::consts::PI;

pub(super) struct Rng {
    state: u64,
}
impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub(super) fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.state)
    }
    pub(super) fn open(&mut self) -> f64 {
        uniform(self.next())
    }
    pub(super) fn exponential(&mut self, rate: f64) -> f64 {
        -self.open().ln() / rate
    }
}
pub(super) fn uniform(word: u64) -> f64 {
    ((word >> 12) as f64 + 0.5) / 4_503_599_627_370_496.0
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(super) struct Streams {
    pub path: Rng,
    pub shadow: Rng,
}
impl Streams {
    pub(super) fn new(seed: u64, batch: usize, path: usize) -> Self {
        let batch_seed = mix(seed ^ mix(batch as u64 ^ 0x4241_5443_485f_3031));
        let sample_seed = mix(batch_seed ^ mix(path as u64 ^ 0x5341_4d50_4c45_3031));
        Self {
            path: Rng::new(mix(sample_seed ^ 0x5041_5448_5f52_4e47)),
            shadow: Rng::new(mix(sample_seed ^ 0x5348_4144_4f57_524e)),
        }
    }
}

pub(super) fn rayleigh(mu: f64) -> f64 {
    3.0 * (1.0 + mu * mu) / (16.0 * PI)
}
pub(super) fn hg(mu: f64) -> f64 {
    0.36 / (4.0 * PI * (1.64 - 1.6 * mu).powf(1.5))
}

fn around(axis: Vec3, mu: f64, phi: f64) -> Vec3 {
    let helper = if axis.y.abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let x = axis.cross(helper).unit();
    let y = axis.cross(x);
    let radius = (1.0 - mu * mu).max(0.0).sqrt();
    (axis * mu + x * (radius * phi.cos()) + y * (radius * phi.sin())).unit()
}

pub(super) fn phase(
    axis: Vec3,
    mie: bool,
    rng: &mut Rng,
    budget: &mut Budget,
) -> Result<Vec3, Unresolved> {
    let mu = if mie {
        let ratio = 0.36 / (0.2 + 1.6 * rng.open());
        ((1.64 - ratio * ratio) / 1.6).clamp(-1.0, 1.0)
    } else {
        loop {
            budget.spend()?;
            let candidate = 2.0 * rng.open() - 1.0;
            if rng.open() <= 1.0_f64.midpoint(candidate * candidate) {
                break candidate;
            }
        }
    };
    // axis is the current BACKWARD ray direction. This is intentionally not
    // PBRT's both-directions-outward convention or its opposite HG sign.
    Ok(around(axis, mu, 2.0 * PI * rng.open()))
}

pub(super) fn cosine(normal: Vec3, rng: &mut Rng) -> Vec3 {
    around(normal, rng.open().sqrt(), 2.0 * PI * rng.open())
}
