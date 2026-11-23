use crate::{Column, Density, Error, Field, Ground, Shape, State, Surface};

/// Aerosol extinction from clear-air visual range and gas extinction, m⁻¹.
/// # Errors
/// Rejects invalid ranges/gas or a range requiring negative aerosol.
pub fn aerosol_extinction(range: f64, gas: f64) -> Result<f64, Error> {
    if !range.is_finite()
        || !(3000.0..=300_000.0).contains(&range)
        || !gas.is_finite()
        || gas < 0.0
        || gas > 3.912 / range
    {
        return Err(Error::Invalid);
    }
    Ok(3.912 / range - gas)
}

/// Koschmieder visual range from aerosol and gas extinction, metres.
/// # Errors
/// Rejects negative/nonfinite extinction or a completely transparent medium.
pub fn visual_range(aerosol: f64, gas: f64) -> Result<f64, Error> {
    if !aerosol.is_finite()
        || !gas.is_finite()
        || aerosol < 0.0
        || gas < 0.0
        || aerosol + gas <= 0.0
    {
        return Err(Error::Invalid);
    }
    Ok(3.912 / (aerosol + gas))
}

/// Whether a query had the actual density of every placed shape it touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fidelity {
    /// All sources were known, including any touched shape's voxels.
    Exact,
    /// At least one touched shape was known only by its footprint.
    Footprint,
}

/// A physical answer and the representation from which it was evaluated.
#[derive(Clone, Copy, Debug)]
pub struct Answer {
    /// Extinction in m⁻¹ or optical depth, according to the called method.
    pub value: f64,
    /// Exact density or a footprint estimate.
    pub fidelity: Fidelity,
}

/// Condensate Sun-path optical depth and independent terrain occlusion.
#[derive(Clone, Copy, Debug)]
pub struct SunPath {
    /// Condensate only, never terrain extinction.
    pub optical_depth: Answer,
    /// Surface field intersects the ray to the weather top.
    pub surface_blocked: bool,
}

/// Borrowed evaluation context. Omit either ground input to retain the M1 field.
/// Voxel providers lend plain CPU shape data, never GPU resources.
pub struct Weather<'a> {
    /// Existing procedural and directed cloud authority.
    pub field: &'a Field,
    /// Optional world-anchored ground grid.
    pub surface: Option<&'a Surface>,
    /// Optional game-authored fog table.
    pub ground: Option<&'a Ground>,
}

