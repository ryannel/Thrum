use thrum_sea::Ambient;
use thrum_sea_model::Spectrum;
use thrum_water_model::Row;
#[cfg(test)]
mod tests;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Parameters {
    pub(super) level_dt_reset: [f32; 4],
    pub(super) gains: [f32; 4],
    pub(super) lengths: [f32; 4],
    pub(super) cache: [[f32; 4]; 8],
    pub(super) long_covariance: [[f32; 4]; 8],
    pub(super) scatter: [f32; 4],
}
#[expect(
    clippy::cast_possible_truncation,
    reason = "nonnegative slope variances are prepared once for the GPU's f32 BRDF"
)]
pub(super) fn long_variances(spectrum: &Spectrum) -> [[f32; 4]; 8] {
    std::array::from_fn(|level| {
        let cutoff = std::f64::consts::PI / f64::from(1_u32 << level);
        let mut covariance = [0.0_f64; 4];
        for c in spectrum
            .long
            .iter()
            .filter(|c| c.wave_vector[0].hypot(c.wave_vector[1]) > cutoff)
        {
            let [x, z] = c.wave_vector;
            let a = 0.5 * c.amplitude.powi(2);
            for (sum, value) in covariance
                .iter_mut()
                .zip([a * x * x, a * x * z, a * z * z, 0.0])
            {
                *sum += value;
            }
        }
        covariance.map(|v| v as f32)
    })
}
impl Parameters {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "GPU inputs are bounded metre and second quantities"
    )]
    pub(super) fn new(
        row: Row,
        spectrum: &Spectrum,
        ambient: &Ambient,
        dt: f64,
        reset: bool,
        previous_level: f64,
        long_variances: [[f32; 4]; 8],
        deep_scatter: [f32; 3],
    ) -> Self {
        Self {
            level_dt_reset: [
                row.level as f32,
                dt as f32,
                f32::from(reset),
                previous_level as f32,
            ],
            gains: row.gains.map(|v| v as f32),
            lengths: [
                spectrum.cascades[0].length() as f32,
                spectrum.cascades[1].length() as f32,
                spectrum.cascades[2].length() as f32,
                spectrum.tail_slope_variance as f32,
            ],
            cache: std::array::from_fn(|i| {
                let l = ambient.levels[i];
                [l.origin[0] as f32, l.origin[1] as f32, l.texel as f32, 0.0]
            }),
            long_covariance: long_variances,
            scatter: [deep_scatter[0], deep_scatter[1], deep_scatter[2], 0.0],
        }
    }
}
