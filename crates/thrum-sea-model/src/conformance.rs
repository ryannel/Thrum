use crate::{LongWaves, Sample, SeaError};

/// A provider evaluated at physical points and ticks (not camera coordinates).
pub trait SeaProvider {
    /// Answer at a point and tick, or report unsupported input/state.
    /// # Errors
    /// Returns the provider's validation or convergence error.
    fn answer(&self, point: [f64; 2], tick: u64) -> Result<Sample, SeaError>;
}

/// A long-wave model at an explicit tick duration, for conformance comparisons.
pub struct Conformance<'a> {
    /// Reference surface.
    pub waves: &'a LongWaves,
    /// Duration of a simulation tick in seconds.
    pub tick_seconds: f64,
}

impl SeaProvider for Conformance<'_> {
    fn answer(&self, point: [f64; 2], tick: u64) -> Result<Sample, SeaError> {
        if !self.tick_seconds.is_finite() || self.tick_seconds <= 0.0 {
            return Err(SeaError::InvalidInput);
        }
        self.waves.evaluate(point, tick as f64 * self.tick_seconds)
    }
}

/// Largest absolute height error and per-axis slope/velocity errors.
/// Each maximum is independent and ranges over the entire stated Cartesian set.
///
/// # Errors
/// Rejects an empty set, propagates provider errors and rejects nonfinite answers.
pub fn compare(
    reference: &impl SeaProvider,
    provider: &impl SeaProvider,
    points: &[[f64; 2]],
    ticks: &[u64],
) -> Result<Sample, SeaError> {
    if points.is_empty() || ticks.is_empty() {
        return Err(SeaError::InvalidInput);
    }
    let mut max = Sample::default();
    for &tick in ticks {
        for &point in points {
            let a = reference.answer(point, tick)?;
            let b = provider.answer(point, tick)?;
            if [a.height, a.height_dt, b.height, b.height_dt]
                .into_iter()
                .chain(a.slope)
                .chain(b.slope)
                .chain(a.velocity)
                .chain(b.velocity)
                .any(|v| !v.is_finite())
            {
                return Err(SeaError::InvalidInput);
            }
            max.height = max.height.max((a.height - b.height).abs());
            for axis in 0..2 {
                max.slope[axis] = max.slope[axis].max((a.slope[axis] - b.slope[axis]).abs());
            }
            for axis in 0..3 {
                max.velocity[axis] =
                    max.velocity[axis].max((a.velocity[axis] - b.velocity[axis]).abs());
            }
            max.height_dt = max.height_dt.max((a.height_dt - b.height_dt).abs());
        }
    }
    Ok(max)
}
