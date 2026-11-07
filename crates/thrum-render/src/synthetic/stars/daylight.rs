//! Conservative omission of the SDR star composite in bright incident light.

use super::super::{LIGHT_BYTES, Projection, SyntheticView};

const BIN_SIDE: usize = 34;
const BIN_SCALE: f64 = 16.0;
const BIN_OFFSET: f64 = 17.0;
const SUPPORTED_CHORD: f64 = 1.0 / (4.0 * BIN_SCALE);
pub(super) const ANGULAR_SIGMA_DEGREES: f64 = 0.045;
const ANGULAR_SIGMA: f64 = ANGULAR_SIGMA_DEGREES.to_radians();
pub(super) const PIXEL_SIGMA: f64 = 0.7;
pub(super) const MAX_CENTRE_NDC: f64 = 2.0;
pub(super) const RADIUS_SIGMAS: f64 = 4.0;
pub(super) const PIXEL_HALF_WIDTH: f64 = 0.5;

pub(super) struct DaylightBound {
    inverse_at_one_code: f64,
    direct_lux: f32,
}

impl DaylightBound {
    pub(super) fn new(records: &[[f32; 4]], light: &[u8; LIGHT_BYTES]) -> Self {
        let peak = local_flux_bound(records) / (std::f64::consts::TAU * ANGULAR_SIGMA.powi(2));
        // Positive spectral weights bound each transmission channel by 1.5.
        // Round-to-nearest additive fp16 can double a positive increment.
        // Neutral Highlights has a 1.15 bound for positive RGB changes.
        // sRGB adds at most 12.92, with 1% slack for shader arithmetic.
        let inverse_at_one_code = peak * 1.5 * 2.0 * 1.15 * 12.92 * 255.0 * 1.01;
        Self {
            inverse_at_one_code,
            direct_lux: direct_horizontal_lux(light),
        }
    }

    pub(super) fn update(&mut self, light: &[u8; LIGHT_BYTES]) {
        self.direct_lux = direct_horizontal_lux(light);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the normalized fade is packed as a shader f32"
    )]
    pub(super) fn strength(&self, view: &SyntheticView, extent: [u32; 2]) -> f32 {
        let Some(inverse_floor) = view.star_camera_inverse_floor(self.direct_lux) else {
            return 1.0;
        };
        if inverse_floor <= self.inverse_at_one_code || !bounded_footprint(view, extent) {
            return 1.0;
        }
        // Fade before dropping the graph pass. The undithered change is less
        // than one code. Dither headroom can add one code, for at most two
        // integer codes after quantization.
        let t = ((inverse_floor / self.inverse_at_one_code - 1.0) / 0.05).clamp(0.0, 1.0);
        (1.0 - t * t * (3.0 - 2.0 * t)) as f32
    }
}

fn direct_horizontal_lux(light: &[u8; LIGHT_BYTES]) -> f32 {
    let words = light.as_chunks::<4>().0;
    let vertical = f32::from_le_bytes(words[1]).max(0.0);
    let rgb = glam::Vec3::from_array([4, 5, 6].map(|index| f32::from_le_bytes(words[index])));
    vertical * rgb.dot(glam::Vec3::new(0.2126, 0.7152, 0.0722))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "checked directions produce nonnegative coordinates strictly below the grid extent"
)]
fn local_flux_bound(records: &[[f32; 4]]) -> f64 {
    // Shift each axis independently by zero or half a cell, giving eight
    // grids. One grid encloses the entire query footprint in a single cell.
    let mut bins = vec![0.0_f64; BIN_SIDE.pow(3)];
    let mut max = 0.0_f64;
    for grid in 0..8 {
        bins.fill(0.0);
        for &[x, y, z, lux] in records {
            let key = |value: f32, axis| {
                let shift = if grid & (1 << axis) == 0 { 0.0 } else { 0.5 };
                (f64::from(value) * BIN_SCALE + BIN_OFFSET + shift) as usize
            };
            let index = (key(x, 0) * BIN_SIDE + key(y, 1)) * BIN_SIDE + key(z, 2);
            bins[index] += f64::from(lux);
            max = max.max(bins[index]);
        }
    }
    max
}

fn bounded_footprint(view: &SyntheticView, [width, height]: [u32; 2]) -> bool {
    let Projection::Perspective { fx, fy, .. } = view.projection() else {
        return false;
    };
    let x = f64::from(fx.recip());
    let y = f64::from(fy.recip());
    let dx = 2.0 * x / f64::from(width);
    let dy = 2.0 * y / f64::from(height);
    let pixel_product = (dx * dy).sqrt();
    let sigma = |x: f64, y: f64| {
        let norm = (1.0 + x * x + y * y).sqrt();
        (ANGULAR_SIGMA * norm * norm.sqrt() / pixel_product).max(PIXEL_SIGMA)
    };
    // The shader rejects centres beyond 2 NDC and quads outside the viewport.
    // Normalization of (slope_x, slope_y, 1) cannot increase chord length.
    let max_x = MAX_CENTRE_NDC * x;
    let max_y = MAX_CENTRE_NDC * y;
    let radius = RADIUS_SIGMAS * sigma(max_x, max_y) + PIXEL_HALF_WIDTH;
    let s = sigma((x + dx * radius).min(max_x), (y + dy * radius).min(max_y));
    let chord = (RADIUS_SIGMAS * s + PIXEL_HALF_WIDTH) * dx.hypot(dy);
    // Include f32 projection and checked catalogue/basis norm tolerances.
    s <= 32.0 && chord + 0.001 < SUPPORTED_CHORD
}
