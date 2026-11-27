//! Incident light at a lens crossing the sampled water surface.

/// RGB transmission to the meter. Signed depth is positive under water and
/// downwelling coefficients are inverse metres. Surface transmission is the
/// optical power fraction entering the water at zero depth.
///
/// The submerged fraction models the lens as a vertical 10 cm aperture. The
/// attenuation distance uses the centre of its underwater portion. At and
/// below 5 cm depth this is the lens centre itself.
#[must_use]
pub fn meter_light_factor(
    signed_depth: f64,
    downwelling: [f64; 3],
    surface_transmission: f64,
) -> [f64; 3] {
    const HALF_APERTURE: f64 = 0.05;
    let wet = ((signed_depth + HALF_APERTURE) / (2.0 * HALF_APERTURE)).clamp(0.0, 1.0);
    if wet == 0.0 {
        return [1.0; 3];
    }
    let depth = (signed_depth - HALF_APERTURE)
        .max(0.0)
        .midpoint(signed_depth + HALF_APERTURE);
    downwelling.map(|rate| {
        let transmitted = surface_transmission * (-rate * depth).exp();
        wet.mul_add(transmitted - 1.0, 1.0)
    })
}
