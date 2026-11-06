use super::{
    Budget, Reason, Single, Unresolved,
    geometry::{self, Vec3},
    quadrature::{self, Settings},
    random::{self, Rng, Streams},
};
use std::f64::consts::PI;

/// Finite interaction queries are named observables, never the default oracle.
#[derive(Clone, Copy)]
pub(super) enum Query {
    AllOrders,
    ThroughInteractions(usize),
}

#[derive(Clone, Copy)]
pub(super) struct TraceOptions {
    pub query: Query,
    pub roulette_survival: f64,
}
impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            query: Query::AllOrders,
            roulette_survival: 0.8,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SingleSettings {
    pub optical_depth: Settings,
    pub radiance: Settings,
    /// Initial panels per geometric interval, for independent convergence checks.
    pub radiance_subdivisions: u32,
}
impl Default for SingleSettings {
    fn default() -> Self {
        Self {
            optical_depth: Settings::default(),
            radiance: Settings {
                absolute: 1e-14,
                ..Settings::default()
            },
            radiance_subdivisions: 1,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Medium {
    pub rayleigh: f64,
    pub mie_scattering: f64,
    pub mie_extinction: f64,
    pub ozone: f64,
    pub ground_albedo: f64,
}
#[derive(Clone, Copy)]
pub(super) struct Coefficients {
    pub(super) rayleigh: f64,
    pub(super) mie: f64,
    pub(super) extinction: f64,
}
impl Medium {
    pub(super) fn band(band: usize) -> Self {
        Self {
            rayleigh: [5.802e-6, 13.558e-6, 33.1e-6][band],
            mie_scattering: 3.996e-6,
            mie_extinction: 4.4e-6,
            ozone: [0.650e-6, 1.881e-6, 0.085e-6][band],
            ground_albedo: 0.1,
        }
    }
    fn majorant(self) -> f64 {
        self.rayleigh + self.mie_extinction + self.ozone
    }
    pub(super) fn at(self, point: Vec3) -> Coefficients {
        let height = (point.length() - geometry::RADIUS).max(0.0);
        let rayleigh = self.rayleigh * (-height / 8000.0).exp();
        let mie_density = (-height / 1200.0).exp();
        Coefficients {
            rayleigh,
            mie: self.mie_scattering * mie_density,
            extinction: rayleigh
                + self.mie_extinction * mie_density
                + self.ozone * (1.0 - (height - 25_000.0).abs() / 15_000.0).max(0.0),
        }
    }
}

pub(super) fn optical_depth(
    origin: Vec3,
    direction: Vec3,
    length: f64,
    medium: Medium,
    settings: Settings,
) -> Result<f64, Unresolved> {
    Ok(quadrature::integrate(
        &geometry::splits(origin, direction, length, None),
        settings,
        |distance| Ok(medium.at(origin + direction * distance).extinction),
    )?
    .value)
}

pub(super) fn exact_sun(
    point: Vec3,
    sun: Vec3,
    medium: Medium,
    settings: Settings,
) -> Result<f64, Unresolved> {
    let boundary = geometry::boundary(point, sun);
    if boundary.ground {
        return Ok(0.0);
    }
    Ok((-optical_depth(point, sun, boundary.distance, medium, settings)?).exp())
}

pub(super) fn single(
    origin: Vec3,
    sun: Vec3,
    direction: Vec3,
    medium: Medium,
    settings: SingleSettings,
) -> Result<Single, Unresolved> {
    let boundary = geometry::boundary(origin, direction);
    let mu = direction.dot(sun).clamp(-1.0, 1.0);
    if !(1..=64).contains(&settings.radiance_subdivisions) {
        return Err(Unresolved::new(Reason::InvalidCase));
    }
    let geometric = geometry::splits(origin, direction, boundary.distance, Some(sun));
    let mut panels = Vec::new();
    for interval in geometric.windows(2) {
        for index in 0..settings.radiance_subdivisions {
            panels.push(
                interval[0]
                    + (interval[1] - interval[0]) * f64::from(index)
                        / f64::from(settings.radiance_subdivisions),
            );
        }
    }
    panels.push(boundary.distance);
    let integrated = quadrature::integrate(&panels, settings.radiance, |distance| {
        let point = origin + direction * distance;
        let coefficients = medium.at(point);
        let view_t =
            (-optical_depth(origin, direction, distance, medium, settings.optical_depth)?).exp();
        Ok(view_t
            * exact_sun(point, sun, medium, settings.optical_depth)?
            * (coefficients.rayleigh * random::rayleigh(mu) + coefficients.mie * random::hg(mu)))
    })?;
    let ground = if boundary.ground {
        let point = (origin + direction * boundary.distance).unit() * geometry::RADIUS;
        (-optical_depth(
            origin,
            direction,
            boundary.distance,
            medium,
            settings.optical_depth,
        )?)
        .exp()
            * medium.ground_albedo
            / PI
            * point.unit().dot(sun).max(0.0)
            * exact_sun(point, sun, medium, settings.optical_depth)?
    } else {
        0.0
    };
    Ok(Single {
        volume: integrated.value,
        unscattered_ground: ground,
        radiance: integrated.value + ground,
        estimated_error: integrated.error,
    })
}

/// Unbiased ratio tracking, with its own RNG stream. No deterministic optical
/// depth is substituted into the all-orders estimator's next-event connections.
pub(super) fn ratio_transmittance(
    length: f64,
    majorant: f64,
    mut extinction: impl FnMut(f64) -> f64,
    rng: &mut Rng,
    budget: &mut Budget,
) -> Result<f64, Unresolved> {
    if majorant == 0.0 {
        return Ok(1.0);
    }
    let mut distance = 0.0;
    let mut product = 1.0;
    loop {
        budget.spend()?;
        distance += rng.exponential(majorant);
        if distance >= length {
            return Ok(product);
        }
        let sigma = extinction(distance);
        if !(0.0..=majorant).contains(&sigma) {
            return Err(Unresolved::new(Reason::MajorantViolation));
        }
        product *= 1.0 - sigma / majorant;
        if product == 0.0 {
            return Ok(0.0);
        }
    }
}

pub(super) fn sampled_sun(
    point: Vec3,
    sun: Vec3,
    medium: Medium,
    rng: &mut Rng,
    budget: &mut Budget,
) -> Result<f64, Unresolved> {
    let boundary = geometry::boundary(point, sun);
    if boundary.ground {
        return Ok(0.0);
    }
    ratio_transmittance(
        boundary.distance,
        medium.majorant(),
        |distance| medium.at(point + sun * distance).extinction,
        rng,
        budget,
    )
}

#[derive(Default)]
pub(super) struct PathValue {
    pub orders: [f64; 6],
    pub atmosphere: f64,
    pub ground: f64,
    pub ground_vertices: f64,
    pub direct_volume: f64,
    pub one_ground_vertex: f64,
    pub one_ground_volume: f64,
    pub volume_order: usize,
    pub largest_event: f64,
    pub interactions: usize,
}
impl PathValue {
    fn add(
        &mut self,
        value: f64,
        ground_count: usize,
        ground_vertex: bool,
    ) -> Result<(), Unresolved> {
        if !value.is_finite() || value < 0.0 {
            return Err(Unresolved::new(Reason::NonFinite));
        }
        self.orders[self.volume_order.min(5)] += value;
        if ground_count > 0 {
            self.ground += value;
        } else {
            self.atmosphere += value;
        }
        if ground_vertex {
            self.ground_vertices += value;
        } else if self.volume_order == 1 && ground_count == 0 {
            self.direct_volume += value;
        }
        if ground_count == 1 {
            if ground_vertex {
                self.one_ground_vertex += value;
            } else {
                self.one_ground_volume += value;
            }
        }
        self.largest_event = self.largest_event.max(value);
        if self
            .orders
            .iter()
            .chain([
                &self.atmosphere,
                &self.ground,
                &self.ground_vertices,
                &self.direct_volume,
                &self.one_ground_vertex,
                &self.one_ground_volume,
            ])
            .any(|sum| !sum.is_finite())
        {
            return Err(Unresolved::new(Reason::NonFinite));
        }
        Ok(())
    }
}

enum Hit {
    Space,
    Ground(Vec3),
    Medium(Vec3, Coefficients),
}
fn walk(
    origin: Vec3,
    direction: Vec3,
    medium: Medium,
    rng: &mut Rng,
    budget: &mut Budget,
) -> Result<Hit, Unresolved> {
    let boundary = geometry::boundary(origin, direction);
    let majorant = medium.majorant();
    let mut distance = 0.0;
    if majorant > 0.0 {
        loop {
            budget.spend()?;
            distance += rng.exponential(majorant);
            if distance >= boundary.distance {
                break;
            }
            let point = origin + direction * distance;
            let coefficients = medium.at(point);
            if !(0.0..=majorant).contains(&coefficients.extinction) {
                return Err(Unresolved::new(Reason::MajorantViolation));
            }
            if rng.open() * majorant < coefficients.extinction {
                return Ok(Hit::Medium(point, coefficients));
            }
        }
    }
    Ok(if boundary.ground {
        Hit::Ground((origin + direction * boundary.distance).unit() * geometry::RADIUS)
    } else {
        Hit::Space
    })
}

pub(super) fn trace(
    mut origin: Vec3,
    sun: Vec3,
    mut direction: Vec3,
    medium: Medium,
    rng: &mut Streams,
    budget: &mut Budget,
    options: TraceOptions,
) -> Result<PathValue, Unresolved> {
    let mut result = PathValue::default();
    let mut weight = 1.0;
    let mut ground_count = 0;
    if !options.roulette_survival.is_finite()
        || !(0.0..=1.0).contains(&options.roulette_survival)
        || options.roulette_survival == 0.0
    {
        return Err(Unresolved::new(Reason::InvalidCase));
    }
    if matches!(options.query, Query::ThroughInteractions(0)) {
        return Ok(result);
    }
    loop {
        let hit = walk(origin, direction, medium, &mut rng.path, budget)?;
        if matches!(hit, Hit::Space) {
            return Ok(result);
        }
        result.interactions += 1;
        match hit {
            Hit::Medium(point, coefficients) => {
                result.volume_order += 1;
                let mu = direction.dot(sun).clamp(-1.0, 1.0);
                let illumination = sampled_sun(point, sun, medium, &mut rng.shadow, budget)?;
                result.add(
                    weight
                        * illumination
                        * (coefficients.rayleigh * random::rayleigh(mu)
                            + coefficients.mie * random::hg(mu))
                        / coefficients.extinction,
                    ground_count,
                    false,
                )?;
                let scattering = coefficients.rayleigh + coefficients.mie;
                if scattering == 0.0 {
                    return Ok(result);
                }
                weight *= scattering / coefficients.extinction;
                let mie = rng.path.open() * scattering >= coefficients.rayleigh;
                direction = random::phase(direction, mie, &mut rng.path, budget)?;
                origin = point;
            }
            Hit::Ground(point) => {
                ground_count += 1;
                let normal = point.unit();
                let illumination = sampled_sun(point, sun, medium, &mut rng.shadow, budget)?;
                result.add(
                    weight * medium.ground_albedo / PI * normal.dot(sun).max(0.0) * illumination,
                    ground_count,
                    true,
                )?;
                weight *= medium.ground_albedo;
                direction = random::cosine(normal, &mut rng.path);
                origin = point;
            }
            Hit::Space => unreachable!("space returned before counting an interaction"),
        }
        if !weight.is_finite() {
            return Err(Unresolved::new(Reason::NonFinite));
        }
        if weight == 0.0 {
            return Ok(result);
        }
        if matches!(options.query, Query::ThroughInteractions(limit) if result.interactions >= limit)
        {
            return Ok(result);
        }
        // AllOrders has no bounce truncation. A finite query deliberately asks
        // for a prefix, while every roulette survivor keeps reciprocal weight.
        if result.interactions >= 4
            && !roulette(&mut weight, options.roulette_survival, &mut rng.path)
        {
            return Ok(result);
        }
    }
}

/// The draw is consumed even at survival1, keeping matched-stream prefixes equal.
pub(super) fn roulette(weight: &mut f64, survival: f64, rng: &mut Rng) -> bool {
    if rng.open() >= survival {
        return false;
    }
    *weight /= survival;
    true
}
