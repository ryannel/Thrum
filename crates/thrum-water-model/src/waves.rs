use crate::{Map, MapSample, Profile, Row, WaterError};
use thrum_sea_model::{Forcing, LongWaves, Sample};

/// Fetch-limited gains relative to the world's exposed spectrum.
/// Swell contributes only to the denominator, so shelter never creates swell energy.
/// Use `[1; 4]` for the exposed sea itself.
/// # Errors
/// Refuses invalid forcing or fetch through the sea spectrum's validation.
pub fn shelter(forcing: &Forcing, fetch: f64) -> Result<[f64; 4], WaterError> {
    let field = forcing
        .band_variances()
        .map_err(|_| WaterError::new("waves", "invalid forcing"))?;
    let local = Forcing {
        fetch,
        swells: Vec::new(),
        ..forcing.clone()
    }
    .band_variances()
    .map_err(|_| WaterError::new("waves", "invalid fetch"))?;
    let (a, b) = (local, field);
    Ok(std::array::from_fn(|i| {
        if b[i] > 0.0 {
            (a[i] / b[i]).sqrt().min(1.0)
        } else {
            0.0
        }
    }))
}

/// The shared wave field's four CPU evaluators; caller owns physical time.
pub struct Field {
    /// Long waves followed by three short-wave bands.
    pub layers: [LongWaves; 4],
}

impl Field {
    /// Layer variances from the actual component powers this tick.
    pub fn variances(&self) -> [f64; 4] {
        self.layers.each_ref().map(|l| {
            l.components()
                .iter()
                .map(|c| 0.5 * c.amplitude.powi(2))
                .sum()
        })
    }
}

/// Depth-limited amplitude gains and the lost variance used by foam births.
#[derive(Clone, Copy, Debug)]
pub struct Limited {
    /// Final gains, zero outside wave support.
    pub gains: [f64; 4],
    /// Removed height variance by layer, square metres.
    pub lost_power: [f64; 4],
    /// Derived at this row, never cached per body.
    pub swash_depth: f64,
}

/// Saturated significant-height cap, applied longest first to each layer.
/// Variances and row gains are nonnegative physical inputs from the wave field.
pub fn limit(row: Row, depth: f64, variances: [f64; 4], profile: Profile) -> Limited {
    let variance: f64 = (0..4).map(|i| variances[i] * row.gains[i].powi(2)).sum();
    let swash_depth = (profile.swash_ratio * 4.0 * variance.sqrt()).min(profile.margin_height);
    let cap = 0.6 * (depth + swash_depth).max(0.0);
    let gains = std::array::from_fn(|i| {
        let hs = 4.0 * variances[i].sqrt();
        if hs > 0.0 {
            row.gains[i].min(cap / hs)
        } else {
            row.gains[i]
        }
    });
    Limited {
        gains,
        lost_power: std::array::from_fn(|i| {
            variances[i] * (row.gains[i].powi(2) - gains[i].powi(2))
        }),
        swash_depth,
    }
}

/// Complete query answer. A named margin can be dry now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Surface {
    /// Owning body, including dry shore margins.
    pub body: Option<usize>,
    /// Surface height, slope, material velocity and Eulerian height rate.
    pub sample: Sample,
    /// Still depth, negative on the margin and infinite off the bed.
    pub depth: f64,
    /// Bulk x/z current.
    pub current: [f64; 2],
    /// Surface stands above ground at this time.
    pub wet: bool,
}

impl Map {
    /// Evaluate the map and the wave field at a physical point and time.
    /// # Errors
    /// Rejects nonfinite input or a nonconvergent sea evaluator.
    pub fn surface(
        &self,
        field: &Field,
        point: [f64; 2],
        time: f64,
    ) -> Result<Surface, WaterError> {
        if !time.is_finite() || point.iter().any(|v| !v.is_finite()) {
            return Err(WaterError::new("surface", "nonfinite point or time"));
        }
        self.sample(point).map_or(Ok(Surface::default()), |map| {
            self.surface_on_row(field, point, time, map)
        })
    }

    /// Evaluate a supplied map row; the same path serves both sides of chart ends.
    /// # Errors
    /// Propagates invalid time, point or nonconvergent waves.
    pub fn surface_on_row(
        &self,
        field: &Field,
        point: [f64; 2],
        time: f64,
        map: MapSample,
    ) -> Result<Surface, WaterError> {
        if let Some(surf) = &self.surf
            && map.bed.is_some()
        {
            let mut out = surf.surface(self, field, point, time, map)?;
            self.add_rain(field, point, time, map, &mut out.sample);
            return Ok(out);
        }
        self.plain_surface_on_row(field, point, time, map)
    }

    pub(crate) fn plain_surface_on_row(
        &self,
        field: &Field,
        point: [f64; 2],
        time: f64,
        map: MapSample,
    ) -> Result<Surface, WaterError> {
        let gains = limit(map.row, map.depth, field.variances(), self.profile).gains;
        let mut sample = Sample {
            height: map.row.level,
            slope: self.level_slope(map.body, point),
            velocity: [map.row.current[0], 0.0, map.row.current[1]],
            ..Sample::default()
        };
        for (i, layer) in field.layers.iter().enumerate() {
            let wave = layer
                .scaled(gains[i])
                .and_then(|l| l.evaluate(point, time))
                .map_err(|_| WaterError::new("surface", "invalid or nonconvergent waves"))?;
            sample.height += wave.height;
            sample.height_dt += wave.height_dt;
            for (a, b) in sample.slope.iter_mut().zip(wave.slope) {
                *a += b;
            }
            for (a, b) in sample.velocity.iter_mut().zip(wave.velocity) {
                *a += b;
            }
        }
        self.add_rain(field, point, time, map, &mut sample);
        Ok(Surface {
            body: Some(map.body),
            sample,
            depth: map.depth,
            current: map.row.current,
            wet: map.bed.is_none_or(|bed| sample.height > bed),
        })
    }
}
