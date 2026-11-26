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
    pub(super) heights: [f32; 4],
    pub(super) carriage: [f32; 4],
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
        height_rms: f32,
        heights: [f32; 4],
        deep_scatter: [f32; 3],
        carriage: [f32; 4],
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
            scatter: [
                deep_scatter[0],
                deep_scatter[1],
                deep_scatter[2],
                height_rms,
            ],
            heights,
            carriage,
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "significant heights are prepared once in GPU metres"
)]
pub(super) fn layer_heights(spectrum: &Spectrum) -> [f32; 4] {
    let long = spectrum
        .long
        .iter()
        .map(|c| 0.5 * c.amplitude.powi(2))
        .sum::<f64>();
    [
        long,
        spectrum.cascades[0].variance(),
        spectrum.cascades[1].variance(),
        spectrum.cascades[2].variance(),
    ]
    .map(|v| (4.0 * v.sqrt()) as f32)
}
#[expect(
    clippy::cast_possible_truncation,
    reason = "the GPU crest mask uses the finite f32 RMS height, prepared once"
)]
pub(super) fn height_rms(spectrum: &Spectrum) -> f32 {
    spectrum.variance().sqrt() as f32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the bounded spectrum moments are sent to the shader in f32"
)]
pub(super) fn jacobian_variance(spectrum: &Spectrum, choppiness: f64) -> [f32; 4] {
    // In the small-displacement limit the determinant is one plus the
    // Jacobian trace. A travelling mode's trace has amplitude q * k * a.
    [
        spectrum.long.as_slice(),
        spectrum.cascades[0].components(),
        spectrum.cascades[1].components(),
        spectrum.cascades[2].components(),
    ]
    .map(|band| {
        band.iter()
            .map(|mode| {
                let [x, z] = mode.wave_vector;
                0.5 * (choppiness * mode.amplitude).powi(2) * (x * x + z * z)
            })
            .sum::<f64>() as f32
    })
}

// The display clock is reduced in f64 before it is sent to the shader.
pub(super) fn carriage(seconds: f64, period: Option<std::time::Duration>) -> [f32; 4] {
    let period = period.unwrap_or(std::time::Duration::from_secs(4));
    let elapsed_periods = seconds / period.as_secs_f64();
    let phase = elapsed_periods.rem_euclid(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the foam seed is an exact small integer"
    )]
    let cycle = elapsed_periods.floor().rem_euclid(256.0) as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "unit interval shader phase"
    )]
    let phase = phase as f32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the continuous cascade phase uses shader time in seconds"
    )]
    let seconds = seconds as f32;
    [phase, period.as_secs_f32(), cycle, seconds]
}
