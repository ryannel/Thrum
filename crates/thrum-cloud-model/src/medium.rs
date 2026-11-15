use crate::{Conformance, Error, Orders, PhaseTable, Sample, Shape, State, Weather, smooth};

/// Local medium coefficients, before multiplying by true Sun light.
#[derive(Clone, Copy, Debug, Default)]
pub struct Coefficients {
    /// Extinction, m⁻¹.
    pub extinction: f64,
    /// Scattering coefficient times phase and self-shadow, m⁻¹ sr⁻¹.
    pub solar_kernel: f64,
    /// Diffuse ambient source per metre.
    pub ambient_source: f64,
}

impl Coefficients {
    /// Scale all three coefficients by the same partition weight.
    #[must_use]
    pub fn weighted(self, weight: f64) -> Self {
        Self {
            extinction: self.extinction * weight,
            solar_kernel: self.solar_kernel * weight,
            ambient_source: self.ambient_source * weight,
        }
    }
}

/// Light inputs evaluated independently of the camera's near/far partition.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    /// Four phase values from `Orders::phases`, evaluated once per ray.
    pub phases: [f64; 4],
    /// Whole condensate optical depth toward the Sun.
    pub sun_depth: f64,
    /// Vertical depths above and below for the diffuse two-stream term.
    pub vertical_depths: [f64; 2],
    /// Sky and ground hemispheric source radiances, with no cloud feedback.
    /// For isotropic scattering, incident hemisphere irradiance E contributes
    /// E / (2π); perform that conversion once when preparing these inputs.
    pub ambient: [f64; 2],
    /// Droplet mean cosine.
    pub mean_cosine: f64,
}

impl Light {
    /// Cloud light's four orders without incident sunlight; same weights,
    /// extinction scales and phase table as the existing optical reference.
    pub fn coefficients(self, extinction: f64) -> Coefficients {
        let kernel: f64 = (0..4)
            .map(|i| {
                Orders::WEIGHTS[i]
                    * (-Orders::EXTINCTION[i] * self.sun_depth).exp()
                    * self.phases[i]
            })
            .sum();
        let ambient: f64 = (0..2)
            .map(|i| {
                self.ambient[i] / (1.0 + 0.75 * (1.0 - self.mean_cosine) * self.vertical_depths[i])
            })
            .sum();
        Coefficients {
            extinction,
            solar_kernel: extinction * kernel,
            ambient_source: extinction * ambient,
        }
    }
}

impl Orders {
    /// Four f64 phase references. The first order reads the prepared disc table;
    /// later orders use the existing HG asymmetries, without another phase model.
    pub fn phases(table: &PhaseTable, sin_half_squared: f64) -> [f64; 4] {
        let mut values = [0.0; 4];
        values[0] = table.reference(sin_half_squared);
        for (i, &g) in Self::LATER_G.iter().enumerate() {
            values[i + 1] = (1.0 - g * g)
                / (4.0
                    * std::f64::consts::PI
                    * ((1.0 - g).powi(2) + 4.0 * g * sin_half_squared.clamp(0.0, 1.0)).powf(1.5));
        }
        values
    }
}

/// Validated near/far ramp in metres from the camera.
#[derive(Clone, Copy, Debug)]
pub struct Handover {
    start: f64,
    end: f64,
}

impl Handover {
    /// Construct an ordered nonnegative interval.
    /// # Errors
    /// Rejects nonfinite, negative or empty ramps.
    pub fn new(start: f64, end: f64) -> Result<Self, Error> {
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return Err(Error::Invalid);
        }
        Ok(Self { start, end })
    }
    /// Near and far shares, whose sum is exactly one.
    pub fn weights(self, distance: f64) -> [f64; 2] {
        let far = smooth((distance - self.start) / (self.end - self.start));
        [1.0 - far, far]
    }
}

impl Weather<'_> {
    /// f64 medium coefficients for the same coarse density as the point query.
    /// Call `weighted` with the near share before supplying a view-volume cell.
    pub fn medium<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        volume: impl Fn(u64) -> Option<&'v Shape>,
        light: Light,
    ) -> Coefficients {
        light.coefficients(self.extinction(point, state, volume).value)
    }

    /// Evaluate mixed droplets without assigning a fog row the cloud type's
    /// phase. The caller prepares each source's light from its own droplet and
    /// phase table, with the same whole-field Sun path for every source.
    pub fn medium_sources<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        volume: impl Fn(u64) -> Option<&'v Shape>,
        cloud_light: Light,
        ground_light: impl Fn(usize) -> Light,
    ) -> Coefficients {
        let mut c = cloud_light.coefficients(self.field.density(
            point,
            state,
            crate::Density::Coarse,
            volume,
            1.0,
        ));
        if let (Some(surface), Some(ground)) = (self.surface, self.ground) {
            for (i, _) in ground.rows.iter().enumerate() {
                let fog = ground_light(i).coefficients(ground.row_density(
                    i,
                    surface,
                    self.field,
                    point,
                    state,
                    crate::Density::Coarse,
                ));
                c.extinction += fog.extinction;
                c.solar_kernel += fog.solar_kernel;
                c.ambient_source += fog.ambient_source;
            }
        }
        c
    }
}

impl Conformance {
    /// Compare all three medium coefficients through the same sample/provider
    /// seam as weather columns, returning the worst absolute coefficient error.
    /// # Errors
    /// Rejects nonfinite reference/provider coefficients.
    pub fn medium(
        samples: &[Sample],
        reference: impl Fn(&Sample) -> Coefficients,
        provider: impl Fn(&Sample) -> Coefficients,
    ) -> Result<Self, Error> {
        let mut result = Self::default();
        for (index, sample) in samples.iter().enumerate() {
            let a = reference(sample);
            let b = provider(sample);
            for (x, y) in [a.extinction, a.solar_kernel, a.ambient_source]
                .into_iter()
                .zip([b.extinction, b.solar_kernel, b.ambient_source])
            {
                if !x.is_finite() || !y.is_finite() {
                    return Err(Error::Invalid);
                }
                if (x - y).abs() > result.maximum {
                    result = Self {
                        maximum: (x - y).abs(),
                        index,
                    };
                }
            }
        }
        Ok(result)
    }
}
