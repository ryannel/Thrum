use crate::{Surface, WaterError};

/// Any physical query implementation, including a GPU readback adapter.
pub trait Provider {
    /// Answer at a physical point and tick.
    /// # Errors
    /// Reports provider validation or availability failures.
    fn answer(&self, point: [f64; 2], tick: u64) -> Result<Surface, WaterError>;
}

impl<F> Provider for F
where
    F: Fn([f64; 2], u64) -> Result<Surface, WaterError>,
{
    fn answer(&self, p: [f64; 2], t: u64) -> Result<Surface, WaterError> {
        self(p, t)
    }
}

/// Maxima over the stated Cartesian point/tick set, with all support disagreements.
#[derive(Debug, Default)]
pub struct Conformance {
    /// Largest absolute errors in height, slope components, velocity and height rate.
    pub surface: thrum_sea_model::Sample,
    /// Largest depth error.
    pub depth: f64,
    /// Largest bulk current error per axis.
    pub current: [f64; 2],
    /// Every (point, tick) whose body differs.
    pub body_mismatches: Vec<([f64; 2], u64)>,
    /// Every (point, tick) whose wet support differs.
    pub wet_mismatches: Vec<([f64; 2], u64)>,
}

/// Compare any provider to the oracle without tolerances hiding disagreements.
/// # Errors
/// Refuses empty sets, nonfinite answers and provider failures. Equal infinite depths
/// off the bed are valid; unequal finite/infinite depths report infinite error.
pub fn compare(
    reference: &impl Provider,
    provider: &impl Provider,
    points: &[[f64; 2]],
    ticks: &[u64],
) -> Result<Conformance, WaterError> {
    if points.is_empty() || ticks.is_empty() {
        return Err(WaterError::new("conformance", "empty sample set"));
    }
    let mut out = Conformance::default();
    for &tick in ticks {
        for &point in points {
            let a = reference.answer(point, tick)?;
            let b = provider.answer(point, tick)?;
            for s in [a, b] {
                if [s.sample.height, s.sample.height_dt]
                    .into_iter()
                    .chain(s.sample.slope)
                    .chain(s.sample.velocity)
                    .chain(s.current)
                    .any(|v| !v.is_finite())
                    || s.depth.is_nan()
                {
                    return Err(WaterError::new("conformance", "nonfinite answer"));
                }
            }
            out.surface.height = out
                .surface
                .height
                .max((a.sample.height - b.sample.height).abs());
            out.surface.height_dt = out
                .surface
                .height_dt
                .max((a.sample.height_dt - b.sample.height_dt).abs());
            for i in 0..2 {
                out.surface.slope[i] =
                    out.surface.slope[i].max((a.sample.slope[i] - b.sample.slope[i]).abs());
                out.current[i] = out.current[i].max((a.current[i] - b.current[i]).abs());
            }
            for i in 0..3 {
                out.surface.velocity[i] = out.surface.velocity[i]
                    .max((a.sample.velocity[i] - b.sample.velocity[i]).abs());
            }
            out.depth = out.depth.max(
                if a.depth.is_infinite()
                    && a.depth.is_sign_positive()
                    && b.depth.is_infinite()
                    && b.depth.is_sign_positive()
                {
                    0.0
                } else {
                    (a.depth - b.depth).abs()
                },
            );
            if a.body != b.body {
                out.body_mismatches.push((point, tick));
            }
            if a.wet != b.wet {
                out.wet_mismatches.push((point, tick));
            }
        }
    }
    Ok(out)
}
