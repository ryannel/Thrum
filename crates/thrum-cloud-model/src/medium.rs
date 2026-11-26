use crate::{Conformance, Error, Sample, Shape, State, Weather, smooth};

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
    /// Disc-averaged droplet phase and the broader beam phase, in sr⁻¹.
    pub phases: [f64; 2],
    /// Depth toward the Sun and depth through the whole Sun column.
    pub sun_depths: [f64; 2],
    /// Droplet phase's second Legendre moment.
    pub forward_share: f64,
    /// Broad lobe weight of the droplet phase.
    pub broad_weight: f64,
    /// Cosines of the lit plane's normal with the Sun and the viewer.
    pub plane_cosines: [f64; 2],
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
    /// Cloud light before adding incident sunlight or a view boundary.
    pub fn coefficients(self, extinction: f64) -> Coefficients {
        let [sun, column] = self.sun_depths;
        let [incidence, viewer] = self.plane_cosines;
        let reduced = 1.0 - self.forward_share;
        let beam = (-self.broad_weight.max(reduced) * sun).exp();
        let remaining = (-reduced * sun).exp();
        let beam = self.phases[0] * beam + self.phases[1] * (remaining - beam).max(0.0);

        let reduced_cosine = (1.0 - self.mean_cosine) * incidence;
        let x = reduced_cosine * sun;
        let column = column.max(sun);
        let escape = 3.0 / 7.0 * (1.0 + 2.0 * incidence);
        let transmitted = ((escape + (1.0 - escape) * (-reduced * column).exp())
            / (1.0 + 0.75 * reduced_cosine * column))
            .min(1.0);
        let mean = (incidence / (2.0 * std::f64::consts::PI)
            * (1.0 - transmitted * (1.0 + 1.5 * x) + (2.0 * escape - 1.0) * (1.0 - remaining)))
            .max(0.0);
        let flow = 3.0 * incidence / (4.0 * std::f64::consts::PI) * (transmitted - remaining);
        let kernel = beam + (mean - self.mean_cosine * flow * viewer).max(0.0);
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
    /// f64 medium coefficients for the coarse density, including column erosion.
    /// Call `weighted` with the near share before supplying a view-volume cell.
    pub fn medium<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        volume: impl Fn(u64) -> Option<&'v Shape>,
        light: Light,
    ) -> Coefficients {
        light.coefficients(self.density(point, state, crate::Density::Coarse, volume))
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