impl Weather<'_> {
    /// Combined uneroded column; ground sources add their vertical integrals.
    pub fn column(&self, point: [f64; 3], state: &State) -> Column {
        let mut c = self.field.query(point, state);
        if let (Some(surface), Some(ground)) = (self.surface, self.ground) {
            for (i, r) in ground.rows.iter().enumerate() {
                let a = state.ground().amount(i, r.radiative);
                if a == 0.0 {
                    continue;
                }
                let [base, top] = ground.column(i, surface, [point[0], point[2]], a);
                if top <= base {
                    continue;
                }
                // Integrate the soft vertical profile with the exact cubic
                // primitive; the horizontal factors are independent of height.
                let depth = top - base;
                let u = (depth / r.softness).min(1.0);
                let integral = if depth >= r.softness {
                    depth - r.softness * 0.5
                } else {
                    r.softness * (u.powi(3) - 0.5 * u.powi(4))
                };
                let sigma = ground.row_density(
                    i,
                    surface,
                    self.field,
                    [point[0], base, point[2]],
                    state,
                    Density::Uneroded,
                );
                let tau = sigma / crate::smooth(depth / r.softness) * integral;
                if tau > 0.0 {
                    if c.optical_depth == 0.0 {
                        c.base = base;
                        c.top = top;
                    } else {
                        c.base = c.base.min(base);
                        c.top = c.top.max(top);
                    }
                    c.optical_depth += tau;
                    c.coverage = c.coverage.max(a);
                }
            }
        }
        c
    }

    /// One density, with optional ground sources added before attenuation.
    /// Missing placed volumes use the eroded column form, as in `Field::density`.
    pub fn density<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
        volume: impl Fn(u64) -> Option<&'v Shape>,
    ) -> f64 {
        self.density_with_footprint(point, state, fidelity, volume, false)
            .value
    }

    // Only visibility queries preserve a footprint's integral. Lighting and
    // Sun paths evaluate the full/coarse density, including column-form erosion.
    fn density_with_footprint<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        fidelity: Density,
        volume: impl Fn(u64) -> Option<&'v Shape>,
        preserve_footprint: bool,
    ) -> Answer {
        let (cloud, used_footprint) =
            if state.parameters().coverage == 0.0 && state.shapes().is_empty() {
                (0.0, false)
            } else {
                self.field.density_with_footprint(
                    point,
                    state,
                    fidelity,
                    volume,
                    1.0,
                    preserve_footprint,
                )
            };
        let value = match (self.surface, self.ground) {
            (Some(s), Some(g)) => cloud + g.density(s, self.field, point, state, fidelity),
            _ => cloud,
        };
        Answer {
            value,
            fidelity: if used_footprint {
                Fidelity::Footprint
            } else {
                Fidelity::Exact
            },
        }
    }

    /// Coarse condensate extinction at any height, and representation fidelity.
    /// Footprint-only estimates preserve the uneroded column integral.
    pub fn extinction<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        volume: impl Fn(u64) -> Option<&'v Shape>,
    ) -> Answer {
        self.density_with_footprint(point, state, Density::Coarse, volume, true)
    }

    /// Condensate plus clear-air visual range by Koschmieder (metres).
    pub fn visual_range<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        volume: impl Fn(u64) -> Option<&'v Shape>,
    ) -> Answer {
        let mut a = self.extinction(point, state, volume);
        a.value = 3.912 / (a.value + 3.912 / state.ground().visual_range());
        a
    }

    /// Midpoint f64 reference between arbitrary finite points. `steps` selects
    /// numerical quadrature accuracy; fidelity describes content, not error.
    /// # Errors
    /// Rejects nonfinite endpoints or zero sample count.
    pub fn optical_depth<'v>(
        &self,
        from: [f64; 3],
        to: [f64; 3],
        state: &State,
        steps: u32,
        density: Density,
        volume: impl Fn(u64) -> Option<&'v Shape>,
    ) -> Result<Answer, Error> {
        self.integrate(from, to, state, steps, density, volume, true)
    }

    fn integrate<'v>(
        &self,
        from: [f64; 3],
        to: [f64; 3],
        state: &State,
        steps: u32,
        density: Density,
        volume: impl Fn(u64) -> Option<&'v Shape>,
        preserve_footprint: bool,
    ) -> Result<Answer, Error> {
        if steps == 0 || !from.iter().chain(&to).all(|v| v.is_finite()) {
            return Err(Error::Invalid);
        }
        let delta: [f64; 3] = std::array::from_fn(|i| to[i] - from[i]);
        let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !length.is_finite() {
            return Err(Error::Invalid);
        }
        let mut answer = Answer {
            value: 0.0,
            fidelity: Fidelity::Exact,
        };
        for n in 0..steps {
            let t = (f64::from(n) + 0.5) / f64::from(steps);
            let p = std::array::from_fn(|i| from[i] + t * delta[i]);
            let sample =
                self.density_with_footprint(p, state, density, &volume, preserve_footprint);
            answer.value += sample.value * length / f64::from(steps);
            if sample.fidelity == Fidelity::Footprint {
                answer.fidelity = Fidelity::Footprint;
            }
        }
        Ok(answer)
    }

    /// Reference to the supplied weather top, with true azimuth and floored
    /// elevation (degrees). Surface visibility is reported apart from optical depth.
    /// # Errors
    /// Rejects nonfinite angles/top, nonpositive floor, or invalid quadrature.
    pub fn sun_path<'v>(
        &self,
        point: [f64; 3],
        state: &State,
        elevation: f64,
        azimuth: f64,
        floor: f64,
        top: f64,
        steps: u32,
        density: Density,
        volume: impl Fn(u64) -> Option<&'v Shape>,
    ) -> Result<SunPath, Error> {
        if ![elevation, azimuth, floor, top]
            .iter()
            .all(|v| v.is_finite())
            || !(0.0..=90.0).contains(&floor)
            || floor == 0.0
            || !(-90.0..=90.0).contains(&elevation)
            || top < point[1]
        {
            return Err(Error::Invalid);
        }
        let e = elevation.max(floor).to_radians();
        let length = (top - point[1]) / e.sin();
        let end = [
            point[0] + length * e.cos() * azimuth.cos(),
            top,
            point[2] + length * e.cos() * azimuth.sin(),
        ];
        let optical_depth = self.integrate(point, end, state, steps, density, volume, false)?;
        let mut surface_blocked = false;
        if let Some(s) = self.surface {
            for n in 0..steps {
                let t = (f64::from(n) + 0.5) / f64::from(steps);
                let p: [f64; 3] = std::array::from_fn(|i| point[i] + t * (end[i] - point[i]));
                surface_blocked |= p[1] < s.sample([p[0], p[2]])[0];
            }
        }
        Ok(SunPath {
            optical_depth,
            surface_blocked,
        })
    }
}
