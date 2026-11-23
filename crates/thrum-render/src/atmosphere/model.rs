//! Independent direct-light quadrature in metres. GPU LUTs use the same
//! declared coefficients, but their sampling/mapping code is separate.

use glam::Vec3;

use super::spectrum::EARTH;

use crate::shader_constants::{ATMOSPHERE_TOP as TOP, PLANET_RADIUS as RADIUS};

pub(super) fn sun_transmittance(height: f32, mu: f32) -> [f32; 3] {
    sun_transmission(height, mu, [EARTH.mie_extinction; 8])
}

pub(super) fn sun_transmittance_with_aerosol(
    height: f32,
    mu: f32,
    aerosol: super::Aerosol,
) -> [f32; 3] {
    sun_transmission(height, mu, aerosol.band_extinction())
}

fn sun_transmission(height: f32, mu: f32, aerosol: [f32; 8]) -> [f32; 3] {
    let r = RADIUS + height;
    if mu < -(height * (2.0 * RADIUS + height)).sqrt() / r {
        return [0.0; 3];
    }
    let b = r * mu;
    let c = (TOP - height) * (2.0 * RADIUS + TOP + height);
    let q = (b * b + c).sqrt();
    let distance = if mu >= 0.0 {
        c / (q + b).max(0.001)
    } else {
        q - b
    };
    let mut columns = Vec3::ZERO;
    let step = distance / 64.0;
    for index in 0..64 {
        let t = (index as f32 + 0.5) * step;
        let numerator = height * (2.0 * RADIUS + height) + t * (2.0 * r * mu + t);
        let h = numerator / ((RADIUS * RADIUS + numerator).sqrt() + RADIUS);
        columns += Vec3::new(
            (-h / EARTH.rayleigh_scale_height).exp(),
            (-h / EARTH.mie_scale_height).exp(),
            (1.0 - (h - EARTH.ozone_centre).abs() / EARTH.ozone_half_width).max(0.0),
        ) * step;
    }
    EARTH.project(std::array::from_fn(|i| {
        let depth =
            EARTH.rayleigh[i] * columns.x + aerosol[i] * columns.y + EARTH.ozone[i] * columns.z;
        (-depth).exp()
    }))
}

/// Horizon cosine/elevation and sky/ground angular spans for the fixed sky LUT.
/// This small CPU product is shared by every GPU producer and lookup consumer.
pub(super) fn sky_mapping(height: f32) -> [f32; 4] {
    let horizon_mu = -(height * (2.0 * RADIUS + height)).sqrt() / (RADIUS + height);
    let horizon_elevation = horizon_mu.asin();
    [
        horizon_mu,
        horizon_elevation,
        std::f32::consts::FRAC_PI_2 - horizon_elevation,
        std::f32::consts::FRAC_PI_2 + horizon_elevation,
    ]
}

#[cfg(test)]
mod tests;
