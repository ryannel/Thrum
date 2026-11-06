//! Independent direct-light quadrature in metres. GPU LUTs use the same
//! declared coefficients, but their sampling/mapping code is separate.

use glam::Vec3;

const RADIUS: f32 = 6_371_000.0;
const TOP: f32 = 100_000.0;

pub(super) fn sun_transmittance(height: f32, mu: f32) -> [f32; 3] {
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
    let mut depth = Vec3::ZERO;
    let step = distance / 64.0;
    for index in 0..64 {
        let t = (index as f32 + 0.5) * step;
        let numerator = height * (2.0 * RADIUS + height) + t * (2.0 * r * mu + t);
        let h = numerator / ((RADIUS * RADIUS + numerator).sqrt() + RADIUS);
        let rayleigh = Vec3::new(5.802e-6, 13.558e-6, 33.100e-6) * (-h / 8_000.0).exp();
        let mie = Vec3::splat(4.4e-6 * (-h / 1_200.0).exp());
        let ozone = Vec3::new(0.650e-6, 1.881e-6, 0.085e-6)
            * (1.0 - (h - 25_000.0).abs() / 15_000.0).max(0.0);
        depth += (rayleigh + mie + ozone) * step;
    }
    (-depth).exp().to_array()
}
